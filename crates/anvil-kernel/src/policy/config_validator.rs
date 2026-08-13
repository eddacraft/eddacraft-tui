use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::Path;

use crate::policy::config::ArchitectureConfig;
use crate::policy::config_diagnostics::ArchitectureConfigDiagnostic;

/// Keep in sync with `anvil_architecture::ARCHITECTURE_YAML_MAX_SIZE`.
pub const ARCHITECTURE_CONFIG_MAX_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchitectureConfigValidationReport {
    pub valid: bool,
    pub diagnostics: Vec<ArchitectureConfigDiagnostic>,
}

#[derive(Debug, Default)]
pub struct ArchitectureConfigValidator;

#[derive(Debug)]
pub enum ArchitectureConfigValidationError {
    Parse(serde_yaml::Error),
    Invalid(Vec<ArchitectureConfigDiagnostic>),
}

impl std::fmt::Display for ArchitectureConfigValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "{error}"),
            Self::Invalid(diagnostics) => {
                let messages = diagnostics
                    .iter()
                    .filter(|diagnostic| diagnostic.is_error())
                    .map(|diagnostic| diagnostic.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; ");
                write!(f, "{messages}")
            }
        }
    }
}

impl std::error::Error for ArchitectureConfigValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(error) => Some(error),
            Self::Invalid(_) => None,
        }
    }
}

impl From<serde_yaml::Error> for ArchitectureConfigValidationError {
    fn from(error: serde_yaml::Error) -> Self {
        Self::Parse(error)
    }
}

impl ArchitectureConfigValidator {
    pub fn validate(&self, config: &ArchitectureConfig) -> ArchitectureConfigValidationReport {
        let mut diagnostics = Vec::new();

        validate_layer_names(config, &mut diagnostics);
        validate_layer_paths(config, &mut diagnostics);
        validate_allowed_imports(config, &mut diagnostics);

        let valid = !diagnostics
            .iter()
            .any(ArchitectureConfigDiagnostic::is_error);
        ArchitectureConfigValidationReport { valid, diagnostics }
    }
}

pub fn validate_architecture_config(
    config: &ArchitectureConfig,
) -> ArchitectureConfigValidationReport {
    ArchitectureConfigValidator.validate(config)
}

/// Read an architecture config file with the same hard size cap as
/// `anvil-architecture` YAML parsing.
pub fn read_architecture_config_capped(path: &Path) -> Result<String, std::io::Error> {
    let file = std::fs::File::open(path)?;
    let size = file.metadata()?.len();
    if size > ARCHITECTURE_CONFIG_MAX_BYTES {
        return Err(architecture_config_over_cap(path));
    }

    let mut contents = String::with_capacity(usize::try_from(size).unwrap_or(0));
    file.take(ARCHITECTURE_CONFIG_MAX_BYTES.saturating_add(1))
        .read_to_string(&mut contents)?;
    if contents.len() as u64 > ARCHITECTURE_CONFIG_MAX_BYTES {
        return Err(architecture_config_over_cap(path));
    }
    Ok(contents)
}

fn architecture_config_over_cap(path: &Path) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!(
            "{} exceeds the {ARCHITECTURE_CONFIG_MAX_BYTES}-byte read cap",
            path.display()
        ),
    )
}

pub fn parse_validated_architecture_config(
    yaml: &str,
) -> Result<ArchitectureConfig, ArchitectureConfigValidationError> {
    let config = ArchitectureConfig::from_yaml_any(yaml)?;
    let report = validate_architecture_config(&config);
    if report.valid {
        Ok(config)
    } else {
        Err(ArchitectureConfigValidationError::Invalid(
            report
                .diagnostics
                .into_iter()
                .filter(ArchitectureConfigDiagnostic::is_error)
                .collect(),
        ))
    }
}

fn validate_layer_names(
    config: &ArchitectureConfig,
    diagnostics: &mut Vec<ArchitectureConfigDiagnostic>,
) {
    let mut seen = HashSet::new();
    for layer in &config.layers {
        let name = layer.name.trim();
        if name.is_empty() {
            diagnostics.push(ArchitectureConfigDiagnostic::error(
                "empty-layer-name",
                "Layer names must not be empty",
                "layers",
                "name",
            ));
            continue;
        }

        if !seen.insert(name.to_string()) {
            diagnostics.push(ArchitectureConfigDiagnostic::error(
                "duplicate-layer-name",
                format!("Layer name \"{name}\" is defined more than once"),
                format!("layers.{name}"),
                name,
            ));
        }
    }
}

fn validate_layer_paths(
    config: &ArchitectureConfig,
    diagnostics: &mut Vec<ArchitectureConfigDiagnostic>,
) {
    for layer in &config.layers {
        if layer.paths.is_empty() {
            diagnostics.push(ArchitectureConfigDiagnostic::warning(
                "empty-layer",
                format!("Layer \"{}\" does not match any paths", layer.name),
                format!("layers.{}.paths", layer.name),
                "paths",
            ));
        }

        for path in &layer.paths {
            if path.trim().is_empty() {
                diagnostics.push(ArchitectureConfigDiagnostic::error(
                    "empty-layer-path",
                    format!("Layer \"{}\" contains an empty path pattern", layer.name),
                    format!("layers.{}.paths", layer.name),
                    "paths",
                ));
            }
        }
    }

    for (left_index, left) in config.layers.iter().enumerate() {
        for right in config.layers.iter().skip(left_index + 1) {
            for left_path in &left.paths {
                for right_path in &right.paths {
                    if path_patterns_overlap(left_path, right_path) {
                        diagnostics.push(ArchitectureConfigDiagnostic::error(
                            "overlapping-layer-paths",
                            format!(
                                "Layer \"{}\" path \"{}\" overlaps with layer \"{}\" path \"{}\"",
                                left.name, left_path, right.name, right_path
                            ),
                            format!("layers.{}.paths", right.name),
                            right_path,
                        ));
                    }
                }
            }
        }
    }
}

fn validate_allowed_imports(
    config: &ArchitectureConfig,
    diagnostics: &mut Vec<ArchitectureConfigDiagnostic>,
) {
    let layer_names: HashSet<&str> = config
        .layers
        .iter()
        .map(|layer| layer.name.as_str())
        .collect();
    let mut per_layer_seen: HashMap<&str, HashSet<&str>> = HashMap::new();

    for layer in &config.layers {
        for allowed in &layer.allowed_imports {
            if !layer_names.contains(allowed.as_str()) {
                diagnostics.push(ArchitectureConfigDiagnostic::error(
                    "unknown-allowed-import",
                    format!(
                        "Layer \"{}\" allows imports from unknown layer \"{}\"",
                        layer.name, allowed
                    ),
                    format!("layers.{}.allowed_imports", layer.name),
                    allowed,
                ));
            }

            let seen = per_layer_seen.entry(layer.name.as_str()).or_default();
            if !seen.insert(allowed.as_str()) {
                diagnostics.push(ArchitectureConfigDiagnostic::warning(
                    "duplicate-allowed-import",
                    format!(
                        "Layer \"{}\" repeats allowed import \"{}\"",
                        layer.name, allowed
                    ),
                    format!("layers.{}.allowed_imports", layer.name),
                    allowed,
                ));
            }
        }
    }
}

fn path_patterns_overlap(left: &str, right: &str) -> bool {
    let left = pattern_prefix(left);
    let right = pattern_prefix(right);

    !left.is_empty() && !right.is_empty() && (left.starts_with(&right) || right.starts_with(&left))
}

fn pattern_prefix(pattern: &str) -> String {
    let mut normalised = pattern.trim().replace('\\', "/");
    for suffix in ["**/*", "**", "*"] {
        if let Some(prefix) = normalised.strip_suffix(suffix) {
            normalised = prefix.to_string();
            break;
        }
    }
    normalised
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validate(yaml: &str) -> ArchitectureConfigValidationReport {
        let config = ArchitectureConfig::from_yaml(yaml).unwrap();
        validate_architecture_config(&config)
    }

    #[test]
    fn architecture_config_validator_accepts_typical_config() {
        let report = validate(
            r#"
layers:
  - name: domain
    paths: ["src/domain/*"]
    allowed_imports: [domain]
  - name: infrastructure
    paths: ["src/infra/*"]
    allowed_imports: [domain, infrastructure]
"#,
        );

        assert!(report.valid);
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn architecture_config_validator_blocks_duplicate_layer_names() {
        let report = validate(
            r#"
layers:
  - name: domain
    paths: ["src/domain/*"]
  - name: domain
    paths: ["src/core/*"]
"#,
        );

        assert!(!report.valid);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.code == "duplicate-layer-name"
                    && d.section == "layers.domain"
                    && d.is_error())
        );
    }

    #[test]
    fn architecture_config_validator_blocks_unknown_allowed_imports() {
        let report = validate(
            r#"
layers:
  - name: api
    paths: ["src/api/*"]
    allowed_imports: [domain]
"#,
        );

        assert!(!report.valid);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.code == "unknown-allowed-import" && d.key == "domain")
        );
    }

    #[test]
    fn architecture_config_validator_blocks_overlapping_paths() {
        let report = validate(
            r#"
layers:
  - name: app
    paths: ["src/*"]
  - name: ui
    paths: ["src/ui/*"]
"#,
        );

        assert!(!report.valid);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.code == "overlapping-layer-paths"
                    && d.message.contains("app")
                    && d.message.contains("ui"))
        );
    }

    #[test]
    fn architecture_config_validator_warns_for_empty_layers() {
        let report = validate(
            r"
layers:
  - name: empty
    paths: []
",
        );

        assert!(report.valid);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.code == "empty-layer" && !d.is_error())
        );
    }

    #[test]
    fn read_architecture_config_capped_rejects_oversized_file() {
        const OVERSIZE_BYTES: usize = 1024 * 1024 + 1;
        assert_eq!(ARCHITECTURE_CONFIG_MAX_BYTES, 1024 * 1024);

        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("architecture.yaml");
        std::fs::write(&path, vec![b'a'; OVERSIZE_BYTES]).unwrap();

        let err = read_architecture_config_capped(&path).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("read cap"));
    }

    #[test]
    fn architecture_config_validator_parse_validated_rejects_invalid_config() {
        let err = parse_validated_architecture_config(
            r#"
layers:
  - name: api
    paths: ["src/api/*"]
    allowed_imports: [domain]
"#,
        )
        .unwrap_err();

        assert!(err.to_string().contains("unknown layer"));
    }
}
