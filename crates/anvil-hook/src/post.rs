//! MLP-005 post-hook helpers: post-rewrite parse and merge-witness planning.

use thiserror::Error;

/// One entry from git's `post-rewrite` stdin: the old commit SHA
/// that was replaced and the new commit SHA that replaced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewritePair {
    pub old_sha: String,
    pub new_sha: String,
}

#[derive(Debug, Error)]
pub enum PostRewriteParseError {
    /// A line didn't have exactly two whitespace-separated tokens.
    #[error("post-rewrite stdin line {line_number}: expected `<old> <new>`, got {raw:?}")]
    Malformed { line_number: usize, raw: String },
    /// An old or new SHA was empty or non-ASCII.
    #[error("post-rewrite stdin line {line_number}: invalid sha {raw:?}")]
    InvalidSha { line_number: usize, raw: String },
}

/// Parse git's `post-rewrite` stdin shape.
///
/// Per the git documentation, each line is `<old-sha> <new-sha>`
/// followed by an optional extra-info field. We ignore the extra
/// field; only the two SHAs are load-bearing for witness
/// regeneration.
pub fn parse_post_rewrite_input(stdin: &str) -> Result<Vec<RewritePair>, PostRewriteParseError> {
    let mut out = Vec::new();
    for (idx, raw) in stdin.lines().enumerate() {
        let line_number = idx + 1;
        if raw.trim().is_empty() {
            continue;
        }
        let mut parts = raw.split_whitespace();
        let old = parts
            .next()
            .ok_or_else(|| PostRewriteParseError::Malformed {
                line_number,
                raw: raw.to_string(),
            })?;
        let new = parts
            .next()
            .ok_or_else(|| PostRewriteParseError::Malformed {
                line_number,
                raw: raw.to_string(),
            })?;
        validate_sha(old, line_number)?;
        validate_sha(new, line_number)?;
        out.push(RewritePair {
            old_sha: old.to_string(),
            new_sha: new.to_string(),
        });
    }
    Ok(out)
}

fn validate_sha(raw: &str, line_number: usize) -> Result<(), PostRewriteParseError> {
    if raw.is_empty() || !raw.is_ascii() {
        return Err(PostRewriteParseError::InvalidSha {
            line_number,
            raw: raw.to_string(),
        });
    }
    Ok(())
}

/// Structured plan for a merge-commit witness append.
///
/// The witness writer extension (MLP-002 follow-up) will consume
/// this shape directly: one `parent_commits[i]` entry pairs with
/// one `prev_line_hashes[i]` entry, in parent order. The merge
/// commit itself goes on `merge_commit_sha`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeWitnessPlan {
    pub merge_commit_sha: String,
    /// Parent commit SHAs in `git rev-list --parents` order (first
    /// parent first).
    pub parent_commits: Vec<String>,
    /// Chain head `prev_line_hash` from each parent's witness chain,
    /// indexed identically to `parent_commits`. `None` means that
    /// parent had no witness (legacy commit; pre-baseline; etc.) —
    /// the writer records the gap rather than failing.
    pub prev_line_hashes: Vec<Option<String>>,
}

/// Build a [`MergeWitnessPlan`] from a merge commit's SHA and an
/// iterator of `(parent_sha, chain_head)` pairs.
///
/// `chain_head` is `None` when the parent had no witnessed history
/// (e.g. the merge brings in a branch that adopted Anvil after
/// branching off). The plan preserves the gap so the writer can
/// emit a witness that's honest about the missing edge.
pub fn merge_witness_plan(
    merge_commit_sha: impl Into<String>,
    parents: impl IntoIterator<Item = (String, Option<String>)>,
) -> MergeWitnessPlan {
    let mut commits = Vec::new();
    let mut hashes = Vec::new();
    for (sha, head) in parents {
        commits.push(sha);
        hashes.push(head);
    }
    MergeWitnessPlan {
        merge_commit_sha: merge_commit_sha.into(),
        parent_commits: commits,
        prev_line_hashes: hashes,
    }
}

/// Pinned `validation_at` string for retroactive witnesses produced
/// by `post-rewrite`. Stable; downstream readers should not
/// hard-code a different value.
pub const POST_REWRITE_VALIDATION_AT: &str = "post-rewrite-recovery";

/// A retroactive witness record built when a commit is amended or
/// rebased.
///
/// The library doesn't write witnesses directly — that's the
/// writer's job — but it does provide the consistent shape so every
/// caller tags the records identically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetroactiveWitness {
    pub commit_sha: String,
    /// Always [`POST_REWRITE_VALIDATION_AT`].
    pub validation_at: &'static str,
    /// The old SHA that this retroactive witness replaces.
    pub replaces_old_sha: String,
}

impl RetroactiveWitness {
    pub fn from_pair(pair: &RewritePair) -> Self {
        Self {
            commit_sha: pair.new_sha.clone(),
            validation_at: POST_REWRITE_VALIDATION_AT,
            replaces_old_sha: pair.old_sha.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_post_rewrite_handles_two_sha_lines() {
        let input = "aaa bbb\nccc ddd\n";
        let pairs = parse_post_rewrite_input(input).unwrap();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].old_sha, "aaa");
        assert_eq!(pairs[0].new_sha, "bbb");
        assert_eq!(pairs[1].old_sha, "ccc");
        assert_eq!(pairs[1].new_sha, "ddd");
    }

    #[test]
    fn parse_post_rewrite_ignores_extra_info_field() {
        // git can append an extra info field after the two SHAs;
        // we ignore it.
        let input = "aaa bbb extra-info-blob\n";
        let pairs = parse_post_rewrite_input(input).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].new_sha, "bbb");
    }

    #[test]
    fn parse_post_rewrite_skips_blank_lines() {
        let input = "aaa bbb\n\n\nccc ddd\n";
        let pairs = parse_post_rewrite_input(input).unwrap();
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn parse_post_rewrite_handles_empty_input() {
        let pairs = parse_post_rewrite_input("").unwrap();
        assert!(pairs.is_empty());
    }

    #[test]
    fn parse_post_rewrite_rejects_single_token_line() {
        let err = parse_post_rewrite_input("aaa\n").unwrap_err();
        match err {
            PostRewriteParseError::Malformed { line_number, .. } => {
                assert_eq!(line_number, 1);
            }
            PostRewriteParseError::InvalidSha { .. } => {
                panic!("expected Malformed, got InvalidSha")
            }
        }
    }

    #[test]
    fn parse_post_rewrite_rejects_non_ascii_sha() {
        let err = parse_post_rewrite_input("café bbb\n").unwrap_err();
        match err {
            PostRewriteParseError::InvalidSha { line_number, .. } => {
                assert_eq!(line_number, 1);
            }
            PostRewriteParseError::Malformed { .. } => {
                panic!("expected InvalidSha, got Malformed")
            }
        }
    }

    #[test]
    fn merge_witness_plan_preserves_parent_order() {
        let plan = merge_witness_plan(
            "merge-sha",
            [
                ("parent-1".to_string(), Some("hash-1".to_string())),
                ("parent-2".to_string(), Some("hash-2".to_string())),
                ("parent-3".to_string(), None),
            ],
        );
        assert_eq!(plan.merge_commit_sha, "merge-sha");
        assert_eq!(
            plan.parent_commits,
            vec!["parent-1", "parent-2", "parent-3"]
        );
        assert_eq!(
            plan.prev_line_hashes,
            vec![Some("hash-1".to_string()), Some("hash-2".to_string()), None]
        );
    }

    #[test]
    fn merge_witness_plan_indices_align_one_to_one() {
        // The writer extension will iterate the two arrays in
        // lockstep; a length mismatch would crash it. Confirm the
        // builder keeps them aligned.
        let plan = merge_witness_plan(
            "m",
            [
                ("p1".to_string(), None),
                ("p2".to_string(), Some("h".to_string())),
            ],
        );
        assert_eq!(plan.parent_commits.len(), plan.prev_line_hashes.len());
    }

    #[test]
    fn merge_witness_plan_handles_no_parents() {
        // Edge case: octopus merge with zero parents (shouldn't
        // happen in practice but the builder shouldn't panic).
        let plan = merge_witness_plan("orphan", std::iter::empty());
        assert!(plan.parent_commits.is_empty());
        assert!(plan.prev_line_hashes.is_empty());
    }

    #[test]
    fn retroactive_witness_validation_at_is_pinned() {
        // ADR-038 §D-6 names the tag explicitly. Don't drift.
        assert_eq!(POST_REWRITE_VALIDATION_AT, "post-rewrite-recovery");
    }

    #[test]
    fn retroactive_witness_from_pair_carries_both_shas() {
        let pair = RewritePair {
            old_sha: "old-aaa".to_string(),
            new_sha: "new-bbb".to_string(),
        };
        let w = RetroactiveWitness::from_pair(&pair);
        assert_eq!(w.commit_sha, "new-bbb");
        assert_eq!(w.replaces_old_sha, "old-aaa");
        assert_eq!(w.validation_at, POST_REWRITE_VALIDATION_AT);
    }
}
