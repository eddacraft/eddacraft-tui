//! Pre-push hook: resolve L4 policy and optionally run `ValidationEngine`.

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
    /// New branch on the remote. `remote_sha` is [`ZERO_SHA`]; the
    /// v1 CLI walks `git rev-list <local_sha>` (full reachable
    /// history). A "branch-new-edges-only" walk via `--not --remotes`
    /// is a deferred follow-up — until then, operators with deep
    /// histories should pin a `cutoff_commit` in `anvil/policy.<ext>`
    /// or set `OnNoWitness::Allow` on the relevant branch rule.
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
    // The git wire format puts a SHA-1 hex (40 chars) here, and
    // SHA-256 (64 chars) for repos opted into that hash algorithm.
    // Either way the token MUST be ASCII hex — anything else (`-foo`,
    // a path, a refspec) would otherwise reach `git rev-list` as a
    // revspec/option and walk the wrong commits. We accept the zero
    // sentinel as a special case; reject everything else that isn't
    // pure hex.
    if !is_hex_sha(raw) {
        return Err(PrePushParseError::InvalidSha {
            line_number,
            sha: raw.to_string(),
        });
    }
    Ok(())
}

/// True when `raw` is the zero sentinel OR a non-empty ASCII hex
/// string of plausible SHA length (4..=64 chars; covers short SHAs
/// through SHA-256). Strict-by-design so a corrupted stdin token
/// can't be forwarded to `git rev-list` as an option or revspec.
#[must_use]
pub fn is_hex_sha(raw: &str) -> bool {
    if is_zero_sha(raw) {
        return true;
    }
    let len = raw.len();
    if !(4..=64).contains(&len) {
        return false;
    }
    raw.bytes().all(|b| b.is_ascii_hexdigit())
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
    fn parse_rejects_dash_prefixed_sha_token() {
        // A leading `-` would be interpreted as a git option if it
        // reached `git rev-list`. Refuse at parse time so a corrupted
        // stdin can't smuggle in `--all` or similar.
        let input = "refs/heads/main -all refs/heads/main bbb222\n";
        let err = parse_pre_push_input(input).unwrap_err();
        match err {
            PrePushParseError::InvalidSha { line_number, .. } => assert_eq!(line_number, 1),
            other => panic!("expected InvalidSha, got {other:?}"),
        }
    }

    #[test]
    fn parse_rejects_revspec_sha_token() {
        // `HEAD~3` looks like a revspec; reject so it can't reach
        // git's rev parser.
        let input = "refs/heads/main HEAD~3 refs/heads/main bbb222\n";
        let err = parse_pre_push_input(input).unwrap_err();
        match err {
            PrePushParseError::InvalidSha { line_number, .. } => assert_eq!(line_number, 1),
            other => panic!("expected InvalidSha, got {other:?}"),
        }
    }

    #[test]
    fn parse_rejects_short_sha_token() {
        // SHAs shorter than 4 hex chars don't disambiguate anything;
        // reject so a stray `abc` token can't reach git.
        let input = "refs/heads/main ab refs/heads/main bbb222\n";
        let err = parse_pre_push_input(input).unwrap_err();
        match err {
            PrePushParseError::InvalidSha { line_number, .. } => assert_eq!(line_number, 1),
            other => panic!("expected InvalidSha, got {other:?}"),
        }
    }

    #[test]
    fn is_hex_sha_accepts_typical_short_and_full_shas() {
        assert!(is_hex_sha("aaaa")); // 4-char short SHA
        assert!(is_hex_sha("0123456789abcdef0123456789abcdef01234567")); // 40-char SHA-1
        assert!(is_hex_sha(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        )); // 64-char SHA-256
        assert!(is_hex_sha(ZERO_SHA));
    }

    #[test]
    fn is_hex_sha_rejects_dashes_and_non_hex() {
        assert!(!is_hex_sha("HEAD"));
        assert!(!is_hex_sha("refs/heads/main"));
        assert!(!is_hex_sha("commit-sha-1"));
        assert!(!is_hex_sha("--all"));
        assert!(!is_hex_sha("xyz0"));
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
