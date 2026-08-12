use anvil_checks::secret::{SecretCheckConfig, SecretFinding, scan_content_with_limit};
use anvil_kernel_types::{Category, Diagnostic, DiagnosticSource, Location, Mode, Severity};

use crate::{ChangeKind, InterceptRule, RuleDecision, RuleInput, mode_id_part, sanitise_id_part};

pub const SECRET_RULE_ID: &str = "secret-detection";

#[derive(Debug, Clone)]
pub struct SecretDetectionRule {
    config: SecretCheckConfig,
}

impl SecretDetectionRule {
    #[must_use]
    pub const fn new(config: SecretCheckConfig) -> Self {
        Self { config }
    }

    #[must_use]
    pub const fn config(&self) -> &SecretCheckConfig {
        &self.config
    }

    #[must_use]
    pub fn diagnostics(&self, input: &RuleInput<'_>, mode: &Mode) -> Vec<Diagnostic> {
        self.diagnostics_with_limit(input, mode, usize::MAX)
    }

    #[must_use]
    pub fn diagnostics_with_limit(
        &self,
        input: &RuleInput<'_>,
        mode: &Mode,
        limit: usize,
    ) -> Vec<Diagnostic> {
        if limit == 0 {
            return Vec::new();
        }
        self.findings_with_limit(input, limit)
            .into_iter()
            .map(|finding| finding_to_diagnostic(&finding, mode.clone()))
            .collect()
    }

    fn findings_with_limit(&self, input: &RuleInput<'_>, limit: usize) -> Vec<SecretFinding> {
        if limit == 0 {
            return Vec::new();
        }
        // Deletions are not content writes — allow even if a caller retained
        // prior content on the event (matches antipattern/regex_content).
        if input.change_kind == ChangeKind::Removed {
            return Vec::new();
        }
        if self.should_skip_path(input) {
            return Vec::new();
        }
        let Some(content) = input.content else {
            return Vec::new();
        };
        let content = String::from_utf8_lossy(content);
        let path = input.path.to_string_lossy();
        scan_content_with_limit(content.as_ref(), path.as_ref(), &self.config, limit)
    }

    fn should_skip_path(&self, input: &RuleInput<'_>) -> bool {
        // Lockfiles are NOT skipped: `scan_content_with_limit` gives them a
        // restricted URL-credential-only scan (GH #2584), so an integrity hash
        // never trips the rule but a credential written into a `resolved` URL
        // still does. Returning `false` forces them past the `.lock` entry in
        // `skip_extensions` so `Cargo.lock`/`yarn.lock` get that scan too.
        if anvil_checks::filter::is_lockfile(input.path) {
            return false;
        }
        let path = input.path.to_string_lossy();
        self.config
            .skip_extensions
            .iter()
            .any(|extension| path.ends_with(extension))
    }
}

impl Default for SecretDetectionRule {
    fn default() -> Self {
        Self::new(SecretCheckConfig::default())
    }
}

impl InterceptRule for SecretDetectionRule {
    fn rule_id(&self) -> &str {
        SECRET_RULE_ID
    }

    fn needs_content(&self) -> bool {
        true
    }

    fn evaluate(&self, input: &RuleInput<'_>) -> RuleDecision {
        let findings = self.findings_with_limit(input, 1);
        let Some(first) = findings.first() else {
            return RuleDecision::Allow;
        };

        let message = format!("Potential secret detected ({})", first.pattern_name);
        match u32::try_from(first.line) {
            Ok(line) if line > 0 => RuleDecision::interrupt_at(SECRET_RULE_ID, message, line),
            _ => RuleDecision::interrupt(SECRET_RULE_ID, message),
        }
    }

    fn diagnostics(&self, input: &RuleInput<'_>, mode: &Mode) -> Vec<Diagnostic> {
        SecretDetectionRule::diagnostics(self, input, mode)
    }

    fn diagnostics_with_limit(
        &self,
        input: &RuleInput<'_>,
        mode: &Mode,
        limit: usize,
    ) -> Vec<Diagnostic> {
        SecretDetectionRule::diagnostics_with_limit(self, input, mode, limit)
    }
}

fn finding_to_diagnostic(finding: &SecretFinding, mode: Mode) -> Diagnostic {
    Diagnostic::new(
        format!(
            "diag_secret_{}_{}_{}_{}",
            mode_id_part(&mode),
            sanitise_id_part(&finding.file),
            finding.line,
            sanitise_id_part(&finding.pattern_name)
        ),
        Severity::Error,
        format!("Potential secret detected ({})", finding.pattern_name),
        Location {
            file: finding.file.clone(),
            line: u32::try_from(finding.line).ok(),
            column: None,
            end_line: None,
            end_column: None,
        },
        Category::Secret,
        DiagnosticSource {
            rule_id: SECRET_RULE_ID.to_string(),
            source_module: "anvil-checks::secret".to_string(),
        },
        mode,
    )
    .with_remediation_hint("Use a placeholder or environment variable instead.")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use anvil_kernel_types::{Category, Mode, Severity};

    use super::*;
    use crate::{ChangeKind, InterceptRule, RuleInput};

    fn input<'a>(path: &'a Path, content: Option<&'a [u8]>) -> RuleInput<'a> {
        RuleInput {
            path,
            change_kind: ChangeKind::Modified,
            content,
        }
    }

    #[test]
    fn secret_rule_interrupts_on_changed_content() {
        let path = Path::new("src/auth/client.ts");
        let body = b"import { sdk } from './client';\nconst config = { api_key: 'abcdEFGH1234567890' };\nsdk.connect(config);\n";

        let decision = SecretDetectionRule::default().evaluate(&input(path, Some(body)));

        match decision {
            RuleDecision::Interrupt(reason) => {
                assert_eq!(reason.rule_id, SECRET_RULE_ID);
                assert_eq!(reason.line, std::num::NonZeroU32::new(2));
                assert!(reason.message.contains("Potential secret detected"));
                assert!(!reason.message.contains("abcdEFGH1234567890"));
            }
            RuleDecision::Allow => panic!("secret fixture should interrupt"),
        }
    }

    #[test]
    fn secret_rule_allows_clean_content() {
        let path = Path::new("src/auth/client.ts");
        let body = b"const config = { endpoint: 'https://example.test' };\n";

        let decision = SecretDetectionRule::default().evaluate(&input(path, Some(body)));

        assert_eq!(decision, RuleDecision::Allow);
    }

    #[test]
    fn secret_rule_allows_lockfile_integrity_hash() {
        // GH #2584: a lockfile's high-entropy integrity hash is a false
        // positive — the restricted URL-only scan must not interrupt on it.
        let path = Path::new("package-lock.json");
        let body = b"{\"integrity\":\"sha512-XI5MPzVNApjAyhQzphX8BkmKsKUxD4LdyK24iZeQGinB\"}\n";

        let decision = SecretDetectionRule::default().evaluate(&input(path, Some(body)));

        assert_eq!(
            decision,
            RuleDecision::Allow,
            "lockfile integrity hashes must not trip the save-time secret rule",
        );
    }

    #[test]
    fn secret_rule_interrupts_on_lockfile_url_credential() {
        // A credential embedded in a lockfile `resolved` URL IS a real secret
        // and must still interrupt, even though the integrity hashes around it
        // are ignored.
        let path = Path::new("package-lock.json");
        let body = b"{\n  \"resolved\": \"https://deployer:s3cr3tT0ken@npm.private.example/x/-/x-1.0.0.tgz\"\n}\n";

        let decision = SecretDetectionRule::default().evaluate(&input(path, Some(body)));

        match decision {
            RuleDecision::Interrupt(reason) => {
                assert_eq!(reason.rule_id, SECRET_RULE_ID);
                assert!(!reason.message.contains("s3cr3tT0ken"));
            }
            RuleDecision::Allow => panic!("a credential URL in a lockfile must interrupt"),
        }
    }

    #[test]
    fn secret_rule_allows_missing_or_non_secret_binary_content() {
        let path = Path::new("src/auth/client.ts");
        let rule = SecretDetectionRule::default();

        assert_eq!(rule.evaluate(&input(path, None)), RuleDecision::Allow);
        assert_eq!(
            rule.evaluate(&input(path, Some(b"\xff"))),
            RuleDecision::Allow
        );
    }

    #[test]
    fn secret_rule_respects_skipped_extensions() {
        let path = Path::new("pnpm-lock.yaml.lock");
        let body = b"const config = { api_key: 'abcdEFGH1234567890' };\n";

        let decision = SecretDetectionRule::default().evaluate(&input(path, Some(body)));

        assert_eq!(decision, RuleDecision::Allow);
    }

    #[test]
    fn secret_rule_scans_mixed_invalid_utf8_content() {
        let path = Path::new("src/auth/client.ts");
        let body = b"const config = { api_key: 'abcdEFGH1234567890' };\n\xff";

        let decision = SecretDetectionRule::default().evaluate(&input(path, Some(body)));

        match decision {
            RuleDecision::Interrupt(reason) => {
                assert_eq!(reason.rule_id, SECRET_RULE_ID);
                assert_eq!(reason.line, std::num::NonZeroU32::new(1));
            }
            RuleDecision::Allow => panic!("mixed invalid UTF-8 secret fixture should interrupt"),
        }
    }

    #[test]
    fn secret_rule_maps_findings_to_canonical_diagnostics() {
        let path = Path::new("src/auth/client.ts");
        let body = b"const config = { api_key: 'abcdEFGH1234567890' };\n";

        let diagnostics = SecretDetectionRule::default().diagnostics(
            &input(path, Some(body)),
            &Mode::Unknown("pre-write".to_string()),
        );

        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.schema_version, "anvil.diagnostic.v1");
        assert_eq!(
            diagnostic.id,
            "diag_secret_pre_write_src_auth_client_ts_1_api_key"
        );
        assert_eq!(diagnostic.severity, Severity::Error);
        assert_eq!(diagnostic.category, Category::Secret);
        assert_eq!(diagnostic.location.file, "src/auth/client.ts");
        assert_eq!(diagnostic.location.line, Some(1));
        assert_eq!(diagnostic.mode, Mode::Unknown("pre-write".to_string()));
        assert_eq!(diagnostic.source.rule_id, SECRET_RULE_ID);
        assert!(!diagnostic.summary.contains("abcdEFGH1234567890"));
    }

    #[test]
    fn secret_rule_allows_removed_changes_even_with_content() {
        // Sibling rules (antipattern, regex_content, path_deny) allow Removed
        // even when a caller supplies prior content. Secret detection must not
        // block a deletion of a file that previously held a secret.
        let path = Path::new("src/auth/client.ts");
        let body = b"const config = { api_key: 'abcdEFGH1234567890' };\n";
        let removed = RuleInput {
            path,
            change_kind: ChangeKind::Removed,
            content: Some(body.as_slice()),
        };
        let rule = SecretDetectionRule::default();

        assert_eq!(rule.evaluate(&removed), RuleDecision::Allow);
        assert!(
            rule.diagnostics(&removed, &Mode::Unknown("pre-write".to_string()))
                .is_empty()
        );
    }
}
