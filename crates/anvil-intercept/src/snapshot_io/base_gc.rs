//! GBASE-008 (ADR-105 §5/§9): **refcount GC over ACTMO-registered worktrees'
//! merge-bases**.
//!
//! The shared base store ([`super::base_store`]) accumulates one write-once
//! `<sha>.base` artefact per merge-base a worktree was ever warmed from. When a
//! registered worktree rebases, its old merge-base base is left behind — the
//! **new shared-base orphan class** ADR-105 introduces. This module reclaims it.
//!
//! It is the merge-base-keyed **analogue of, not a replacement for**, the
//! per-worktree `<hash>.root` companion sweep
//! ([`super::sweep_orphan_snapshots_on_start`], CIB-096): that sweep reclaims a
//! per-worktree `.snap` whose *worktree directory* is gone; this GC reclaims a
//! shared `.base` whose *sha* is referenced by no live registered worktree's
//! current merge-base. Different orphan classes, different keep-sets.
//!
//! # The keep-set — how the daemon learns each worktree's merge-base (design A)
//!
//! The daemon is git-free on the hot path (ADR-061/064), but ADR-105 §5 sanctions
//! **background** compute for detection. GC is rare (startup + a low-cadence
//! periodic pass, never the hot path — design B), so the keep-set source is a
//! **bounded `git merge-base` shell-out per registered worktree per GC pass**
//! ([`GitMergeBaseResolver`], design A option 1). Option 3 (the build child
//! recording its sha) was rejected: it would stale between rebases exactly when GC
//! most needs the truth, and a per-pass shell-out on the background pool matches
//! the council posture. The resolver is a **seam** ([`KeepSetResolver`]) so tests
//! run hermetically — no git, no real process state.
//!
//! The keep-set is a deliberate **conservative superset**: for each worktree the
//! resolver contributes *every* plausible merge-base key (the default-branch base
//! **and** the `@{upstream}`-refined base when the branch tracks one — ADR-105 §6),
//! so whichever key the build child actually produced under is retained.
//! **Over-retention (keeping a slightly-stale base) is safe; wrongly reclaiming a
//! referenced base is not** — so any *uncertainty* is resolved toward keeping.
//!
//! Known gap: a base produced via a manual `anvil graph-base build
//! --merge-base <sha>` invocation whose sha matches neither keep-set key is
//! reclaimable on the next pass. The daemon's own trigger never passes
//! `--merge-base`, so this is reachable only out-of-band; worst case is a
//! cold-start rebuild (write-once store), never corruption.
//!
//! # Claim interplay (design C)
//!
//! A base under **active production** is never removed: [`super::base_store`]'s
//! [`reclaim_unreferenced_base`](super::base_store::reclaim_unreferenced_base)
//! classifies any `.producing/<sha>.lock` **under the same `.guard` `flock`** it
//! uses for the unlink, and skips a live/undecidable claim. A *stale* lock is not
//! GC's to reclaim — the claim path owns lock reclaim. The base unlink itself
//! rides that guard, honestly extending the module's destruction invariant to base
//! artefacts.
//!
//! # Epoch-stale bases (design D, ADR-105 §9)
//!
//! An epoch-mismatched base is GC-eligible **at zero refs like any other
//! unreferenced base** — no special casing. GC keys purely on the refcount: if no
//! live worktree's merge-base equals its sha, it is eligible (claim-permitting),
//! whether its bytes are current-epoch or stale. The load path already refuses to
//! *return* a stale-epoch base ([`super::base_store::load_base`]); GC reclaims it
//! here once unreferenced.
//!
//! # Failure posture
//!
//! Every path is **non-fatal and fail-safe**: a missing/unreadable base dir is a
//! no-op, a resolver that cannot resolve a worktree's merge-base makes the whole
//! pass reclaim **nothing** (keep everything), and a per-sha reclaim error is
//! logged and skipped, never a panic (ADR-105 §6).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::base_store::{self, BaseReclaimOutcome, ClaimProcs};

/// One registered worktree's current merge-base resolution — the keep-set source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeBase {
    /// The worktree's current merge-base resolved to one or more plausible base
    /// keys (the default-branch base, plus the `@{upstream}`-refined base when the
    /// branch tracks one). **Every** key is kept — a conservative superset, so the
    /// key the build child produced under is never wrongly reclaimed.
    Resolved(Vec<String>),
    /// The worktree references **no** shared base: an ADR-105 §8 uncovered topology
    /// (detached HEAD, no default branch, or no merge-base with it). Deterministic,
    /// **not** an error — it contributes nothing to the keep-set and never blocks
    /// reclaim (the worktree persists via the per-worktree path, not a base).
    Uncovered,
    /// The merge-base could **not** be resolved this pass (a transient failure:
    /// `git` could not be spawned, or the repo was mid-operation). **Uncertain** —
    /// the worktree may reference any base, so a pass that sees this reclaims
    /// **nothing** (fail-safe keep).
    Unavailable,
}

/// The keep-set source: resolves each registered worktree's current merge-base
/// sha(s). Injected so GC tests are hermetic (no git subprocess, no real process
/// state) — mirrors the [`ClaimProcs`] discipline in [`super::base_store`].
pub trait KeepSetResolver: Send + Sync {
    /// Resolve `worktree`'s current merge-base key(s) (design A). Bounded to a
    /// small, fixed number of `git` invocations per call (one per GC pass per
    /// worktree — GC is rare).
    fn merge_base(&self, worktree: &Path) -> MergeBase;
}

/// Production [`KeepSetResolver`]: a bounded `git merge-base` shell-out per
/// worktree (design A option 1, ADR-105 §5 background compute). Mirrors the CLI
/// producer's [`resolve_base_commit`](../../../anvil_cli/graph_base_producer)
/// resolution but keeps **both** candidate keys (default-branch and
/// `@{upstream}`-refined) rather than preferring the refinement, so the keep-set
/// is a conservative superset of whatever the child produced under.
pub struct GitMergeBaseResolver;

impl KeepSetResolver for GitMergeBaseResolver {
    fn merge_base(&self, worktree: &Path) -> MergeBase {
        // Resolve the default branch (origin/HEAD, then the common fallbacks). An
        // absent default branch is an uncovered topology (ADR-105 §8), not a
        // transient error — such a worktree references no base.
        let default = match resolve_default_branch(worktree) {
            Ok(Some(branch)) => branch,
            Ok(None) => return MergeBase::Uncovered,
            Err(GitUnavailable) => return MergeBase::Unavailable,
        };

        let mut keys: BTreeSet<String> = BTreeSet::new();
        match run_git(
            worktree,
            &["merge-base", "--end-of-options", "HEAD", &default],
        ) {
            Ok(Some(sha)) => {
                keys.insert(sha);
            }
            // No merge-base with the default branch — may still track an upstream.
            Ok(None) => {}
            Err(GitUnavailable) => return MergeBase::Unavailable,
        }

        // `@{upstream}` refinement, only when the branch actually tracks one
        // (ADR-105 §6). Best-effort: a missing upstream leaves the default key.
        if let Ok(Some(upstream)) = run_git(
            worktree,
            &[
                "rev-parse",
                "--abbrev-ref",
                "--symbolic-full-name",
                "@{upstream}",
            ],
        ) && let Ok(Some(refined)) = run_git(
            worktree,
            &["merge-base", "--end-of-options", "HEAD", &upstream],
        ) {
            keys.insert(refined);
        }

        if keys.is_empty() {
            MergeBase::Uncovered
        } else {
            MergeBase::Resolved(keys.into_iter().collect())
        }
    }
}

/// `git` itself could not be run (spawn failure) — a transient, pass-aborting
/// condition, distinct from a clean "no such ref" (which is `Ok(None)`).
struct GitUnavailable;

/// Run `git -C <worktree> <args>` and return the trimmed first line of stdout.
///
/// - `Ok(Some(line))` — git ran, exited 0, and printed a non-empty first line.
/// - `Ok(None)` — git ran but exited non-zero or printed nothing (a clean
///   "unresolvable ref" — e.g. detached HEAD, no upstream, no merge-base).
/// - `Err(GitUnavailable)` — git could not be spawned at all (missing binary,
///   permission) — the pass treats this as uncertainty.
fn run_git(worktree: &Path, args: &[&str]) -> Result<Option<String>, GitUnavailable> {
    let output = Command::new("git")
        .arg("-C")
        .arg(worktree)
        .args(args)
        .output()
        .map_err(|_| GitUnavailable)?;
    if !output.status.success() {
        return Ok(None);
    }
    let line = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_owned();
    Ok(if line.is_empty() { None } else { Some(line) })
}

/// Resolve the repo's default branch ref for `worktree` (ADR-105 §6): `origin/HEAD`
/// first, then the conventional `origin/main` / `origin/master` fallbacks.
///
/// - `Ok(Some(ref))` — a default branch resolved.
/// - `Ok(None)` — no default branch is resolvable (an uncovered topology).
/// - `Err(GitUnavailable)` — git could not be spawned.
fn resolve_default_branch(worktree: &Path) -> Result<Option<String>, GitUnavailable> {
    // `origin/HEAD` → e.g. `origin/main` (the configured default remote branch).
    if let Some(head) = run_git(
        worktree,
        &[
            "rev-parse",
            "--abbrev-ref",
            "--end-of-options",
            "origin/HEAD",
        ],
    )? && head != "origin/HEAD"
    {
        return Ok(Some(head));
    }
    // Fall back to the conventional default branch names, verifying each names a
    // real commit before trusting it.
    for candidate in ["origin/main", "origin/master"] {
        if run_git(
            worktree,
            &[
                "rev-parse",
                "--verify",
                "--quiet",
                "--end-of-options",
                &format!("{candidate}^{{commit}}"),
            ],
        )?
        .is_some()
        {
            return Ok(Some(candidate.to_owned()));
        }
    }
    Ok(None)
}

/// The tally a GC pass reports (structured logging + test assertions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GcOutcome {
    /// Bases retained because their sha is in the keep-set (a live worktree
    /// references it).
    pub kept: usize,
    /// Bases unlinked because no live worktree referenced their sha and no live
    /// claim held it.
    pub reclaimed: usize,
    /// Bases left in place because a live production claim held their sha
    /// (claim-respecting GC, ADR-105 §5).
    pub skipped_claimed: usize,
    /// `true` when a worktree merge-base was [`MergeBase::Unavailable`], so the
    /// keep-set was uncertain and the pass reclaimed **nothing** (fail-safe keep).
    pub aborted_uncertain: bool,
}

/// GBASE-008: sweep the base store, reclaiming every `<sha>.base` no live
/// registered worktree references, claim-permitting (ADR-105 §5/§9).
///
/// 1. Resolve the keep-set (the union of every registered worktree's plausible
///    merge-base key). If **any** worktree is [`MergeBase::Unavailable`], the
///    keep-set is uncertain — reclaim nothing, count all bases as kept, return
///    with `aborted_uncertain = true`.
/// 2. Enumerate `<sha>.base` artefacts under `base_dir` (deterministic order).
/// 3. For each base not in the keep-set, delegate to
///    [`reclaim_unreferenced_base`](base_store::reclaim_unreferenced_base) — which
///    skips a live claim and unlinks under the `.guard`.
///
/// Best-effort and non-fatal: a missing/unreadable `base_dir` yields an empty
/// [`GcOutcome`]; a per-sha reclaim error is logged and skipped.
pub fn sweep_unreferenced_bases(
    base_dir: &Path,
    worktrees: &[PathBuf],
    resolver: &dyn KeepSetResolver,
    procs: &dyn ClaimProcs,
) -> GcOutcome {
    // 1. Keep-set (conservative superset). Track uncertainty separately so a single
    // unresolvable worktree fail-safes the whole pass to keep.
    let mut keep: BTreeSet<String> = BTreeSet::new();
    let mut uncertain = false;
    for worktree in worktrees {
        match resolver.merge_base(worktree) {
            MergeBase::Resolved(shas) => keep.extend(shas),
            MergeBase::Uncovered => {}
            MergeBase::Unavailable => uncertain = true,
        }
    }

    // 2. Enumerate base artefacts (sorted for determinism).
    let bases = enumerate_base_shas(base_dir);

    if uncertain {
        // Fail-safe: an unresolvable worktree could reference any base, so reclaim
        // nothing this pass. A later pass (the merge-base resolves) does the work.
        let outcome = GcOutcome {
            kept: bases.len(),
            reclaimed: 0,
            skipped_claimed: 0,
            aborted_uncertain: true,
        };
        tracing::warn!(
            target: "anvil_intercept::base_gc",
            bases = outcome.kept,
            "shared-base GC skipped a pass: a registered worktree's merge-base was \
             unresolvable (keep-set uncertain); keeping all bases",
        );
        return outcome;
    }

    // 3. Reclaim the unreferenced, unclaimed bases.
    let mut outcome = GcOutcome::default();
    for sha in &bases {
        if keep.contains(sha) {
            outcome.kept += 1;
            continue;
        }
        match base_store::reclaim_unreferenced_base(base_dir, sha, procs) {
            Ok(BaseReclaimOutcome::Reclaimed) => outcome.reclaimed += 1,
            Ok(BaseReclaimOutcome::SkippedClaimed) => outcome.skipped_claimed += 1,
            // A base that vanished under us (a raced peer) is neither kept nor
            // reclaimed by us — nothing to count.
            Ok(BaseReclaimOutcome::Absent) => {}
            Err(err) => {
                tracing::warn!(
                    target: "anvil_intercept::base_gc",
                    sha = %sha,
                    error = %err,
                    "shared-base GC could not reclaim an unreferenced base; skipping",
                );
            }
        }
    }

    if outcome.reclaimed > 0 || outcome.skipped_claimed > 0 {
        tracing::info!(
            target: "anvil_intercept::base_gc",
            kept = outcome.kept,
            reclaimed = outcome.reclaimed,
            skipped_claimed = outcome.skipped_claimed,
            "shared-base GC pass complete (GBASE-008)",
        );
    }
    outcome
}

/// GBASE-008 daemon entry point (design B): run one GC pass over the default base
/// store with the production git resolver, gated on the **same** persistence
/// condition the save-time path and the GBASE-003 trigger use — an affirmative
/// `ANVIL_PERSIST_GRAPH` **and** a resolvable base store dir. Returns `None` (a
/// no-op) when the gate is off, so a caller can wire it unconditionally.
///
/// **Blocking**: it shells out `git merge-base` per worktree and does filesystem
/// I/O, so the daemon runs it on a blocking pool — never the single-thread async
/// runtime or the ref-watch background thread (which must never block).
#[must_use]
pub fn run_daemon_gc_pass(
    persist_graph_env: Option<&str>,
    worktrees: &[PathBuf],
) -> Option<GcOutcome> {
    if !anvil_graph_cache::snapshot::persist_graph_enabled(persist_graph_env) {
        return None;
    }
    // Mirror the trigger/save-time gate: require a resolvable base store dir, not
    // just the flag — the base lands under `<graph-cache>/base`.
    let base_dir = base_store::default_base_dir()?;
    Some(sweep_unreferenced_bases(
        &base_dir,
        worktrees,
        &GitMergeBaseResolver,
        &base_store::SystemClaimProcs,
    ))
}

/// Enumerate the shas of `<sha>.base` artefacts directly under `base_dir`, sorted
/// for deterministic iteration/logging. A missing/unreadable dir yields an empty
/// list (no-op GC). The `.producing` subdir and any non-`.base` leaf are ignored.
fn enumerate_base_shas(base_dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(base_dir) else {
        return Vec::new();
    };
    let mut shas: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some(base_store::BASE_EXT) {
            continue;
        }
        // The leaf is `<sha>.base`; the stem is the sha. A stem with a path
        // separator can never occur (a dir entry name is a single component), and
        // the downstream `unlinkat` re-validates the leaf name anyway.
        if let Some(sha) = path.file_stem().and_then(|s| s.to_str()) {
            shas.push(sha.to_owned());
        }
    }
    shas.sort();
    shas
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot_io::store;
    use anvil_graph_cache::snapshot::SnapshotPayload;
    use base_store::SystemClaimProcs;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A hermetic [`KeepSetResolver`]: a fixed per-worktree answer, no git.
    #[derive(Default)]
    struct FakeResolver {
        answers: HashMap<PathBuf, MergeBase>,
        calls: AtomicUsize,
    }

    impl FakeResolver {
        fn with(mut self, worktree: impl Into<PathBuf>, answer: MergeBase) -> Self {
            self.answers.insert(worktree.into(), answer);
            self
        }
    }

    impl KeepSetResolver for FakeResolver {
        fn merge_base(&self, worktree: &Path) -> MergeBase {
            self.calls.fetch_add(1, Ordering::Relaxed);
            // An unregistered worktree defaults to Uncovered (references no base).
            self.answers
                .get(worktree)
                .cloned()
                .unwrap_or(MergeBase::Uncovered)
        }
    }

    fn base_dir(tmp: &tempfile::TempDir) -> PathBuf {
        tmp.path().join("graph-cache").join("base")
    }

    /// A gate-clean `ANVILGB1` base payload.
    fn base_bytes() -> Vec<u8> {
        use anvil_graph_cache::{DependencyGraph, SymbolGraph};
        SnapshotPayload::from_graphs(&SymbolGraph::new(), &DependencyGraph::new())
            .unwrap()
            .to_base_bytes()
    }

    fn write_base(dir: &Path, sha: &str) {
        store::write_sealed(dir, &format!("{sha}.base"), &base_bytes()).unwrap();
    }

    fn base_exists(dir: &Path, sha: &str) -> bool {
        dir.join(format!("{sha}.base")).exists()
    }

    fn sha(seed: char) -> String {
        seed.to_string().repeat(40)
    }

    #[test]
    fn keep_set_retention_keeps_a_referenced_base() {
        // (a) a base referenced by a live registered worktree's merge-base survives.
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let referenced = sha('a');
        write_base(&dir, &referenced);

        let wt = PathBuf::from("/wt/live");
        let resolver =
            FakeResolver::default().with(&wt, MergeBase::Resolved(vec![referenced.clone()]));

        let out = sweep_unreferenced_bases(&dir, &[wt], &resolver, &SystemClaimProcs);
        assert_eq!(out.kept, 1, "the referenced base is kept");
        assert_eq!(out.reclaimed, 0);
        assert!(
            base_exists(&dir, &referenced),
            "referenced base survives GC"
        );
    }

    #[test]
    fn unreferenced_base_is_reclaimed() {
        // (b) a base no worktree references is unlinked.
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let referenced = sha('a');
        let orphan = sha('b');
        write_base(&dir, &referenced);
        write_base(&dir, &orphan);

        let wt = PathBuf::from("/wt/live");
        let resolver =
            FakeResolver::default().with(&wt, MergeBase::Resolved(vec![referenced.clone()]));

        let out = sweep_unreferenced_bases(&dir, &[wt], &resolver, &SystemClaimProcs);
        assert_eq!(out.reclaimed, 1, "the orphan base is reclaimed");
        assert_eq!(out.kept, 1, "the referenced base is kept");
        assert!(base_exists(&dir, &referenced), "referenced base survives");
        assert!(!base_exists(&dir, &orphan), "orphan base is gone");
    }

    #[test]
    fn upstream_refined_key_is_kept_by_the_conservative_superset() {
        // The keep-set unions BOTH plausible keys, so a base keyed under the
        // upstream refinement is retained even if the default-branch key differs.
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let default_key = sha('a');
        let upstream_key = sha('b');
        write_base(&dir, &upstream_key); // child produced under the refined key

        let wt = PathBuf::from("/wt/live");
        let resolver = FakeResolver::default().with(
            &wt,
            MergeBase::Resolved(vec![default_key.clone(), upstream_key.clone()]),
        );

        let out = sweep_unreferenced_bases(&dir, &[wt], &resolver, &SystemClaimProcs);
        assert_eq!(out.reclaimed, 0, "the upstream-keyed base is not reclaimed");
        assert!(base_exists(&dir, &upstream_key));
    }

    #[test]
    fn claim_respecting_gc_never_removes_a_base_under_production() {
        // (c) a sha under an active production claim is never removed even at zero
        // refs; once the claim releases, the next pass reclaims it.
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let producing = sha('c');
        write_base(&dir, &producing);

        // A live claim by THIS process (SystemClaimProcs stamps the current, live
        // pid) → the sha is under active production.
        let claim = match base_store::claim(&dir, &producing, &SystemClaimProcs).unwrap() {
            base_store::ClaimOutcome::Acquired(c) => c,
            base_store::ClaimOutcome::Contended => panic!("first claim must acquire"),
        };

        // No worktree references `producing` (empty keep-set), but the live claim
        // must protect it.
        let resolver = FakeResolver::default();
        let out = sweep_unreferenced_bases(&dir, &[], &resolver, &SystemClaimProcs);
        assert_eq!(out.skipped_claimed, 1, "the claimed base is skipped");
        assert_eq!(out.reclaimed, 0);
        assert!(
            base_exists(&dir, &producing),
            "a base under active production is never removed",
        );

        // Release the claim → the next pass reclaims the now-unclaimed orphan.
        claim.release();
        let out2 = sweep_unreferenced_bases(&dir, &[], &resolver, &SystemClaimProcs);
        assert_eq!(out2.reclaimed, 1, "released, the orphan is reclaimed");
        assert!(!base_exists(&dir, &producing));
    }

    #[test]
    fn epoch_stale_base_is_reclaimed_at_zero_refs() {
        // (d) an epoch/magic-mismatched base is GC-eligible at zero refs like any
        // other unreferenced base — no special casing. An ANVILGC1 (per-worktree)
        // artefact written under a base leaf is a stand-in for a stale-epoch base:
        // load_base refuses it, and GC reclaims it once unreferenced.
        use anvil_graph_cache::{DependencyGraph, SymbolGraph};
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let stale = sha('d');
        let gc_bytes = SnapshotPayload::from_graphs(&SymbolGraph::new(), &DependencyGraph::new())
            .unwrap()
            .to_bytes(); // ANVILGC1, not a current-epoch base
        store::write_sealed(&dir, &format!("{stale}.base"), &gc_bytes).unwrap();
        assert!(matches!(
            base_store::load_base(&dir, &stale),
            base_store::BaseLoadOutcome::Ignored
        ));

        let resolver = FakeResolver::default();
        let out = sweep_unreferenced_bases(&dir, &[], &resolver, &SystemClaimProcs);
        assert_eq!(out.reclaimed, 1, "the epoch-stale base is reclaimed");
        assert!(!base_exists(&dir, &stale));
    }

    #[test]
    fn unavailable_merge_base_aborts_the_pass_fail_safe() {
        // An unresolvable worktree merge-base makes the keep-set uncertain: the
        // pass reclaims NOTHING, even a base that looks orphaned.
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let orphan = sha('b');
        write_base(&dir, &orphan);

        let wt = PathBuf::from("/wt/broken");
        let resolver = FakeResolver::default().with(&wt, MergeBase::Unavailable);

        let out = sweep_unreferenced_bases(&dir, &[wt], &resolver, &SystemClaimProcs);
        assert!(out.aborted_uncertain, "uncertainty aborts the reclaim pass");
        assert_eq!(out.reclaimed, 0);
        assert_eq!(out.kept, 1, "all bases counted as kept");
        assert!(base_exists(&dir, &orphan), "fail-safe: nothing reclaimed");
    }

    #[test]
    fn uncovered_topology_contributes_nothing_and_blocks_nothing() {
        // A detached-HEAD / no-default-branch worktree references no base, so it
        // neither keeps a base nor (unlike Unavailable) aborts the pass.
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let orphan = sha('b');
        write_base(&dir, &orphan);

        let wt = PathBuf::from("/wt/detached");
        let resolver = FakeResolver::default().with(&wt, MergeBase::Uncovered);

        let out = sweep_unreferenced_bases(&dir, &[wt], &resolver, &SystemClaimProcs);
        assert!(
            !out.aborted_uncertain,
            "uncovered is deterministic, not uncertain"
        );
        assert_eq!(out.reclaimed, 1, "the orphan is reclaimed");
        assert!(!base_exists(&dir, &orphan));
    }

    #[test]
    fn empty_and_missing_base_dir_are_a_noop_no_panic() {
        // (f) determinism/no-panic on empty dirs / missing base dir.
        let resolver = FakeResolver::default();

        // Missing dir.
        let missing = PathBuf::from("/nonexistent/graph-cache/base");
        let out = sweep_unreferenced_bases(&missing, &[], &resolver, &SystemClaimProcs);
        assert_eq!(out, GcOutcome::default());

        // Existing but empty base dir.
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        store::ensure_dir(&dir).unwrap();
        let out = sweep_unreferenced_bases(&dir, &[], &resolver, &SystemClaimProcs);
        assert_eq!(out, GcOutcome::default());
    }

    #[test]
    fn producing_subdir_is_not_mistaken_for_a_base() {
        // The `.producing` claim subdir (and its guard/locks) must never be
        // enumerated as a base artefact.
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let referenced = sha('a');
        write_base(&dir, &referenced);
        // Force the `.producing` dir + guard into existence via a claim.
        let claim = match base_store::claim(&dir, &sha('e'), &SystemClaimProcs).unwrap() {
            base_store::ClaimOutcome::Acquired(c) => c,
            base_store::ClaimOutcome::Contended => panic!("acquire"),
        };
        claim.release();

        let shas = enumerate_base_shas(&dir);
        assert_eq!(shas, vec![referenced], "only the .base leaf is enumerated");
    }

    #[test]
    fn gc_races_claim_production_without_removing_a_claimed_base() {
        // Concurrency (campaign lesson: single-shot green lies). Loop many rounds,
        // each racing a GC pass against a live production claim on the SAME
        // unreferenced sha. The invariant: GC must NEVER unlink a base while its
        // claim is held. We hold the claim across the GC pass, then assert the base
        // survived, then release and confirm a follow-up pass reclaims it.
        for round in 0..150u32 {
            let tmp = tempfile::tempdir().unwrap();
            let dir = base_dir(&tmp);
            let s = format!("{round:040x}");
            write_base(&dir, &s);

            let claim = match base_store::claim(&dir, &s, &SystemClaimProcs).unwrap() {
                base_store::ClaimOutcome::Acquired(c) => c,
                base_store::ClaimOutcome::Contended => panic!("round {round}: acquire"),
            };

            // Race N GC passes against the held claim from several threads.
            let dir_a = Arc::new(dir.clone());
            std::thread::scope(|scope| {
                for _ in 0..4 {
                    let dir = Arc::clone(&dir_a);
                    scope.spawn(move || {
                        let resolver = FakeResolver::default();
                        let out = sweep_unreferenced_bases(&dir, &[], &resolver, &SystemClaimProcs);
                        // Under a held claim every pass must skip, never reclaim.
                        assert_eq!(out.reclaimed, 0, "round {round}: reclaimed a claimed base");
                    });
                }
            });
            assert!(
                base_exists(&dir, &s),
                "round {round}: a base under a held claim must survive every GC pass",
            );

            claim.release();
            let resolver = FakeResolver::default();
            let out = sweep_unreferenced_bases(&dir, &[], &resolver, &SystemClaimProcs);
            assert_eq!(
                out.reclaimed, 1,
                "round {round}: released orphan is reclaimed"
            );
        }
    }

    #[test]
    fn many_worktrees_union_into_the_keep_set() {
        // The keep-set is the union across all registered worktrees; each is
        // resolved exactly once per pass (bounded shell-outs, design A/B).
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let a = sha('a');
        let b = sha('b');
        let orphan = sha('c');
        write_base(&dir, &a);
        write_base(&dir, &b);
        write_base(&dir, &orphan);

        let wt1 = PathBuf::from("/wt/1");
        let wt2 = PathBuf::from("/wt/2");
        let resolver = FakeResolver::default()
            .with(&wt1, MergeBase::Resolved(vec![a.clone()]))
            .with(&wt2, MergeBase::Resolved(vec![b.clone()]));

        let out = sweep_unreferenced_bases(
            &dir,
            &[wt1.clone(), wt2.clone()],
            &resolver,
            &SystemClaimProcs,
        );
        assert_eq!(out.kept, 2, "both referenced bases kept");
        assert_eq!(out.reclaimed, 1, "only the unreferenced base reclaimed");
        assert_eq!(
            resolver.calls.load(Ordering::Relaxed),
            2,
            "each worktree resolved exactly once per pass",
        );
        assert!(base_exists(&dir, &a) && base_exists(&dir, &b) && !base_exists(&dir, &orphan));
    }
}
