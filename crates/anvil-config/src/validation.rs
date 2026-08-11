//! MLP-013: hard-pinned rule classes.
//!
//! Some rule classes — `secrets` and `command-safety` — cannot be
//! disabled via config. Per ADR-039 the enforcement is at the
//! **parser** level: a config that attempts to disable a hard-pinned
//! class fails parse rather than producing a runtime "rule was
//! silently disabled" state. The same pattern is used by ADR-015
//! ambiguous-ownership hard-caps.
//!
//! Tuning a hard-pinned class is still allowed (severity, mode where
//! `mode` is not `disabled`, custom params). Only the act of turning
//! the class off is refused. Per-finding `@anvil-ignore` (ADR-004) is
//! a separate channel and continues to work — the parser doesn't
//! intercept it.

use serde_json::Value;

/// The names of hard-pinned rule classes. The spec pins these via
/// ADR-039; changing the list requires an ADR amendment.
///
/// Listed as an array constant rather than a typed enum so future
/// rule classes can be added without forcing a downstream API break
/// — consumers compare strings, not enum variants.
pub const HARD_PINNED_CLASSES: &[&str] = &["secrets", "command-safety"];

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error(
        "hard-pinned rule class `{class}` cannot be disabled at \
         `{path}` — set `enabled: true` or remove the override \
         entirely. ADR-039 §D-2 pins this class for safety reasons; \
         per-finding suppression via `@anvil-ignore` (ADR-004) remains \
         available."
    )]
    HardPinnedDisabled { class: String, path: String },
    #[error(
        "hard-pinned rule class `{class}` cannot be set to mode \
         `{mode}` at `{path}` — `disabled` / `off` modes are not \
         accepted for this class. ADR-039 §D-2."
    )]
    HardPinnedModeDisabled {
        class: String,
        mode: String,
        path: String,
    },
}

/// Validate a parsed config (`serde_json::Value`) against the
/// hard-pinned rule-class policy.
///
/// Recognised disable-attempt shapes (all rejected):
///
/// 1. `{ enforcement: { rules: { <class>: { enabled: false } } } }`
/// 2. `{ enforcement: { rules: { <class>: false } } }`
/// 3. `{ enforcement: { rules: { <class>: { mode: "disabled" | "off" | "none" } } } }`
/// 4. `{ rules: { <class>: { enabled: false } } }` (legacy flat shape)
/// 5. `{ rules: { <class>: false } }` (legacy flat shape)
/// 6. `{ enforcement: { rules: { <class>: "disabled" | "off" | "none" } } }`
///    (scalar mode syntax also accepted by `RuleMode::parse` / `apply_rule`)
/// 7. `{ rules: { <class>: "disabled" | "off" | "none" } }` (legacy scalar)
///
/// Anything else under a hard-pinned class is accepted as a tuning
/// (severity, custom params, `mode: "warn"`/`"block"`, etc.). The
/// permissive default is deliberate — operators should be free to
/// tighten or soften enforcement, just not turn it off.
pub fn validate_hard_pinned_classes(value: &Value) -> Result<(), ValidationError> {
    let Some(root) = value.as_object() else {
        return Ok(());
    };

    // Walk both the canonical and legacy locations. Each call yields
    // the (object, path-prefix) pair where the per-class entries
    // live, or None when the layer doesn't exist.
    for (rules_obj, prefix) in canonical_and_legacy_rules(root) {
        check_rules_object(rules_obj, &prefix)?;
    }
    Ok(())
}

/// Yield each `rules: { ... }` block we care about, along with the
/// path prefix for diagnostics.
fn canonical_and_legacy_rules(
    root: &serde_json::Map<String, Value>,
) -> Vec<(&serde_json::Map<String, Value>, String)> {
    let mut out = Vec::with_capacity(2);

    if let Some(enforcement) = root.get("enforcement").and_then(Value::as_object)
        && let Some(rules) = enforcement.get("rules").and_then(Value::as_object)
    {
        out.push((rules, "enforcement.rules".to_string()));
    }

    // Legacy flat shape from the existing `.anvilrc` schema. The
    // legacy reader at `crates/anvil-cli/src/commands/gate.rs`
    // accepts `{ "rules": [...] }` and `{ "checks": [...] }` — we
    // honour the same idea here so a downstream migration doesn't
    // introduce silent re-enables.
    if let Some(rules) = root.get("rules").and_then(Value::as_object) {
        out.push((rules, "rules".to_string()));
    }

    out
}

fn check_rules_object(
    rules: &serde_json::Map<String, Value>,
    prefix: &str,
) -> Result<(), ValidationError> {
    for class in HARD_PINNED_CLASSES {
        let Some(entry) = rules.get(*class) else {
            continue;
        };
        // Shape 2 / 5: `class: false`.
        if entry.as_bool() == Some(false) {
            return Err(ValidationError::HardPinnedDisabled {
                class: (*class).to_string(),
                path: format!("{prefix}.{class}"),
            });
        }
        // Shape 6 / 7: `class: "disabled" | "off" | "none"` (scalar mode).
        // Same normalisation as object `mode` and `RuleMode::parse`.
        if let Some(raw) = entry.as_str() {
            let normalised = raw.to_ascii_lowercase();
            if matches!(normalised.as_str(), "disabled" | "off" | "none") {
                return Err(ValidationError::HardPinnedModeDisabled {
                    class: (*class).to_string(),
                    mode: raw.to_string(),
                    path: format!("{prefix}.{class}"),
                });
            }
        }
        if let Some(obj) = entry.as_object() {
            // Shape 1 / 4: `class: { enabled: false }`.
            if obj.get("enabled").and_then(Value::as_bool) == Some(false) {
                return Err(ValidationError::HardPinnedDisabled {
                    class: (*class).to_string(),
                    path: format!("{prefix}.{class}.enabled"),
                });
            }
            // Shape 3: `class: { mode: "disabled" | "off" | "none" }`.
            if let Some(mode) = obj.get("mode").and_then(Value::as_str) {
                let normalised = mode.to_ascii_lowercase();
                if matches!(normalised.as_str(), "disabled" | "off" | "none") {
                    return Err(ValidationError::HardPinnedModeDisabled {
                        class: (*class).to_string(),
                        mode: mode.to_string(),
                        path: format!("{prefix}.{class}.mode"),
                    });
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn empty_config_is_accepted() {
        assert_eq!(validate_hard_pinned_classes(&json!({})), Ok(()));
        assert_eq!(validate_hard_pinned_classes(&json!(null)), Ok(()));
    }

    #[test]
    fn rejects_secrets_enabled_false_canonical() {
        let v = json!({"enforcement": {"rules": {"secrets": {"enabled": false}}}});
        let err = validate_hard_pinned_classes(&v).unwrap_err();
        assert!(
            matches!(err, ValidationError::HardPinnedDisabled { ref class, .. } if class == "secrets")
        );
    }

    #[test]
    fn rejects_command_safety_enabled_false_canonical() {
        let v = json!({"enforcement": {"rules": {"command-safety": {"enabled": false}}}});
        let err = validate_hard_pinned_classes(&v).unwrap_err();
        assert!(
            matches!(err, ValidationError::HardPinnedDisabled { ref class, .. } if class == "command-safety")
        );
    }

    #[test]
    fn rejects_class_as_bool_false() {
        let v = json!({"enforcement": {"rules": {"secrets": false}}});
        let err = validate_hard_pinned_classes(&v).unwrap_err();
        assert!(
            matches!(err, ValidationError::HardPinnedDisabled { ref class, .. } if class == "secrets")
        );
    }

    #[test]
    fn rejects_legacy_flat_shape() {
        let v = json!({"rules": {"command-safety": {"enabled": false}}});
        let err = validate_hard_pinned_classes(&v).unwrap_err();
        assert!(
            matches!(err, ValidationError::HardPinnedDisabled { ref class, ref path, .. } if class == "command-safety" && path.starts_with("rules."))
        );
    }

    #[test]
    fn rejects_mode_disabled() {
        let v = json!({"enforcement": {"rules": {"secrets": {"mode": "disabled"}}}});
        let err = validate_hard_pinned_classes(&v).unwrap_err();
        assert!(
            matches!(err, ValidationError::HardPinnedModeDisabled { ref class, .. } if class == "secrets")
        );
    }

    #[test]
    fn rejects_mode_off_case_insensitive() {
        let v = json!({"enforcement": {"rules": {"command-safety": {"mode": "OFF"}}}});
        let err = validate_hard_pinned_classes(&v).unwrap_err();
        assert!(matches!(
            err,
            ValidationError::HardPinnedModeDisabled { .. }
        ));
    }

    #[test]
    fn rejects_scalar_mode_off_canonical() {
        // Scalar `"secrets": "off"` is a supported RuleMode form; hard-pin
        // must reject it the same way as `{ mode: "off" }`.
        let v = json!({"enforcement": {"rules": {"secrets": "off"}}});
        let err = validate_hard_pinned_classes(&v).unwrap_err();
        assert!(
            matches!(
                err,
                ValidationError::HardPinnedModeDisabled {
                    ref class,
                    ref mode,
                    ref path,
                    ..
                } if class == "secrets" && mode == "off" && path == "enforcement.rules.secrets"
            ),
            "expected HardPinnedModeDisabled for scalar off, got {err:?}"
        );
    }

    #[test]
    fn rejects_scalar_mode_disabled_and_none_case_insensitive() {
        for (class, mode) in [
            ("secrets", "disabled"),
            ("secrets", "DISABLED"),
            ("command-safety", "none"),
            ("command-safety", "Off"),
        ] {
            let v = json!({"enforcement": {"rules": {class: mode}}});
            let err = validate_hard_pinned_classes(&v).unwrap_err();
            assert!(
                matches!(
                    err,
                    ValidationError::HardPinnedModeDisabled {
                        class: ref c,
                        mode: ref m,
                        ..
                    } if c == class && m == mode
                ),
                "class={class} mode={mode}: expected HardPinnedModeDisabled, got {err:?}"
            );
        }
    }

    #[test]
    fn rejects_scalar_mode_off_legacy_flat_shape() {
        let v = json!({"rules": {"command-safety": "off"}});
        let err = validate_hard_pinned_classes(&v).unwrap_err();
        assert!(
            matches!(
                err,
                ValidationError::HardPinnedModeDisabled {
                    ref class,
                    ref path,
                    ..
                } if class == "command-safety" && path == "rules.command-safety"
            ),
            "expected legacy scalar rejection, got {err:?}"
        );
    }

    #[test]
    fn accepts_scalar_mode_warn_for_hard_pinned() {
        // Softening to warn is tuning, not a disable.
        let v = json!({"enforcement": {"rules": {"secrets": "warn"}}});
        assert_eq!(validate_hard_pinned_classes(&v), Ok(()));
    }

    #[test]
    fn accepts_tuning_severity() {
        let v = json!({"enforcement": {"rules": {"secrets": {"severity": "error"}}}});
        assert_eq!(validate_hard_pinned_classes(&v), Ok(()));
    }

    #[test]
    fn accepts_explicit_enabled_true() {
        let v = json!({"enforcement": {"rules": {"secrets": {"enabled": true}}}});
        assert_eq!(validate_hard_pinned_classes(&v), Ok(()));
    }

    #[test]
    fn accepts_mode_warn() {
        let v = json!({"enforcement": {"rules": {"secrets": {"mode": "warn"}}}});
        assert_eq!(validate_hard_pinned_classes(&v), Ok(()));
    }

    #[test]
    fn accepts_mode_block() {
        let v = json!({"enforcement": {"rules": {"command-safety": {"mode": "block"}}}});
        assert_eq!(validate_hard_pinned_classes(&v), Ok(()));
    }

    #[test]
    fn non_hard_pinned_class_can_be_disabled() {
        // ESLint, coverage, etc. are NOT hard-pinned; the parser
        // doesn't intercept their disable.
        let v = json!({"enforcement": {"rules": {"eslint": {"enabled": false}}}});
        assert_eq!(validate_hard_pinned_classes(&v), Ok(()));
    }

    #[test]
    fn rejects_first_hard_pinned_class_when_multiple_disabled() {
        // The validator stops at the first violation; the test
        // confirms the error names a specific class rather than a
        // generic "something disabled".
        let v = json!({
            "enforcement": {
                "rules": {
                    "secrets": {"enabled": false},
                    "command-safety": {"enabled": false},
                }
            }
        });
        let err = validate_hard_pinned_classes(&v).unwrap_err();
        match err {
            ValidationError::HardPinnedDisabled { class, .. } => {
                // Either class is acceptable for the v1 contract;
                // documentation says the validator surfaces the first
                // violation it sees and stops.
                assert!(class == "secrets" || class == "command-safety");
            }
            other @ ValidationError::HardPinnedModeDisabled { .. } => {
                panic!("expected HardPinnedDisabled, got mode-disabled error: {other:?}");
            }
        }
    }

    #[test]
    fn error_message_includes_actionable_guidance() {
        let v = json!({"enforcement": {"rules": {"secrets": {"enabled": false}}}});
        let err = validate_hard_pinned_classes(&v).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("secrets"), "msg should name the class");
        assert!(msg.contains("ADR-039"), "msg should cite the governing ADR");
        assert!(
            msg.contains("@anvil-ignore"),
            "msg should point at the per-finding bypass"
        );
    }
}
