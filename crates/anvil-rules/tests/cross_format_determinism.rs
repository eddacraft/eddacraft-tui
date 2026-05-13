//! Cross-format determinism for `rules_sha` (MLP-012).
//!
//! Locks the headline guarantee: equivalent configs expressed as yaml,
//! json, and toml produce the same `rules_sha`. The chain of reasoning
//! is:
//!
//! 1. `anvil_config::parse_str` decodes any of the three formats into
//!    a common `serde_json::Value`.
//! 2. `anvil_config::canonical_json_bytes` produces byte-identical
//!    canonical bytes for equivalent values (MLP-011's headline
//!    invariant).
//! 3. `anvil_rules::config_sha_from_canonical` hashes those bytes.
//! 4. `rules_sha` includes that digest as the `config_sha` field, so
//!    the final `rules_sha` is format-independent.
//!
//! If MLP-011's canonical encoding ever drifts or this crate stops
//! using it, this test fails and witness lines start to diverge across
//! machines using different config formats.

use std::path::Path;

use anvil_config::{ConfigFormat, canonical_json_bytes, parse_str};
use anvil_rules::{config_sha_from_canonical, rules_sha};

const YAML: &str = r"
enforcement:
  rules:
    secrets:
      enabled: true
      severity: high
    command-safety:
      enabled: true
      severity: critical
project:
  name: anvil-test
";

const JSON: &str = r#"{"enforcement":{"rules":{"secrets":{"enabled":true,"severity":"high"},"command-safety":{"enabled":true,"severity":"critical"}}},"project":{"name":"anvil-test"}}"#;

const TOML: &str = r#"[enforcement.rules.secrets]
enabled = true
severity = "high"

[enforcement.rules.command-safety]
enabled = true
severity = "critical"

[project]
name = "anvil-test"
"#;

fn config_sha(raw: &str, fmt: ConfigFormat) -> String {
    // The `path` argument is only used to enrich parse errors; the
    // call won't surface a path here, so a placeholder is fine.
    let value = parse_str(raw, fmt, Path::new("<test>")).expect("parse config");
    let canonical = canonical_json_bytes(&value).expect("canonical bytes");
    config_sha_from_canonical(&canonical)
}

#[test]
fn yaml_json_toml_collapse_to_same_config_sha() {
    let y = config_sha(YAML, ConfigFormat::Yaml);
    let j = config_sha(JSON, ConfigFormat::Json);
    let t = config_sha(TOML, ConfigFormat::Toml);
    assert_eq!(y, j, "yaml/json config_sha must match");
    assert_eq!(j, t, "json/toml config_sha must match");
}

#[test]
fn yaml_json_toml_collapse_to_same_rules_sha() {
    let rules = ["AI-001", "secret-aws-key", "command-safety-rm-rf"];
    let y_cfg = config_sha(YAML, ConfigFormat::Yaml);
    let j_cfg = config_sha(JSON, ConfigFormat::Json);
    let t_cfg = config_sha(TOML, ConfigFormat::Toml);
    let y = rules_sha("0.7.0-beta", "0.10.0", rules, &y_cfg).unwrap();
    let j = rules_sha("0.7.0-beta", "0.10.0", rules, &j_cfg).unwrap();
    let t = rules_sha("0.7.0-beta", "0.10.0", rules, &t_cfg).unwrap();
    assert_eq!(y, j, "yaml/json rules_sha must match");
    assert_eq!(j, t, "json/toml rules_sha must match");
}

#[test]
fn differing_configs_produce_differing_rules_sha() {
    // Same rule set, different config — must produce different
    // `rules_sha`. This is the inverse of the headline test:
    // determinism alone is worthless if `config_sha` doesn't
    // actually influence the digest.
    let rules = ["AI-001", "secret-aws-key"];
    let a_cfg = config_sha(YAML, ConfigFormat::Yaml);

    let alt_yaml = r"
enforcement:
  rules:
    secrets:
      enabled: true
      severity: low
";
    let b_cfg = config_sha(alt_yaml, ConfigFormat::Yaml);
    assert_ne!(a_cfg, b_cfg, "different configs must hash differently");

    let a = rules_sha("0.7.0-beta", "0.10.0", rules, &a_cfg).unwrap();
    let b = rules_sha("0.7.0-beta", "0.10.0", rules, &b_cfg).unwrap();
    assert_ne!(a, b, "different config_sha must yield different rules_sha");
}

#[test]
fn rule_id_changes_produce_different_rules_sha_even_with_same_config() {
    let cfg = config_sha(YAML, ConfigFormat::Yaml);
    let a = rules_sha("0.7.0-beta", "0.10.0", ["AI-001", "secret-aws-key"], &cfg).unwrap();
    let b = rules_sha("0.7.0-beta", "0.10.0", ["AI-001"], &cfg).unwrap();
    assert_ne!(a, b);
}

#[test]
fn rule_id_order_at_call_site_does_not_affect_rules_sha() {
    let cfg = config_sha(YAML, ConfigFormat::Yaml);
    let a = rules_sha(
        "0.7.0-beta",
        "0.10.0",
        ["AI-001", "secret-aws-key", "command-safety-rm-rf"],
        &cfg,
    )
    .unwrap();
    let b = rules_sha(
        "0.7.0-beta",
        "0.10.0",
        ["command-safety-rm-rf", "secret-aws-key", "AI-001"],
        &cfg,
    )
    .unwrap();
    assert_eq!(a, b, "rule sort happens inside RulesShaInput::try_new");
}
