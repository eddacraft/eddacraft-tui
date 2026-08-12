//! The `gate` section of the unified project config (UCFG-004,
//! ADR-120 pt 4).
//!
//! Gate *composition* lives here — per-check configuration tables,
//! optional score thresholds, and gate-global configuration — while
//! check *selection* stays the top-level `checks` list that gate runs
//! already treat as authoritative. One truth per concern: a check is
//! selected by membership in `checks` (or, when that list is absent,
//! by having an entry in `gate.checks`); the section never carries a
//! second `enabled` flag that could disagree with the list.
//!
//! The schema is additive-tolerant: unknown keys inside `gate` are
//! ignored (never a parse error) so later work can grow the section
//! without breaking older binaries.

use std::collections::BTreeMap;

use serde_json::Value;

/// Typed view of the `gate` section of a parsed project config value.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GateSection {
    /// Optional schema marker carried over from `gate-config.json`'s
    /// integer `version`. Purely informational.
    pub version: Option<u64>,
    /// Named score thresholds (e.g. `overall_score`). Reserved: no
    /// gate run consumes these yet; the fold (UCFG-005) preserves
    /// them so operator intent is not dropped.
    pub thresholds: BTreeMap<String, u64>,
    /// Gate-global configuration table.
    pub global_config: Option<BTreeMap<String, Value>>,
    /// Per-check configuration tables, keyed by canonical check name.
    /// Key presence doubles as selection only when the top-level
    /// `checks` list is absent.
    pub checks: BTreeMap<String, BTreeMap<String, Value>>,
}

/// Shape errors for a present-but-malformed `gate` section. Absence is
/// `Ok(None)`, never an error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GateSectionError {
    #[error("invalid gate: expected an object, found {found}")]
    NotAnObject { found: &'static str },

    #[error("invalid gate.{path}: expected {expected}, found {found}")]
    WrongType {
        path: String,
        expected: &'static str,
        found: &'static str,
    },
}

fn kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

impl GateSection {
    /// Read the `gate` key from a parsed project-config value.
    /// `Ok(None)` when the key is absent or null.
    pub fn from_config_value(config: &Value) -> Result<Option<Self>, GateSectionError> {
        let Some(gate) = config.get("gate") else {
            return Ok(None);
        };
        if gate.is_null() {
            return Ok(None);
        }
        let obj = gate
            .as_object()
            .ok_or(GateSectionError::NotAnObject { found: kind(gate) })?;

        let version = match obj.get("version") {
            None | Some(Value::Null) => None,
            Some(v) => Some(v.as_u64().ok_or_else(|| GateSectionError::WrongType {
                path: "version".to_string(),
                expected: "unsigned integer",
                found: kind(v),
            })?),
        };

        let mut thresholds = BTreeMap::new();
        match obj.get("thresholds") {
            None | Some(Value::Null) => {}
            Some(v) => {
                let table = v.as_object().ok_or_else(|| GateSectionError::WrongType {
                    path: "thresholds".to_string(),
                    expected: "table of unsigned integers",
                    found: kind(v),
                })?;
                for (name, raw) in table {
                    let score = raw.as_u64().ok_or_else(|| GateSectionError::WrongType {
                        path: format!("thresholds.{name}"),
                        expected: "unsigned integer",
                        found: kind(raw),
                    })?;
                    thresholds.insert(name.clone(), score);
                }
            }
        }

        let global_config = match obj.get("global_config") {
            None | Some(Value::Null) => None,
            Some(v) => {
                let table = v.as_object().ok_or_else(|| GateSectionError::WrongType {
                    path: "global_config".to_string(),
                    expected: "table",
                    found: kind(v),
                })?;
                Some(table.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            }
        };

        let mut checks = BTreeMap::new();
        match obj.get("checks") {
            None | Some(Value::Null) => {}
            Some(v) => {
                let table = v.as_object().ok_or_else(|| GateSectionError::WrongType {
                    path: "checks".to_string(),
                    expected: "table of per-check config tables",
                    found: kind(v),
                })?;
                for (name, raw) in table {
                    let config = match raw {
                        // `check-name: {}` and `check-name:` (null) are both
                        // "selected, no config".
                        Value::Null => BTreeMap::new(),
                        Value::Object(map) => {
                            map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                        }
                        other => {
                            return Err(GateSectionError::WrongType {
                                path: format!("checks.{name}"),
                                expected: "table (or empty)",
                                found: kind(other),
                            });
                        }
                    };
                    checks.insert(name.clone(), config);
                }
            }
        }

        Ok(Some(Self {
            version,
            thresholds,
            global_config,
            checks,
        }))
    }

    /// The check names carried by the section, sorted (BTreeMap order).
    /// Selection input only when the top-level `checks` list is absent.
    #[must_use]
    pub fn check_names(&self) -> Vec<String> {
        self.checks.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConfigFormat, parse_str};
    use serde_json::json;

    #[test]
    fn absent_or_null_gate_is_none() {
        assert_eq!(
            GateSection::from_config_value(&json!({"checks": ["a"]})).unwrap(),
            None
        );
        assert_eq!(
            GateSection::from_config_value(&json!({"gate": null})).unwrap(),
            None
        );
    }

    #[test]
    fn full_section_parses_with_all_fields() {
        let v = json!({
            "gate": {
                "version": 1,
                "thresholds": {"overall_score": 80},
                "global_config": {"strict": true},
                "checks": {
                    "secret-detection": {"max_findings": 0},
                    "antipattern-scan": {},
                    "import-boundaries": null,
                },
            }
        });
        let s = GateSection::from_config_value(&v).unwrap().unwrap();
        assert_eq!(s.version, Some(1));
        assert_eq!(s.thresholds["overall_score"], 80);
        assert_eq!(s.global_config.as_ref().unwrap()["strict"], json!(true));
        assert_eq!(
            s.check_names(),
            vec!["antipattern-scan", "import-boundaries", "secret-detection"]
        );
        assert_eq!(s.checks["secret-detection"]["max_findings"], json!(0),);
        assert!(s.checks["import-boundaries"].is_empty());
    }

    #[test]
    fn unknown_keys_inside_gate_are_tolerated() {
        let v = json!({"gate": {"future_field": {"x": 1}, "checks": {"a": {}}}});
        let s = GateSection::from_config_value(&v).unwrap().unwrap();
        assert_eq!(s.check_names(), vec!["a"]);
    }

    #[test]
    fn wrong_shapes_error_with_dotted_paths() {
        let cases = [
            (json!({"gate": []}), "expected an object"),
            (json!({"gate": {"version": "one"}}), "gate.version"),
            (
                json!({"gate": {"thresholds": {"overall": "high"}}}),
                "gate.thresholds.overall",
            ),
            (
                json!({"gate": {"checks": {"a": ["nope"]}}}),
                "gate.checks.a",
            ),
            (json!({"gate": {"global_config": 3}}), "gate.global_config"),
        ];
        for (v, needle) in cases {
            let err = GateSection::from_config_value(&v).unwrap_err();
            assert!(
                err.to_string().contains(needle),
                "expected {needle} in: {err}"
            );
        }
    }

    /// Cross-format round-trip (UCFG-004 validation): the same logical
    /// section parses identically from yaml, json, and toml spellings.
    #[test]
    fn section_parses_identically_across_formats() {
        let yaml = "checks: [\"secret-detection\"]\ngate:\n  version: 1\n  thresholds:\n    overall_score: 80\n  checks:\n    secret-detection:\n      max_findings: 0\n";
        let jsonc = r#"{"checks":["secret-detection"],"gate":{"version":1,"thresholds":{"overall_score":80},"checks":{"secret-detection":{"max_findings":0}}}}"#;
        let toml = "checks = [\"secret-detection\"]\n[gate]\nversion = 1\n[gate.thresholds]\noverall_score = 80\n[gate.checks.secret-detection]\nmax_findings = 0\n";
        let parsed: Vec<GateSection> = [
            (yaml, ConfigFormat::Yaml),
            (jsonc, ConfigFormat::Json),
            (toml, ConfigFormat::Toml),
        ]
        .into_iter()
        .map(|(body, format)| {
            let value = parse_str(body, format, std::path::Path::new("fixture"))
                .unwrap_or_else(|e| panic!("{format:?}: {e}"));
            GateSection::from_config_value(&value)
                .unwrap_or_else(|e| panic!("{format:?}: {e}"))
                .unwrap_or_else(|| panic!("{format:?}: section expected"))
        })
        .collect();
        assert_eq!(parsed[0], parsed[1]);
        assert_eq!(parsed[1], parsed[2]);
    }
}
