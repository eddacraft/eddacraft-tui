use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// A single recorded finding in the baseline.
///
/// The schema is intentionally small. Anvil's rule engine carries a
/// much richer `Warning` shape (see `anvil_checks::antipattern::Warning`),
/// but the baseline only needs enough to *identify* a finding stably
/// across scans — not to re-render its diagnostic text. Storing
/// everything would couple the baseline format to an evolving engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineFinding {
    /// Repo-relative path with forward slashes (Windows callers
    /// must normalise before populating this field). Forward-slash
    /// normalisation is part of the cross-platform determinism
    /// guarantee.
    pub file_path: String,

    /// 16-character lowercase hex digest derived via
    /// [`compute_fingerprint`]. Validated at deserialise time by
    /// [`super::store::Baseline::validate`].
    pub fingerprint: String,

    /// Rule identifier, e.g. `"anti-pattern:guardrail-suppression"`
    /// or `"secret:aws-access-key"`. Stable string keys are required
    /// for cross-version diffability.
    pub rule_id: String,
}

impl BaselineFinding {
    /// Sort key used by [`super::store::Baseline::canonicalise`] to
    /// keep the on-disk findings array deterministic. Two scans of
    /// the same tree on different machines must produce the same
    /// `baseline.json` bytes.
    pub(crate) fn sort_key(&self) -> (&str, &str, &str) {
        (&self.rule_id, &self.file_path, &self.fingerprint)
    }
}

/// Errors emitted by [`compute_fingerprint`].
#[derive(Debug, Error)]
pub enum FingerprintError {
    /// `rule_id` was empty or contained a non-ASCII character. Rule
    /// IDs are ASCII kebab-case identifiers; the constraint dodges
    /// the Unicode-normalisation collision class.
    #[error("rule_id must be non-empty ASCII; got {raw:?}")]
    InvalidRuleId { raw: String },
    /// `snippet` was empty after normalisation. A fingerprint over
    /// nothing would collide for every `rule_id`; refuse at the
    /// boundary.
    #[error("snippet must contain at least one non-whitespace character")]
    EmptySnippet,
}

/// Compute the 16-character lowercase hex fingerprint for a finding.
///
/// The hash domain-separates `rule_id` from `snippet` with a NUL
/// byte (`\0`), then SHA-256s, then truncates to 16 hex chars (64
/// bits — collision risk acceptable at baseline-finding population
/// sizes; the full digest is overkill for an identity tag).
///
/// `snippet` is normalised via [`normalize_snippet`] before hashing,
/// so trivial whitespace edits or line-only moves do not invalidate
/// the fingerprint. Semantic edits (renaming a variable, changing a
/// literal) *do* invalidate it: that is the move-resistance scope
/// — moves yes, edits no.
pub fn compute_fingerprint(rule_id: &str, snippet: &str) -> Result<String, FingerprintError> {
    validate_rule_id(rule_id)?;
    let normalised = normalize_snippet(snippet);
    if normalised.is_empty() {
        return Err(FingerprintError::EmptySnippet);
    }
    let mut hasher = Sha256::new();
    hasher.update(rule_id.as_bytes());
    hasher.update([0u8]);
    hasher.update(normalised.as_bytes());
    let digest = hasher.finalize();
    Ok(hex::encode(&digest[..8]))
}

/// Normalise a snippet for fingerprinting.
///
/// The normalisation collapses runs of ASCII whitespace into a single
/// space and trims leading and trailing whitespace. It does NOT
/// lowercase, transliterate, or alter non-ASCII bytes — that would
/// silently merge semantically distinct rule hits.
pub fn normalize_snippet(snippet: &str) -> String {
    let mut out = String::with_capacity(snippet.len());
    let mut prev_was_space = true; // suppress leading whitespace
    for c in snippet.chars() {
        if c.is_ascii_whitespace() {
            if !prev_was_space {
                out.push(' ');
                prev_was_space = true;
            }
        } else {
            out.push(c);
            prev_was_space = false;
        }
    }
    while out.ends_with(' ') {
        out.pop();
    }
    out
}

fn validate_rule_id(raw: &str) -> Result<(), FingerprintError> {
    if raw.is_empty() || !raw.is_ascii() {
        return Err(FingerprintError::InvalidRuleId {
            raw: raw.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_16_lowercase_hex() {
        let fp = compute_fingerprint("rule-a", "let x = 1;").unwrap();
        assert_eq!(fp.len(), 16);
        assert!(
            fp.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
    }

    #[test]
    fn fingerprint_changes_when_rule_changes() {
        let a = compute_fingerprint("rule-a", "let x = 1;").unwrap();
        let b = compute_fingerprint("rule-b", "let x = 1;").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn fingerprint_changes_when_snippet_changes() {
        let a = compute_fingerprint("rule-a", "let x = 1;").unwrap();
        let b = compute_fingerprint("rule-a", "let x = 2;").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn fingerprint_stable_across_trivial_whitespace() {
        // Same content, different indentation / line wrapping —
        // must produce the same fingerprint (the move-resistance
        // contract).
        let a = compute_fingerprint("rule-a", "let x = 1;").unwrap();
        let b = compute_fingerprint("rule-a", "  let x = 1;").unwrap();
        let c = compute_fingerprint("rule-a", "let  x = 1;").unwrap();
        let d = compute_fingerprint("rule-a", "let\tx =\n1;").unwrap();
        assert_eq!(a, b);
        assert_eq!(a, c);
        assert_eq!(a, d);
    }

    #[test]
    fn fingerprint_rejects_empty_rule_id() {
        let err = compute_fingerprint("", "let x = 1;").unwrap_err();
        assert!(matches!(err, FingerprintError::InvalidRuleId { .. }));
    }

    #[test]
    fn fingerprint_rejects_non_ascii_rule_id() {
        let err = compute_fingerprint("règle", "let x = 1;").unwrap_err();
        assert!(matches!(err, FingerprintError::InvalidRuleId { .. }));
    }

    #[test]
    fn fingerprint_rejects_empty_snippet() {
        let err = compute_fingerprint("rule-a", "").unwrap_err();
        assert!(matches!(err, FingerprintError::EmptySnippet));
    }

    #[test]
    fn fingerprint_rejects_whitespace_only_snippet() {
        let err = compute_fingerprint("rule-a", "   \n\t  ").unwrap_err();
        assert!(matches!(err, FingerprintError::EmptySnippet));
    }

    #[test]
    fn fingerprint_is_deterministic_across_runs() {
        let a = compute_fingerprint("rule-a", "let x = 1;").unwrap();
        let b = compute_fingerprint("rule-a", "let x = 1;").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn fingerprint_preserves_non_ascii_snippet_content() {
        // Non-ASCII rule_ids are rejected; non-ASCII *snippets* are
        // accepted (a string literal in a Rust file legitimately
        // contains UTF-8). Two semantically different non-ASCII
        // snippets must hash differently.
        let a = compute_fingerprint("rule-a", "let s = \"café\";").unwrap();
        let b = compute_fingerprint("rule-a", "let s = \"cafe\";").unwrap();
        assert_ne!(a, b);
    }

    // Pinned canary digest for `golden_fingerprint_pin_literal`.
    // Computed at landing time from rule_id || 0x00 ||
    // normalize_snippet("  // @ts-ignore:\t  this is fine  ")
    //   = "// @ts-ignore: this is fine"
    // Update only with a release note — every existing baseline
    // depends on this exact value.
    const PINNED_FINGERPRINT: &str = "70c86a3617211686";

    #[test]
    fn golden_fingerprint_pin_literal() {
        // The snippet is deliberately whitespace-noisy so the test
        // exercises `normalize_snippet` (collapse + trim) on the way
        // through. If normalisation is ever bypassed or swapped, the
        // digest won't match this literal.
        let fp = compute_fingerprint(
            "anti-pattern:guardrail-suppression",
            "  // @ts-ignore:\t  this is fine  ",
        )
        .unwrap();
        assert_eq!(fp, PINNED_FINGERPRINT);
    }

    #[test]
    fn golden_fingerprint_matches_hand_rolled_normalised_bytes() {
        // Companion check: the encoder agrees with a hand-rolled
        // canonical-bytes path for the same logical input. If the
        // two diverge, the encoder has drifted from its documented
        // contract.
        let fp = compute_fingerprint(
            "anti-pattern:guardrail-suppression",
            "  // @ts-ignore:\t  this is fine  ",
        )
        .unwrap();
        let mut hasher = Sha256::new();
        hasher.update(b"anti-pattern:guardrail-suppression");
        hasher.update([0u8]);
        // Hand-rolled = normalised form, not the raw input.
        hasher.update(b"// @ts-ignore: this is fine");
        let expected = hex::encode(&hasher.finalize()[..8]);
        assert_eq!(fp, expected);
    }

    #[test]
    fn normalize_collapses_whitespace_runs() {
        assert_eq!(normalize_snippet("  a   b  "), "a b");
        assert_eq!(normalize_snippet("\t\na\n\tb\n"), "a b");
    }

    #[test]
    fn normalize_preserves_internal_punctuation() {
        assert_eq!(normalize_snippet("let x = 1;"), "let x = 1;");
        assert_eq!(normalize_snippet("a,b"), "a,b");
    }

    #[test]
    fn normalize_empty_input_yields_empty_string() {
        assert_eq!(normalize_snippet(""), "");
        assert_eq!(normalize_snippet("   \t\n  "), "");
    }

    #[test]
    fn fingerprint_domain_separates_rule_id_from_snippet() {
        // Without the NUL separator, `rule_id || snippet` collides
        // across the boundary: ("ab", "c") and ("a", "bc") hash the
        // same concatenated bytes. That would let a rule-id rename
        // masquerade as a snippet edit (or vice versa) in the
        // baseline identity.
        let a = compute_fingerprint("ab", "c").unwrap();
        let b = compute_fingerprint("a", "bc").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn fingerprint_preserves_snippet_case() {
        // Normalisation must not lowercase. A semantic edit that
        // only changes identifier case still invalidates the
        // fingerprint (moves yes, edits no).
        let a = compute_fingerprint("rule-a", "let X = 1;").unwrap();
        let b = compute_fingerprint("rule-a", "let x = 1;").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn normalize_does_not_collapse_non_ascii_whitespace() {
        // Contract: only ASCII whitespace is collapsed. NBSP and
        // other Unicode Zs characters are payload, not separators —
        // treating them as space would silently merge distinct hits.
        let nbsp = "a\u{00a0}b";
        assert_eq!(normalize_snippet(nbsp), nbsp);
        assert_ne!(normalize_snippet(nbsp), "a b");
    }

    #[test]
    fn normalize_does_not_trim_non_ascii_padding() {
        // Leading/trailing NBSP is not trim-able ASCII whitespace.
        assert_eq!(
            normalize_snippet("\u{00a0}token\u{00a0}"),
            "\u{00a0}token\u{00a0}"
        );
    }
}
