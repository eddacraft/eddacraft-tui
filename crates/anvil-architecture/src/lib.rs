// Anvil architecture enforcement — boundary definitions, import rules, drift detection.

use std::path::Path;

use anyhow::{Context, Result};

/// A single architecture violation.
#[derive(Debug, Clone)]
pub struct ArchViolation {
    pub rule: String,
    pub file: String,
    pub message: String,
}

/// Result of validating the architecture configuration.
#[derive(Debug)]
pub struct ValidationResult {
    pub valid: bool,
    pub violations: Vec<ArchViolation>,
}

/// Validate the architecture configuration at `project_root/.anvil/architecture.yaml`.
///
/// For the beta release this only checks that the YAML is parseable.
/// Full boundary checking is deferred to a later phase.
pub fn validate(project_root: &Path) -> Result<ValidationResult> {
    let config_path = project_root.join(".anvil/architecture.yaml");

    if !config_path.exists() {
        return Ok(ValidationResult {
            valid: true,
            violations: vec![],
        });
    }

    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;

    // Verify the file is valid YAML by deserialising to a generic value.
    let _value: serde_yaml::Value = serde_yaml::from_str(&content)
        .with_context(|| format!("invalid YAML in {}", config_path.display()))?;

    Ok(ValidationResult {
        valid: true,
        violations: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_config_returns_valid() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = validate(tmp.path()).unwrap();
        assert!(result.valid);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn valid_yaml_returns_valid() {
        let tmp = tempfile::TempDir::new().unwrap();
        let anvil_dir = tmp.path().join(".anvil");
        std::fs::create_dir_all(&anvil_dir).unwrap();
        std::fs::write(
            anvil_dir.join("architecture.yaml"),
            "boundaries:\n  - name: core\n    path: src/core\n",
        )
        .unwrap();
        let result = validate(tmp.path()).unwrap();
        assert!(result.valid);
    }

    #[test]
    fn invalid_yaml_returns_error() {
        let tmp = tempfile::TempDir::new().unwrap();
        let anvil_dir = tmp.path().join(".anvil");
        std::fs::create_dir_all(&anvil_dir).unwrap();
        std::fs::write(
            anvil_dir.join("architecture.yaml"),
            ":\n  :\n  - :\n  bad: [",
        )
        .unwrap();
        let result = validate(tmp.path());
        assert!(result.is_err());
    }
}
