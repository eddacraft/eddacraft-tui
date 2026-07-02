use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::Instant;

use wait_timeout::ChildExt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::loader::LoadedPolicy;

#[derive(Debug, Clone, Serialize)]
pub struct OpaResult {
    pub success: bool,
    pub violations: Vec<PolicyViolation>,
    pub metadata: OpaMetadata,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpaMetadata {
    pub policy_count: usize,
    pub execution_time_ms: u64,
    pub opa_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyViolation {
    pub rule: String,
    pub severity: String,
    pub message: String,
    pub path: Option<String>,
    pub policy: Option<String>,
    pub category: Option<String>,
    pub fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyntaxResult {
    pub valid: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TestResult {
    pub passed: u32,
    pub failed: u32,
    pub errors: Vec<String>,
    pub details: Vec<TestDetail>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TestDetail {
    pub name: String,
    pub passed: bool,
    pub message: Option<String>,
}

/// Network-capable and runtime-sensitive OPA built-ins removed from the
/// capabilities profile used for every evaluation (CIB-108).
///
/// Workspace policies (`.anvil/policies/*.rego`) are untrusted input; these
/// built-ins would let a policy make outbound requests (`http.send`), resolve
/// DNS (`net.lookup_ip_addr`), or read the process environment
/// (`opa.runtime`) from developer and CI machines.
pub const OPA_DENIED_BUILTINS: [&str; 3] = ["http.send", "net.lookup_ip_addr", "opa.runtime"];

#[derive(Debug, thiserror::Error)]
pub enum OpaError {
    #[error("OPA binary not found: {0}")]
    BinaryNotFound(String),
    #[error(
        "failed to derive a restricted OPA capabilities profile; \
         refusing to evaluate policies without built-in restrictions: {0}"
    )]
    CapabilitiesDerivation(String),
    #[error("OPA execution failed: {0}")]
    Execution(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("OPA timed out after {0}ms")]
    Timeout(u64),
    #[error("unexpected OPA output shape at {pointer}: {snippet}")]
    UnexpectedShape {
        pointer: String,
        /// Truncated raw output for diagnosis; full output may be very large.
        snippet: String,
    },
}

const UNEXPECTED_SHAPE_SNIPPET_CHAR_LIMIT: usize = 512;

fn snippet_for_error(raw: &str) -> String {
    let mut iter = raw.char_indices();
    match iter.nth(UNEXPECTED_SHAPE_SNIPPET_CHAR_LIMIT) {
        // At least LIMIT+1 chars — truncate at the char boundary. The byte
        // count reported is the full original length so operators comparing
        // CI logs against an `opa eval` replay can match sizes.
        Some((cut, _)) => format!("{}…<truncated; original {} bytes>", &raw[..cut], raw.len()),
        None => raw.to_string(),
    }
}

pub struct OpaExecutor {
    binary_path: String,
    timeout_ms: u64,
    query: String,
    /// Memoised restricted capabilities JSON (CIB-108), derived once per
    /// executor from `opa capabilities --current` so the profile always
    /// matches the installed binary version. Stores the failure too so a
    /// broken binary fails closed consistently instead of being re-probed.
    restricted_capabilities: OnceLock<Result<String, String>>,
}

impl OpaExecutor {
    pub fn new(binary_path: Option<&str>, timeout_ms: Option<u64>) -> Self {
        Self {
            binary_path: binary_path.unwrap_or("opa").to_string(),
            timeout_ms: timeout_ms.unwrap_or(30_000),
            query: "data.anvil.policies".to_string(),
            restricted_capabilities: OnceLock::new(),
        }
    }

    #[must_use]
    pub fn with_query(mut self, query: &str) -> Self {
        self.query = query.to_string();
        self
    }

    pub fn is_available(&self) -> bool {
        match Command::new(&self.binary_path).arg("version").output() {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    pub fn version(&self) -> Option<String> {
        let output = Command::new(&self.binary_path)
            .arg("version")
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&output.stdout);
        text.lines()
            .find(|l| l.starts_with("Version:"))
            .map(|l| l.trim_start_matches("Version:").trim().to_string())
    }

    pub fn evaluate(
        &self,
        policies: &[LoadedPolicy],
        input: &serde_json::Value,
    ) -> Result<OpaResult, OpaError> {
        let start = Instant::now();
        let policy_dir = tempfile::TempDir::new()?;
        let input_dir = tempfile::TempDir::new()?;

        for policy in policies {
            let dest = policy_dir.path().join(format!("{}.rego", policy.name));
            std::fs::write(&dest, &policy.content)?;
        }

        let input_path = input_dir.path().join("input.json");
        let input_str = serde_json::to_string_pretty(input)?;
        std::fs::write(&input_path, &input_str)?;

        // CIB-108: restrict built-ins on every eval. The profile lives in
        // `input_dir`, NOT `policy_dir` — `--data <dir>` loads every JSON
        // file in the directory into the data document, while `--input` only
        // reads the single file passed.
        let capabilities_json = self.restricted_capabilities()?;
        let capabilities_path = input_dir.path().join("capabilities.json");
        std::fs::write(&capabilities_path, capabilities_json)?;

        let mut child = Command::new(&self.binary_path)
            .arg("eval")
            .arg("--data")
            .arg(policy_dir.path())
            .arg("--input")
            .arg(&input_path)
            .arg("--capabilities")
            .arg(&capabilities_path)
            .arg("--format")
            .arg("json")
            .arg(&self.query)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    OpaError::BinaryNotFound(self.binary_path.clone())
                } else {
                    OpaError::Io(e)
                }
            })?;

        // Drain stdout/stderr in dedicated threads so a full OS pipe buffer
        // cannot wedge the child — and so we don't have to call
        // wait_with_output() after wait_timeout(). Calling both on the same
        // Child is undefined: on Linux the second wait can return a bogus
        // ExitStatus of 0 (masking real failures as silent passes); on macOS
        // it can surface ECHILD. This rewrite uses a single wait path.
        let mut stdout_handle = child
            .stdout
            .take()
            .expect("stdout is piped above; take() is called once");
        let mut stderr_handle = child
            .stderr
            .take()
            .expect("stderr is piped above; take() is called once");
        // Reader closures return a Result so that a pipe I/O error surfaces
        // as OpaError::Io rather than silently producing a truncated buffer
        // that downstream parsing would misinterpret (JSON parse error, empty
        // stderr, etc.).
        let stdout_reader = std::thread::spawn(move || -> std::io::Result<Vec<u8>> {
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut stdout_handle, &mut buf)?;
            Ok(buf)
        });
        let stderr_reader = std::thread::spawn(move || -> std::io::Result<Vec<u8>> {
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut stderr_handle, &mut buf)?;
            Ok(buf)
        });

        let timeout = std::time::Duration::from_millis(self.timeout_ms);
        let Some(status) = child.wait_timeout(timeout)? else {
            let _ = child.kill();
            let _ = child.wait();
            // kill() closes the child's pipe handles on unix, which makes the
            // reader threads' read_to_end return; on windows the same call
            // terminates the process and the pipe write ends are closed by
            // the kernel. We still join to avoid dangling threads, but we're
            // about to return Timeout so the payloads/errors are discarded.
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(OpaError::Timeout(self.timeout_ms));
        };

        // Two failure modes for each reader: (outer) the thread panicked —
        // surface as Execution; (inner) read_to_end returned an io::Error —
        // surface as Io. Previously both were silently substituted with an
        // empty Vec, which masked pipe failures as JSON parse errors further
        // downstream.
        let stdout_bytes = stdout_reader
            .join()
            .map_err(|e| OpaError::Execution(format!("stdout reader thread panicked: {e:?}")))??;
        let stderr_bytes = stderr_reader
            .join()
            .map_err(|e| OpaError::Execution(format!("stderr reader thread panicked: {e:?}")))??;

        #[allow(clippy::cast_possible_truncation)]
        let elapsed = start.elapsed().as_millis() as u64;

        let opa_version = self.version();

        if !status.success() {
            let stderr = String::from_utf8_lossy(&stderr_bytes).to_string();
            let stdout_text = String::from_utf8_lossy(&stdout_bytes);
            // CIB-108: a policy that needs a denied built-in fails to
            // compile against the restricted capabilities profile; give the
            // operator an actionable message instead of a bare type error.
            let error = denied_builtin_error(&stderr, &stdout_text).unwrap_or(stderr);
            return Ok(OpaResult {
                success: false,
                violations: Vec::new(),
                metadata: OpaMetadata {
                    policy_count: policies.len(),
                    execution_time_ms: elapsed,
                    opa_version,
                },
                error: Some(error),
            });
        }

        let stdout = String::from_utf8_lossy(&stdout_bytes);
        let violations = self.extract_violations(&stdout, policies)?;

        Ok(OpaResult {
            success: violations.is_empty(),
            violations,
            metadata: OpaMetadata {
                policy_count: policies.len(),
                execution_time_ms: elapsed,
                opa_version,
            },
            error: None,
        })
    }

    pub fn validate_syntax(&self, content: &str) -> Result<SyntaxResult, OpaError> {
        let tmp = tempfile::TempDir::new()?;
        let path = tmp.path().join("check.rego");
        std::fs::write(&path, content)?;

        let output = Command::new(&self.binary_path)
            .arg("check")
            .arg(&path)
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    OpaError::BinaryNotFound(self.binary_path.clone())
                } else {
                    OpaError::Io(e)
                }
            })?;

        if output.status.success() {
            Ok(SyntaxResult {
                valid: true,
                errors: Vec::new(),
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let errors: Vec<String> = stderr
                .lines()
                .filter(|l| !l.is_empty())
                .map(std::string::ToString::to_string)
                .collect();
            Ok(SyntaxResult {
                valid: false,
                errors,
            })
        }
    }

    pub fn run_tests(&self, policy_dir: &Path, verbose: bool) -> Result<TestResult, OpaError> {
        // CIB-108: `opa test <dir>` executes untrusted rego too, so the same
        // restricted capabilities profile applies. It is written to its own
        // temp dir because `opa test` loads every file in `policy_dir`
        // (including JSON) as data/policies.
        let capabilities_json = self.restricted_capabilities()?;
        let capabilities_dir = tempfile::TempDir::new()?;
        let capabilities_path = capabilities_dir.path().join("capabilities.json");
        std::fs::write(&capabilities_path, capabilities_json)?;
        let capabilities_str = capabilities_path.to_string_lossy();

        let mut args = vec![
            "test",
            "--capabilities",
            &capabilities_str,
            "--format",
            "json",
        ];
        if verbose {
            args.push("-v");
        }

        let dir_str = policy_dir.to_string_lossy();
        args.push(&dir_str);

        let output = Command::new(&self.binary_path)
            .args(&args)
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    OpaError::BinaryNotFound(self.binary_path.clone())
                } else {
                    OpaError::Io(e)
                }
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut details = Vec::new();
        let mut errors = Vec::new();

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if let Some(mapped) = denied_builtin_error(&stderr, &stdout) {
                errors.push(mapped);
            } else if !stderr.is_empty() {
                errors.push(stderr.to_string());
            }
        }

        match serde_json::from_str::<Vec<OpaTestEntry>>(&stdout) {
            Ok(results) => {
                for entry in &results {
                    details.push(TestDetail {
                        name: entry.name.clone(),
                        passed: entry.fail.is_none() || !entry.fail.unwrap_or(false),
                        message: entry.output.clone(),
                    });
                }
            }
            Err(e) => {
                if !stdout.trim().is_empty() {
                    errors.push(format!("Failed to parse OPA test output: {e}"));
                }
            }
        }

        #[allow(clippy::cast_possible_truncation)]
        let passed = details.iter().filter(|d| d.passed).count() as u32;
        #[allow(clippy::cast_possible_truncation)]
        let failed = details.iter().filter(|d| !d.passed).count() as u32;

        Ok(TestResult {
            passed,
            failed,
            errors,
            details,
        })
    }

    /// Restricted capabilities profile for this executor (CIB-108),
    /// memoised on first use. Fails closed: any derivation failure aborts
    /// evaluation rather than falling back to unrestricted built-ins.
    fn restricted_capabilities(&self) -> Result<&str, OpaError> {
        self.restricted_capabilities
            .get_or_init(|| self.derive_restricted_capabilities())
            .as_deref()
            .map_err(|e| OpaError::CapabilitiesDerivation(e.clone()))
    }

    /// Derive the profile from the configured binary (`opa capabilities
    /// --current`) so it always matches the installed OPA version, then
    /// remove the denied built-ins and empty `allow_net` as defence in
    /// depth (an empty allowlist denies every host even if a
    /// network-capable built-in slipped through).
    ///
    /// Uses the same drain-then-`wait_timeout` strategy as `evaluate`: this
    /// runs (memoised) on the default gate path, so a hung binary must fail
    /// closed within the executor timeout instead of stalling the gate, and
    /// the capabilities document is large enough to fill an OS pipe buffer.
    fn derive_restricted_capabilities(&self) -> Result<String, String> {
        let mut child = Command::new(&self.binary_path)
            .args(["capabilities", "--current"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;

        let mut stdout_handle = child
            .stdout
            .take()
            .expect("stdout is piped above; take() is called once");
        let mut stderr_handle = child
            .stderr
            .take()
            .expect("stderr is piped above; take() is called once");
        let stdout_reader = std::thread::spawn(move || -> std::io::Result<Vec<u8>> {
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut stdout_handle, &mut buf)?;
            Ok(buf)
        });
        let stderr_reader = std::thread::spawn(move || -> std::io::Result<Vec<u8>> {
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut stderr_handle, &mut buf)?;
            Ok(buf)
        });

        let timeout = std::time::Duration::from_millis(self.timeout_ms);
        let Some(status) = child.wait_timeout(timeout).map_err(|e| e.to_string())? else {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(format!(
                "opa capabilities timed out after {}ms",
                self.timeout_ms
            ));
        };

        let stdout_bytes = stdout_reader
            .join()
            .map_err(|e| format!("stdout reader thread panicked: {e:?}"))?
            .map_err(|e| e.to_string())?;
        let stderr_bytes = stderr_reader
            .join()
            .map_err(|e| format!("stderr reader thread panicked: {e:?}"))?
            .map_err(|e| e.to_string())?;

        if !status.success() {
            let stderr = String::from_utf8_lossy(&stderr_bytes);
            let stderr = stderr.trim();
            return Err(if stderr.is_empty() {
                format!("opa capabilities exited with status {status}")
            } else {
                stderr.to_string()
            });
        }

        let mut capabilities: serde_json::Value = serde_json::from_slice(&stdout_bytes)
            .map_err(|e| format!("opa capabilities returned invalid JSON: {e}"))?;

        let Some(builtins) = capabilities
            .get_mut("builtins")
            .and_then(serde_json::Value::as_array_mut)
        else {
            return Err("opa capabilities output has no builtins list".to_string());
        };

        builtins.retain(|builtin| {
            builtin
                .get("name")
                .and_then(serde_json::Value::as_str)
                .is_none_or(|name| !OPA_DENIED_BUILTINS.contains(&name))
        });
        capabilities["allow_net"] = serde_json::json!([]);

        Ok(capabilities.to_string())
    }

    fn extract_violations(
        &self,
        stdout: &str,
        policies: &[LoadedPolicy],
    ) -> Result<Vec<PolicyViolation>, OpaError> {
        let parsed: serde_json::Value = serde_json::from_str(stdout)?;
        let mut violations = Vec::new();

        // `opa eval --format json` emits `{"result": [{"expressions": [{"value": ...}]}]}`.
        // Distinguish four cases that used to collapse into "zero violations":
        //   - `result` missing or not an array → OPA output doesn't match the contract
        //     we understand; raise UnexpectedShape so callers don't silently mark the
        //     gate as passed.
        //   - `result` present but empty → the query matched no rules ("no decisions"),
        //     which we map to zero violations (the only sensible gate mapping, but
        //     semantically distinct from "no violations from rules that ran"). Logged
        //     at warn so operators can spot an undeclared/empty policy pack.
        //   - `result[i].expressions[0].value` missing or literal null → schema has
        //     drifted or policy evaluated to null (e.g. missing input field); raise
        //     UnexpectedShape. Both previously masked OPA output changes as silent
        //     passes.
        //   - `result` with more than one entry → iterate all entries so a partial /
        //     multi-expression query doesn't silently drop entries[1..].
        let Some(result_array) = parsed.get("result").and_then(|v| v.as_array()) else {
            return Err(OpaError::UnexpectedShape {
                pointer: "/result".to_string(),
                snippet: snippet_for_error(stdout),
            });
        };

        if result_array.is_empty() {
            // No tracing crate wired into this workspace yet; eprintln! is the
            // interim channel so an operator noticing an empty pack in CI logs
            // can spot a misconfigured policy_dir. Swap to tracing::warn! if
            // structured logging arrives.
            eprintln!(
                "anvil-policy: OPA returned empty `result` array for query `{}` — \
                 the gate will pass, but this may indicate a misconfigured policy pack.",
                self.query
            );
            return Ok(violations);
        }

        for (i, entry) in result_array.iter().enumerate() {
            let value =
                entry
                    .pointer("/expressions/0/value")
                    .ok_or_else(|| OpaError::UnexpectedShape {
                        pointer: format!("/result/{i}/expressions/0/value"),
                        snippet: snippet_for_error(stdout),
                    })?;

            // Only a Value::Object is a valid policy map. Null (from a policy
            // evaluating to null, e.g. missing input field) and any other
            // non-object (scalar, array) both indicate schema drift or a query
            // resolving to something we can't interpret — surface as shape
            // error rather than silently returning zero violations.
            let serde_json::Value::Object(map) = value else {
                return Err(OpaError::UnexpectedShape {
                    pointer: format!("/result/{i}/expressions/0/value"),
                    snippet: snippet_for_error(stdout),
                });
            };

            for (policy_name, policy_output) in map {
                self.extract_from_policy_output(
                    policy_name,
                    policy_output,
                    policies,
                    &mut violations,
                );
            }
        }

        Ok(violations)
    }

    #[allow(clippy::unused_self)]
    fn extract_from_policy_output(
        &self,
        policy_name: &str,
        output: &serde_json::Value,
        _policies: &[LoadedPolicy],
        violations: &mut Vec<PolicyViolation>,
    ) {
        let Some(obj) = output.as_object() else {
            return;
        };

        let violation_keys = [
            ("violation", "error"),
            ("violations", "error"),
            ("deny", "error"),
            ("denies", "error"),
            ("warn", "warning"),
            ("warnings", "warning"),
        ];

        for (key, default_severity) in &violation_keys {
            if let Some(arr) = obj.get(*key).and_then(|v| v.as_array()) {
                for item in arr {
                    let v = parse_violation_item(item, policy_name, default_severity);
                    violations.push(v);
                }
            }
        }
    }
}

fn parse_violation_item(
    item: &serde_json::Value,
    policy_name: &str,
    default_severity: &str,
) -> PolicyViolation {
    match item {
        serde_json::Value::String(msg) => {
            let category = infer_category(policy_name);
            let fingerprint = compute_fingerprint(policy_name, policy_name, None, msg);
            PolicyViolation {
                rule: policy_name.to_string(),
                severity: default_severity.to_string(),
                message: msg.clone(),
                path: None,
                policy: Some(policy_name.to_string()),
                category: Some(category),
                fingerprint: Some(fingerprint),
            }
        }
        serde_json::Value::Object(obj) => {
            let rule = obj
                .get("rule")
                .and_then(|v| v.as_str())
                .unwrap_or(policy_name)
                .to_string();
            let message = obj
                .get("message")
                .or_else(|| obj.get("msg"))
                .and_then(|v| v.as_str())
                .unwrap_or("policy violation")
                .to_string();
            let severity = obj
                .get("severity")
                .and_then(|v| v.as_str())
                .map_or_else(|| default_severity.to_string(), normalise_severity);
            let path = obj.get("path").and_then(|v| v.as_str()).map(String::from);
            let category = obj
                .get("category")
                .and_then(|v| v.as_str())
                .map_or_else(|| infer_category(policy_name), String::from);
            let fingerprint = compute_fingerprint(&rule, policy_name, path.as_deref(), &message);

            PolicyViolation {
                rule,
                severity,
                message,
                path,
                policy: Some(policy_name.to_string()),
                category: Some(category),
                fingerprint: Some(fingerprint),
            }
        }
        _ => PolicyViolation {
            rule: policy_name.to_string(),
            severity: default_severity.to_string(),
            message: item.to_string(),
            path: None,
            policy: Some(policy_name.to_string()),
            category: Some(infer_category(policy_name)),
            fingerprint: None,
        },
    }
}

/// CIB-108: map an OPA compile error caused by a denied built-in to an
/// actionable message. Returns `None` when the failure is unrelated to the
/// capabilities restriction.
fn denied_builtin_error(stderr: &str, stdout: &str) -> Option<String> {
    let denied = OPA_DENIED_BUILTINS.iter().find(|name| {
        let needle = format!("undefined function {name}");
        stderr.contains(&needle) || stdout.contains(&needle)
    })?;
    let detail = if stderr.trim().is_empty() {
        stdout
    } else {
        stderr
    };
    Some(format!(
        "policy requires the OPA built-in \"{denied}\", which is not permitted: \
         network-capable and runtime-sensitive built-ins are disabled during \
         policy evaluation (CIB-108). {detail}"
    ))
}

fn normalise_severity(s: &str) -> String {
    match s.to_lowercase().as_str() {
        "error" | "err" => "error".to_string(),
        "warning" | "warn" => "warning".to_string(),
        "info" => "info".to_string(),
        other => other.to_string(),
    }
}

fn infer_category(name: &str) -> String {
    let lower = name.to_lowercase();
    if lower.contains("security") || lower.contains("secret") || lower.contains("auth") {
        "security".to_string()
    } else if lower.contains("architecture")
        || lower.contains("layer")
        || lower.contains("boundary")
    {
        "architecture".to_string()
    } else if lower.contains("coverage") || lower.contains("test") {
        "coverage".to_string()
    } else if lower.contains("scope") || lower.contains("change") || lower.contains("size") {
        "scope".to_string()
    } else if lower.contains("lint") || lower.contains("quality") || lower.contains("style") {
        "quality".to_string()
    } else if lower.contains("compliance") || lower.contains("license") || lower.contains("audit") {
        "compliance".to_string()
    } else {
        "custom".to_string()
    }
}

fn compute_fingerprint(rule: &str, policy: &str, path: Option<&str>, message: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(policy.as_bytes());
    hasher.update(rule.as_bytes());
    hasher.update(path.unwrap_or("").as_bytes());
    hasher.update(message.as_bytes());
    let hash = hasher.finalize();
    hex::encode(&hash[..8])
}

/// Resolve the OPA binary path: honour `ANVIL_OPA_PATH` first, otherwise
/// fall back to a `which` lookup. Returns `None` when no binary is available.
pub fn find_opa_binary() -> Option<PathBuf> {
    if let Ok(env_path) = std::env::var("ANVIL_OPA_PATH")
        && !env_path.is_empty()
    {
        return Some(PathBuf::from(env_path));
    }
    which::which("opa").ok()
}

#[derive(Debug, Deserialize)]
struct OpaTestEntry {
    #[serde(default)]
    name: String,
    #[serde(default)]
    fail: Option<bool>,
    #[serde(default)]
    output: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise_severity_variants() {
        assert_eq!(normalise_severity("error"), "error");
        assert_eq!(normalise_severity("err"), "error");
        assert_eq!(normalise_severity("WARNING"), "warning");
        assert_eq!(normalise_severity("warn"), "warning");
        assert_eq!(normalise_severity("Info"), "info");
    }

    #[test]
    fn infer_category_from_name() {
        assert_eq!(infer_category("security_baseline"), "security");
        assert_eq!(infer_category("coverage_min"), "coverage");
        assert_eq!(infer_category("architecture_layers"), "architecture");
        assert_eq!(infer_category("change_scope"), "scope");
        assert_eq!(infer_category("lint_rules"), "quality");
        assert_eq!(infer_category("license_check"), "compliance");
        assert_eq!(infer_category("something_else"), "custom");
    }

    #[test]
    fn fingerprint_is_stable() {
        let a = compute_fingerprint("rule1", "policy1", Some("file.ts"), "msg");
        let b = compute_fingerprint("rule1", "policy1", Some("file.ts"), "msg");
        assert_eq!(a, b);
        assert_eq!(a.len(), 16);
    }

    #[test]
    fn fingerprint_changes_with_input() {
        let a = compute_fingerprint("rule1", "policy1", None, "msg1");
        let b = compute_fingerprint("rule1", "policy1", None, "msg2");
        assert_ne!(a, b);
    }

    #[test]
    fn parse_string_violation() {
        let item = serde_json::Value::String("bad pattern found".into());
        let v = parse_violation_item(&item, "security_check", "error");
        assert_eq!(v.message, "bad pattern found");
        assert_eq!(v.severity, "error");
        assert_eq!(v.category.as_deref(), Some("security"));
        assert!(v.fingerprint.is_some());
    }

    #[test]
    fn parse_object_violation() {
        let item = serde_json::json!({
            "rule": "no-secrets",
            "message": "API key exposed",
            "severity": "error",
            "path": "src/config.ts",
            "category": "security"
        });
        let v = parse_violation_item(&item, "security_check", "warning");
        assert_eq!(v.rule, "no-secrets");
        assert_eq!(v.severity, "error"); // overrides default
        assert_eq!(v.path.as_deref(), Some("src/config.ts"));
    }

    #[test]
    fn opa_executor_detects_missing_binary() {
        let executor = OpaExecutor::new(Some("/nonexistent/opa"), None);
        assert!(!executor.is_available());
    }

    #[test]
    fn extract_violations_empty_result_is_no_decisions() {
        // Empty `result` array means the query matched no rules. That is NOT
        // the same as "rules ran and found zero violations" — but for a gate
        // that's asking "any violations?" we return an empty list.
        let executor = OpaExecutor::new(Some("opa"), None);
        let stdout = r#"{"result":[]}"#;
        let vs = executor
            .extract_violations(stdout, &[])
            .expect("empty result ok");
        assert!(vs.is_empty());
    }

    #[test]
    fn extract_violations_missing_result_is_unexpected_shape() {
        let executor = OpaExecutor::new(Some("opa"), None);
        let stdout = r#"{"foo":"bar"}"#;
        let err = executor
            .extract_violations(stdout, &[])
            .expect_err("missing /result should error");
        match err {
            OpaError::UnexpectedShape { pointer, .. } => {
                assert_eq!(pointer, "/result");
            }
            other => panic!("expected UnexpectedShape, got {other:?}"),
        }
    }

    #[test]
    fn extract_violations_missing_expressions_is_unexpected_shape() {
        let executor = OpaExecutor::new(Some("opa"), None);
        // `result[0]` present but no `expressions` — future OPA schema drift.
        let stdout = r#"{"result":[{"bindings":{}}]}"#;
        let err = executor
            .extract_violations(stdout, &[])
            .expect_err("missing expressions should error");
        match err {
            OpaError::UnexpectedShape { pointer, .. } => {
                assert_eq!(pointer, "/result/0/expressions/0/value");
            }
            other => panic!("expected UnexpectedShape, got {other:?}"),
        }
    }

    #[test]
    fn extract_violations_happy_path_still_works() {
        let executor = OpaExecutor::new(Some("opa"), None);
        let stdout = r#"{
            "result": [{
                "expressions": [{
                    "value": {
                        "security_baseline": {
                            "deny": [
                                {"rule": "no-secrets", "message": "API key exposed", "severity": "error"}
                            ]
                        }
                    }
                }]
            }]
        }"#;
        let vs = executor
            .extract_violations(stdout, &[])
            .expect("happy path should parse");
        assert_eq!(vs.len(), 1);
        assert_eq!(vs[0].rule, "no-secrets");
        assert_eq!(vs[0].severity, "error");
    }

    #[test]
    fn snippet_truncates_large_raw() {
        let big = "x".repeat(2 * UNEXPECTED_SHAPE_SNIPPET_CHAR_LIMIT);
        let out = snippet_for_error(&big);
        assert!(out.contains("truncated"));
        assert!(out.len() < big.len());
    }

    #[test]
    fn snippet_handles_multibyte_utf8_at_boundary() {
        // Regression: the previous implementation byte-sliced `raw[..512]`
        // which panics if byte 512 falls inside a multi-byte codepoint.
        // Build a string whose char count exceeds the limit but whose byte
        // layout puts a multi-byte codepoint straddling the previous cutoff.
        let prefix = "a".repeat(UNEXPECTED_SHAPE_SNIPPET_CHAR_LIMIT - 1);
        let raw = format!("{prefix}€tail"); // `€` is 3 bytes (0xE2 0x82 0xAC)
        // Must not panic.
        let out = snippet_for_error(&raw);
        assert!(
            out.contains("truncated"),
            "expected truncation marker, got {out:?}"
        );
        // Output must itself be valid UTF-8 (String guarantees this, but the
        // assertion documents intent).
        assert!(std::str::from_utf8(out.as_bytes()).is_ok());
    }

    #[test]
    fn extract_violations_null_value_is_unexpected_shape() {
        // Policy evaluated to null (e.g. missing input field) — previously
        // returned zero violations silently, now must raise so the gate does
        // not pass spuriously.
        let executor = OpaExecutor::new(Some("opa"), None);
        let stdout = r#"{"result":[{"expressions":[{"value":null}]}]}"#;
        let err = executor
            .extract_violations(stdout, &[])
            .expect_err("null value should error");
        match err {
            OpaError::UnexpectedShape { pointer, .. } => {
                assert_eq!(pointer, "/result/0/expressions/0/value");
            }
            other => panic!("expected UnexpectedShape, got {other:?}"),
        }
    }

    #[test]
    fn extract_violations_iterates_all_result_entries() {
        // Multi-entry result: both entries contribute violations. Previously
        // only result[0] was processed, so a violation in result[1] would
        // silently pass the gate.
        let executor = OpaExecutor::new(Some("opa"), None);
        let stdout = r#"{
            "result": [
                {"expressions":[{"value":{
                    "security_baseline":{"deny":[
                        {"rule":"no-secrets","message":"first","severity":"error"}
                    ]}
                }}]},
                {"expressions":[{"value":{
                    "change_scope":{"violation":[
                        {"rule":"too-big","message":"second","severity":"error"}
                    ]}
                }}]}
            ]
        }"#;
        let vs = executor
            .extract_violations(stdout, &[])
            .expect("multi-entry result ok");
        assert_eq!(
            vs.len(),
            2,
            "expected violations from both entries, got {vs:?}"
        );
        let msgs: Vec<&str> = vs.iter().map(|v| v.message.as_str()).collect();
        assert!(msgs.contains(&"first"));
        assert!(msgs.contains(&"second"));
    }

    #[cfg(unix)]
    fn write_fake_opa_script(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(name);
        let mut f = std::fs::File::create(&path).expect("create fake opa");
        writeln!(f, "#!/bin/sh").unwrap();
        f.write_all(body.as_bytes()).unwrap();
        drop(f);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    #[cfg(unix)]
    #[test]
    fn evaluate_times_out_on_hanging_binary() {
        // Regression guard for the double-wait bug: a hanging child must
        // produce a deterministic Timeout error, not a silent success.
        let dir = tempfile::TempDir::new().unwrap();
        // `exec` so the shell is replaced by `sleep` — otherwise killing the
        // child (the shell) leaves `sleep` holding the stdout/stderr pipes and
        // the reader threads block until `sleep` naturally exits. The
        // `capabilities` branch answers instantly so the CIB-108 derivation
        // step succeeds and the hang is exercised on the eval call itself.
        let script = write_fake_opa_script(
            dir.path(),
            "fake_opa_hang",
            "if [ \"$1\" = \"capabilities\" ]; then\n\
             \x20 echo '{\"builtins\":[{\"name\":\"eq\"}]}'\n\
             \x20 exit 0\n\
             fi\n\
             exec sleep 30\n",
        );
        let executor = OpaExecutor::new(Some(script.to_str().unwrap()), Some(200));
        let err = executor
            .evaluate(&[], &serde_json::json!({}))
            .expect_err("hanging binary must time out");
        match err {
            OpaError::Timeout(ms) => assert_eq!(ms, 200),
            other => panic!("expected Timeout, got {other:?}"),
        }
    }

    /// Fake `opa` for the CIB-108 capabilities tests: serves a capabilities
    /// document containing the denied built-ins, then REQUIRES a
    /// `--capabilities <file>` argument on eval/test calls and fails unless
    /// the denied built-ins were filtered out and `allow_net` was emptied.
    #[cfg(unix)]
    const ENFORCING_FAKE_OPA: &str = r#"if [ "$1" = "capabilities" ]; then
  echo '{"builtins":[{"name":"eq"},{"name":"count"},{"name":"http.send"},{"name":"net.lookup_ip_addr"},{"name":"opa.runtime"}]}'
  exit 0
fi
if [ "$1" = "version" ]; then
  echo 'Version: 0.0.0-fake'
  exit 0
fi
caps=""
prev=""
for arg in "$@"; do
  if [ "$prev" = "--capabilities" ]; then caps="$arg"; fi
  prev="$arg"
done
if [ -z "$caps" ] || [ ! -f "$caps" ]; then
  echo "invoked without --capabilities" >&2
  exit 1
fi
for denied in http.send net.lookup_ip_addr opa.runtime; do
  if grep -q "\"$denied\"" "$caps"; then
    echo "denied built-in $denied still present in capabilities" >&2
    exit 1
  fi
done
if ! grep -q '"allow_net":\[\]' "$caps"; then
  echo "allow_net not emptied in capabilities" >&2
  exit 1
fi
if [ "$1" = "test" ]; then
  echo '[]'
else
  echo '{"result":[]}'
fi
"#;

    #[cfg(unix)]
    #[test]
    fn evaluate_passes_restricted_capabilities_to_opa() {
        let dir = tempfile::TempDir::new().unwrap();
        let script = write_fake_opa_script(dir.path(), "fake_opa_caps", ENFORCING_FAKE_OPA);
        let executor = OpaExecutor::new(Some(script.to_str().unwrap()), Some(5_000));
        let result = executor
            .evaluate(&[], &serde_json::json!({}))
            .expect("evaluate ok");
        assert!(
            result.success,
            "eval must be invoked with a restricted --capabilities file; error={:?}",
            result.error
        );
        assert!(
            result.error.is_none(),
            "unexpected error: {:?}",
            result.error
        );
    }

    #[cfg(unix)]
    #[test]
    fn run_tests_passes_restricted_capabilities_to_opa() {
        let dir = tempfile::TempDir::new().unwrap();
        let script = write_fake_opa_script(dir.path(), "fake_opa_caps_test", ENFORCING_FAKE_OPA);
        let policy_dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            policy_dir.path().join("example_test.rego"),
            "package test\n\ntest_ok { true }\n",
        )
        .unwrap();
        let executor = OpaExecutor::new(Some(script.to_str().unwrap()), Some(5_000));
        let result = executor
            .run_tests(policy_dir.path(), false)
            .expect("run_tests ok");
        assert!(
            result.errors.is_empty(),
            "opa test must be invoked with a restricted --capabilities file; errors={:?}",
            result.errors
        );
    }

    #[cfg(unix)]
    #[test]
    fn evaluate_fails_closed_when_capabilities_cannot_be_derived() {
        let dir = tempfile::TempDir::new().unwrap();
        let script = write_fake_opa_script(
            dir.path(),
            "fake_opa_no_caps",
            "if [ \"$1\" = \"capabilities\" ]; then\n\
             \x20 echo 'capabilities subcommand unsupported' >&2\n\
             \x20 exit 1\n\
             fi\n\
             echo '{\"result\":[]}'\n",
        );
        let executor = OpaExecutor::new(Some(script.to_str().unwrap()), Some(5_000));
        let err = executor
            .evaluate(&[], &serde_json::json!({}))
            .expect_err("derivation failure must fail closed, not run unrestricted");
        let msg = err.to_string();
        assert!(
            msg.contains("refusing to evaluate"),
            "expected fail-closed wording, got {msg:?}"
        );
        assert!(
            msg.contains("capabilities subcommand unsupported"),
            "expected underlying stderr detail, got {msg:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn capabilities_derivation_times_out_on_hanging_binary() {
        // The derivation runs (memoised) on the default gate path, so a hung
        // binary must fail closed within the executor timeout rather than
        // stalling the gate indefinitely. `exec` for the same pipe-holding
        // reason as evaluate_times_out_on_hanging_binary.
        let dir = tempfile::TempDir::new().unwrap();
        let script = write_fake_opa_script(
            dir.path(),
            "fake_opa_caps_hang",
            "if [ \"$1\" = \"capabilities\" ]; then\n\
             \x20 exec sleep 30\n\
             fi\n\
             echo '{\"result\":[]}'\n",
        );
        let executor = OpaExecutor::new(Some(script.to_str().unwrap()), Some(200));
        let start = std::time::Instant::now();
        let err = executor
            .evaluate(&[], &serde_json::json!({}))
            .expect_err("hung capabilities derivation must fail closed");
        assert!(
            start.elapsed() < std::time::Duration::from_secs(10),
            "derivation must respect the executor timeout, took {:?}",
            start.elapsed()
        );
        let msg = err.to_string();
        assert!(
            msg.contains("refusing to evaluate"),
            "expected fail-closed wording, got {msg:?}"
        );
        assert!(
            msg.contains("timed out"),
            "expected timeout detail, got {msg:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn evaluate_reports_denied_builtin_clearly() {
        let dir = tempfile::TempDir::new().unwrap();
        let script = write_fake_opa_script(
            dir.path(),
            "fake_opa_denied",
            "if [ \"$1\" = \"capabilities\" ]; then\n\
             \x20 echo '{\"builtins\":[{\"name\":\"eq\"}]}'\n\
             \x20 exit 0\n\
             fi\n\
             echo '1 error occurred: policy.rego:4: rego_type_error: undefined function http.send' >&2\n\
             exit 2\n",
        );
        let executor = OpaExecutor::new(Some(script.to_str().unwrap()), Some(5_000));
        let result = executor
            .evaluate(&[], &serde_json::json!({}))
            .expect("denied builtin surfaces as OpaResult error, not a hard Err");
        assert!(!result.success);
        let msg = result.error.as_deref().unwrap_or("");
        assert!(msg.contains("http.send"), "got {msg:?}");
        assert!(msg.contains("not permitted"), "got {msg:?}");
        assert!(msg.contains("CIB-108"), "got {msg:?}");
    }

    #[cfg(unix)]
    #[test]
    fn evaluate_propagates_stderr_on_nonzero_exit() {
        let dir = tempfile::TempDir::new().unwrap();
        let script = write_fake_opa_script(
            dir.path(),
            "fake_opa_fail",
            "if [ \"$1\" = \"capabilities\" ]; then\n\
             \x20 echo '{\"builtins\":[{\"name\":\"eq\"}]}'\n\
             \x20 exit 0\n\
             fi\n\
             echo 'boom: parse error' >&2\nexit 2\n",
        );
        let executor = OpaExecutor::new(Some(script.to_str().unwrap()), Some(5_000));
        let result = executor
            .evaluate(&[], &serde_json::json!({}))
            .expect("non-zero exit is returned as Ok with error set, not a hard Err");
        assert!(!result.success, "non-zero exit must set success=false");
        assert!(result.violations.is_empty());
        let err_msg = result.error.as_deref().unwrap_or("");
        assert!(
            err_msg.contains("boom: parse error"),
            "expected stderr text propagated, got {err_msg:?}"
        );
    }
}
