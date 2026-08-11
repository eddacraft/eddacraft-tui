//! MLP-013 integration: parse + validate hard-pinned classes
//! end-to-end across all three formats. Two parallel goals:
//!
//! 1. The validator rejects a disable attempt regardless of which
//!    on-disk format the operator chose. A `secrets: { enabled:
//!    false }` in yaml fails identically to the same shape in json
//!    or toml.
//! 2. The validator is layered on top of `parse_str`, not embedded
//!    in it. This means consumers can choose to validate (the v1
//!    expected path) or skip validation (intermediate states where
//!    the config is being constructed programmatically). The
//!    layering also lets new validators ride on the same intermediate
//!    `serde_json::Value` without re-touching the parser.

use anvil_config::{ConfigFormat, ValidationError, parse_str, validate_hard_pinned_classes};
use std::path::Path;

#[test]
fn yaml_disable_secrets_is_rejected() {
    let yaml = "\
enforcement:
  rules:
    secrets:
      enabled: false
";
    let v = parse_str(yaml, ConfigFormat::Yaml, Path::new("test.yaml")).unwrap();
    let err = validate_hard_pinned_classes(&v).unwrap_err();
    assert!(
        matches!(err, ValidationError::HardPinnedDisabled { ref class, .. } if class == "secrets")
    );
}

#[test]
fn yaml_scalar_mode_off_secrets_is_rejected() {
    // Scalar mode syntax (`secrets: off`) is a supported rule-mode form;
    // hard-pin must refuse it the same way as object `mode: off`.
    let yaml = "\
enforcement:
  rules:
    secrets: off
";
    let v = parse_str(yaml, ConfigFormat::Yaml, Path::new("test.yaml")).unwrap();
    let err = validate_hard_pinned_classes(&v).unwrap_err();
    assert!(
        matches!(
            err,
            ValidationError::HardPinnedModeDisabled { ref class, ref mode, .. }
                if class == "secrets" && mode.eq_ignore_ascii_case("off")
        ),
        "expected HardPinnedModeDisabled for yaml scalar off, got {err:?}"
    );
}

#[test]
fn json_disable_command_safety_is_rejected() {
    let json = r#"{"enforcement": {"rules": {"command-safety": {"enabled": false}}}}"#;
    let v = parse_str(json, ConfigFormat::Json, Path::new("test.json")).unwrap();
    let err = validate_hard_pinned_classes(&v).unwrap_err();
    assert!(
        matches!(err, ValidationError::HardPinnedDisabled { ref class, .. } if class == "command-safety")
    );
}

#[test]
fn toml_disable_secrets_is_rejected() {
    let toml = "\
[enforcement.rules.secrets]
enabled = false
";
    let v = parse_str(toml, ConfigFormat::Toml, Path::new("test.toml")).unwrap();
    let err = validate_hard_pinned_classes(&v).unwrap_err();
    assert!(
        matches!(err, ValidationError::HardPinnedDisabled { ref class, .. } if class == "secrets")
    );
}

#[test]
fn equivalent_disable_attempts_across_formats_fail_identically() {
    // Three on-disk configs all express the same intent ("disable
    // secrets") in their native idiom. All three should fail with
    // the same class — the rule cares about intent, not syntax.
    let yaml = "\
enforcement:
  rules:
    secrets:
      enabled: false
";
    let json = r#"{"enforcement": {"rules": {"secrets": {"enabled": false}}}}"#;
    let toml = "\
[enforcement.rules.secrets]
enabled = false
";

    for (contents, fmt, name) in [
        (yaml, ConfigFormat::Yaml, "yaml"),
        (json, ConfigFormat::Json, "json"),
        (toml, ConfigFormat::Toml, "toml"),
    ] {
        let v = parse_str(contents, fmt, Path::new(name)).unwrap();
        let err = validate_hard_pinned_classes(&v).unwrap_err();
        match err {
            ValidationError::HardPinnedDisabled { class, .. } => {
                assert_eq!(class, "secrets", "{name} should flag secrets");
            }
            other @ ValidationError::HardPinnedModeDisabled { .. } => {
                panic!("{name} expected HardPinnedDisabled, got mode-disabled: {other:?}");
            }
        }
    }
}

#[test]
fn tuning_only_config_passes_through_all_formats() {
    // Severity tuning is allowed across formats.
    let yaml = "\
enforcement:
  rules:
    secrets:
      severity: error
";
    let json = r#"{"enforcement": {"rules": {"secrets": {"severity": "error"}}}}"#;
    let toml = "\
[enforcement.rules.secrets]
severity = \"error\"
";

    for (contents, fmt, name) in [
        (yaml, ConfigFormat::Yaml, "yaml"),
        (json, ConfigFormat::Json, "json"),
        (toml, ConfigFormat::Toml, "toml"),
    ] {
        let v = parse_str(contents, fmt, Path::new(name)).unwrap();
        validate_hard_pinned_classes(&v)
            .unwrap_or_else(|e| panic!("{name} tuning-only config should pass; got {e}"));
    }
}
