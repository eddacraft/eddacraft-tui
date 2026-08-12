//! GBASE-008 (ADR-105): refcount GC over merge-bases of ACTMO-registered
//! worktrees.
//!
//! Drops unreferenced shared bases only; never touches an in-use claim.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, PoisonError};

use super::base_store::{self, BaseReclaimOutcome, ClaimProcs};
use crate::broadcaster::TelemetryBroadcaster;
use crate::telemetry::{TelemetryCorrelation, TelemetryEmitter};

/// GBASE-011 (ADR-090 + ADR-105 §5): emits the worktree-scoped health envelopes
/// for shared-base GC failures through the **real** [`TelemetryBroadcaster`]
/// fan-out — the GC-side analogue of the trigger's `BaseTriggerNotifier`, so a GC
/// degradation is an envelope an opted-in subscriber receives, not only a log line.
///
/// Two message classes:
/// - **GC error** (a per-sha reclaim I/O failure) — [`Self::notify_gc_error`];
/// - **GC deferred** (the keep-set was uncertain, so the pass fail-safe-kept every
///   base) — [`Self::notify_gc_deferred`], an *operational* (lower-priority)
///   signal.
///
/// Both are scoped to **all** worktrees the daemon passed the pass: an unreferenced
/// base maps to no live worktree, and GC health affects every worktree that shares
/// the store, so the honest recipient set is the whole currently-registered set.
#[derive(Clone)]
pub struct BaseGcNotifier {
    broadcaster: Arc<TelemetryBroadcaster>,
    emitter: Arc<Mutex<TelemetryEmitter>>,
}

impl BaseGcNotifier {
    #[must_use]
    pub fn new(broadcaster: Arc<TelemetryBroadcaster>) -> Self {
        Self {
            broadcaster,
            emitter: Arc::new(Mutex::new(TelemetryEmitter::new())),
        }
    }

    /// Broadcast the "shared-base GC error" health envelope for every `worktree`.
    fn notify_gc_error(&self, worktrees: &[PathBuf]) {
        for worktree in worktrees {
            let envelope = {
                let mut emitter = self.emitter.lock().unwrap_or_else(PoisonError::into_inner);
                emitter.base_gc_error_health_envelope(
                    TelemetryCorrelation::default(),
                    worktree,
                    "shared-base GC could not reclaim an unreferenced base (I/O error); \
                     the pass skipped it and continues",
                )
            };
            let _ = self.broadcaster.broadcast(&envelope);
        }
    }

    /// Broadcast the operational "shared-base GC pass deferred" envelope for every
    /// `worktree` (a keep-set-uncertain fail-safe deferral).
    fn notify_gc_deferred(&self, worktrees: &[PathBuf]) {
        for worktree in worktrees {
            let envelope = {
                let mut emitter = self.emitter.lock().unwrap_or_else(PoisonError::into_inner);
                emitter.base_gc_deferred_health_envelope(
                    TelemetryCorrelation::default(),
                    worktree,
                    "shared-base GC deferred a pass: a registered worktree's merge-base \
                     was unresolvable (keep-set uncertain); keeping all bases",
                )
            };
            let _ = self.broadcaster.broadcast(&envelope);
        }
    }
}

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
///
/// # Classification table (fail-safe: uncertainty aborts, never under-retains)
///
/// Every git invocation is classified by **exit code + stderr shape**, so a
/// *deterministic absence* (which the build child would resolve identically, so
/// contributing nothing is correct) is never confused with an *unexpected
/// failure* (I/O error, repo mid-operation, OOM-kill) that must abort the pass
/// before any unlink:
///
/// | git call                          | outcome                                   | → |
/// | --------------------------------- | ----------------------------------------- | - |
/// | `merge-base HEAD <ref>` exit 0    | prints a sha                              | keep the sha |
/// | `merge-base HEAD <ref>` exit 1    | commits share no merge-base (documented)  | no key (deterministic) |
/// | `merge-base` exit >1 / signal     | unexpected                                | **Unavailable → abort** |
/// | `rev-parse` ref exit 0            | ref resolves                              | resolved |
/// | `rev-parse` ref non-zero, stderr matches a deterministic missing-ref / non-git-root shape | ref genuinely absent | missing (try next / uncovered) |
/// | `rev-parse` ref non-zero, stderr **unmatched** | unexpected                    | **Unavailable → abort** |
/// | `@{upstream}` exit 0              | branch tracks an upstream                 | refine |
/// | `@{upstream}` non-zero, stderr matches a deterministic no-upstream shape | branch tracks none | skip refinement (deterministic) |
/// | `@{upstream}` non-zero, stderr **unmatched** | unexpected (I/O, object store, …) | **Unavailable → abort** |
/// | any call: spawn failure / killed by signal | git could not run                | **Unavailable → abort** |
pub struct GitMergeBaseResolver {
    /// The git binary to invoke — `"git"` in production; a test double swaps it to
    /// exercise the real [`run_git`] classification against controlled failures.
    git_bin: std::ffi::OsString,
}

impl GitMergeBaseResolver {
    /// The production resolver, invoking `git` off `PATH`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            git_bin: std::ffi::OsString::from("git"),
        }
    }

    #[cfg(test)]
    fn with_git_bin(bin: impl Into<std::ffi::OsString>) -> Self {
        Self {
            git_bin: bin.into(),
        }
    }
}

impl Default for GitMergeBaseResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl KeepSetResolver for GitMergeBaseResolver {
    fn merge_base(&self, worktree: &Path) -> MergeBase {
        // Resolve the default branch (origin/HEAD, then the common fallbacks). An
        // absent default branch is an uncovered topology (ADR-105 §8), not a
        // transient error — such a worktree references no base. An UNEXPECTED git
        // failure here aborts (Unavailable) rather than silently shrinking the
        // keep-set.
        let default = match resolve_default_branch(&self.git_bin, worktree) {
            Ok(Some(branch)) => branch,
            Ok(None) => return MergeBase::Uncovered,
            Err(GitUnavailable) => return MergeBase::Unavailable,
        };

        let mut keys: BTreeSet<String> = BTreeSet::new();
        match classify_merge_base(run_git(
            &self.git_bin,
            worktree,
            &["merge-base", "--end-of-options", "HEAD", &default],
        )) {
            MergeBaseCall::Found(sha) => {
                keys.insert(sha);
            }
            // Exit 1: HEAD and the default branch share no merge-base — a
            // deterministic "no shared base" (the child resolves the same). The
            // branch may still track an upstream, so fall through to the refinement.
            MergeBaseCall::NoBase => {}
            MergeBaseCall::Unavailable => return MergeBase::Unavailable,
        }

        // `@{upstream}` refinement, only when the branch actually tracks one
        // (ADR-105 §6). Exit 0 ⇒ tracking. A *deterministic* no-upstream shape
        // (stderr matches [`is_deterministic_no_upstream`]) ⇒ skip refinement —
        // the build child falls back to the default-branch key identically, so
        // the keep-set stays correct. An **unmatched** non-zero exit (I/O error,
        // object-store trouble, empty/garbled stderr) is uncertainty → Unavailable
        // so the pass never reclaims a live base keyed only under the refinement.
        let upstream = match run_git(
            &self.git_bin,
            worktree,
            &[
                "rev-parse",
                "--abbrev-ref",
                "--symbolic-full-name",
                "@{upstream}",
            ],
        ) {
            GitRun::Failed => return MergeBase::Unavailable,
            GitRun::Exited {
                code: 0,
                stdout: Some(up),
                ..
            } => Some(up),
            // Exit 0 with empty stdout is not a tracked upstream in practice.
            GitRun::Exited { code: 0, .. } => None,
            GitRun::Exited { stderr, .. } => {
                if is_deterministic_no_upstream(&stderr) {
                    None
                } else {
                    return MergeBase::Unavailable;
                }
            }
        };
        if let Some(upstream) = upstream {
            match classify_merge_base(run_git(
                &self.git_bin,
                worktree,
                &["merge-base", "--end-of-options", "HEAD", &upstream],
            )) {
                MergeBaseCall::Found(refined) => {
                    keys.insert(refined);
                }
                MergeBaseCall::NoBase => {}
                // The refinement key is the one the child may have produced under;
                // an unexpected failure computing it is uncertainty → abort rather
                // than risk reclaiming a base keyed on the upstream refinement.
                MergeBaseCall::Unavailable => return MergeBase::Unavailable,
            }
        }

        if keys.is_empty() {
            MergeBase::Uncovered
        } else {
            MergeBase::Resolved(keys.into_iter().collect())
        }
    }
}

/// A `git` invocation could not be reduced to a deterministic answer (spawn
/// failure, killed by signal, or an unexpected non-zero exit) — pass-aborting.
struct GitUnavailable;

/// The raw result of running a git command, before command-specific
/// classification. `pub(crate)` — the raw-run vocabulary is shared with
/// [`crate::persistence_route`] (GBASE-009), so the daemon has **one** fail-safe
/// git-run table for both the GC keep-set and route decisions.
pub(crate) enum GitRun {
    /// git ran to completion: `code` is the exit status, `stdout` the trimmed
    /// first output line (if non-empty), `stderr` the full stderr text.
    Exited {
        code: i32,
        stdout: Option<String>,
        stderr: String,
    },
    /// git could **not** be run to a clean exit — the process failed to spawn OR
    /// was killed by a signal (no exit code). Always unexpected → callers map this
    /// to [`GitUnavailable`] / [`MergeBase::Unavailable`].
    Failed,
}

/// The bounded wall-clock a single `git` probe may take before it is killed and
/// classified [`GitRun::Failed`] (⇒ `Unavailable`, re-evaluated later). A hung git
/// (a wedged lock, a stuck credential helper, a dead network filesystem) must
/// never pin a background pool thread indefinitely — a pre-existing `.output()`
/// gap that GBASE-009's route resolver multiplies (a probe per registered
/// worktree). 10 s is far above a healthy `merge-base`/`rev-parse` (milliseconds)
/// yet bounds the pathological case.
const GIT_PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Run `<git_bin> -C <worktree> <args>` and capture its exit code + first stdout
/// line + stderr, or [`GitRun::Failed`] on a spawn failure, signal death, or a
/// timeout ([`GIT_PROBE_TIMEOUT`]). Never interprets the result — that is each
/// caller's command-specific job (see the classification table on
/// [`GitMergeBaseResolver`]).
///
/// Killability comes from polling `try_wait` rather than a blocking `.output()`;
/// the pipes are read **after** the child exits. That is deadlock-free without any
/// drain threads: a child that has exited wrote at most one pipe-buffer of output
/// (had it written more it would have blocked on the full pipe and NOT exited), so
/// the buffered bytes are safely readable in full. The bounded git probes here
/// (a sha / a ref) are kilobytes at most; a pathological chatty child that fills
/// the pipe and blocks simply never exits and is killed at the deadline.
pub(crate) fn run_git(git_bin: &std::ffi::OsStr, worktree: &Path, args: &[&str]) -> GitRun {
    use std::io::Read;
    use std::process::Stdio;
    use std::time::Instant;

    // Spawn failure (missing binary, permission) — unexpected.
    let Ok(mut child) = Command::new(git_bin)
        .arg("-C")
        .arg(worktree)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    else {
        return GitRun::Failed;
    };

    let deadline = Instant::now() + GIT_PROBE_TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    // Hung probe: kill + reap, then classify Failed (⇒ Unavailable).
                    let _ = child.kill();
                    let _ = child.wait();
                    return GitRun::Failed;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(_) => return GitRun::Failed,
        }
    };

    // The child has exited ⇒ its buffered output fits the pipe; read it directly.
    let mut stdout_bytes = Vec::new();
    if let Some(mut out) = child.stdout.take() {
        let _ = out.read_to_end(&mut stdout_bytes);
    }
    let mut stderr_bytes = Vec::new();
    if let Some(mut err) = child.stderr.take() {
        let _ = err.read_to_end(&mut stderr_bytes);
    }

    // `code()` is `None` when the child was terminated by a signal (e.g. OOM-kill)
    // — an unexpected death, never a deterministic answer.
    let Some(code) = status.code() else {
        return GitRun::Failed;
    };
    let stdout = String::from_utf8_lossy(&stdout_bytes)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_owned();
    let stdout = if stdout.is_empty() {
        None
    } else {
        Some(stdout)
    };
    let stderr = String::from_utf8_lossy(&stderr_bytes).into_owned();
    GitRun::Exited {
        code,
        stdout,
        stderr,
    }
}

/// The classified outcome of a `git merge-base HEAD <ref>` call. `pub(crate)` —
/// shared with [`crate::persistence_route`].
pub(crate) enum MergeBaseCall {
    /// Exit 0: a merge-base sha was printed.
    Found(String),
    /// Exit 1: the commits share no merge-base — a **documented, deterministic**
    /// outcome (`git merge-base` returns 1 when no common ancestor exists).
    NoBase,
    /// Exit >1, spawn failure, or signal death — unexpected → abort the pass.
    Unavailable,
}

/// Classify a `git merge-base` run by its **documented exit-code semantics**:
/// `0` = a merge-base was found, `1` = no merge-base exists (deterministic),
/// anything else (or a spawn/signal failure) = an unexpected error.
pub(crate) fn classify_merge_base(run: GitRun) -> MergeBaseCall {
    // Spawn failure / signal death is always unexpected.
    let GitRun::Exited { code, stdout, .. } = run else {
        return MergeBaseCall::Unavailable;
    };
    match (code, stdout) {
        (0, Some(sha)) => MergeBaseCall::Found(sha),
        // Exit 0 with no output cannot occur for `merge-base` in practice; exit 1
        // is the documented "the commits do not share a common ancestor" —
        // deterministic, not an error.
        (0, None) | (1, _) => MergeBaseCall::NoBase,
        // Exit >1 (128 = bad usage/object, 129 = usage) or any other code =
        // unexpected.
        _ => MergeBaseCall::Unavailable,
    }
}

/// The classified outcome of a ref-resolving `git rev-parse` call. `pub(crate)` —
/// shared with [`crate::persistence_route`] (GBASE-009).
pub(crate) enum RefCall {
    /// Exit 0: the ref resolved (the printed value is carried).
    Resolved(String),
    /// A **deterministic** absence — the ref does not exist, or the root is not a
    /// git repository (stderr matched a known missing-ref shape). Try the next
    /// candidate / treat as an uncovered topology.
    Missing,
    /// An **unexpected** non-zero exit (stderr unmatched), spawn failure, or signal
    /// — abort the pass rather than under-retain.
    Unavailable,
}

/// Whether `stderr` matches a **deterministic** "this ref/commit does not exist"
/// or "this is not a git repository" shape (ADR-105 §8 uncovered topologies).
///
/// This is deliberately **fragile-but-fail-safe**: the matched shapes are the
/// stable `git rev-parse` fatals for a genuinely-absent ref or non-git root, and
/// an **unmatched** non-zero exit is classified [`RefCall::Unavailable`] (abort),
/// **never** [`RefCall::Missing`] — so a message drift can only ever cause an
/// over-cautious extra keep, never an under-retaining wrong reclaim.
fn is_deterministic_missing_ref(stderr: &str) -> bool {
    let s = stderr.to_ascii_lowercase();
    s.contains("unknown revision")
        || s.contains("bad revision")
        || s.contains("needed a single revision")
        || s.contains("not a git repository")
}

/// Whether `stderr` matches a **deterministic** "this branch tracks no upstream"
/// shape from `git rev-parse --abbrev-ref --symbolic-full-name @{upstream}`.
///
/// Matched shapes are the stable fatals git emits when the current HEAD has no
/// configured upstream (or is detached and therefore cannot track one). An
/// **unmatched** non-zero exit is classified as unexpected ([`MergeBase::Unavailable`]),
/// never as "no upstream" — so a message drift or I/O fatal can only ever force
/// an over-cautious keep-all GC pass, never reclaim a live upstream-keyed base.
fn is_deterministic_no_upstream(stderr: &str) -> bool {
    let s = stderr.to_ascii_lowercase();
    // `fatal: no upstream configured for branch '…'`
    s.contains("no upstream configured")
        // `fatal: HEAD does not point to a branch` (detached HEAD)
        || s.contains("does not point to a branch")
        // Older / alternate phrasing still used in some git builds.
        || s.contains("no upstream")
}

/// Classify a ref-resolving `git rev-parse` run (see [`is_deterministic_missing_ref`]).
pub(crate) fn classify_ref(run: GitRun) -> RefCall {
    match run {
        GitRun::Failed => RefCall::Unavailable,
        GitRun::Exited {
            code: 0,
            stdout: Some(value),
            ..
        } => RefCall::Resolved(value),
        // Exit 0 with empty output does not occur for the `rev-parse` forms used
        // here (both print on success); treat defensively as a deterministic miss.
        GitRun::Exited { code: 0, .. } => RefCall::Missing,
        GitRun::Exited { stderr, .. } => {
            if is_deterministic_missing_ref(&stderr) {
                RefCall::Missing
            } else {
                RefCall::Unavailable
            }
        }
    }
}

/// Resolve the repo's default branch ref for `worktree` (ADR-105 §6): `origin/HEAD`
/// first, then the conventional `origin/main` / `origin/master` fallbacks. Each
/// `rev-parse` is classified so a deterministic missing ref falls through while an
/// unexpected git failure aborts.
///
/// **Keep-set-only fallback (intentional divergence from routing).** The
/// `origin/main`/`origin/master` fallback below is a *conservative superset* for
/// the GC keep-set, where over-retention (keeping a slightly-stale base) is safe.
/// GBASE-009's route resolver
/// ([`crate::persistence_route::GitRouteResolver`]) deliberately does **not**
/// reuse this — it probes `origin/HEAD` **only**, matching the producer exactly,
/// because routing an `origin/HEAD`-unset repo as covered would key a base on a
/// sha the producer can never produce. Keep the two resolvers divergent on this
/// fallback.
///
/// - `Ok(Some(ref))` — a default branch resolved.
/// - `Ok(None)` — no default branch is resolvable (a deterministic uncovered
///   topology — including a genuinely-non-git root).
/// - `Err(GitUnavailable)` — an unexpected git failure (abort the pass).
fn resolve_default_branch(
    git_bin: &std::ffi::OsStr,
    worktree: &Path,
) -> Result<Option<String>, GitUnavailable> {
    // `origin/HEAD` → e.g. `origin/main` (the configured default remote branch).
    // When origin/HEAD is unset git prints `origin/HEAD` (exit 0) or fatals with a
    // deterministic `unknown revision` shape — both fall through to the candidates.
    match classify_ref(run_git(
        git_bin,
        worktree,
        &[
            "rev-parse",
            "--abbrev-ref",
            "--end-of-options",
            "origin/HEAD",
        ],
    )) {
        RefCall::Resolved(head) if head != "origin/HEAD" => return Ok(Some(head)),
        RefCall::Resolved(_) | RefCall::Missing => {}
        RefCall::Unavailable => return Err(GitUnavailable),
    }
    // Fall back to the conventional default branch names, verifying each names a
    // real commit before trusting it. `--verify` (no `--quiet`, so the deterministic
    // fatal reaches stderr for classification) fatals with a known missing-ref shape
    // when the ref is absent → try the next candidate.
    for candidate in ["origin/main", "origin/master"] {
        match classify_ref(run_git(
            git_bin,
            worktree,
            &[
                "rev-parse",
                "--verify",
                "--end-of-options",
                &format!("{candidate}^{{commit}}"),
            ],
        )) {
            RefCall::Resolved(_) => return Ok(Some(candidate.to_owned())),
            RefCall::Missing => {}
            RefCall::Unavailable => return Err(GitUnavailable),
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
    sweep_unreferenced_bases_inner(base_dir, worktrees, resolver, procs, None)
}

/// GBASE-011: the sweep with an optional [`BaseGcNotifier`] wired. The public
/// [`sweep_unreferenced_bases`] delegates here with `None` (keeping every existing
/// caller/test signature-stable); the daemon path passes the notifier so GC
/// failures raise ADR-090 worktree-scoped health envelopes. Behaviour is otherwise
/// **identical** — emission is purely additive, and every failure stays non-fatal
/// (a reclaim error still skips-and-continues; an uncertain keep-set still
/// fail-safe-keeps every base).
fn sweep_unreferenced_bases_inner(
    base_dir: &Path,
    worktrees: &[PathBuf],
    resolver: &dyn KeepSetResolver,
    procs: &dyn ClaimProcs,
    notifier: Option<&BaseGcNotifier>,
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
        // GBASE-011: this fail-safe deferral is an OPERATIONAL health signal — an
        // expected, self-healing safety pass, not a hard error — so it raises the
        // lower-priority "GC pass deferred" envelope for every registered worktree.
        if let Some(notifier) = notifier {
            notifier.notify_gc_deferred(worktrees);
        }
        return outcome;
    }

    // 3. Reclaim the unreferenced, unclaimed bases.
    let mut outcome = GcOutcome::default();
    // GBASE-011 rate-limit: a per-PASS latch so a store with many erroring shas
    // raises exactly ONE "GC error" envelope-set this pass (dedupe identical
    // failures). Each pass call starts fresh, so a clean pass emits nothing and the
    // next erroring pass re-emits — "success resets the latch", pass-scoped.
    let mut gc_error_emitted = false;
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
                // GBASE-011: raise the "GC error" health envelope for every
                // registered worktree ONCE per pass (the erroring sha is still
                // skipped and the pass continues — skip semantics unchanged).
                if let Some(notifier) = notifier
                    && !gc_error_emitted
                {
                    notifier.notify_gc_error(worktrees);
                    gc_error_emitted = true;
                }
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
/// condition the save-time path and the GBASE-003 trigger use — an enabled
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
    notifier: Option<&BaseGcNotifier>,
) -> Option<GcOutcome> {
    if !anvil_graph_cache::snapshot::persist_graph_enabled(persist_graph_env) {
        return None;
    }
    // Mirror the trigger/save-time gate: require a resolvable base store dir, not
    // just the flag — the base lands under `<graph-cache>/base`.
    let base_dir = base_store::default_base_dir()?;
    Some(sweep_unreferenced_bases_inner(
        &base_dir,
        worktrees,
        &GitMergeBaseResolver::new(),
        &base_store::SystemClaimProcs,
        notifier,
    ))
}

/// The result of a [`purge_all_bases`] operator sweep. `purged` are the shas
/// whose `<sha>.base` was unlinked; `skipped_claimed` are shas left in place
/// because a **live production claim** holds them (the safe semantic — see the
/// function docs); `errors` counts per-sha I/O failures (each non-fatal).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PurgeOutcome {
    /// Shas whose base artefact was unlinked.
    pub purged: Vec<String>,
    /// Shas skipped because a live claim is producing them (kept, not force-yanked).
    pub skipped_claimed: Vec<String>,
    /// Per-sha I/O failures during the sweep (non-fatal; the sweep continues).
    pub errors: usize,
}

/// GBASE-010 operator escape hatch: **purge every base artefact** in the store,
/// as the disk-pressure remediation behind `anvil graph-base gc --purge-all`.
///
/// Unlike the keep-set GC pass ([`run_daemon_gc_pass`]), this ignores references
/// entirely — the operator has explicitly asked for the store to be emptied. It
/// still rides the **same guard-`flock` + claim-respecting destruction** as GC
/// ([`base_store::reclaim_unreferenced_base`]): a base under an **active
/// production claim** is **skipped** (reported in `skipped_claimed`), never
/// force-yanked out from under a live producer. That is the deliberately **safe
/// semantic** — non-blocking (the CLI never waits on a producer) and
/// non-destructive to in-flight work; the operator can re-run once production
/// settles, or the base is simply re-produced on the next ref-change trigger
/// (ADR-105 §6). A **stale** producing lock does not block the unlink and is left
/// for the claim path to reclaim (GC never touches a lock — the same rule as the
/// keep-set pass); such locks are byte-negligible and self-heal on the next
/// `claim()`.
#[must_use]
pub fn purge_all_bases(base_dir: &Path, procs: &dyn ClaimProcs) -> PurgeOutcome {
    let mut outcome = PurgeOutcome::default();
    for sha in enumerate_base_shas(base_dir) {
        match base_store::reclaim_unreferenced_base(base_dir, &sha, procs) {
            Ok(base_store::BaseReclaimOutcome::Reclaimed) => outcome.purged.push(sha),
            Ok(base_store::BaseReclaimOutcome::SkippedClaimed) => {
                outcome.skipped_claimed.push(sha);
            }
            // Absent = a raced peer removed it first; count it as purged-effect
            // (the store no longer holds it), not an error.
            Ok(base_store::BaseReclaimOutcome::Absent) => {}
            Err(_) => outcome.errors += 1,
        }
    }
    outcome
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

    // ---- run_git classification (Copilot FINDING 1): deterministic absence must
    // never be confused with an unexpected git failure. These exercise the REAL
    // `run_git` + classification against real processes (a `#!/bin/sh` fake git and
    // a missing binary), not the sweep-level `FakeResolver` double. ----

    /// Create an executable `#!/bin/sh` fake-git whose behaviour is `body` (a
    /// `case "$*"` over the invocation args). Returns the tempdir (keep it alive)
    /// and the binary path to hand to [`GitMergeBaseResolver::with_git_bin`].
    fn fake_git(body: &str) -> (tempfile::TempDir, std::ffi::OsString) {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("fake-git");
        std::fs::write(&bin, format!("#!/bin/sh\n{body}\n")).unwrap();
        let mut perms = std::fs::metadata(&bin).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&bin, perms).unwrap();
        (dir, bin.into_os_string())
    }

    #[test]
    fn classify_merge_base_exit_codes() {
        // Pure classification: exit 0+sha = found; exit 1 = deterministic no-base;
        // exit >1 / signal-death = unexpected.
        let found = GitRun::Exited {
            code: 0,
            stdout: Some("deadbeef".to_owned()),
            stderr: String::new(),
        };
        assert!(matches!(classify_merge_base(found), MergeBaseCall::Found(s) if s == "deadbeef"));
        assert!(matches!(
            classify_merge_base(GitRun::Exited {
                code: 1,
                stdout: None,
                stderr: String::new()
            }),
            MergeBaseCall::NoBase
        ));
        assert!(matches!(
            classify_merge_base(GitRun::Exited {
                code: 128,
                stdout: None,
                stderr: "fatal: bad object".to_owned()
            }),
            MergeBaseCall::Unavailable
        ));
        assert!(matches!(
            classify_merge_base(GitRun::Failed),
            MergeBaseCall::Unavailable
        ));
    }

    #[test]
    fn classify_ref_distinguishes_missing_from_unexpected() {
        // Deterministic missing-ref / non-git-root shapes → Missing; an UNMATCHED
        // non-zero stderr → Unavailable (fail-safe: never under-retain).
        for shape in [
            "fatal: ambiguous argument 'origin/HEAD': unknown revision or path",
            "fatal: bad revision 'origin/main'",
            "fatal: Needed a single revision",
            "fatal: not a git repository (or any parent)",
        ] {
            assert!(
                matches!(
                    classify_ref(GitRun::Exited {
                        code: 128,
                        stdout: None,
                        stderr: shape.to_owned()
                    }),
                    RefCall::Missing
                ),
                "deterministic shape must classify Missing: {shape}",
            );
        }
        // An unexpected fatal (I/O error) is NOT a deterministic absence → abort.
        assert!(matches!(
            classify_ref(GitRun::Exited {
                code: 128,
                stdout: None,
                stderr: "fatal: unable to read tree; disk I/O error".to_owned()
            }),
            RefCall::Unavailable
        ));
        // Spawn/signal failure → Unavailable.
        assert!(matches!(classify_ref(GitRun::Failed), RefCall::Unavailable));
        // A clean resolution → Resolved.
        assert!(matches!(
            classify_ref(GitRun::Exited {
                code: 0,
                stdout: Some("origin/main".to_owned()),
                stderr: String::new()
            }),
            RefCall::Resolved(v) if v == "origin/main"
        ));
    }

    #[test]
    fn real_run_git_spawn_failure_is_unavailable_and_aborts_pass() {
        // (ii) THE fail-safe hole regression, exercised through the REAL run_git on
        // a real failure mode: a non-existent git binary → spawn failure → Failed →
        // Unavailable → the pass reclaims NOTHING even with an unreferenced base.
        let resolver = GitMergeBaseResolver::with_git_bin("/nonexistent/definitely-not-git");
        let wt = PathBuf::from("/wt/any");
        assert!(
            matches!(resolver.merge_base(&wt), MergeBase::Unavailable),
            "a git spawn failure must classify Unavailable, not Uncovered",
        );

        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let orphan = sha('b');
        write_base(&dir, &orphan);
        let out = sweep_unreferenced_bases(&dir, &[wt], &resolver, &SystemClaimProcs);
        assert!(
            out.aborted_uncertain,
            "an unexpected git failure aborts the pass"
        );
        assert_eq!(out.reclaimed, 0);
        assert!(
            base_exists(&dir, &orphan),
            "fail-safe: a git failure must never shrink the keep-set and reclaim",
        );
    }

    #[test]
    fn real_run_git_merge_base_exit_1_is_uncovered_and_pass_proceeds() {
        // (i) `git merge-base` exit 1 (no shared history) is a DETERMINISTIC no-base
        // — the worktree references no base, so the pass proceeds and reclaims an
        // unrelated orphan. Exercised through the real run_git via a fake git.
        let (_g, bin) = fake_git(
            "case \"$*\" in \
               *merge-base*) exit 1 ;; \
               *@{upstream}*) echo 'fatal: no upstream configured' >&2; exit 128 ;; \
               *origin/HEAD*) echo origin/main; exit 0 ;; \
               *) exit 0 ;; \
             esac",
        );
        let resolver = GitMergeBaseResolver::with_git_bin(bin);
        let wt = PathBuf::from("/wt/unrelated");
        assert!(
            matches!(resolver.merge_base(&wt), MergeBase::Uncovered),
            "merge-base exit 1 (+ no upstream) resolves to Uncovered",
        );

        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let orphan = sha('b');
        write_base(&dir, &orphan);
        let out = sweep_unreferenced_bases(&dir, &[wt], &resolver, &SystemClaimProcs);
        assert!(
            !out.aborted_uncertain,
            "a deterministic no-base does not abort"
        );
        assert_eq!(out.reclaimed, 1, "the unreferenced orphan is reclaimed");
        assert!(!base_exists(&dir, &orphan));
    }

    #[test]
    fn real_run_git_merge_base_exit_gt1_is_unavailable_and_aborts() {
        // `git merge-base` exit >1 is an UNEXPECTED failure (not the documented
        // exit-1 no-base) → Unavailable → abort, zero unlinks.
        let (_g, bin) = fake_git(
            "case \"$*\" in \
               *merge-base*) echo 'fatal: unexpected' >&2; exit 3 ;; \
               *@{upstream}*) exit 128 ;; \
               *origin/HEAD*) echo origin/main; exit 0 ;; \
               *) exit 0 ;; \
             esac",
        );
        let resolver = GitMergeBaseResolver::with_git_bin(bin);
        let wt = PathBuf::from("/wt/broken");
        assert!(matches!(resolver.merge_base(&wt), MergeBase::Unavailable));

        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let orphan = sha('b');
        write_base(&dir, &orphan);
        let out = sweep_unreferenced_bases(&dir, &[wt], &resolver, &SystemClaimProcs);
        assert!(out.aborted_uncertain);
        assert_eq!(out.reclaimed, 0);
        assert!(base_exists(&dir, &orphan));
    }

    #[test]
    fn real_run_git_deterministic_missing_default_branch_is_uncovered() {
        // No resolvable default branch (origin/HEAD unset + no origin/main|master),
        // every rev-parse fatal matching a deterministic missing-ref shape → the
        // worktree is Uncovered (it never PRODUCES a base), so the pass proceeds.
        let (_g, bin) = fake_git(
            "case \"$*\" in \
               *origin/HEAD*) echo \"fatal: ambiguous argument 'origin/HEAD': unknown revision\" >&2; exit 128 ;; \
               *rev-parse*) echo 'fatal: Needed a single revision' >&2; exit 128 ;; \
               *) exit 0 ;; \
             esac",
        );
        let resolver = GitMergeBaseResolver::with_git_bin(bin);
        let wt = PathBuf::from("/wt/no-default");
        assert!(matches!(resolver.merge_base(&wt), MergeBase::Uncovered));

        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let orphan = sha('b');
        write_base(&dir, &orphan);
        let out = sweep_unreferenced_bases(&dir, &[wt], &resolver, &SystemClaimProcs);
        assert!(!out.aborted_uncertain);
        assert_eq!(out.reclaimed, 1);
    }

    #[test]
    fn real_run_git_unexpected_rev_parse_stderr_is_unavailable_and_aborts() {
        // A non-zero rev-parse whose stderr does NOT match a deterministic shape
        // (an I/O error) → Unavailable → abort. This is the exact hazard FINDING 1
        // named: an unexpected failure must not masquerade as a missing ref.
        let (_g, bin) = fake_git(
            "case \"$*\" in \
               *origin/HEAD*) echo 'fatal: unable to read tree; disk I/O error' >&2; exit 128 ;; \
               *) exit 0 ;; \
             esac",
        );
        let resolver = GitMergeBaseResolver::with_git_bin(bin);
        let wt = PathBuf::from("/wt/io-error");
        assert!(matches!(resolver.merge_base(&wt), MergeBase::Unavailable));

        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let orphan = sha('b');
        write_base(&dir, &orphan);
        let out = sweep_unreferenced_bases(&dir, &[wt], &resolver, &SystemClaimProcs);
        assert!(out.aborted_uncertain);
        assert_eq!(out.reclaimed, 0);
        assert!(base_exists(&dir, &orphan));
    }

    #[test]
    fn real_run_git_fatal_upstream_resolution_is_unavailable_and_aborts() {
        // Regression for clawpatch finding
        // fnd_sig-feat-library-895b9882f0-dc8b_d96805a06d: a fatal non-zero
        // `@{upstream}` probe (exit 128 + unmatched stderr, e.g. object-store
        // I/O) must NOT be treated as "no upstream". The keep-set would otherwise
        // drop the upstream-refined key and GC could reclaim a live base.
        let (_g, bin) = fake_git(
            "case \"$*\" in \
               *origin/HEAD*) echo origin/main; exit 0 ;; \
               *merge-base*origin/main*) echo defaultsha; exit 0 ;; \
               *@{upstream}*) echo 'fatal: unable to read tree; disk I/O error' >&2; exit 128 ;; \
               *) exit 0 ;; \
             esac",
        );
        let resolver = GitMergeBaseResolver::with_git_bin(bin);
        let wt = PathBuf::from("/wt/upstream-io-error");
        assert!(
            matches!(resolver.merge_base(&wt), MergeBase::Unavailable),
            "a fatal @{{upstream}} probe must classify Unavailable, not Resolved(default only)",
        );

        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        // Simulate a base keyed by the upstream refinement (what a live producer
        // would have written under). With the buggy classification this orphan
        // would be reclaimed; with the fix the pass must abort and keep it.
        let upstream_keyed = sha('u');
        write_base(&dir, &upstream_keyed);
        let out = sweep_unreferenced_bases(&dir, &[wt], &resolver, &SystemClaimProcs);
        assert!(
            out.aborted_uncertain,
            "fatal upstream-resolution must abort the GC pass"
        );
        assert_eq!(out.reclaimed, 0);
        assert!(
            base_exists(&dir, &upstream_keyed),
            "fail-safe: fatal upstream error must never reclaim a live base",
        );
    }

    #[test]
    fn real_run_git_deterministic_no_upstream_skips_refinement_and_keeps_default() {
        // The complementary case: a genuine "no upstream configured" fatal is a
        // deterministic skip — the default-branch key alone is the keep-set, and
        // the pass proceeds (no abort).
        let (_g, bin) = fake_git(
            "case \"$*\" in \
               *origin/HEAD*) echo origin/main; exit 0 ;; \
               *merge-base*origin/main*) echo defaultsha; exit 0 ;; \
               *@{upstream}*) echo \"fatal: no upstream configured for branch 'main'\" >&2; exit 128 ;; \
               *) exit 0 ;; \
             esac",
        );
        let resolver = GitMergeBaseResolver::with_git_bin(bin);
        let wt = PathBuf::from("/wt/no-upstream");
        match resolver.merge_base(&wt) {
            MergeBase::Resolved(keys) => {
                assert_eq!(keys, vec!["defaultsha".to_owned()]);
            }
            other => panic!("expected Resolved([defaultsha]), got {other:?}"),
        }

        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let default_keyed = "defaultsha".to_owned();
        let orphan = sha('o');
        // write_base uses 40-char hex stems; plant the default key via store.
        store::write_sealed(&dir, &format!("{default_keyed}.base"), &base_bytes()).unwrap();
        write_base(&dir, &orphan);
        let out = sweep_unreferenced_bases(&dir, &[wt], &resolver, &SystemClaimProcs);
        assert!(
            !out.aborted_uncertain,
            "deterministic no-upstream must not abort"
        );
        assert_eq!(out.kept, 1);
        assert_eq!(out.reclaimed, 1);
        assert!(base_exists(&dir, &default_keyed));
        assert!(!base_exists(&dir, &orphan));
    }

    #[test]
    fn is_deterministic_no_upstream_shapes() {
        assert!(is_deterministic_no_upstream(
            "fatal: no upstream configured for branch 'main'"
        ));
        assert!(is_deterministic_no_upstream(
            "fatal: HEAD does not point to a branch"
        ));
        assert!(!is_deterministic_no_upstream(
            "fatal: unable to read tree; disk I/O error"
        ));
        assert!(!is_deterministic_no_upstream(""));
    }

    // ---- GBASE-011: GC-failure health envelopes (ADR-090) ----

    use crate::broadcaster::TelemetryBroadcaster;
    use crate::fanout::{Fanout, OwnershipResolver, SubscriberId};

    /// A resolver that authorises each subscriber for exactly its own worktree.
    struct WtResolver {
        owners: Vec<(SubscriberId, String)>,
    }
    impl OwnershipResolver for WtResolver {
        fn is_authorised(&self, _s: &SubscriberId, _sess: &str) -> bool {
            false
        }
        fn is_authorised_for_worktree(&self, sub: &SubscriberId, wt: &str) -> bool {
            self.owners.iter().any(|(o, w)| o == sub && w == wt)
        }
    }

    /// Build a broadcaster + [`BaseGcNotifier`] with one subscriber per worktree
    /// path. Returns the notifier and the receivers in the same order.
    fn gc_fixture(
        worktrees: &[&str],
    ) -> (BaseGcNotifier, Vec<tokio::sync::mpsc::Receiver<String>>) {
        let mut owners = Vec::new();
        let mut subs = Vec::new();
        for wt in worktrees {
            let sub = SubscriberId::new(format!("owner-{wt}"));
            owners.push((sub.clone(), (*wt).to_owned()));
            subs.push(sub);
        }
        let broadcaster = Arc::new(TelemetryBroadcaster::new(Arc::new(Fanout::new(Box::new(
            WtResolver { owners },
        )))));
        let receivers = subs
            .into_iter()
            .map(|s| broadcaster.register(s, None))
            .collect();
        (BaseGcNotifier::new(broadcaster), receivers)
    }

    /// Create `<sha>.base` as a DIRECTORY so the reclaim `unlinkat` (which never
    /// removes a dir) fails `EISDIR` deterministically, regardless of uid — the
    /// hermetic stand-in for a per-sha reclaim I/O error. Tightens the base dir to
    /// `0o700` first so a sibling `write_base` still passes `write_sealed`'s
    /// group/other-accessibility guard (`create_dir_all` alone leaves it `0o755`).
    fn write_unremovable_base(dir: &Path, sha: &str) {
        std::fs::create_dir_all(dir).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        std::fs::create_dir_all(dir.join(format!("{sha}.base"))).unwrap();
    }

    #[test]
    fn gc_reclaim_error_emits_gc_error_envelope_and_pass_continues() {
        // (c) A per-sha reclaim I/O error raises the "shared-base GC error" envelope
        // for EVERY registered worktree, and the pass continues — a later reclaimable
        // orphan is still reclaimed (skip semantics unchanged).
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let erroring = sha('b'); // sorts first; unlinkat → EISDIR → Err
        let good = sha('c'); // sorts after; a normal reclaimable orphan
        write_unremovable_base(&dir, &erroring);
        write_base(&dir, &good);

        // No worktree references either sha ⇒ both are reclaim candidates.
        let resolver = FakeResolver::default();
        let (notifier, mut rxs) = gc_fixture(&["/wt/a", "/wt/b"]);
        let worktrees = [PathBuf::from("/wt/a"), PathBuf::from("/wt/b")];

        let out = sweep_unreferenced_bases_inner(
            &dir,
            &worktrees,
            &resolver,
            &SystemClaimProcs,
            Some(&notifier),
        );

        // The pass continued past the error: the good orphan was reclaimed.
        assert_eq!(out.reclaimed, 1, "the pass continues past the erroring sha");
        assert!(!base_exists(&dir, &good), "the good orphan is gone");
        assert!(
            dir.join(format!("{erroring}.base")).exists(),
            "the erroring base is left in place (skipped, unchanged semantics)",
        );
        // Every registered worktree's subscriber got the GC-error class.
        for rx in &mut rxs {
            let frame = rx
                .try_recv()
                .expect("each worktree gets a GC-error envelope");
            assert!(
                frame.contains("shared-base GC error"),
                "the GC-error class is delivered: {frame}",
            );
        }
    }

    #[test]
    fn gc_multiple_reclaim_errors_emit_one_envelope_per_pass() {
        // Rate-limit: two erroring shas in ONE pass raise exactly ONE envelope per
        // worktree (the pass-scoped latch dedupes identical failures).
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        write_unremovable_base(&dir, &sha('b'));
        write_unremovable_base(&dir, &sha('c'));

        let resolver = FakeResolver::default();
        let (notifier, mut rxs) = gc_fixture(&["/wt/a"]);
        let worktrees = [PathBuf::from("/wt/a")];

        let out = sweep_unreferenced_bases_inner(
            &dir,
            &worktrees,
            &resolver,
            &SystemClaimProcs,
            Some(&notifier),
        );
        assert_eq!(out.reclaimed, 0, "both erroring bases are skipped");

        let rx = &mut rxs[0];
        assert!(
            rx.try_recv().is_ok(),
            "the first reclaim error emits one envelope",
        );
        assert!(
            rx.try_recv().is_err(),
            "a second reclaim error in the same pass is rate-limited (one envelope)",
        );
    }

    #[test]
    fn gc_clean_pass_emits_no_envelope() {
        // (f) A pass with no error emits nothing — the pass-scoped latch resets each
        // pass, so a clean pass is silent and a later erroring pass re-emits.
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        write_base(&dir, &sha('b')); // a normal reclaimable orphan (no error)

        let resolver = FakeResolver::default();
        let (notifier, mut rxs) = gc_fixture(&["/wt/a"]);
        let worktrees = [PathBuf::from("/wt/a")];

        let out = sweep_unreferenced_bases_inner(
            &dir,
            &worktrees,
            &resolver,
            &SystemClaimProcs,
            Some(&notifier),
        );
        assert_eq!(out.reclaimed, 1, "the clean orphan is reclaimed");
        assert!(
            rxs[0].try_recv().is_err(),
            "a clean pass emits no health envelope",
        );
    }

    #[test]
    fn gc_uncertain_abort_emits_operational_deferred_envelope_per_worktree() {
        // (d) A keep-set-uncertain deferral raises the OPERATIONAL "GC pass deferred"
        // envelope (lower `normal` priority) for every registered worktree; no base
        // is reclaimed (fail-safe keep).
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let orphan = sha('b');
        write_base(&dir, &orphan);

        // One worktree resolves Unavailable ⇒ the whole pass defers.
        let unresolved = PathBuf::from("/wt/a");
        let resolver = FakeResolver::default().with(&unresolved, MergeBase::Unavailable);
        let (notifier, mut rxs) = gc_fixture(&["/wt/a", "/wt/b"]);
        let worktrees = [unresolved.clone(), PathBuf::from("/wt/b")];

        let out = sweep_unreferenced_bases_inner(
            &dir,
            &worktrees,
            &resolver,
            &SystemClaimProcs,
            Some(&notifier),
        );
        assert!(
            out.aborted_uncertain,
            "an unresolvable worktree defers the pass"
        );
        assert_eq!(out.reclaimed, 0, "fail-safe keeps every base");
        assert!(base_exists(&dir, &orphan), "the orphan is kept this pass");

        for rx in &mut rxs {
            let frame = rx
                .try_recv()
                .expect("each worktree gets a deferral envelope");
            assert!(
                frame.contains("shared-base GC pass deferred"),
                "the deferral class is delivered: {frame}",
            );
            assert!(
                frame.contains("\"priority\":\"normal\""),
                "the deferral is marked operational (normal priority): {frame}",
            );
        }
    }

    // ========================================================================
    // GBASE-010 operator escape hatch: purge_all_bases
    // ========================================================================

    #[test]
    fn purge_all_empties_the_store_on_demand() {
        // The disk-pressure remediation: with no active claims, purge-all unlinks
        // every base regardless of references, leaving the store empty.
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        for seed in ['a', 'b', 'c', 'd'] {
            write_base(&dir, &sha(seed));
        }

        let outcome = purge_all_bases(&dir, &SystemClaimProcs);
        assert_eq!(outcome.purged.len(), 4, "every base is purged: {outcome:?}");
        assert!(outcome.skipped_claimed.is_empty());
        assert_eq!(outcome.errors, 0);
        for seed in ['a', 'b', 'c', 'd'] {
            assert!(!base_exists(&dir, &sha(seed)), "the store is emptied");
        }
        assert!(
            enumerate_base_shas(&dir).is_empty(),
            "no base artefacts remain after purge-all",
        );
    }

    #[test]
    fn purge_all_empties_the_store_safely_while_a_claim_is_active() {
        // The safe semantic: purge-all empties the store but SKIPS a sha under an
        // active production claim (never force-yanked from a live producer). The
        // rest of the store is emptied; the claimed base survives and is reported.
        let tmp = tempfile::tempdir().unwrap();
        let dir = base_dir(&tmp);
        let claimed = sha('c');
        write_base(&dir, &claimed);
        write_base(&dir, &sha('a'));
        write_base(&dir, &sha('b'));

        // A live claim by this process protects `claimed`.
        let claim = match base_store::claim(&dir, &claimed, &SystemClaimProcs).unwrap() {
            base_store::ClaimOutcome::Acquired(c) => c,
            base_store::ClaimOutcome::Contended => panic!("first claim must acquire"),
        };

        let outcome = purge_all_bases(&dir, &SystemClaimProcs);
        assert_eq!(
            outcome.skipped_claimed,
            vec![claimed.clone()],
            "the actively-claimed sha is skipped, not yanked: {outcome:?}",
        );
        assert_eq!(outcome.purged.len(), 2, "the unclaimed bases are purged");
        assert_eq!(outcome.errors, 0);
        assert!(
            base_exists(&dir, &claimed),
            "the claimed base survives purge"
        );
        assert!(!base_exists(&dir, &sha('a')));
        assert!(!base_exists(&dir, &sha('b')));

        // Once production settles, a re-run empties the rest of the store.
        claim.release();
        let outcome2 = purge_all_bases(&dir, &SystemClaimProcs);
        assert_eq!(outcome2.purged, vec![claimed.clone()]);
        assert!(
            enumerate_base_shas(&dir).is_empty(),
            "re-running after the claim releases empties the store",
        );
    }
}
