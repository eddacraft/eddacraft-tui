//! INTR-007: rule configuration — build a populated [`RuleRegistry`]
//! from the `.anvil.<ext>` `enforcement.intercept-rules` block so
//! projects can declare deny lists, regex patterns, and which built-in
//! rules are enabled without code changes.
//!
//! Defaults (absent file or absent block): secret detection enabled,
//! antipattern scanning disabled, no path-deny patterns, no
//! regex-content patterns. Malformed config returns a typed
//! [`RuleConfigError`] rather than silently degrading to defaults
//! (operator-config no-silent-defaults rule); only a genuinely missing
//! file/block folds into the defaults.
//!
//! Globs and regexes are compiled once here, at construction, and cached
//! inside the rule instances for their lifetime — the hot path never
//! recompiles.
//!
//! Accepted shape (yaml shown; json/toml equivalents per
//! `anvil_config::discover` precedence):
//!
//! ```yaml
//! enforcement:
//!   intercept-rules:
//!     secret-detection:
//!       enabled: true        # default true
//!     antipattern:
//!       enabled: true        # default false
//!     path-deny:
//!       patterns: ["**/.env*", "secrets/**"]
//!     regex-content:
//!       patterns: ["FORBIDDEN_TOKEN"]
//! ```
//!
//! `secret-detection: false` / `antipattern: true` boolean shorthands
//! are also accepted. Unknown keys inside `intercept-rules` — including
//! inside the per-rule objects — are typed errors, not ignored: a typo
//! must not silently disable a rule.
//!
//! `antipattern: true` registers [`AntipatternScanRule::default()`]
//! (all default patterns, `Error` severity threshold); per-operator
//! threshold/pattern tuning is deliberately not exposed in v1 (per-rule
//! granularity is out of module scope). A config that explicitly
//! disables every rule and configures no patterns yields an **empty
//! registry that allows everything** — callers that consider that a
//! misconfiguration should check [`RuleRegistry::is_empty`] at startup
//! and warn.

use std::path::Path;

use serde_json::Value;

use crate::InterceptRule;
use crate::antipattern::AntipatternScanRule;
use crate::path_deny::{PathDenyConfig, PathDenyError, PathDenyListRule};
use crate::regex_content::{RegexContentConfig, RegexContentError, RegexContentRule};
use crate::registry::{RegistryError, RuleRegistry};
use crate::secret::SecretDetectionRule;

/// Error building a rule registry from configuration.
#[derive(Debug, thiserror::Error)]
pub enum RuleConfigError {
    /// Config discovery hit an io error (e.g. permission denied on the
    /// workspace directory). Distinct from "no config exists", which
    /// folds into defaults.
    #[error("failed to discover intercept-rules config: {0}")]
    Discover(#[from] std::io::Error),
    /// The discovered config file failed to parse.
    #[error(transparent)]
    Parse(#[from] anvil_config::ParseError),
    /// The config parsed but the `enforcement.intercept-rules` block is
    /// malformed — wrong type, unknown rule key, or invalid entry.
    #[error("invalid intercept-rules config at `{path}`: {reason}")]
    Invalid { path: String, reason: String },
    /// A configured path-deny glob failed to compile.
    #[error(transparent)]
    PathDeny(#[from] PathDenyError),
    /// A configured regex-content pattern failed to compile.
    #[error(transparent)]
    RegexContent(#[from] RegexContentError),
    /// Registry assembly failed (duplicate rule ids — unreachable with
    /// the fixed built-in set, but typed rather than unwrapped).
    #[error(transparent)]
    Registry(#[from] RegistryError),
}

/// Typed view of the `enforcement.intercept-rules` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterceptRulesConfig {
    /// Register the INTR-002 secret-detection wrapper. Default `true`.
    pub secret_detection: bool,
    /// Register the INTR-003 antipattern wrapper. Default `false` —
    /// the hot-path default stays minimal until a project opts in.
    pub antipattern: bool,
    /// Glob patterns for the INTR-004 path-deny rule. Empty = rule not
    /// registered.
    pub path_deny_patterns: Vec<String>,
    /// Regex patterns for the INTR-005 regex-content rule. Empty = rule
    /// not registered.
    pub regex_content_patterns: Vec<String>,
}

impl Default for InterceptRulesConfig {
    fn default() -> Self {
        Self {
            secret_detection: true,
            antipattern: false,
            path_deny_patterns: Vec::new(),
            regex_content_patterns: Vec::new(),
        }
    }
}

impl InterceptRulesConfig {
    /// Parse a full config document (the root value of `.anvil.<ext>`).
    ///
    /// `Null` (an empty config file) and an absent `enforcement` or
    /// `intercept-rules` block all fold into [`Self::default`]; any
    /// present-but-malformed value is a typed error.
    pub fn from_value(value: &Value) -> Result<Self, RuleConfigError> {
        if value.is_null() {
            return Ok(Self::default());
        }
        let Some(root) = value.as_object() else {
            return Err(invalid("config", "expected an object"));
        };
        let Some(enforcement) = root.get("enforcement") else {
            return Ok(Self::default());
        };
        let Some(enforcement) = enforcement.as_object() else {
            return Err(invalid("enforcement", "expected an object"));
        };
        let Some(block) = enforcement.get("intercept-rules") else {
            return Ok(Self::default());
        };
        let Some(block) = block.as_object() else {
            return Err(invalid("enforcement.intercept-rules", "expected an object"));
        };

        let mut config = Self::default();
        for (key, entry) in block {
            let path = format!("enforcement.intercept-rules.{key}");
            match key.as_str() {
                "secret-detection" => config.secret_detection = parse_enabled(entry, &path)?,
                "antipattern" => config.antipattern = parse_enabled(entry, &path)?,
                "path-deny" => config.path_deny_patterns = parse_patterns(entry, &path)?,
                "regex-content" => config.regex_content_patterns = parse_patterns(entry, &path)?,
                _ => {
                    return Err(invalid(
                        &path,
                        "unknown rule key; expected one of secret-detection, \
                         antipattern, path-deny, regex-content",
                    ));
                }
            }
        }
        Ok(config)
    }

    /// Compile the configured patterns and assemble the registry.
    ///
    /// Registration order is fixed and deterministic: path-deny first
    /// (path-only, lets the registry skip content reads when nothing
    /// else is registered), then secret-detection, antipattern, and
    /// regex-content.
    pub fn into_registry(self) -> Result<RuleRegistry, RuleConfigError> {
        let mut rules: Vec<Box<dyn InterceptRule>> = Vec::new();
        if !self.path_deny_patterns.is_empty() {
            rules.push(Box::new(PathDenyListRule::new(PathDenyConfig::new(
                self.path_deny_patterns,
            ))?));
        }
        if self.secret_detection {
            rules.push(Box::new(SecretDetectionRule::default()));
        }
        if self.antipattern {
            rules.push(Box::new(AntipatternScanRule::default()));
        }
        if !self.regex_content_patterns.is_empty() {
            rules.push(Box::new(RegexContentRule::new(RegexContentConfig::new(
                self.regex_content_patterns,
            ))?));
        }
        Ok(RuleRegistry::with_rules(rules)?)
    }
}

/// Build a registry from the config document rooted at `value`.
pub fn registry_from_value(value: &Value) -> Result<RuleRegistry, RuleConfigError> {
    InterceptRulesConfig::from_value(value)?.into_registry()
}

/// Discover `.anvil.<ext>` in `workspace_root` (yaml → yml → json →
/// toml precedence per [`anvil_config::discover`]), parse it, and build
/// the registry. No config file at all is not an error — it folds into
/// the default registry (secret detection only).
pub fn registry_from_workspace(workspace_root: &Path) -> Result<RuleRegistry, RuleConfigError> {
    match anvil_config::discover(workspace_root, ".anvil")? {
        None => InterceptRulesConfig::default().into_registry(),
        Some(discovered) => registry_from_value(&anvil_config::parse_file(&discovered.path)?),
    }
}

fn invalid(path: &str, reason: &str) -> RuleConfigError {
    RuleConfigError::Invalid {
        path: path.to_string(),
        reason: reason.to_string(),
    }
}

/// Accept `true` / `false` or `{ enabled: bool }`. Extra keys inside the
/// object are typed errors — a typo like `enabld` must not silently fall
/// back to the default.
fn parse_enabled(entry: &Value, path: &str) -> Result<bool, RuleConfigError> {
    if let Some(flag) = entry.as_bool() {
        return Ok(flag);
    }
    if let Some(object) = entry.as_object() {
        reject_unknown_keys(object, &["enabled"], path)?;
        let Some(enabled) = object.get("enabled") else {
            return Err(invalid(
                &format!("{path}.enabled"),
                "missing `enabled` flag",
            ));
        };
        return enabled
            .as_bool()
            .ok_or_else(|| invalid(&format!("{path}.enabled"), "expected a boolean"));
    }
    Err(invalid(
        path,
        "expected a boolean or an object with boolean `enabled`",
    ))
}

/// Reject keys outside `allowed` so a typo inside a rule object is a
/// typed error rather than a silently ignored setting.
fn reject_unknown_keys(
    object: &serde_json::Map<String, Value>,
    allowed: &[&str],
    path: &str,
) -> Result<(), RuleConfigError> {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(invalid(
                &format!("{path}.{key}"),
                &format!("unknown key; expected only {}", allowed.join(", ")),
            ));
        }
    }
    Ok(())
}

/// Accept `[string]` or `{ patterns: [string] }`. Extra keys inside the
/// object are typed errors.
fn parse_patterns(entry: &Value, path: &str) -> Result<Vec<String>, RuleConfigError> {
    let (list, path) = if let Some(list) = entry.as_array() {
        (list, path.to_string())
    } else if let Some(object) = entry.as_object() {
        reject_unknown_keys(object, &["patterns"], path)?;
        let path = format!("{path}.patterns");
        let Some(patterns) = object.get("patterns") else {
            return Err(invalid(&path, "missing `patterns` list"));
        };
        let Some(list) = patterns.as_array() else {
            return Err(invalid(&path, "expected a list of strings"));
        };
        (list, path)
    } else {
        return Err(invalid(
            path,
            "expected a list of strings or an object with a `patterns` list",
        ));
    };

    list.iter()
        .map(|item| {
            item.as_str()
                .map(str::to_string)
                .ok_or_else(|| invalid(&path, "expected every pattern to be a string"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::json;

    use super::*;
    use crate::secret::SECRET_RULE_ID;
    use crate::{ChangeKind, RegistryDecision, RuleInput};

    #[test]
    fn absent_config_builds_default_registry() {
        let workspace = tempfile::TempDir::new().expect("tempdir");

        let registry = registry_from_workspace(workspace.path()).expect("default registry");

        assert_eq!(registry.rule_ids(), vec![SECRET_RULE_ID]);
        assert!(registry.any_needs_content());
    }

    #[test]
    fn absent_blocks_fold_into_defaults() {
        for value in [json!(null), json!({}), json!({ "enforcement": {} })] {
            let config = InterceptRulesConfig::from_value(&value).expect("defaults");
            assert_eq!(config, InterceptRulesConfig::default());
        }
    }

    #[test]
    fn populated_config_constructs_all_enabled_rules() {
        let registry = registry_from_value(&json!({
            "enforcement": {
                "intercept-rules": {
                    "secret-detection": { "enabled": true },
                    "antipattern": { "enabled": true },
                    "path-deny": { "patterns": ["**/.env*"] },
                    "regex-content": { "patterns": ["FORBIDDEN_TOKEN"] },
                }
            }
        }))
        .expect("populated registry");

        assert_eq!(
            registry.rule_ids(),
            vec![
                "path-deny",
                "secret-detection",
                "antipattern-scan",
                "regex-content"
            ]
        );
    }

    #[test]
    fn boolean_shorthand_and_disabling_secret_detection_work() {
        let registry = registry_from_value(&json!({
            "enforcement": {
                "intercept-rules": {
                    "secret-detection": false,
                    "antipattern": true,
                }
            }
        }))
        .expect("registry");

        assert_eq!(registry.rule_ids(), vec!["antipattern-scan"]);
    }

    #[test]
    fn malformed_config_is_a_typed_error_not_a_silent_default() {
        let cases = [
            (json!([]), "config"),
            (json!({ "enforcement": [] }), "enforcement"),
            (
                json!({ "enforcement": { "intercept-rules": [] } }),
                "enforcement.intercept-rules",
            ),
            (
                json!({ "enforcement": { "intercept-rules": { "path-deny": "nope" } } }),
                "enforcement.intercept-rules.path-deny",
            ),
            (
                json!({ "enforcement": { "intercept-rules": { "path-deny": { "patterns": [1] } } } }),
                "enforcement.intercept-rules.path-deny.patterns",
            ),
            (
                json!({ "enforcement": { "intercept-rules": { "secret-detection": "yes" } } }),
                "enforcement.intercept-rules.secret-detection",
            ),
            (
                json!({ "enforcement": { "intercept-rules": { "regex-contnet": {} } } }),
                "enforcement.intercept-rules.regex-contnet",
            ),
            (
                json!({ "enforcement": { "intercept-rules": { "secret-detection": {} } } }),
                "enforcement.intercept-rules.secret-detection.enabled",
            ),
            (
                json!({ "enforcement": { "intercept-rules": { "path-deny": {} } } }),
                "enforcement.intercept-rules.path-deny.patterns",
            ),
            (
                json!({ "enforcement": { "intercept-rules": {
                    "antipattern": { "enabled": true, "sevrity": "error" }
                } } }),
                "enforcement.intercept-rules.antipattern.sevrity",
            ),
            (
                json!({ "enforcement": { "intercept-rules": {
                    "path-deny": { "patterns": [], "pattern": [] }
                } } }),
                "enforcement.intercept-rules.path-deny.pattern",
            ),
        ];

        for (value, expected_path) in cases {
            let err = registry_from_value(&value).expect_err("malformed config must error");
            match err {
                RuleConfigError::Invalid { path, .. } => {
                    assert_eq!(path, expected_path, "for value {value}");
                }
                other => panic!("expected Invalid for {value}, got {other:?}"),
            }
        }
    }

    #[test]
    fn invalid_patterns_surface_as_typed_compile_errors() {
        let glob_err = registry_from_value(&json!({
            "enforcement": { "intercept-rules": { "path-deny": { "patterns": ["["] } } }
        }))
        .expect_err("bad glob");
        assert!(matches!(glob_err, RuleConfigError::PathDeny(_)));

        let regex_err = registry_from_value(&json!({
            "enforcement": { "intercept-rules": { "regex-content": { "patterns": ["(unclosed"] } } }
        }))
        .expect_err("bad regex");
        assert!(matches!(regex_err, RuleConfigError::RegexContent(_)));
    }

    #[test]
    fn constructed_registry_interrupts_on_configured_content() {
        let registry = registry_from_value(&json!({
            "enforcement": {
                "intercept-rules": {
                    "secret-detection": false,
                    "regex-content": { "patterns": ["FORBIDDEN_TOKEN"] },
                }
            }
        }))
        .expect("registry");

        let path = Path::new("src/api.rs");
        let body = b"const X: &str = \"FORBIDDEN_TOKEN\";\n";
        let decision = registry.evaluate(&RuleInput {
            path,
            change_kind: ChangeKind::Modified,
            content: Some(body),
        });

        match decision {
            RegistryDecision::Interrupt(reason) => {
                assert_eq!(reason.rule_id, "regex-content");
                assert!(reason.message.contains("FORBIDDEN_TOKEN"));
            }
            RegistryDecision::Allow => panic!("configured pattern should interrupt"),
        }
    }

    #[test]
    fn discovered_yaml_config_round_trips_through_the_registry() {
        let workspace = tempfile::TempDir::new().expect("tempdir");
        std::fs::write(
            workspace.path().join(".anvil.yaml"),
            "enforcement:\n  intercept-rules:\n    path-deny:\n      patterns:\n        - \"**/.env*\"\n",
        )
        .expect("write config");

        let registry = registry_from_workspace(workspace.path()).expect("registry");

        assert_eq!(registry.rule_ids(), vec!["path-deny", SECRET_RULE_ID]);

        let path = Path::new("config/.env");
        let decision = registry.evaluate(&RuleInput {
            path,
            change_kind: ChangeKind::Created,
            content: None,
        });
        assert!(matches!(decision, RegistryDecision::Interrupt(_)));
    }
}
