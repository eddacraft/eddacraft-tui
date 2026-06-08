//! The `anvil.capsule-verification.v1` document — closed-state
//! verdicts where missing evidence is `degraded`, never `pass`
//! (ADR-074 §Verdict model, ADR-072 §4).

use serde::{Deserialize, Serialize};

use crate::canonical::canonical_json_bytes;
use crate::errors::CapsuleError;

/// The verification schema identifier this crate produces and accepts.
pub const VERIFICATION_SCHEMA: &str = "anvil.capsule-verification.v1";

/// Closed-state verdict (ADR-074). `degraded != pass`, `error != pass`,
/// missing evidence `!=` clean evidence.
///
/// Per ADR-002 this is **advisory evidence**: `block` is a
/// verification-CLI verdict (non-zero exit from `anvil capsule
/// verify`), never a save-time gate on user code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    /// All required evidence present and verified; no block-level finding.
    Pass,
    /// Verified, with non-blocking findings.
    Warn,
    /// Evidence missing, stale, or partially unverifiable — **not** `pass`.
    Degraded,
    /// Witness break, digest mismatch, invalid/expired exception, or
    /// policy violation.
    Block,
    /// Tool/internal failure — do not overclaim.
    Error,
}

impl Verdict {
    /// The `anvil capsule verify` exit code (ADR-074 exit-code table):
    /// `0` pass/warn (warnings over blocks, ADR-002), `1` block,
    /// `2` degraded, `3` error.
    #[must_use]
    pub fn exit_code(self) -> i32 {
        match self {
            Self::Pass | Self::Warn => 0,
            Self::Block => 1,
            Self::Degraded => 2,
            Self::Error => 3,
        }
    }

    /// The lowercase verdict token (`pass`/`warn`/`degraded`/`block`/
    /// `error`) — the stable string both the JSON encoding and the CLI's
    /// human output use. Exhaustive `match`, so adding a variant is a
    /// compile error here rather than silent `Debug`-derived drift.
    #[must_use]
    pub fn as_token(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Warn => "warn",
            Self::Degraded => "degraded",
            Self::Block => "block",
            Self::Error => "error",
        }
    }

    /// Severity rank for combination: `error > block > degraded >
    /// warn > pass`. `error` outranks `block` because an overclaiming
    /// tool failure must never be laundered into a definite verdict.
    fn rank(self) -> u8 {
        match self {
            Self::Pass => 0,
            Self::Warn => 1,
            Self::Degraded => 2,
            Self::Block => 3,
            Self::Error => 4,
        }
    }

    /// The worse of two verdicts under [`Self::rank`].
    #[must_use]
    pub fn worst(self, other: Self) -> Self {
        if other.rank() > self.rank() {
            other
        } else {
            self
        }
    }
}

/// One named verification check's outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckResult {
    /// Stable check name (e.g. `manifest-digests`, `witness-chain`).
    pub name: String,
    /// The check's own closed-state verdict.
    pub verdict: Verdict,
    /// Optional human-readable detail. Missing, never `null`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// The `verification.json` document: per-check outcomes plus the
/// combined verdict.
///
/// Construct via [`Self::from_checks`] — direct field assignment
/// bypasses the worst-of derivation and the empty-checks-is-`degraded`
/// invariant, and such a document will be rejected by
/// [`Self::from_json_bytes`] on its next parse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapsuleVerification {
    /// Always [`VERIFICATION_SCHEMA`]; gated on parse.
    pub schema: String,
    /// Worst-of all check verdicts ([`Verdict::worst`]); `degraded`
    /// when `checks` is empty. Enforced on parse.
    pub verdict: Verdict,
    /// The individual checks, in execution order.
    pub checks: Vec<CheckResult>,
}

impl CapsuleVerification {
    /// Combine check results into a document. The overall verdict is
    /// the worst individual verdict; **no checks at all is `degraded`**
    /// — an empty verification must never read as `pass` (ADR-072 §4).
    #[must_use]
    pub fn from_checks(checks: Vec<CheckResult>) -> Self {
        let verdict = Self::derive_verdict(&checks);
        Self {
            schema: VERIFICATION_SCHEMA.to_string(),
            verdict,
            checks,
        }
    }

    /// Worst-of over `checks`; `Degraded` when empty (missing evidence
    /// never passes — ADR-072 §4).
    fn derive_verdict(checks: &[CheckResult]) -> Verdict {
        checks
            .iter()
            .map(|check| check.verdict)
            .reduce(Verdict::worst)
            .unwrap_or(Verdict::Degraded)
    }

    /// Encode as canonical JSON bytes (sorted keys, minimal whitespace).
    ///
    /// # Errors
    ///
    /// [`CapsuleError::Serialise`] if encoding fails (practically
    /// unreachable).
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, CapsuleError> {
        let value =
            serde_json::to_value(self).map_err(|e| CapsuleError::Serialise(e.to_string()))?;
        canonical_json_bytes(&value).map_err(|e| CapsuleError::Serialise(e.to_string()))
    }

    /// Parse, schema-gate, and consistency-check a verification
    /// document from file bytes.
    ///
    /// The schema version is probed **before** strict deserialisation
    /// (see [`crate::schema_gate`]), and the stored `verdict` is
    /// re-derived from `checks`: a document whose verdict disagrees
    /// with its own checks is a laundering attempt (or a hand-assembled
    /// document that bypassed [`Self::from_checks`]) and is rejected —
    /// the schema layer never trusts a stored verdict it can refute,
    /// regardless of whether the caller verified the manifest digest
    /// first.
    ///
    /// # Errors
    ///
    /// [`CapsuleError::SchemaMismatch`] for a foreign schema version;
    /// [`CapsuleError::Parse`] for malformed JSON or unknown fields;
    /// [`CapsuleError::InconsistentVerdict`] when the stored verdict
    /// disagrees with the worst-of derivation over `checks`.
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, CapsuleError> {
        crate::schema_gate(bytes, VERIFICATION_SCHEMA)?;
        let doc: Self =
            serde_json::from_slice(bytes).map_err(|e| CapsuleError::Parse(e.to_string()))?;
        let derived = Self::derive_verdict(&doc.checks);
        if doc.verdict != derived {
            // Use the stable token, not `Debug`, so the message can never
            // drift from the canonical string if a variant is renamed.
            return Err(CapsuleError::InconsistentVerdict {
                claimed: doc.verdict.as_token().to_string(),
                derived: derived.as_token().to_string(),
            });
        }
        Ok(doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(name: &str, verdict: Verdict) -> CheckResult {
        CheckResult {
            name: name.to_string(),
            verdict,
            detail: None,
        }
    }

    /// Pin the ADR-074 exit-code table verbatim.
    #[test]
    fn verification_exit_codes_match_adr074() {
        assert_eq!(Verdict::Pass.exit_code(), 0);
        assert_eq!(Verdict::Warn.exit_code(), 0);
        assert_eq!(Verdict::Block.exit_code(), 1);
        assert_eq!(Verdict::Degraded.exit_code(), 2);
        assert_eq!(Verdict::Error.exit_code(), 3);
    }

    #[test]
    fn verification_verdict_combination_takes_the_worst() {
        assert_eq!(Verdict::Pass.worst(Verdict::Warn), Verdict::Warn);
        assert_eq!(Verdict::Warn.worst(Verdict::Degraded), Verdict::Degraded);
        assert_eq!(Verdict::Degraded.worst(Verdict::Block), Verdict::Block);
        assert_eq!(Verdict::Block.worst(Verdict::Error), Verdict::Error);
        assert_eq!(Verdict::Error.worst(Verdict::Pass), Verdict::Error);
    }

    /// ADR-072 §4: an empty verification (no evidence checked) must
    /// never read as `pass`.
    #[test]
    fn verification_with_no_checks_is_degraded() {
        let doc = CapsuleVerification::from_checks(Vec::new());
        assert_eq!(doc.verdict, Verdict::Degraded);
    }

    #[test]
    fn verification_overall_verdict_is_worst_of_checks() {
        let doc = CapsuleVerification::from_checks(vec![
            check("manifest-digests", Verdict::Pass),
            check("witness-chain", Verdict::Block),
            check("exceptions", Verdict::Warn),
        ]);
        assert_eq!(doc.verdict, Verdict::Block);
    }

    #[test]
    fn verification_round_trips_through_canonical_bytes() {
        let doc = CapsuleVerification::from_checks(vec![CheckResult {
            name: "witness-chain".to_string(),
            verdict: Verdict::Warn,
            detail: Some("1 merge line".to_string()),
        }]);
        let bytes = doc.to_canonical_bytes().unwrap();
        let parsed = CapsuleVerification::from_json_bytes(&bytes).unwrap();
        assert_eq!(parsed, doc);
        assert_eq!(parsed.to_canonical_bytes().unwrap(), bytes);
    }

    #[test]
    fn verification_verdicts_serialise_lowercase() {
        let bytes = serde_json::to_vec(&Verdict::Degraded).unwrap();
        assert_eq!(bytes, br#""degraded""#);
    }

    #[test]
    fn verification_rejects_unknown_schema_version() {
        let mut doc = CapsuleVerification::from_checks(Vec::new());
        doc.schema = "anvil.capsule-verification.v999".to_string();
        let bytes = serde_json::to_vec(&doc).unwrap();
        let err = CapsuleVerification::from_json_bytes(&bytes).unwrap_err();
        assert!(matches!(err, CapsuleError::SchemaMismatch { .. }));
    }

    /// A v2 document with fields this crate doesn't know must fail as
    /// `SchemaMismatch` ("upgrade your Anvil"), not an opaque
    /// unknown-field parse error — the probe runs before strict parse.
    #[test]
    fn verification_schema_probe_beats_unknown_field_errors() {
        let raw = br#"{"schema":"anvil.capsule-verification.v2","verdict":"pass","checks":[],"v2_field":1}"#;
        let err = CapsuleVerification::from_json_bytes(raw).unwrap_err();
        assert!(matches!(err, CapsuleError::SchemaMismatch { .. }));
    }

    /// Verdict laundering: a document claiming `pass` while its own
    /// checks derive `block` is rejected at the schema layer — the
    /// stored verdict is never trusted when it can be refuted.
    #[test]
    fn verification_rejects_laundered_verdict() {
        let raw = concat!(
            r#"{"schema":"anvil.capsule-verification.v1","verdict":"pass","#,
            r#""checks":[{"name":"manifest-digests","verdict":"block"}]}"#
        );
        let err = CapsuleVerification::from_json_bytes(raw.as_bytes()).unwrap_err();
        assert!(matches!(err, CapsuleError::InconsistentVerdict { .. }));
    }

    /// Empty checks claiming `pass` is the other laundering shape —
    /// the derivation says `degraded`.
    #[test]
    fn verification_rejects_empty_checks_claiming_pass() {
        let raw = br#"{"schema":"anvil.capsule-verification.v1","verdict":"pass","checks":[]}"#;
        let err = CapsuleVerification::from_json_bytes(raw).unwrap_err();
        assert!(matches!(
            err,
            CapsuleError::InconsistentVerdict { claimed, derived }
                if claimed == "pass" && derived == "degraded"
        ));
    }

    /// Pin the full chain a CI consumer relies on:
    /// `from_checks` → worst-of → `exit_code`.
    #[test]
    fn verification_exit_code_chain_from_checks() {
        let blocked = CapsuleVerification::from_checks(vec![
            check("manifest-digests", Verdict::Pass),
            check("witness-chain", Verdict::Block),
        ]);
        assert_eq!(blocked.verdict.exit_code(), 1);
        let degraded = CapsuleVerification::from_checks(vec![
            check("manifest-digests", Verdict::Pass),
            check("witness-chain", Verdict::Degraded),
        ]);
        assert_eq!(degraded.verdict.exit_code(), 2);
    }
}
