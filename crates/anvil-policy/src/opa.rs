use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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

#[derive(Debug, thiserror::Error)]
pub enum OpaError {
    #[error("OPA binary not found: {0}")]
    BinaryNotFound(String),
    #[error("OPA execution failed: {0}")]
    Execution(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("OPA timed out after {0}ms")]
    Timeout(u64),
}

pub struct OpaExecutor {
    binary_path: String,
    timeout_ms: u64,
    query: String,
}

impl OpaExecutor {
    pub fn new(binary_path: Option<&str>, timeout_ms: Option<u64>) -> Self {
        Self {
            binary_path: binary_path.unwrap_or("opa").to_string(),
            timeout_ms: timeout_ms.unwrap_or(30_000),
            query: "data.anvil.policies".to_string(),
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

        let mut child = Command::new(&self.binary_path)
            .arg("eval")
            .arg("--data")
            .arg(policy_dir.path())
            .arg("--input")
            .arg(&input_path)
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

        let timeout = std::time::Duration::from_millis(self.timeout_ms);
        if child.wait_timeout(timeout)?.is_none() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(OpaError::Timeout(self.timeout_ms));
        }
        let output = child.wait_with_output()?;

        #[allow(clippy::cast_possible_truncation)]
        let elapsed = start.elapsed().as_millis() as u64;

        let opa_version = self.version();

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Ok(OpaResult {
                success: false,
                violations: Vec::new(),
                metadata: OpaMetadata {
                    policy_count: policies.len(),
                    execution_time_ms: elapsed,
                    opa_version,
                },
                error: Some(stderr),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
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
        let mut args = vec!["test", "--format", "json"];
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
            if !stderr.is_empty() {
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

    fn extract_violations(
        &self,
        stdout: &str,
        policies: &[LoadedPolicy],
    ) -> Result<Vec<PolicyViolation>, OpaError> {
        let parsed: serde_json::Value = serde_json::from_str(stdout)?;
        let mut violations = Vec::new();

        let value = parsed
            .pointer("/result/0/expressions/0/value")
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        if let serde_json::Value::Object(map) = value {
            for (policy_name, policy_output) in &map {
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
    if let Ok(env_path) = std::env::var("ANVIL_OPA_PATH") {
        if !env_path.is_empty() {
            return Some(PathBuf::from(env_path));
        }
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
}
