//! Pre-push hook helpers (MLP-004).
//!
//! Pure data primitives for the `pre-push` git hook. Git invokes the
//! pre-push hook with one line per ref being pushed on stdin:
//!
//! ```text
//! <local-ref> SP <local-sha1> SP <remote-ref> SP <remote-sha1> LF
//! ```
//!
//! Special cases per `githooks(5)`:
//!
//! - Branch creation: `remote-sha1` is the 40-character all-zero SHA.
//! - Branch deletion: `local-sha1` is the 40-character all-zero SHA
//!   and `local-ref` is `(delete)`.
//!
//! Scope (MLP-004 v1):
//!
//! - [`parse_pre_push_input`] — split stdin into a typed [`PushRef`]
//!   list with explicit [`PushKind::Create`] / [`PushKind::Delete`] /
//!   [`PushKind::Update`] classification so the caller doesn't
//!   re-parse the zero SHA.
//! - [`is_zero_sha`] — predicate over the 40-zero deletion / creation
//!   marker. Exposed publicly because the caller often checks raw
//!   git output (e.g. `rev-list` results) against it.
//! - [`ZERO_SHA`] — the bare 40-zero string, pinned as a constant so
//!   downstream callers don't re-count to forty.
//!
//! Out of scope (deferred to consumers / CLI lane):
//!
//! - Walking `<remote-sha>..<local-sha>` — `git rev-list` is the
//!   caller's job; this crate doesn't shell out to git.
//! - Per-commit witness verification — composes
//!   [`anvil_witness::verify_chain`] with the SHAs this parser yields.
//! - Per-branch policy resolution — `anvil-l4` owns the resolver; the
//!   CLI subcommand threads policy → push-ref → verdict.
//! - `validate_at_l4` server-side execution — owned by the future
//!   validate-at-l4 command (CLI lane); MLP-006's library deliberately
//!   defers it.

use thiserror::Error;

/// The 40-character all-zero SHA git uses as a sentinel for "no such
/// commit" on branch creation (remote side) and branch deletion
/// (local side). Pinned as a constant so consumers don't recount the
/// zeros and so a typo surfaces at compile time.
pub const ZERO_SHA: &str = "0000000000000000000000000000000000000000";

/// What kind of update a [`PushRef`] represents.
///
/// Derived from the zero-SHA sentinels per `githooks(5)`. The CLI
/// caller uses this to skip range walks for `Delete` (no commits to
/// validate) and to treat `Create` specially (`git rev-list
/// <local-sha>` rather than `<remote-sha>..<local-sha>` because the
/// zero SHA isn't a valid ancestor).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushKind {
    /// New branch on the remote. `remote_sha` is [`ZERO_SHA`]; walk
    /// `local_sha`'s full ancestry that isn't on any other remote.
    Create,
    /// Branch deletion. `local_sha` is [`ZERO_SHA`]; nothing to
    /// validate.
    Delete,
    /// Fast-forward or non-fast-forward update of an existing ref.
    /// Walk `<remote_sha>..<local_sha>`.
    Update,
}

/// One parsed entry from git's pre-push stdin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushRef {
    /// Local ref name being pushed (e.g. `refs/heads/feat/foo`).
    /// For deletions git passes the literal string `"(delete)"`.
    pub local_ref: String,
    /// Local SHA at the tip of `local_ref`. [`ZERO_SHA`] when this
    /// push is a deletion.
    pub local_sha: String,
    /// Remote ref name being updated (e.g. `refs/heads/main`).
    pub remote_ref: String,
    /// Remote SHA before the push. [`ZERO_SHA`] when the branch is
    /// being created on the remote.
    pub remote_sha: String,
    /// Computed classification of the update; redundant with
    /// inspecting the SHAs but cheaper for the caller and resilient
    /// against future special cases (e.g. atomic-push extensions).
    pub kind: PushKind,
}

impl PushRef {
    /// Branch name slice of [`Self::remote_ref`] for `refs/heads/<name>`
    /// shaped refs, otherwise the full ref string. The pre-push hook
    /// resolves policy against the *remote* branch name because that's
    /// the thing being protected; the local ref may differ
    /// (e.g. `HEAD:refs/heads/main`).
    #[must_use]
    pub fn branch_name(&self) -> &str {
        self.remote_ref
            .strip_prefix("refs/heads/")
            .unwrap_or(&self.remote_ref)
    }
}

#[derive(Debug, Error)]
pub enum PrePushParseError {
    /// A line didn't have exactly four whitespace-separated tokens.
    #[error(
        "pre-push stdin line {line_number}: \
expected `<local-ref> <local-sha> <remote-ref> <remote-sha>`, got {raw:?}"
    )]
    Malformed { line_number: usize, raw: String },
    /// A SHA token was empty or contained non-ASCII bytes.
    #[error("pre-push stdin line {line_number}: invalid sha {sha:?}")]
    InvalidSha { line_number: usize, sha: String },
    /// A ref token was empty.
    #[error("pre-push stdin line {line_number}: empty ref")]
    EmptyRef { line_number: usize },
    /// Both `local_sha` and `remote_sha` are zero — git never asks the
    /// hook to confirm that nothing should happen.
    #[error("pre-push stdin line {line_number}: both shas are zero")]
    BothZero { line_number: usize },
}

/// Parse git's `pre-push` stdin shape.
///
/// Returns one [`PushRef`] per non-blank line; classification into
/// [`PushKind`] is performed eagerly so the caller doesn't re-examine
/// the SHAs.
pub fn parse_pre_push_input(stdin: &str) -> Result<Vec<PushRef>, PrePushParseError> {
    let mut out = Vec::new();
    for (idx, raw) in stdin.lines().enumerate() {
        let line_number = idx + 1;
        if raw.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = raw.split_whitespace().collect();
        if parts.len() != 4 {
            return Err(PrePushParseError::Malformed {
                line_number,
                raw: raw.to_string(),
            });
        }
        let local_ref = parts[0];
        let local_sha = parts[1];
        let remote_ref = parts[2];
        let remote_sha = parts[3];
        validate_ref(local_ref, line_number)?;
        validate_ref(remote_ref, line_number)?;
        validate_sha(local_sha, line_number)?;
        validate_sha(remote_sha, line_number)?;
        let local_zero = is_zero_sha(local_sha);
        let remote_zero = is_zero_sha(remote_sha);
        let kind = match (local_zero, remote_zero) {
            (true, true) => return Err(PrePushParseError::BothZero { line_number }),
            (true, false) => PushKind::Delete,
            (false, true) => PushKind::Create,
            (false, false) => PushKind::Update,
        };
        out.push(PushRef {
            local_ref: local_ref.to_string(),
            local_sha: local_sha.to_string(),
            remote_ref: remote_ref.to_string(),
            remote_sha: remote_sha.to_string(),
            kind,
        });
    }
    Ok(out)
}

/// True when `sha` is the 40-character all-zero string git uses as a
/// "no commit" sentinel. Comparison is byte-exact; we deliberately do
/// not lowercase or normalise — the git wire format is fixed.
#[must_use]
pub fn is_zero_sha(sha: &str) -> bool {
    sha == ZERO_SHA
}

fn validate_ref(raw: &str, line_number: usize) -> Result<(), PrePushParseError> {
    if raw.is_empty() {
        return Err(PrePushParseError::EmptyRef { line_number });
    }
    Ok(())
}

fn validate_sha(raw: &str, line_number: usize) -> Result<(), PrePushParseError> {
    if raw.is_empty() || !raw.is_ascii() {
        return Err(PrePushParseError::InvalidSha {
            line_number,
            sha: raw.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_sha_is_exactly_forty_zeros() {
        assert_eq!(ZERO_SHA.len(), 40);
        assert!(ZERO_SHA.chars().all(|c| c == '0'));
    }

    #[test]
    fn parse_handles_typical_update_line() {
        let input = "refs/heads/feat/foo aaa111 refs/heads/feat/foo bbb222\n";
        let refs = parse_pre_push_input(input).unwrap();
        assert_eq!(refs.len(), 1);
        let r = &refs[0];
        assert_eq!(r.local_ref, "refs/heads/feat/foo");
        assert_eq!(r.local_sha, "aaa111");
        assert_eq!(r.remote_ref, "refs/heads/feat/foo");
        assert_eq!(r.remote_sha, "bbb222");
        assert_eq!(r.kind, PushKind::Update);
    }

    #[test]
    fn parse_classifies_branch_creation_via_remote_zero_sha() {
        let input = format!("refs/heads/new aaa111 refs/heads/new {ZERO_SHA}\n");
        let refs = parse_pre_push_input(&input).unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].kind, PushKind::Create);
    }

    #[test]
    fn parse_classifies_branch_deletion_via_local_zero_sha() {
        let input = format!("(delete) {ZERO_SHA} refs/heads/old bbb222\n");
        let refs = parse_pre_push_input(&input).unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].kind, PushKind::Delete);
        assert_eq!(refs[0].local_ref, "(delete)");
    }

    #[test]
    fn parse_rejects_both_zero_shas_as_meaningless() {
        let input = format!("refs/heads/x {ZERO_SHA} refs/heads/x {ZERO_SHA}\n");
        let err = parse_pre_push_input(&input).unwrap_err();
        match err {
            PrePushParseError::BothZero { line_number } => assert_eq!(line_number, 1),
            other => panic!("expected BothZero, got {other:?}"),
        }
    }

    #[test]
    fn parse_handles_multiple_refs_in_one_push() {
        let input = "\
refs/heads/main aaa111 refs/heads/main bbb222
refs/heads/feat/x ccc333 refs/heads/feat/x ddd444
";
        let refs = parse_pre_push_input(input).unwrap();
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].local_ref, "refs/heads/main");
        assert_eq!(refs[1].local_ref, "refs/heads/feat/x");
    }

    #[test]
    fn parse_skips_blank_lines() {
        let input = "\nrefs/heads/main aaa111 refs/heads/main bbb222\n\n";
        let refs = parse_pre_push_input(input).unwrap();
        assert_eq!(refs.len(), 1);
    }

    #[test]
    fn parse_handles_empty_input() {
        // git fires the hook even on no-op pushes; an empty stdin is
        // a valid signal "nothing to validate."
        let refs = parse_pre_push_input("").unwrap();
        assert!(refs.is_empty());
    }

    #[test]
    fn parse_rejects_three_token_line() {
        let input = "refs/heads/main aaa111 refs/heads/main\n";
        let err = parse_pre_push_input(input).unwrap_err();
        match err {
            PrePushParseError::Malformed { line_number, .. } => assert_eq!(line_number, 1),
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    #[test]
    fn parse_rejects_non_ascii_sha() {
        let input = "refs/heads/main café refs/heads/main bbb222\n";
        let err = parse_pre_push_input(input).unwrap_err();
        match err {
            PrePushParseError::InvalidSha { line_number, .. } => assert_eq!(line_number, 1),
            other => panic!("expected InvalidSha, got {other:?}"),
        }
    }

    #[test]
    fn branch_name_strips_refs_heads_prefix() {
        let r = PushRef {
            local_ref: "refs/heads/main".to_string(),
            local_sha: "aaa".to_string(),
            remote_ref: "refs/heads/main".to_string(),
            remote_sha: "bbb".to_string(),
            kind: PushKind::Update,
        };
        assert_eq!(r.branch_name(), "main");
    }

    #[test]
    fn branch_name_passes_through_non_heads_ref() {
        let r = PushRef {
            local_ref: "refs/tags/v1".to_string(),
            local_sha: "aaa".to_string(),
            remote_ref: "refs/tags/v1".to_string(),
            remote_sha: "bbb".to_string(),
            kind: PushKind::Update,
        };
        assert_eq!(r.branch_name(), "refs/tags/v1");
    }

    #[test]
    fn branch_name_strips_only_the_heads_prefix() {
        let r = PushRef {
            local_ref: "refs/heads/feat/x/y".to_string(),
            local_sha: "aaa".to_string(),
            remote_ref: "refs/heads/feat/x/y".to_string(),
            remote_sha: "bbb".to_string(),
            kind: PushKind::Update,
        };
        assert_eq!(r.branch_name(), "feat/x/y");
    }

    #[test]
    fn is_zero_sha_recognises_canonical_form() {
        assert!(is_zero_sha(ZERO_SHA));
    }

    #[test]
    fn is_zero_sha_rejects_short_zero_string() {
        // A short SHA of zeros (`0000000`) is NOT the sentinel — git
        // always pads to 40. Be strict so we don't confuse a partial
        // hash with the sentinel.
        assert!(!is_zero_sha("0000000"));
    }

    #[test]
    fn is_zero_sha_rejects_non_zero_sha() {
        assert!(!is_zero_sha("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
    }

    #[test]
    fn parse_rejects_empty_local_ref() {
        // git enforces non-empty refs but defense in depth — if the
        // stdin shape is corrupted we'd rather error than silently
        // accept an empty pattern that some downstream match might
        // treat as "everything."
        let input = " aaa111 refs/heads/main bbb222\n";
        let err = parse_pre_push_input(input).unwrap_err();
        // Note: leading whitespace + split_whitespace collapses, so
        // this actually parses three tokens and Malformed fires
        // first. Pin that.
        match err {
            PrePushParseError::Malformed { .. } => {}
            other => panic!("expected Malformed (whitespace collapse), got {other:?}"),
        }
    }
}
