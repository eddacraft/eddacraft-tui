//! Explicit capsule disposal (ADR-078, GITGOV-013).
//!
//! Retention is keep-until-explicitly-pruned: nothing in Anvil deletes
//! capsule evidence automatically. This module plans and applies the
//! `anvil capsule prune` surface over an in-repo staging root:
//!
//! - Candidates are **schema-gated**: only immediate subdirectories whose
//!   `manifest.json` parses as `anvil.capsule.v1` participate; everything
//!   else is skipped (reported, never deleted). Symlinks are never followed.
//! - Ordering is the head commit's **committer date** resolved from the
//!   repository, tie-broken by head SHA then directory name — a total,
//!   deterministic order. A capsule whose head the repository does not know
//!   cannot be ordered honestly and is **always kept** (outside the
//!   `keep_last` accounting).
//! - Application deletes **via the git index** for tracked paths (the
//!   `git rm -r` equivalent) so the disposal is staged and cannot be
//!   silently reverted by `git restore`; untracked leftovers are removed
//!   from the filesystem. Committing remains the operator's act.

use std::path::{Path, PathBuf};

use crate::errors::CapsuleError;
use crate::manifest::CapsuleManifest;

/// One capsule directory the planner could identify.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapsuleRef {
    /// The capsule directory.
    pub dir: PathBuf,
    /// `range.head` from the manifest.
    pub head: String,
    /// Head committer date (Unix seconds) when the repository knows the
    /// commit; `None` marks the capsule unorderable (always kept).
    pub committer_time: Option<i64>,
}

/// A staging-root entry the planner refused to treat as a capsule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedEntry {
    /// The entry's path.
    pub path: PathBuf,
    /// Human-readable reason it is not a candidate.
    pub reason: String,
}

/// The deterministic prune plan for one (repo, staging root, `keep_last`).
#[derive(Debug, Clone, Default)]
pub struct PrunePlan {
    /// Orderable capsules kept (the newest `keep_last`).
    pub keep: Vec<CapsuleRef>,
    /// Orderable capsules selected for deletion (oldest first).
    pub delete: Vec<CapsuleRef>,
    /// Capsules whose head the repository does not know — always kept,
    /// outside the `keep_last` accounting (ADR-078).
    pub unordered: Vec<CapsuleRef>,
    /// Entries skipped by the candidate gate, with reasons.
    pub skipped: Vec<SkippedEntry>,
}

/// One capsule directory `apply_prune` failed to remove.
#[derive(Debug)]
pub struct PruneFailure {
    /// The capsule directory that could not be (fully) removed.
    pub dir: PathBuf,
    /// What went wrong.
    pub error: String,
}

/// Plan a prune of `staging_root` keeping the newest `keep_last`
/// orderable capsules.
///
/// `keep_last == 0` is refused (ADR-078: delete-everything is a manual
/// `git rm` decision, not a prune invocation). A missing `staging_root`
/// yields an empty plan — nothing is staged, nothing to prune.
///
/// # Errors
///
/// [`CapsuleError::Prune`] when `keep_last` is zero or the staging root
/// cannot be read.
pub fn plan_prune(
    repo_root: &Path,
    staging_root: &Path,
    keep_last: usize,
) -> Result<PrunePlan, CapsuleError> {
    if keep_last == 0 {
        return Err(CapsuleError::Prune(
            "--keep-last 0 is refused: deleting every capsule is a manual `git rm` \
             decision, not a prune invocation (ADR-078)"
                .to_string(),
        ));
    }
    let mut plan = PrunePlan::default();
    if !staging_root.exists() {
        return Ok(plan);
    }

    let mut orderable: Vec<CapsuleRef> = Vec::new();
    let mut entries: Vec<PathBuf> = std::fs::read_dir(staging_root)
        .map_err(|e| {
            CapsuleError::Prune(format!(
                "reading staging root {}: {e}",
                staging_root.display()
            ))
        })?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    // Deterministic scan order regardless of readdir order.
    entries.sort_unstable();

    for path in entries {
        let meta = match std::fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(e) => {
                plan.skipped.push(SkippedEntry {
                    path,
                    reason: format!("unreadable: {e}"),
                });
                continue;
            }
        };
        if meta.file_type().is_symlink() {
            plan.skipped.push(SkippedEntry {
                path,
                reason: "symlink — never followed (ADR-078)".to_string(),
            });
            continue;
        }
        if !meta.is_dir() {
            plan.skipped.push(SkippedEntry {
                path,
                reason: "not a directory".to_string(),
            });
            continue;
        }
        let manifest_path = path.join("manifest.json");
        let Ok(bytes) = std::fs::read(&manifest_path) else {
            plan.skipped.push(SkippedEntry {
                path,
                reason: "no readable manifest.json — not a capsule".to_string(),
            });
            continue;
        };
        let manifest = match CapsuleManifest::from_json_bytes(&bytes) {
            Ok(manifest) => manifest,
            Err(e) => {
                plan.skipped.push(SkippedEntry {
                    path,
                    reason: format!("manifest.json is not anvil.capsule.v1: {e}"),
                });
                continue;
            }
        };
        let capsule = CapsuleRef {
            committer_time: head_committer_time(repo_root, &manifest.range.head),
            head: manifest.range.head,
            dir: path,
        };
        if capsule.committer_time.is_some() {
            orderable.push(capsule);
        } else {
            plan.unordered.push(capsule);
        }
    }

    // Total order, newest first: committer date desc, then head SHA, then
    // directory name (ADR-078 — deterministic even on timestamp ties).
    orderable.sort_unstable_by(|a, b| {
        b.committer_time
            .cmp(&a.committer_time)
            .then_with(|| a.head.cmp(&b.head))
            .then_with(|| a.dir.cmp(&b.dir))
    });
    let delete = orderable.split_off(keep_last.min(orderable.len()));
    plan.keep = orderable;
    plan.delete = delete;
    Ok(plan)
}

/// Apply a plan's deletions. Tracked paths are removed via the git index
/// (staged deletion); untracked leftovers are removed from the
/// filesystem. Failures are collected per capsule — the prune continues
/// past them so the resulting state is fully reported (ADR-078).
#[must_use]
pub fn apply_prune(repo_root: &Path, plan: &PrunePlan) -> Vec<PruneFailure> {
    let mut failures = Vec::new();
    for capsule in &plan.delete {
        if let Err(error) = remove_capsule_dir(repo_root, &capsule.dir) {
            failures.push(PruneFailure {
                dir: capsule.dir.clone(),
                error,
            });
        }
    }
    failures
}

/// Remove one capsule directory: staged deletion for tracked content,
/// filesystem removal for whatever remains.
fn remove_capsule_dir(repo_root: &Path, dir: &Path) -> Result<(), String> {
    let tracked =
        crate::collect::git_stdout(repo_root, &["ls-files", "-z", "--", &dir.to_string_lossy()])
            .map_err(|e| format!("checking tracked state: {e}"))?;
    if !tracked.trim_end_matches('\0').is_empty() {
        // `-f` only bypasses the staged/modified-content guard — the
        // decision to delete was already made by the schema-gated plan;
        // it never widens which paths are removed.
        crate::collect::git_stdout(
            repo_root,
            &["rm", "-r", "-q", "-f", "--", &dir.to_string_lossy()],
        )
        .map_err(|e| format!("git rm: {e}"))?;
    }
    if dir.exists() {
        std::fs::remove_dir_all(dir).map_err(|e| format!("removing directory: {e}"))?;
    }
    Ok(())
}

/// The head commit's committer date, or `None` when the repository does
/// not know the commit (shallow clone, foreign capsule) — the honest
/// "cannot order" signal.
fn head_committer_time(repo_root: &Path, head: &str) -> Option<i64> {
    // `^{commit}` refuses non-commit objects that happen to share a name.
    let spec = format!("{head}^{{commit}}");
    let out = crate::collect::git_stdout(repo_root, &["log", "-1", "--format=%ct", &spec]).ok()?;
    out.trim().parse::<i64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{CapsuleRange, Producer};
    use std::process::Command;

    fn git(dir: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(dir)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()
            .expect("spawn git");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// Commit with a deterministic identity and a pinned committer date,
    /// returning the commit SHA. Pinned dates make the ordering tests
    /// independent of wall-clock and machine config.
    fn commit_at(dir: &Path, message: &str, epoch: &str) -> String {
        let date = format!("{epoch} +0000");
        let output = Command::new("git")
            .args([
                "-c",
                "user.name=prune-test",
                "-c",
                "user.email=prune@test.invalid",
                "commit",
                "-q",
                "--allow-empty",
                "-m",
                message,
            ])
            .env("GIT_COMMITTER_DATE", &date)
            .env("GIT_AUTHOR_DATE", &date)
            .env_remove("GIT_DIR")
            .current_dir(dir)
            .output()
            .expect("spawn git commit");
        assert!(
            output.status.success(),
            "commit failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let sha = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(dir)
            .output()
            .expect("rev-parse");
        String::from_utf8(sha.stdout).unwrap().trim().to_string()
    }

    fn repo() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        git(tmp.path(), &["init", "-q"]);
        tmp
    }

    /// Write a minimal valid capsule directory whose manifest points at
    /// `head`.
    fn write_capsule_dir(root: &Path, name: &str, head: &str) -> PathBuf {
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let manifest = CapsuleManifest::new(
            CapsuleRange {
                base: "0".repeat(40),
                head: head.to_string(),
                witness_seq_start: None,
                witness_seq_end: None,
            },
            Producer {
                anvil_version: "0.0.0-test".to_string(),
            },
        );
        std::fs::write(
            dir.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        dir
    }

    #[test]
    fn keep_last_zero_is_refused() {
        let repo = repo();
        let err =
            plan_prune(repo.path(), &repo.path().join("anvil/evidence/capsules"), 0).unwrap_err();
        assert!(err.to_string().contains("keep-last 0"), "{err}");
    }

    #[test]
    fn missing_staging_root_is_an_empty_plan() {
        let repo = repo();
        let plan =
            plan_prune(repo.path(), &repo.path().join("anvil/evidence/capsules"), 1).unwrap();
        assert!(plan.keep.is_empty() && plan.delete.is_empty());
        assert!(plan.unordered.is_empty() && plan.skipped.is_empty());
    }

    #[test]
    fn orders_by_committer_date_and_keeps_newest() {
        let repo = repo();
        let old = commit_at(repo.path(), "old", "1700000000");
        let mid = commit_at(repo.path(), "mid", "1700000100");
        let new = commit_at(repo.path(), "new", "1700000200");
        let root = repo.path().join("staging");
        write_capsule_dir(&root, "cap-old", &old);
        write_capsule_dir(&root, "cap-mid", &mid);
        write_capsule_dir(&root, "cap-new", &new);

        let plan = plan_prune(repo.path(), &root, 2).unwrap();
        let keep: Vec<_> = plan.keep.iter().map(|c| c.head.as_str()).collect();
        let delete: Vec<_> = plan.delete.iter().map(|c| c.head.as_str()).collect();
        assert_eq!(keep, vec![new.as_str(), mid.as_str()]);
        assert_eq!(delete, vec![old.as_str()]);
    }

    #[test]
    fn timestamp_ties_break_on_head_then_dir_name() {
        let repo = repo();
        let a = commit_at(repo.path(), "a", "1700000000");
        let b = commit_at(repo.path(), "b", "1700000000"); // same second
        let root = repo.path().join("staging");
        // Two capsules over the SAME head exercise the dir-name tiebreak.
        write_capsule_dir(&root, "zz-dup", &b);
        write_capsule_dir(&root, "aa-dup", &b);
        write_capsule_dir(&root, "cap-a", &a);

        let plan = plan_prune(repo.path(), &root, 1).unwrap();
        // Total order is (time desc, head asc, dir asc): deterministic
        // regardless of readdir order.
        let ordered: Vec<_> = plan
            .keep
            .iter()
            .chain(plan.delete.iter())
            .map(|c| {
                (
                    c.head.as_str(),
                    c.dir.file_name().unwrap().to_str().unwrap(),
                )
            })
            .collect();
        let mut expected = vec![
            (a.as_str(), "cap-a"),
            (b.as_str(), "aa-dup"),
            (b.as_str(), "zz-dup"),
        ];
        expected.sort_by(|x, y| x.0.cmp(y.0).then(x.1.cmp(y.1)));
        assert_eq!(ordered, expected);
        // Re-planning yields the identical split.
        let again = plan_prune(repo.path(), &root, 1).unwrap();
        assert_eq!(plan.keep, again.keep);
        assert_eq!(plan.delete, again.delete);
    }

    #[test]
    fn unknown_head_is_kept_outside_the_accounting() {
        let repo = repo();
        let known = commit_at(repo.path(), "known", "1700000000");
        let root = repo.path().join("staging");
        write_capsule_dir(&root, "cap-known", &known);
        write_capsule_dir(&root, "cap-foreign", &"f".repeat(40));

        let plan = plan_prune(repo.path(), &root, 1).unwrap();
        assert_eq!(plan.unordered.len(), 1, "foreign capsule is unordered");
        assert_eq!(plan.keep.len(), 1, "keep_last applies to orderable only");
        assert!(plan.delete.is_empty());
    }

    #[test]
    fn non_capsules_and_symlinks_are_skipped_never_deleted() {
        let repo = repo();
        let head = commit_at(repo.path(), "c", "1700000000");
        let root = repo.path().join("staging");
        write_capsule_dir(&root, "cap", &head);
        std::fs::create_dir_all(root.join("not-a-capsule")).unwrap();
        std::fs::write(root.join("not-a-capsule/manifest.json"), b"{}").unwrap();
        std::fs::write(root.join("loose-file"), b"x").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(root.join("cap"), root.join("cap-link")).unwrap();

        let plan = plan_prune(repo.path(), &root, 1).unwrap();
        let skipped: Vec<_> = plan
            .skipped
            .iter()
            .map(|s| s.path.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        assert!(
            skipped.contains(&"not-a-capsule".to_string()),
            "{skipped:?}"
        );
        assert!(skipped.contains(&"loose-file".to_string()), "{skipped:?}");
        #[cfg(unix)]
        assert!(skipped.contains(&"cap-link".to_string()), "{skipped:?}");
        assert!(plan.delete.is_empty(), "nothing eligible for deletion");
    }

    #[test]
    fn apply_stages_tracked_deletions_and_removes_untracked() {
        let repo = repo();
        let old = commit_at(repo.path(), "old", "1700000000");
        let new = commit_at(repo.path(), "new", "1700000100");
        let root = repo.path().join("anvil/evidence/capsules");
        let old_dir = write_capsule_dir(&root, "cap-old", &old);
        write_capsule_dir(&root, "cap-new", &new);
        // Track the old capsule (ADR-073 staged evidence), leave an
        // untracked scratch file inside it too.
        git(repo.path(), &["add", "anvil/evidence/capsules/cap-old"]);
        std::fs::write(old_dir.join("scratch.tmp"), b"x").unwrap();

        let plan = plan_prune(repo.path(), &root, 1).unwrap();
        assert_eq!(plan.delete.len(), 1);
        let failures = apply_prune(repo.path(), &plan);
        assert!(failures.is_empty(), "{failures:?}");
        assert!(!old_dir.exists(), "directory removed from the working tree");
        // The tracked manifest is gone from the index (staged deletion) —
        // a bare `git restore` cannot silently resurrect it.
        let out = Command::new("git")
            .args(["ls-files", "--", "anvil/evidence/capsules/cap-old"])
            .current_dir(repo.path())
            .output()
            .unwrap();
        assert!(
            out.stdout.is_empty(),
            "index no longer lists the pruned capsule"
        );
    }
}
