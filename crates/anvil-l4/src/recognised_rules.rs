//! MLP2-019: verify witness `rules_sha` against recognised rule versions.

use std::collections::HashMap;

use thiserror::Error;

use crate::policy::OnNoWitness;

/// The bundle of facts the L4 server knows about one recognised
/// `rules_sha`. Mirrors the four hash inputs that produced the digest
/// (see `anvil_rules::RulesShaInput`) plus a recognised-at timestamp
/// so a registry consumer can decide whether to trust ancient entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleSetMetadata {
    /// The 64-character lowercase hex digest. Acts as the registry
    /// key; must match `anvil_rules::rules_sha`'s output exactly.
    pub rules_sha: String,
    /// Anvil version that produced this rule set, e.g. `"0.7.0-beta"`.
    /// Consumed by MLP2-018's [`crate::evaluate_version_floor`] when
    /// composing with the policy's `required_anvil_version` floor.
    pub anvil_version: String,
    /// OPA runtime version pinned at build time, e.g. `"0.10.0"`.
    pub opa_runtime_version: String,
    /// Sorted-deduped rule identifiers carried in the rule set.
    pub rule_ids: Vec<String>,
    /// The `config_sha` over the canonical rule-config bytes — the
    /// fourth hash input. Lets a future consumer cross-check that the
    /// recognised entry's inputs round-trip through the digest.
    pub config_sha: String,
    /// ISO-8601 UTC timestamp at which the L4 server first published
    /// this `rules_sha` (format `YYYY-MM-DDTHH:MM:SSZ`). Matches the
    /// witness-line timestamp shape so a stale-cut-off policy can
    /// compare directly.
    pub recognised_at: String,
}

/// Errors returned by [`RecognisedRulesRegistry`] mutators.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RegistryError {
    /// Two distinct metadata records were inserted under the same
    /// `rules_sha`. Surfaced eagerly rather than letting the second
    /// silently shadow the first — a registry-build path that emits
    /// conflicting entries is a bug, not a tie-break case.
    #[error("rules_sha {rules_sha} already recognised with different metadata")]
    Conflict { rules_sha: String },
    /// An empty `rules_sha` was supplied. The witness wire format
    /// uses `None` for "no digest"; an empty string is a malformed
    /// caller and must not be silently coerced.
    #[error("rules_sha is empty; supply a 64-char hex digest or omit the entry")]
    EmptyDigest,
    /// `rules_sha` is not a 64-char lowercase hex digest. Loose
    /// shapes (e.g. abbreviated SHAs, mixed case) would let an
    /// upstream typo land in the registry. The byte-exact shape is
    /// load-bearing for digest equality.
    #[error("rules_sha must be 64-char lowercase hex; got {raw:?}")]
    InvalidDigestShape { raw: String },
}

/// `HashMap<rules_sha, RuleSetMetadata>` wrapper providing
/// constant-time lookup and insert-deterministic conflict reporting.
#[derive(Debug, Clone, Default)]
pub struct RecognisedRulesRegistry {
    entries: HashMap<String, RuleSetMetadata>,
}

impl RecognisedRulesRegistry {
    /// Build an empty registry. Recognises nothing — every witness
    /// `rules_sha` will be routed via the unrecognised branch.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Build a registry from an explicit list of [`RuleSetMetadata`]
    /// entries. Validates each entry's digest shape and refuses two
    /// distinct records under the same digest.
    pub fn with_entries<I: IntoIterator<Item = RuleSetMetadata>>(
        entries: I,
    ) -> Result<Self, RegistryError> {
        let mut r = Self::empty();
        for entry in entries {
            r.insert(entry)?;
        }
        Ok(r)
    }

    /// Insert one [`RuleSetMetadata`] entry. Returns
    /// [`RegistryError::Conflict`] when a different record is already
    /// registered under the same `rules_sha`; re-inserting an
    /// identical record is a no-op (lets a caller idempotently
    /// re-seed the registry from a stable source).
    pub fn insert(&mut self, entry: RuleSetMetadata) -> Result<(), RegistryError> {
        if entry.rules_sha.is_empty() {
            return Err(RegistryError::EmptyDigest);
        }
        if !is_lowercase_hex_64(&entry.rules_sha) {
            return Err(RegistryError::InvalidDigestShape {
                raw: entry.rules_sha.clone(),
            });
        }
        if let Some(existing) = self.entries.get(&entry.rules_sha) {
            if existing == &entry {
                return Ok(());
            }
            return Err(RegistryError::Conflict {
                rules_sha: entry.rules_sha.clone(),
            });
        }
        self.entries.insert(entry.rules_sha.clone(), entry);
        Ok(())
    }

    /// `O(1)` lookup. Returns `None` for a `rules_sha` the registry
    /// has not been seeded with.
    #[must_use]
    pub fn lookup(&self, rules_sha: &str) -> Option<&RuleSetMetadata> {
        self.entries.get(rules_sha)
    }

    /// True iff `rules_sha` is in the registry. Convenience over
    /// [`Self::lookup`] for callers that only need presence.
    #[must_use]
    pub fn contains(&self, rules_sha: &str) -> bool {
        self.entries.contains_key(rules_sha)
    }

    /// Number of recognised entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True iff the registry has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Outcome of [`evaluate_rules_sha`].
///
/// `Absent` and `Recognised` carry no payload because the L4 caller
/// derives every downstream signal from them deterministically. The
/// three unrecognised-witness variants name the routing decision the
/// branch rule's `OnNoWitness` knob produced; the caller renders each
/// into the user-facing verdict line + exit code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RulesShaOutcome {
    /// Witness carried no `rules_sha`. Distinct from `Recognised`
    /// because the L4 caller may want to emit a
    /// `degraded:rules-sha-absent` diagnostic depending on branch
    /// contract — silently treating "no digest" as "recognised" would
    /// hide that signal.
    Absent,
    /// Witness's `rules_sha` is in the registry. The caller may now
    /// trust the registry's `RuleSetMetadata` for downstream checks
    /// (e.g. MLP2-018's `anvil_version` floor comparison).
    Recognised,
    /// Witness's `rules_sha` is NOT in the registry, and policy
    /// admits the push regardless. The caller still emits the
    /// `rules_sha` so operators see what was admitted.
    AdmitUnrecognised { rules_sha: String },
    /// Witness's `rules_sha` is NOT in the registry, and policy asks
    /// the L4 engine to revalidate. The caller dispatches into the
    /// MLP2-016 `ValidationEngine` pipeline.
    NeedsRevalidation { rules_sha: String },
    /// Witness's `rules_sha` is NOT in the registry, and policy
    /// refuses pushes that don't match a recognised digest. The
    /// caller emits a block verdict.
    Block { rules_sha: String },
}

/// MLP2-019: route a witness's optional `rules_sha` through a
/// [`RecognisedRulesRegistry`] given the branch rule's `OnNoWitness`
/// knob.
///
/// The routing pattern mirrors `decide_commit` so the L4 caller has
/// one vocabulary for "policy admitted / routed / refused" across
/// witness-presence (MLP-004) and witness-recognition (MLP2-019)
/// checks.
///
/// **Note on the knob name.** The parameter is typed as
/// [`OnNoWitness`] because MLP2 v1 reuses the same three-way routing
/// vocabulary (`Allow` / `ValidateAtL4` / `Reject`) for the
/// unrecognised-`rules_sha` case as for the absent-witness case —
/// these are semantically distinct policy axes ("no witness at all"
/// vs "witness present but its `rules_sha` is from an unknown
/// release"). If a future schema bump introduces a separate
/// `on_unrecognised_rules_sha` field, the L4 caller threads that
/// value into this parameter instead; the routing vocabulary stays
/// stable.
#[must_use]
pub fn evaluate_rules_sha(
    registry: &RecognisedRulesRegistry,
    witness_rules_sha: Option<&str>,
    on_no_witness: OnNoWitness,
) -> RulesShaOutcome {
    let Some(rules_sha) = witness_rules_sha else {
        return RulesShaOutcome::Absent;
    };
    if registry.contains(rules_sha) {
        return RulesShaOutcome::Recognised;
    }
    match on_no_witness {
        OnNoWitness::Allow => RulesShaOutcome::AdmitUnrecognised {
            rules_sha: rules_sha.to_string(),
        },
        OnNoWitness::ValidateAtL4 => RulesShaOutcome::NeedsRevalidation {
            rules_sha: rules_sha.to_string(),
        },
        OnNoWitness::Reject => RulesShaOutcome::Block {
            rules_sha: rules_sha.to_string(),
        },
    }
}

fn is_lowercase_hex_64(raw: &str) -> bool {
    raw.len() == 64
        && raw
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(sha: &str, version: &str) -> RuleSetMetadata {
        RuleSetMetadata {
            rules_sha: sha.to_string(),
            anvil_version: version.to_string(),
            opa_runtime_version: "0.10.0".to_string(),
            rule_ids: vec!["AI-001".to_string(), "secret-aws-key".to_string()],
            config_sha: "0".repeat(64),
            recognised_at: "2026-05-14T00:00:00Z".to_string(),
        }
    }

    const KNOWN_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const KNOWN_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const UNKNOWN: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    #[test]
    fn empty_registry_recognises_nothing() {
        let r = RecognisedRulesRegistry::empty();
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
        assert!(!r.contains(KNOWN_A));
        assert!(r.lookup(KNOWN_A).is_none());
    }

    #[test]
    fn with_entries_admits_known_digests() {
        let r = RecognisedRulesRegistry::with_entries(vec![
            meta(KNOWN_A, "0.7.0-beta"),
            meta(KNOWN_B, "0.7.1"),
        ])
        .unwrap();
        assert_eq!(r.len(), 2);
        assert!(r.contains(KNOWN_A));
        assert!(r.contains(KNOWN_B));
        assert!(!r.contains(UNKNOWN));
        let hit = r.lookup(KNOWN_A).unwrap();
        assert_eq!(hit.anvil_version, "0.7.0-beta");
    }

    #[test]
    fn insert_is_idempotent_for_identical_records() {
        let mut r = RecognisedRulesRegistry::empty();
        r.insert(meta(KNOWN_A, "0.7.0-beta")).unwrap();
        // Re-insert identical: no error, len unchanged.
        r.insert(meta(KNOWN_A, "0.7.0-beta")).unwrap();
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn insert_refuses_conflict_with_different_metadata() {
        let mut r = RecognisedRulesRegistry::empty();
        r.insert(meta(KNOWN_A, "0.7.0-beta")).unwrap();
        let err = r.insert(meta(KNOWN_A, "0.7.1")).unwrap_err();
        match err {
            RegistryError::Conflict { rules_sha } => assert_eq!(rules_sha, KNOWN_A),
            other => panic!("expected Conflict, got {other:?}"),
        }
        // Original metadata survives the failed insert.
        assert_eq!(r.lookup(KNOWN_A).unwrap().anvil_version, "0.7.0-beta");
    }

    #[test]
    fn insert_refuses_empty_digest() {
        let mut r = RecognisedRulesRegistry::empty();
        let err = r.insert(meta("", "0.7.0-beta")).unwrap_err();
        assert!(matches!(err, RegistryError::EmptyDigest));
    }

    #[test]
    fn insert_refuses_short_digest() {
        let mut r = RecognisedRulesRegistry::empty();
        // 63 chars — short by one.
        let short = "a".repeat(63);
        let err = r.insert(meta(&short, "0.7.0-beta")).unwrap_err();
        assert!(matches!(err, RegistryError::InvalidDigestShape { .. }));
    }

    #[test]
    fn insert_refuses_long_digest() {
        let mut r = RecognisedRulesRegistry::empty();
        // 65 chars — long by one.
        let long = "a".repeat(65);
        let err = r.insert(meta(&long, "0.7.0-beta")).unwrap_err();
        assert!(matches!(err, RegistryError::InvalidDigestShape { .. }));
    }

    #[test]
    fn insert_refuses_uppercase_digest() {
        // Witness wire shape is lowercase hex; uppercase would land as
        // a registry entry whose key never matches incoming digests
        // (HashMap key equality is byte-exact). Refuse at insert.
        let mut r = RecognisedRulesRegistry::empty();
        let upper = "A".repeat(64);
        let err = r.insert(meta(&upper, "0.7.0-beta")).unwrap_err();
        assert!(matches!(err, RegistryError::InvalidDigestShape { .. }));
    }

    #[test]
    fn insert_refuses_non_hex_chars() {
        let mut r = RecognisedRulesRegistry::empty();
        // 64 chars but with a 'g' (not hex).
        let bad = format!("g{}", "a".repeat(63));
        let err = r.insert(meta(&bad, "0.7.0-beta")).unwrap_err();
        assert!(matches!(err, RegistryError::InvalidDigestShape { .. }));
    }

    fn registry_with_known() -> RecognisedRulesRegistry {
        RecognisedRulesRegistry::with_entries(vec![meta(KNOWN_A, "0.7.0-beta")]).unwrap()
    }

    #[test]
    fn evaluate_returns_absent_when_witness_has_no_rules_sha() {
        let r = registry_with_known();
        let outcome = evaluate_rules_sha(&r, None, OnNoWitness::Reject);
        assert_eq!(outcome, RulesShaOutcome::Absent);
    }

    #[test]
    fn evaluate_returns_recognised_for_known_digest() {
        let r = registry_with_known();
        let outcome = evaluate_rules_sha(&r, Some(KNOWN_A), OnNoWitness::Reject);
        assert_eq!(outcome, RulesShaOutcome::Recognised);
    }

    #[test]
    fn evaluate_routes_unrecognised_via_on_no_witness_reject() {
        let r = registry_with_known();
        let outcome = evaluate_rules_sha(&r, Some(UNKNOWN), OnNoWitness::Reject);
        match outcome {
            RulesShaOutcome::Block { rules_sha } => assert_eq!(rules_sha, UNKNOWN),
            other => panic!("expected Block, got {other:?}"),
        }
    }

    #[test]
    fn evaluate_routes_unrecognised_via_on_no_witness_validate_at_l4() {
        let r = registry_with_known();
        let outcome = evaluate_rules_sha(&r, Some(UNKNOWN), OnNoWitness::ValidateAtL4);
        match outcome {
            RulesShaOutcome::NeedsRevalidation { rules_sha } => assert_eq!(rules_sha, UNKNOWN),
            other => panic!("expected NeedsRevalidation, got {other:?}"),
        }
    }

    #[test]
    fn evaluate_routes_unrecognised_via_on_no_witness_allow() {
        let r = registry_with_known();
        let outcome = evaluate_rules_sha(&r, Some(UNKNOWN), OnNoWitness::Allow);
        match outcome {
            RulesShaOutcome::AdmitUnrecognised { rules_sha } => assert_eq!(rules_sha, UNKNOWN),
            other => panic!("expected AdmitUnrecognised, got {other:?}"),
        }
    }

    #[test]
    fn evaluate_against_empty_registry_routes_via_on_no_witness() {
        // An empty registry has the same effect as every digest being
        // unrecognised — pin the routing so a misconfigured server
        // (no rule pack loaded) doesn't silently admit pushes that
        // policy says should validate.
        let r = RecognisedRulesRegistry::empty();
        let outcome = evaluate_rules_sha(&r, Some(KNOWN_A), OnNoWitness::ValidateAtL4);
        match outcome {
            RulesShaOutcome::NeedsRevalidation { rules_sha } => assert_eq!(rules_sha, KNOWN_A),
            other => panic!("expected NeedsRevalidation, got {other:?}"),
        }
    }

    #[test]
    fn lookup_returns_full_metadata_for_recognised_digest() {
        // Recognised path lets the L4 caller pull anvil_version etc.
        // for downstream checks (MLP2-018 floor compare).
        let r = registry_with_known();
        let md = r.lookup(KNOWN_A).expect("known digest");
        assert_eq!(md.rules_sha, KNOWN_A);
        assert_eq!(md.anvil_version, "0.7.0-beta");
        assert_eq!(md.opa_runtime_version, "0.10.0");
        assert_eq!(md.rule_ids, vec!["AI-001", "secret-aws-key"]);
        assert_eq!(md.config_sha.len(), 64);
        assert_eq!(md.recognised_at, "2026-05-14T00:00:00Z");
    }
}
