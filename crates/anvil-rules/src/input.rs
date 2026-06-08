use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// The embedded OPA runtime version every `rules_sha` producer pins.
///
/// Shared so the witness writer (the pre-commit hook) and the capsule
/// digest collector (GITGOV-006) feed `rules_sha` the **same**
/// constant by construction — a divergence here would make capsule
/// rule identity silently disagree with witnessed lines. Owned by
/// this crate because the value is an input to [`rules_sha`].
pub const OPA_RUNTIME_VERSION: &str = "0.10.0";

/// Errors that can occur while assembling or hashing a
/// [`RulesShaInput`].
#[derive(Debug, Error)]
pub enum RulesShaError {
    /// The canonical-JSON encoder rejected the structured input.
    /// Effectively impossible given the static shape of
    /// [`RulesShaInput`], but kept typed so callers don't paper over
    /// it with `expect`.
    #[error("rules_sha canonical-JSON encoding failed: {0}")]
    Encode(#[from] serde_json::Error),
    /// `config_sha` was not a 64-character lowercase hex string. The
    /// witness chain treats `config_sha` as an opaque digest with a
    /// fixed shape; an out-of-shape value would let two distinct
    /// inputs share a witness identity, so we reject at the boundary.
    #[error("config_sha must be 64 lowercase hex chars; got {raw:?} ({len} chars)")]
    InvalidConfigSha { raw: String, len: usize },
    /// A rule identifier was empty or contained a non-ASCII character.
    /// Rule IDs in this project are ASCII kebab-case identifiers; the
    /// constraint also dodges the Unicode-normalisation hole where two
    /// rule IDs that print identically can have different canonical
    /// bytes.
    #[error("rule id must be non-empty ASCII; got {raw:?}")]
    InvalidRuleId { raw: String },
}

/// Structured input that feeds [`rules_sha`].
///
/// The four fields match the spec verbatim. Build via
/// [`RulesShaInput::try_new`], which sorts and de-duplicates the rule
/// list and validates `config_sha` + rule IDs at the boundary. Use
/// [`RulesShaInput::compute`] to produce the 64-character hex digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RulesShaInput {
    /// Anvil binary version (semver string, e.g. `"0.7.0-beta"`).
    /// Callers should pass their own `env!("CARGO_PKG_VERSION")` so
    /// this string reflects the actual running binary rather than
    /// `anvil-rules`'s package version (which can diverge if the
    /// crate is vendored or pinned independently).
    pub anvil_version: String,

    /// SHA-256 hex digest of the canonical-JSON bytes of the config
    /// that selected this rule set. Always exactly 64 lowercase hex
    /// characters; validated at construction.
    pub config_sha: String,

    /// OPA / Rego runtime version (semver string, e.g. `"0.10.0"`).
    /// Pin via the workspace `regorus` dependency to keep this stable.
    pub opa_runtime_version: String,

    /// Sorted, de-duplicated rule identifiers. The order here is the
    /// order that appears in the canonical JSON encoding, so changing
    /// this field's serialisation will change the hash.
    pub rules: Vec<String>,
}

impl RulesShaInput {
    /// Build an input from raw pieces. Sorts and de-duplicates the
    /// rule iterator; validates `config_sha` is 64 lowercase hex and
    /// each rule id is non-empty ASCII.
    pub fn try_new<I, S>(
        anvil_version: impl Into<String>,
        opa_runtime_version: impl Into<String>,
        rules: I,
        config_sha: impl Into<String>,
    ) -> Result<Self, RulesShaError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let config_sha = config_sha.into();
        validate_config_sha(&config_sha)?;
        let mut sorted: Vec<String> = Vec::new();
        for rule in rules {
            let rule = rule.into();
            validate_rule_id(&rule)?;
            sorted.push(rule);
        }
        sorted.sort();
        sorted.dedup();
        Ok(Self {
            anvil_version: anvil_version.into(),
            config_sha,
            opa_runtime_version: opa_runtime_version.into(),
            rules: sorted,
        })
    }

    /// Compute the 64-character lowercase hex `rules_sha` for this
    /// input.
    ///
    /// Builds the canonical JSON encoding directly from named fields
    /// (rather than round-tripping through [`serde_json::to_value`])
    /// so the output is independent of whether `serde_json`'s
    /// `preserve_order` feature happens to be enabled. The top-level
    /// keys are sorted lexicographically: `anvil_version`,
    /// `config_sha`, `opa_runtime_version`, `rules`.
    pub fn compute(&self) -> Result<String, RulesShaError> {
        let mut map: BTreeMap<&'static str, Value> = BTreeMap::new();
        map.insert("anvil_version", Value::String(self.anvil_version.clone()));
        map.insert("config_sha", Value::String(self.config_sha.clone()));
        map.insert(
            "opa_runtime_version",
            Value::String(self.opa_runtime_version.clone()),
        );
        map.insert(
            "rules",
            Value::Array(
                self.rules
                    .iter()
                    .map(|r| Value::String(r.clone()))
                    .collect(),
            ),
        );
        let bytes = serde_json::to_vec(&map)?;
        let digest = Sha256::digest(&bytes);
        Ok(hex::encode(digest))
    }
}

/// One-shot convenience: build a [`RulesShaInput`] and immediately
/// compute its digest. Returns the same errors as
/// [`RulesShaInput::try_new`] and [`RulesShaInput::compute`].
pub fn rules_sha<I, S>(
    anvil_version: impl Into<String>,
    opa_runtime_version: impl Into<String>,
    rules: I,
    config_sha: impl Into<String>,
) -> Result<String, RulesShaError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    RulesShaInput::try_new(anvil_version, opa_runtime_version, rules, config_sha)?.compute()
}

/// Compute `config_sha` from already-canonical-JSON bytes.
///
/// The caller is expected to obtain these bytes from
/// `anvil_config::canonical_json_bytes` — that's the only encoding
/// that guarantees yaml / json / toml inputs collapse to the same
/// digest. This helper is here so witness writers don't have to
/// re-import `sha2`.
pub fn config_sha_from_canonical(canonical_bytes: &[u8]) -> String {
    let digest = Sha256::digest(canonical_bytes);
    hex::encode(digest)
}

fn validate_config_sha(raw: &str) -> Result<(), RulesShaError> {
    let len = raw.chars().count();
    if len != 64
        || !raw
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        return Err(RulesShaError::InvalidConfigSha {
            raw: raw.to_string(),
            len,
        });
    }
    Ok(())
}

fn validate_rule_id(raw: &str) -> Result<(), RulesShaError> {
    if raw.is_empty() || !raw.is_ascii() {
        return Err(RulesShaError::InvalidRuleId {
            raw: raw.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_CONFIG_SHA: &str =
        "0000000000000000000000000000000000000000000000000000000000000000";

    fn sample() -> RulesShaInput {
        RulesShaInput::try_new(
            "0.7.0-beta",
            "0.10.0",
            ["AI-001", "secret-aws-key", "command-safety-rm-rf"],
            VALID_CONFIG_SHA,
        )
        .unwrap()
    }

    #[test]
    fn rules_are_sorted_and_deduped_at_construction() {
        let input = RulesShaInput::try_new(
            "0.7.0-beta",
            "0.10.0",
            ["zebra", "alpha", "alpha", "mike"],
            VALID_CONFIG_SHA,
        )
        .unwrap();
        assert_eq!(input.rules, vec!["alpha", "mike", "zebra"]);
    }

    #[test]
    fn try_new_rejects_short_config_sha() {
        let err = RulesShaInput::try_new("0.7.0-beta", "0.10.0", ["a"], "abc").unwrap_err();
        match err {
            RulesShaError::InvalidConfigSha { len, .. } => assert_eq!(len, 3),
            other => panic!("expected InvalidConfigSha, got {other:?}"),
        }
    }

    #[test]
    fn try_new_rejects_long_config_sha() {
        let err =
            RulesShaInput::try_new("0.7.0-beta", "0.10.0", ["a"], "a".repeat(65)).unwrap_err();
        assert!(matches!(err, RulesShaError::InvalidConfigSha { .. }));
    }

    #[test]
    fn try_new_rejects_uppercase_config_sha() {
        let err =
            RulesShaInput::try_new("0.7.0-beta", "0.10.0", ["a"], "A".repeat(64)).unwrap_err();
        assert!(matches!(err, RulesShaError::InvalidConfigSha { .. }));
    }

    #[test]
    fn try_new_rejects_non_hex_config_sha() {
        let err =
            RulesShaInput::try_new("0.7.0-beta", "0.10.0", ["a"], "g".repeat(64)).unwrap_err();
        assert!(matches!(err, RulesShaError::InvalidConfigSha { .. }));
    }

    #[test]
    fn try_new_rejects_empty_rule_id() {
        let err = RulesShaInput::try_new("0.7.0-beta", "0.10.0", ["", "a"], VALID_CONFIG_SHA)
            .unwrap_err();
        match err {
            RulesShaError::InvalidRuleId { raw } => assert_eq!(raw, ""),
            other => panic!("expected InvalidRuleId, got {other:?}"),
        }
    }

    #[test]
    fn try_new_rejects_non_ascii_rule_id() {
        // Smart-quote characters and accented characters both have
        // distinct Unicode normalisation forms; rejecting non-ASCII
        // sidesteps the entire NFC/NFD class of bugs.
        let err =
            RulesShaInput::try_new("0.7.0-beta", "0.10.0", ["café"], VALID_CONFIG_SHA).unwrap_err();
        assert!(matches!(err, RulesShaError::InvalidRuleId { .. }));
    }

    #[test]
    fn digest_is_64_hex_chars_lowercase() {
        let sha = sample().compute().unwrap();
        assert_eq!(sha.len(), 64);
        assert!(
            sha.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
    }

    #[test]
    fn digest_is_deterministic_across_calls() {
        let a = sample().compute().unwrap();
        let b = sample().compute().unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn golden_digest_pin_literal() {
        // Hard-coded pin: a real string literal, computed once and
        // committed. Any change to field names, key ordering,
        // encoding (whitespace, escaping, number form), or default
        // handling will fail this test against this exact 64-char
        // hex value. Update only with a release note — every
        // existing witness line carries a `rules_sha` that depends
        // on this exact encoding.
        //
        // Input:
        //   anvil_version       = "0.7.0-beta"
        //   config_sha          = "00…00" (64 zeros)
        //   opa_runtime_version = "0.10.0"
        //   rules               = ["AI-001", "command-safety-rm-rf",
        //                          "secret-aws-key"]
        // Canonical JSON (top-level keys sorted; rules already sorted
        // at construction):
        //   {"anvil_version":"0.7.0-beta","config_sha":"0…0",
        //    "opa_runtime_version":"0.10.0","rules":["AI-001",
        //    "command-safety-rm-rf","secret-aws-key"]}
        const PINNED_DIGEST: &str =
            "3c22864908537fba7a1e6d6214efd68c770c5bcdf792edd92eca853670c6c517";
        assert_eq!(sample().compute().unwrap(), PINNED_DIGEST);
    }

    #[test]
    fn golden_digest_matches_hand_rolled_canonical_bytes() {
        // Companion check: the encoder agrees with a hand-rolled
        // canonical-JSON byte string for the same input. If the two
        // diverge, the encoder has drifted from the documented
        // canonical form — fix the encoder, not this test.
        let canonical = br#"{"anvil_version":"0.7.0-beta","config_sha":"0000000000000000000000000000000000000000000000000000000000000000","opa_runtime_version":"0.10.0","rules":["AI-001","command-safety-rm-rf","secret-aws-key"]}"#;
        let manual = hex::encode(Sha256::digest(canonical));
        assert_eq!(sample().compute().unwrap(), manual);
    }

    #[test]
    fn digest_changes_when_anvil_version_changes() {
        let mut a = sample();
        let mut b = sample();
        a.anvil_version = "0.7.0-beta".to_string();
        b.anvil_version = "0.7.1-beta".to_string();
        assert_ne!(a.compute().unwrap(), b.compute().unwrap());
    }

    #[test]
    fn digest_changes_when_opa_runtime_version_changes() {
        let mut a = sample();
        let mut b = sample();
        a.opa_runtime_version = "0.10.0".to_string();
        b.opa_runtime_version = "0.10.1".to_string();
        assert_ne!(a.compute().unwrap(), b.compute().unwrap());
    }

    #[test]
    fn digest_changes_when_rules_change() {
        let a = sample().compute().unwrap();
        let b = RulesShaInput::try_new(
            "0.7.0-beta",
            "0.10.0",
            ["AI-001", "secret-aws-key"],
            VALID_CONFIG_SHA,
        )
        .unwrap()
        .compute()
        .unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn digest_changes_when_config_sha_changes() {
        let a = sample().compute().unwrap();
        let b = RulesShaInput::try_new(
            "0.7.0-beta",
            "0.10.0",
            ["AI-001", "secret-aws-key", "command-safety-rm-rf"],
            "1".repeat(64),
        )
        .unwrap()
        .compute()
        .unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn rule_order_at_input_does_not_affect_digest() {
        let a = RulesShaInput::try_new("0.7.0-beta", "0.10.0", ["c", "a", "b"], VALID_CONFIG_SHA)
            .unwrap()
            .compute()
            .unwrap();
        let b = RulesShaInput::try_new("0.7.0-beta", "0.10.0", ["b", "c", "a"], VALID_CONFIG_SHA)
            .unwrap()
            .compute()
            .unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn duplicate_rules_do_not_affect_digest() {
        let a = RulesShaInput::try_new("0.7.0-beta", "0.10.0", ["a", "b", "c"], VALID_CONFIG_SHA)
            .unwrap()
            .compute()
            .unwrap();
        let b = RulesShaInput::try_new(
            "0.7.0-beta",
            "0.10.0",
            ["a", "b", "c", "a", "b"],
            VALID_CONFIG_SHA,
        )
        .unwrap()
        .compute()
        .unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn empty_rule_set_is_valid() {
        let empty: [&str; 0] = [];
        let sha = rules_sha("0.7.0-beta", "0.10.0", empty, VALID_CONFIG_SHA).unwrap();
        assert_eq!(sha.len(), 64);
    }

    #[test]
    fn config_sha_from_canonical_matches_direct_sha256() {
        let bytes = br#"{"a":1,"b":2}"#;
        let got = config_sha_from_canonical(bytes);
        let expected = hex::encode(Sha256::digest(bytes));
        assert_eq!(got, expected);
        assert_eq!(got.len(), 64);
    }

    #[test]
    fn one_shot_helper_matches_input_compute() {
        let direct = sample().compute().unwrap();
        let via_helper = rules_sha(
            "0.7.0-beta",
            "0.10.0",
            ["AI-001", "secret-aws-key", "command-safety-rm-rf"],
            VALID_CONFIG_SHA,
        )
        .unwrap();
        assert_eq!(direct, via_helper);
    }
}
