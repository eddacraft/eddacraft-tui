//! MLP-011: prove that equivalent configs in yaml / json / toml
//! produce byte-identical canonical JSON and therefore the same
//! `rules_sha`.
//!
//! This is the headline contract of the multi-format loader — without
//! it, MLP-012 (`rules_sha` in witness lines) couldn't trust that two
//! machines with the same logical config but different file formats
//! produce matching hashes.

use anvil_config::{ConfigFormat, canonical_json_bytes, parse_str};
use sha2::{Digest, Sha256};
use std::path::Path;

fn hash(value: &serde_json::Value) -> String {
    let bytes = canonical_json_bytes(value).unwrap();
    let digest = Sha256::digest(&bytes);
    hex_of(&digest)
}

fn hex_of(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
}

#[test]
fn yaml_json_toml_equivalent_configs_hash_identically() {
    let yaml = "\
version: 1
checks:
  - secrets
  - command-safety
thresholds:
  overall_score: 80
";

    let json = r#"{
        "version": 1,
        "checks": ["secrets", "command-safety"],
        "thresholds": {"overall_score": 80}
    }"#;

    let toml = "\
version = 1
checks = [\"secrets\", \"command-safety\"]

[thresholds]
overall_score = 80
";

    let y = parse_str(yaml, ConfigFormat::Yaml, Path::new("policy.yaml")).unwrap();
    let j = parse_str(json, ConfigFormat::Json, Path::new("policy.json")).unwrap();
    let t = parse_str(toml, ConfigFormat::Toml, Path::new("policy.toml")).unwrap();

    let yc = canonical_json_bytes(&y).unwrap();
    let jc = canonical_json_bytes(&j).unwrap();
    let tc = canonical_json_bytes(&t).unwrap();

    assert_eq!(yc, jc, "yaml and json canonical bytes diverge");
    assert_eq!(jc, tc, "json and toml canonical bytes diverge");

    let hy = hash(&y);
    let hj = hash(&j);
    let ht = hash(&t);
    assert_eq!(hy, hj);
    assert_eq!(hj, ht);
}

#[test]
fn reordering_yaml_keys_does_not_change_hash() {
    // Real configs are edited by hand; key order changes shouldn't
    // ripple into a new rules_sha.
    let a = "\
version: 1
checks:
  - secrets
thresholds:
  overall_score: 80
";
    let b = "\
thresholds:
  overall_score: 80
checks:
  - secrets
version: 1
";
    let va = parse_str(a, ConfigFormat::Yaml, Path::new("a.yaml")).unwrap();
    let vb = parse_str(b, ConfigFormat::Yaml, Path::new("b.yaml")).unwrap();
    assert_eq!(hash(&va), hash(&vb));
}

#[test]
fn reordering_json_keys_does_not_change_hash() {
    let a = r#"{"a": 1, "b": 2}"#;
    let b = r#"{"b": 2, "a": 1}"#;
    let va = parse_str(a, ConfigFormat::Json, Path::new("a.json")).unwrap();
    let vb = parse_str(b, ConfigFormat::Json, Path::new("b.json")).unwrap();
    assert_eq!(hash(&va), hash(&vb));
}

#[test]
fn list_order_does_change_hash() {
    // Documenting the opposite case: arrays are ordered, so reordering
    // a list is a meaningful change. Reviewers who try to
    // "canonicalise" by sorting arrays MUST stop here — that would
    // collapse different rule-precedence configs to the same hash.
    let a = r#"{"checks": ["a", "b"]}"#;
    let b = r#"{"checks": ["b", "a"]}"#;
    let va = parse_str(a, ConfigFormat::Json, Path::new("a.json")).unwrap();
    let vb = parse_str(b, ConfigFormat::Json, Path::new("b.json")).unwrap();
    assert_ne!(hash(&va), hash(&vb));
}

#[test]
fn whitespace_in_source_does_not_change_hash() {
    // The canonical encoding strips insignificant whitespace, so a
    // pretty-printed file and a minified file hash the same.
    let pretty = "{\n  \"a\": 1,\n  \"b\": 2\n}\n";
    let minified = r#"{"a":1,"b":2}"#;
    let p = parse_str(pretty, ConfigFormat::Json, Path::new("p.json")).unwrap();
    let m = parse_str(minified, ConfigFormat::Json, Path::new("m.json")).unwrap();
    assert_eq!(hash(&p), hash(&m));
}
