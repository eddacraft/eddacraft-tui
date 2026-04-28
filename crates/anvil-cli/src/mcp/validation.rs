use anvil_checks::reasoning::{ReasoningCheckConfig, run_reasoning_check};
use anvil_checks::secret::{SecretCheckConfig, SecretFinding, scan_content_with_stats};
use anvil_kernel_types::{Category, Diagnostic, DiagnosticSource, Location, Mode, Severity};

const INPUT_RULE_ID: &str = "mcp-validate-write-input";
const PRE_WRITE_MODE: &str = "pre-write";

pub struct PreWriteValidationRequest<'a> {
    pub relative_path: &'a str,
    pub content: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationBackend {
    Daemon,
    Embedded,
}

impl ValidationBackend {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Daemon => "daemon",
            Self::Embedded => "embedded",
        }
    }
}

#[derive(Debug)]
pub struct ValidationResult {
    pub backend: ValidationBackend,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationBackendFailure {
    pub code: &'static str,
    pub message: &'static str,
    pub retriable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum DaemonValidationOutcome {
    Diagnostics(Vec<Diagnostic>),
    Unavailable,
    OperationalFailure(ValidationBackendFailure),
}

pub trait DaemonValidationClient {
    fn validate_pre_write(
        &self,
        request: &PreWriteValidationRequest<'_>,
    ) -> DaemonValidationOutcome;
}

pub struct LocalDaemonValidationClient;

impl DaemonValidationClient for LocalDaemonValidationClient {
    fn validate_pre_write(
        &self,
        _request: &PreWriteValidationRequest<'_>,
    ) -> DaemonValidationOutcome {
        DaemonValidationOutcome::Unavailable
    }
}

pub fn validate_pre_write(
    request: &PreWriteValidationRequest<'_>,
    daemon: &impl DaemonValidationClient,
) -> Result<ValidationResult, ValidationBackendFailure> {
    match daemon.validate_pre_write(request) {
        DaemonValidationOutcome::Diagnostics(diagnostics) => Ok(ValidationResult {
            backend: ValidationBackend::Daemon,
            diagnostics,
        }),
        DaemonValidationOutcome::Unavailable => Ok(embedded_validate_pre_write(request)),
        DaemonValidationOutcome::OperationalFailure(failure) => Err(failure),
    }
}

fn embedded_validate_pre_write(request: &PreWriteValidationRequest<'_>) -> ValidationResult {
    let mut diagnostics = Vec::new();
    let secret_config = SecretCheckConfig::default();
    let (secret_findings, secret_stats) =
        scan_content_with_stats(request.content, request.relative_path, &secret_config);
    diagnostics.extend(secret_findings_to_diagnostics(
        request.relative_path,
        &secret_findings,
    ));

    if secret_stats.lines_skipped_oversize > 0 {
        diagnostics.push(input_diagnostic(
            "oversize-line-skipped",
            "Anvil skipped an oversize line while validating the proposed write.",
            request.relative_path,
        ));
    }

    let reasoning = run_reasoning_check(
        &[(request.relative_path, request.content)],
        &ReasoningCheckConfig::default(),
    );
    diagnostics.extend(reasoning.findings.into_iter().map(with_pre_write_mode));

    ValidationResult {
        backend: ValidationBackend::Embedded,
        diagnostics,
    }
}

fn secret_findings_to_diagnostics(path: &str, findings: &[SecretFinding]) -> Vec<Diagnostic> {
    findings
        .iter()
        .map(|finding| {
            Diagnostic::new(
                format!(
                    "diag_prewrite_{}_{}_{}",
                    sanitise_id_part(path),
                    finding.line,
                    sanitise_id_part(&finding.pattern_name)
                ),
                Severity::Error,
                format!("Potential secret detected ({})", finding.pattern_name),
                Location {
                    file: path.to_string(),
                    line: u32::try_from(finding.line).ok(),
                    column: None,
                    end_line: None,
                    end_column: None,
                },
                Category::Secret,
                DiagnosticSource {
                    rule_id: "secret-detection".to_string(),
                    source_module: "anvil-checks::secret".to_string(),
                },
                Mode::Unknown(PRE_WRITE_MODE.to_string()),
            )
            .with_remediation_hint("Use a placeholder or environment variable instead.")
        })
        .collect()
}

fn input_diagnostic(code: &'static str, message: &'static str, path: &str) -> Diagnostic {
    Diagnostic::new(
        format!(
            "diag_prewrite_{}_{}",
            sanitise_id_part(path),
            sanitise_id_part(code)
        ),
        Severity::Error,
        message,
        Location {
            file: path.to_string(),
            line: None,
            column: None,
            end_line: None,
            end_column: None,
        },
        Category::Policy,
        DiagnosticSource {
            rule_id: INPUT_RULE_ID.to_string(),
            source_module: "anvil-cli::mcp".to_string(),
        },
        Mode::Unknown(PRE_WRITE_MODE.to_string()),
    )
}

fn with_pre_write_mode(mut diagnostic: Diagnostic) -> Diagnostic {
    diagnostic.mode = Mode::Unknown(PRE_WRITE_MODE.to_string());
    diagnostic
}

fn sanitise_id_part(value: &str) -> String {
    let sanitised = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();

    if sanitised.is_empty() {
        "unknown".to_string()
    } else {
        sanitised
    }
}

#[cfg(test)]
mod tests {
    use super::{DaemonValidationClient, DaemonValidationOutcome, PreWriteValidationRequest};
    use super::{ValidationBackend, ValidationBackendFailure};
    use super::{embedded_validate_pre_write, validate_pre_write};

    struct FixtureDaemon {
        outcome: DaemonValidationOutcome,
    }

    impl DaemonValidationClient for FixtureDaemon {
        fn validate_pre_write(
            &self,
            _request: &PreWriteValidationRequest<'_>,
        ) -> DaemonValidationOutcome {
            self.outcome.clone()
        }
    }

    #[test]
    fn daemon_result_wins_when_available() {
        let request = secret_request();
        let embedded = embedded_validate_pre_write(&request);
        let daemon = FixtureDaemon {
            outcome: DaemonValidationOutcome::Diagnostics(embedded.diagnostics.clone()),
        };

        let result = validate_pre_write(&request, &daemon).expect("daemon result is valid");

        assert_eq!(result.backend, ValidationBackend::Daemon);
        assert_eq!(result.diagnostics, embedded.diagnostics);
    }

    #[test]
    fn unavailable_daemon_falls_back_to_embedded_validation() {
        let request = secret_request();
        let daemon = FixtureDaemon {
            outcome: DaemonValidationOutcome::Unavailable,
        };

        let result = validate_pre_write(&request, &daemon).expect("embedded fallback succeeds");

        assert_eq!(result.backend, ValidationBackend::Embedded);
        assert_eq!(result.diagnostics[0].source.rule_id, "secret-detection");
    }

    #[test]
    fn operational_daemon_failure_does_not_fall_back() {
        let request = secret_request();
        let failure = ValidationBackendFailure {
            code: "validation-backend-unavailable",
            message: "Anvil could not validate the proposed write.",
            retriable: true,
        };
        let daemon = FixtureDaemon {
            outcome: DaemonValidationOutcome::OperationalFailure(failure),
        };

        let error = validate_pre_write(&request, &daemon).expect_err("daemon failure blocks");

        assert_eq!(error, failure);
    }

    fn secret_request() -> PreWriteValidationRequest<'static> {
        PreWriteValidationRequest {
            relative_path: "src/secret.ts",
            content: "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n",
        }
    }
}
