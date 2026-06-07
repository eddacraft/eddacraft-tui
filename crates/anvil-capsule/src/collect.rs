//! Evidence collectors for capsule creation (GITGOV-005..008).
//!
//! Collectors read the repository the capsule describes and produce the
//! schema-versioned evidence documents the manifest digests. They reuse
//! the producing tools' own data — `git` plumbing here, witness/rules/
//! baseline crates in their respective collectors — rather than
//! re-modelling evidence (ADR-074 §Schema rules).
//!
//! This module currently owns the **commit/range collector**
//! (GITGOV-005): resolving `base..head` to an ordered, deterministic
//! [`CommitsDocument`] for `commits.json`.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::canonical::canonical_json_bytes;
use crate::errors::CapsuleError;

/// The `commits.json` schema identifier this crate produces and accepts.
pub const COMMITS_SCHEMA: &str = "anvil.capsule-commits.v1";

/// The `commits.json` evidence document: the resolved commit range and
/// every commit in it, oldest first.
///
/// Closed schema, same discipline as the manifest: unknown fields are a
/// parse error because unvouched content must not ride under a recorded
/// digest. Evolution is a new schema version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommitsDocument {
    /// Always [`COMMITS_SCHEMA`]; gated on parse.
    pub schema: String,
    /// Resolved base commit SHA (exclusive end of `base..head`).
    pub base: String,
    /// Resolved head commit SHA.
    pub head: String,
    /// Commits reachable from `head` but not `base`, in deterministic
    /// topological order, **oldest first** (`git rev-list --topo-order
    /// --reverse`). May be empty (`base == head`): present-but-empty is
    /// the ADR-074 discipline — an empty range is evidence, a missing
    /// file is tamper.
    pub commits: Vec<CommitEntry>,
}

/// One commit's identity and footprint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommitEntry {
    /// Full commit SHA.
    pub sha: String,
    /// The commit's tree hash.
    pub tree: String,
    /// Parent commit SHAs in commit order. Empty for a root commit
    /// (reachable when the range merges an orphan branch).
    pub parents: Vec<String>,
    /// Paths changed by this commit, sorted lexicographically.
    ///
    /// Semantics are pinned to **first-parent diff**: for merge commits
    /// the paths are the diff against the first parent (what the merge
    /// landed on its target branch), not a combined diff. Root commits
    /// diff against the empty tree. Non-UTF-8 paths are a collection
    /// error, never lossily rewritten — a mangled path under a clean
    /// digest would be laundered evidence.
    pub changed_paths: Vec<String>,
}

impl CommitsDocument {
    /// Encode as canonical JSON bytes (sorted keys, minimal
    /// whitespace) — the byte form written to `commits.json`.
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

    /// Parse and schema-gate a commits document from file bytes.
    ///
    /// # Errors
    ///
    /// [`CapsuleError::SchemaMismatch`] when the document declares a
    /// schema other than [`COMMITS_SCHEMA`]; [`CapsuleError::Parse`]
    /// for malformed JSON or unknown fields.
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, CapsuleError> {
        crate::schema_gate(bytes, COMMITS_SCHEMA)?;
        serde_json::from_slice(bytes).map_err(|e| CapsuleError::Parse(e.to_string()))
    }
}

/// Resolve `base..head` in the repository at `repo_root` and collect
/// every commit in the range into a [`CommitsDocument`].
///
/// `base` and `head` may be any commit-ish (SHA, ref, tag); both are
/// resolved to full commit SHAs before walking, so the document is
/// self-describing regardless of how the range was spelled. The walk is
/// `git rev-list --topo-order --reverse`, giving a deterministic
/// oldest-first order for a given history.
///
/// # Errors
///
/// [`CapsuleError::Git`] when `git` cannot be spawned, a ref does not
/// resolve to a commit, or output is not valid UTF-8 (including any
/// non-UTF-8 changed path — see [`CommitEntry::changed_paths`]).
pub fn collect_commits(
    repo_root: &Path,
    base: &str,
    head: &str,
) -> Result<CommitsDocument, CapsuleError> {
    let base_sha = resolve_commit(repo_root, base)?;
    let head_sha = resolve_commit(repo_root, head)?;

    let range = format!("{base_sha}..{head_sha}");
    let list = git_stdout(
        repo_root,
        &[
            "rev-list",
            "--topo-order",
            "--reverse",
            "--end-of-options",
            &range,
        ],
    )?;

    let mut commits = Vec::new();
    for sha in list.lines().map(str::trim).filter(|l| !l.is_empty()) {
        commits.push(collect_entry(repo_root, sha)?);
    }

    Ok(CommitsDocument {
        schema: COMMITS_SCHEMA.to_string(),
        base: base_sha,
        head: head_sha,
        commits,
    })
}

/// Resolve a commit-ish to a full commit SHA.
fn resolve_commit(repo_root: &Path, commitish: &str) -> Result<String, CapsuleError> {
    let spec = format!("{commitish}^{{commit}}");
    let out = git_stdout(
        repo_root,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            "--end-of-options",
            &spec,
        ],
    )
    .map_err(|e| CapsuleError::Git(format!("cannot resolve `{commitish}` to a commit: {e}")))?;
    Ok(out.trim().to_string())
}

/// Collect one commit's tree, parents, and first-parent changed paths.
fn collect_entry(repo_root: &Path, sha: &str) -> Result<CommitEntry, CapsuleError> {
    let meta = git_stdout(
        repo_root,
        &["show", "-s", "--format=%T%x1f%P", "--end-of-options", sha],
    )?;
    let meta = meta.trim_end_matches('\n');
    let (tree, parents_field) = meta.split_once('\x1f').ok_or_else(|| {
        CapsuleError::Git(format!("unexpected `git show` output for {sha}: {meta:?}"))
    })?;
    let parents: Vec<String> = parents_field
        .split_whitespace()
        .map(str::to_string)
        .collect();

    // First-parent diff; root commits diff against the empty tree.
    let mut diff_args: Vec<&str> = vec!["diff-tree", "-r", "-z", "--no-commit-id", "--name-only"];
    let first_parent = parents.first().cloned();
    match &first_parent {
        Some(parent) => {
            diff_args.extend(["--end-of-options", parent.as_str(), sha]);
        }
        None => {
            diff_args.extend(["--root", "--end-of-options", sha]);
        }
    }
    let raw = git_stdout_bytes(repo_root, &diff_args)?;

    // -z output: NUL-separated, unquoted paths. Reject non-UTF-8 rather
    // than rewriting it — see `CommitEntry::changed_paths`.
    let mut changed: BTreeSet<String> = BTreeSet::new();
    for chunk in raw.split(|b| *b == 0).filter(|c| !c.is_empty()) {
        let path = std::str::from_utf8(chunk)
            .map_err(|_| CapsuleError::Git(format!("non-UTF-8 changed path in commit {sha}")))?;
        changed.insert(path.to_string());
    }

    Ok(CommitEntry {
        sha: sha.to_string(),
        tree: tree.to_string(),
        parents,
        changed_paths: changed.into_iter().collect(),
    })
}

/// Run `git` in `repo_root` and return its stdout as UTF-8.
fn git_stdout(repo_root: &Path, args: &[&str]) -> Result<String, CapsuleError> {
    let raw = git_stdout_bytes(repo_root, args)?;
    String::from_utf8(raw)
        .map_err(|_| CapsuleError::Git(format!("git {} returned non-UTF-8 output", args[0])))
}

/// Run `git` in `repo_root` and return its raw stdout bytes.
fn git_stdout_bytes(repo_root: &Path, args: &[&str]) -> Result<Vec<u8>, CapsuleError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .map_err(|e| CapsuleError::Git(format!("failed to run git {}: {e}", args[0])))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr = stderr.trim();
        return Err(CapsuleError::Git(if stderr.is_empty() {
            format!("git {} failed with {}", args[0], output.status)
        } else {
            format!("git {} failed with {}: {stderr}", args[0], output.status)
        }));
    }
    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run `git` in `dir`, panicking on failure — test scaffolding only.
    fn git(dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .expect("spawn git");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("utf-8 git output")
    }

    /// Commit with a deterministic identity so tests never depend on
    /// machine-level git config.
    fn commit(dir: &Path, message: &str) {
        git(
            dir,
            &[
                "-c",
                "user.name=capsule-test",
                "-c",
                "user.email=capsule@test.invalid",
                "commit",
                "-q",
                "-m",
                message,
            ],
        );
    }

    fn write(dir: &Path, rel: &str, contents: &str) {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, contents).unwrap();
    }

    /// A scratch repo with two linear commits on top of a root commit.
    /// Returns (dir, `root_sha`, `mid_sha`, `head_sha`).
    fn linear_repo() -> (tempfile::TempDir, String, String, String) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        git(root, &["init", "-q"]);

        write(root, "a.txt", "one");
        git(root, &["add", "."]);
        commit(root, "root");
        let root_sha = git(root, &["rev-parse", "HEAD"]).trim().to_string();

        write(root, "b/nested.txt", "two");
        git(root, &["add", "."]);
        commit(root, "mid");
        let mid_sha = git(root, &["rev-parse", "HEAD"]).trim().to_string();

        write(root, "a.txt", "one-changed");
        write(root, "c.txt", "three");
        git(root, &["add", "."]);
        commit(root, "head");
        let head_sha = git(root, &["rev-parse", "HEAD"]).trim().to_string();

        (dir, root_sha, mid_sha, head_sha)
    }

    #[test]
    fn collect_commits_resolves_range_oldest_first() {
        let (dir, root_sha, mid_sha, head_sha) = linear_repo();

        let doc = collect_commits(dir.path(), &root_sha, &head_sha).unwrap();

        assert_eq!(doc.schema, COMMITS_SCHEMA);
        assert_eq!(doc.base, root_sha);
        assert_eq!(doc.head, head_sha);
        let shas: Vec<&str> = doc.commits.iter().map(|c| c.sha.as_str()).collect();
        assert_eq!(shas, vec![mid_sha.as_str(), head_sha.as_str()]);
    }

    #[test]
    fn collect_commits_records_tree_parents_and_changed_paths() {
        let (dir, root_sha, mid_sha, head_sha) = linear_repo();

        let doc = collect_commits(dir.path(), &root_sha, &head_sha).unwrap();

        let mid = &doc.commits[0];
        assert_eq!(
            mid.tree,
            git(dir.path(), &["rev-parse", &format!("{mid_sha}^{{tree}}")]).trim()
        );
        assert_eq!(mid.parents, vec![root_sha.clone()]);
        assert_eq!(mid.changed_paths, vec!["b/nested.txt".to_string()]);

        let head = &doc.commits[1];
        assert_eq!(head.parents, vec![mid_sha]);
        assert_eq!(
            head.changed_paths,
            vec!["a.txt".to_string(), "c.txt".to_string()],
            "changed paths are sorted"
        );
    }

    #[test]
    fn collect_commits_resolves_refs_to_full_shas() {
        let (dir, root_sha, _, head_sha) = linear_repo();
        let short_base = &root_sha[..10];

        let doc = collect_commits(dir.path(), short_base, "HEAD").unwrap();

        assert_eq!(doc.base, root_sha, "short SHA resolves to full");
        assert_eq!(doc.head, head_sha, "ref resolves to full SHA");
    }

    #[test]
    fn collect_commits_empty_range_is_present_but_empty() {
        let (dir, _, _, head_sha) = linear_repo();

        let doc = collect_commits(dir.path(), &head_sha, &head_sha).unwrap();

        assert_eq!(doc.commits, vec![]);
        assert_eq!(doc.base, doc.head);
    }

    #[test]
    fn collect_commits_merge_uses_first_parent_diff() {
        let (dir, _, _, head_sha) = linear_repo();
        let root = dir.path();

        // Side branch off head, then merge it back with a no-ff merge.
        git(root, &["checkout", "-q", "-b", "side"]);
        write(root, "side.txt", "side");
        git(root, &["add", "."]);
        commit(root, "side work");
        let side_sha = git(root, &["rev-parse", "HEAD"]).trim().to_string();

        git(root, &["checkout", "-q", "-"]);
        git(
            root,
            &[
                "-c",
                "user.name=capsule-test",
                "-c",
                "user.email=capsule@test.invalid",
                "merge",
                "-q",
                "--no-ff",
                "-m",
                "merge side",
                "side",
            ],
        );
        let merge_sha = git(root, &["rev-parse", "HEAD"]).trim().to_string();

        let doc = collect_commits(root, &head_sha, &merge_sha).unwrap();

        let merge = doc
            .commits
            .iter()
            .find(|c| c.sha == merge_sha)
            .expect("merge commit in range");
        assert_eq!(merge.parents, vec![head_sha, side_sha]);
        assert_eq!(
            merge.changed_paths,
            vec!["side.txt".to_string()],
            "merge footprint is the first-parent diff"
        );
    }

    #[test]
    fn collect_commits_orphan_root_in_range_diffs_against_empty_tree() {
        let (dir, _, _, head_sha) = linear_repo();
        let root = dir.path();

        // An orphan branch's root commit, merged into the main line:
        // the range then contains a parentless commit.
        let main_branch = git(root, &["branch", "--show-current"]).trim().to_string();
        git(root, &["checkout", "-q", "--orphan", "orphan"]);
        git(root, &["rm", "-rqf", "."]);
        write(root, "orphan.txt", "orphan");
        git(root, &["add", "."]);
        commit(root, "orphan root");
        let orphan_sha = git(root, &["rev-parse", "HEAD"]).trim().to_string();

        // `checkout -` has no @{-1} when leaving a branch that started
        // unborn; name the branch explicitly.
        git(root, &["checkout", "-q", &main_branch]);
        git(
            root,
            &[
                "-c",
                "user.name=capsule-test",
                "-c",
                "user.email=capsule@test.invalid",
                "merge",
                "-q",
                "--allow-unrelated-histories",
                "--no-ff",
                "-m",
                "merge orphan",
                "orphan",
            ],
        );
        let merge_sha = git(root, &["rev-parse", "HEAD"]).trim().to_string();

        let doc = collect_commits(root, &head_sha, &merge_sha).unwrap();

        let orphan = doc
            .commits
            .iter()
            .find(|c| c.sha == orphan_sha)
            .expect("orphan root in range");
        assert_eq!(orphan.parents, Vec::<String>::new());
        assert_eq!(orphan.changed_paths, vec!["orphan.txt".to_string()]);
    }

    #[test]
    fn collect_commits_is_deterministic() {
        let (dir, root_sha, _, head_sha) = linear_repo();

        let first = collect_commits(dir.path(), &root_sha, &head_sha)
            .unwrap()
            .to_canonical_bytes()
            .unwrap();
        let second = collect_commits(dir.path(), &root_sha, &head_sha)
            .unwrap()
            .to_canonical_bytes()
            .unwrap();

        assert_eq!(first, second);
    }

    #[test]
    fn collect_commits_unresolvable_ref_is_git_error() {
        let (dir, _, _, head_sha) = linear_repo();

        let err = collect_commits(dir.path(), "no-such-ref", &head_sha).unwrap_err();

        assert!(matches!(err, CapsuleError::Git(_)));
        let msg = err.to_string();
        assert!(msg.contains("no-such-ref"), "names the bad ref: {msg}");
    }

    #[test]
    fn collect_commits_handles_paths_with_spaces_and_unicode() {
        let (dir, _, _, head_sha) = linear_repo();
        let root = dir.path();

        write(root, "with space.txt", "spaced");
        write(root, "ünïcode.txt", "unicode");
        git(root, &["add", "."]);
        commit(root, "tricky paths");
        let new_head = git(root, &["rev-parse", "HEAD"]).trim().to_string();

        let doc = collect_commits(root, &head_sha, &new_head).unwrap();

        assert_eq!(
            doc.commits[0].changed_paths,
            vec!["with space.txt".to_string(), "ünïcode.txt".to_string()]
        );
    }

    fn sample_document() -> CommitsDocument {
        CommitsDocument {
            schema: COMMITS_SCHEMA.to_string(),
            base: "1111111111111111111111111111111111111111".to_string(),
            head: "2222222222222222222222222222222222222222".to_string(),
            commits: vec![CommitEntry {
                sha: "2222222222222222222222222222222222222222".to_string(),
                tree: "3333333333333333333333333333333333333333".to_string(),
                parents: vec!["1111111111111111111111111111111111111111".to_string()],
                changed_paths: vec!["src/lib.rs".to_string()],
            }],
        }
    }

    #[test]
    fn collect_commits_document_round_trips_through_canonical_bytes() {
        let doc = sample_document();
        let bytes = doc.to_canonical_bytes().unwrap();
        let parsed = CommitsDocument::from_json_bytes(&bytes).unwrap();
        assert_eq!(parsed, doc);
        assert_eq!(parsed.to_canonical_bytes().unwrap(), bytes);
    }

    #[test]
    fn collect_commits_document_rejects_unknown_schema_version() {
        let mut doc = sample_document();
        doc.schema = "anvil.capsule-commits.v999".to_string();
        let bytes = serde_json::to_vec(&doc).unwrap();
        let err = CommitsDocument::from_json_bytes(&bytes).unwrap_err();
        assert!(matches!(err, CapsuleError::SchemaMismatch { .. }));
    }

    /// Closed schema: unknown fields are a parse error, same digest
    /// discipline as the manifest.
    #[test]
    fn collect_commits_document_rejects_unknown_fields() {
        let bytes = sample_document().to_canonical_bytes().unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        value["smuggled"] = serde_json::json!(true);
        let raw = serde_json::to_vec(&value).unwrap();
        let err = CommitsDocument::from_json_bytes(&raw).unwrap_err();
        assert!(matches!(err, CapsuleError::Parse(_)));
    }

    /// Golden pin: the exact canonical encoding is the digest contract.
    /// A diff here is a schema-epoch event, not a refactor.
    #[test]
    fn collect_commits_document_canonical_bytes_golden() {
        let bytes = sample_document().to_canonical_bytes().unwrap();
        assert_eq!(
            std::str::from_utf8(&bytes).unwrap(),
            concat!(
                r#"{"base":"1111111111111111111111111111111111111111","#,
                r#""commits":[{"changed_paths":["src/lib.rs"],"#,
                r#""parents":["1111111111111111111111111111111111111111"],"#,
                r#""sha":"2222222222222222222222222222222222222222","#,
                r#""tree":"3333333333333333333333333333333333333333"}],"#,
                r#""head":"2222222222222222222222222222222222222222","#,
                r#""schema":"anvil.capsule-commits.v1"}"#
            )
        );
    }
}
