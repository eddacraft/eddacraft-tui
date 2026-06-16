//! DSV-006 / Sub-phase A Task 10 — the daemon's two cooperating work pools, the
//! per-workspace in-flight admission token, and the chunked-yield background
//! scan loop (ADR-061 §4 resource model).
//!
//! # Why two pools, not one
//!
//! The save-time daemon serves latency-sensitive `validate_paths` requests
//! concurrently with whole-repo background scans. ADR-061 §4 splits the one
//! per-host work budget into **two cooperating rayon pools**: a small
//! *interactive* pool (`validate_paths` only, never starved) and a *background*
//! pool (full scans). The earlier "one pool, interactive preempts background"
//! framing is unimplementable — rayon has no task preemption and no priority
//! between jobs queued on the same pool. Separate pools own dedicated OS
//! threads, and rayon never steals work across pools, so a saturated background
//! pool cannot drain the interactive pool's threads.
//!
//! Co-operation between the pools (background scans handing cores back under
//! interactive load) is the chunked cancel/yield loop ([`run_chunked_scan`] +
//! [`ScanCancel`], Task 10b): rayon has no task preemption, so the background
//! scan voluntarily checks a cancel flag at every chunk boundary and stops
//! within one chunk when interactive work arrives. This module also owns the
//! **construction** of the pools (the hard predecessor of Task 8, which runs
//! the antipattern check on the interactive pool) plus the admission token.
//!
//! # Admission token
//!
//! [`WorkspaceAdmission`] bounds how many `validate_paths` requests for a single
//! [`WorktreeKey`] may be in flight at once. It layers over the IPC listener's
//! per-connection semaphore (`ipc.rs`): the semaphore caps total connections,
//! the admission token stops one busy workspace from monopolising the shared
//! interactive pool.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::available_parallelism;

use rayon::ThreadPool;

use crate::rule_cache::WorktreeKey;

/// Errors raised while constructing the daemon work pools.
#[derive(Debug, thiserror::Error)]
pub enum SchedulerError {
    /// rayon refused to build a thread pool (e.g. a zero-thread request, which
    /// the [`PoolBudget`] floors guard against, or an OS thread-spawn failure).
    #[error("failed to build daemon work pool `{name}`: {source}")]
    PoolBuild {
        /// The pool that failed to build.
        name: &'static str,
        /// The underlying rayon error.
        source: rayon::ThreadPoolBuildError,
    },
}

/// Thread budget for the two cooperating pools, derived from one per-host core
/// budget.
///
/// Policy ([`PoolBudget::from_cores`]):
///
/// - The host budget is Anvil's standard half-cores cap (`cores / 2`, the same
///   policy as `anvil-rayon-init`), but floored at 2 rather than that crate's 1:
///   this budget is split across two pools, so it needs at least two threads to
///   give each side a non-empty pool.
/// - The **interactive** pool is deliberately *small* — at most 4 threads (spec
///   appendix ≈2–4) and never more than half the host budget — because it only
///   serves the short, latency-sensitive `validate_paths` path.
/// - The **background** pool takes the remainder, floored at 1.
///
/// The invariants `interactive ∈ [1, 4]`, `background ≥ 1`, and
/// `background ≥ interactive` hold for every core count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolBudget {
    /// Threads reserved for the interactive (`validate_paths`) pool.
    pub interactive: usize,
    /// Threads reserved for the background (full-scan) pool.
    pub background: usize,
}

impl PoolBudget {
    /// Largest interactive pool we will build; the interactive path is short and
    /// latency-bound, so more threads buy nothing and only crowd the host.
    const MAX_INTERACTIVE: usize = 4;

    /// Split a host's available cores into interactive + background budgets.
    #[must_use]
    pub fn from_cores(available_cores: usize) -> Self {
        let host_budget = (available_cores / 2).max(2);
        let interactive = (host_budget / 2).clamp(1, Self::MAX_INTERACTIVE);
        let background = host_budget.saturating_sub(interactive).max(1);
        Self {
            interactive,
            background,
        }
    }

    /// Derive the budget from the current host's reported parallelism, falling
    /// back to a single core if the OS cannot report it.
    #[must_use]
    pub fn from_host() -> Self {
        let cores = available_parallelism().map_or(1, std::num::NonZero::get);
        Self::from_cores(cores)
    }
}

/// The daemon's two cooperating rayon pools.
///
/// The interactive pool is reserved for `validate_paths`; the background pool
/// runs full scans. They are separate pools with dedicated OS threads, so a
/// saturated background pool cannot starve interactive work.
#[derive(Debug)]
pub struct WorkScheduler {
    interactive: ThreadPool,
    background: ThreadPool,
    budget: PoolBudget,
}

impl WorkScheduler {
    /// Build both pools sized from the current host's parallelism.
    pub fn new() -> Result<Self, SchedulerError> {
        Self::with_budget(PoolBudget::from_host())
    }

    /// Build both pools from an explicit budget (used by tests and any caller
    /// that wants a deterministic split).
    pub fn with_budget(budget: PoolBudget) -> Result<Self, SchedulerError> {
        let interactive = build_pool("anvil-intd-interactive", budget.interactive)?;
        let background = build_pool("anvil-intd-background", budget.background)?;
        Ok(Self {
            interactive,
            background,
            budget,
        })
    }

    /// The interactive pool — `validate_paths` work (and the Task 8 antipattern
    /// check) runs here, never on the global pool.
    #[must_use]
    pub fn interactive(&self) -> &ThreadPool {
        &self.interactive
    }

    /// The background pool — full scans run here.
    #[must_use]
    pub fn background(&self) -> &ThreadPool {
        &self.background
    }

    /// The thread budget the pools were built with.
    #[must_use]
    pub fn budget(&self) -> PoolBudget {
        self.budget
    }
}

fn build_pool(name: &'static str, threads: usize) -> Result<ThreadPool, SchedulerError> {
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .thread_name(move |i| format!("{name}-{i}"))
        .build()
        .map_err(|source| SchedulerError::PoolBuild { name, source })
}

/// Cooperative cancellation handle for a background scan (Task 10b).
///
/// rayon has no task preemption and the two pools never steal across each other
/// ([`WorkScheduler`]), so cooperation — a saturated background scan handing its
/// cores back when interactive load arrives — is *explicit*: the scan checks
/// this flag at every chunk boundary, and any other thread (e.g. the IPC
/// listener admitting a `validate_paths` request) flips it via [`ScanCancel::cancel`].
///
/// Cheap to clone — every clone observes the same flag.
#[derive(Debug, Clone, Default)]
pub struct ScanCancel {
    flag: Arc<AtomicBool>,
}

impl ScanCancel {
    /// A fresh, un-cancelled handle.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Request the scan to yield at its next chunk boundary. Idempotent and safe
    /// to call from any thread.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Release);
    }

    /// Whether cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }
}

/// How a chunked background scan ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanOutcome {
    /// Every item was processed.
    Completed,
    /// Cancellation was observed at a chunk boundary; the scan yielded after
    /// `processed` items. At most one chunk's worth of work runs after a
    /// [`ScanCancel::cancel`] requested mid-chunk. A yield only ever happens
    /// *before* a not-yet-started chunk, so `processed` is always the sum of
    /// whole completed chunks — never a partial-chunk count.
    Yielded {
        /// Items processed before yielding. Doubles as a resume offset: the
        /// caller may continue with `&items[processed..]` on a fresh
        /// [`run_chunked_scan`] call.
        processed: usize,
    },
}

/// Run `work` over `items` in chunks of `chunk_size`, checking `cancel` *before*
/// each chunk so a background scan hands its cores back within one chunk of a
/// cancel request (ADR-061 §4 cooperative yield).
///
/// The flag is checked at chunk boundaries, never per item, so the per-item
/// overhead is just the work itself; the trade-off is latency-bounded by the
/// chunk size. Because the check is before each chunk:
///
/// - over a non-empty `items`, cancelling before the first chunk yields
///   immediately with `processed == 0` and `work` is never called;
/// - cancelling mid-chunk always finishes the current chunk (at most
///   `chunk_size - 1` further items). If a later chunk remains, the check before
///   it yields with `processed` a whole number of completed chunks. If the
///   cancel lands in the *final* chunk there is no later boundary to observe it,
///   so the scan finishes that chunk and returns [`ScanOutcome::Completed`] —
///   a mid-chunk cancel near the end does not guarantee `Yielded`. Either way no
///   *new* chunk starts once the flag is set.
///
/// An empty `items` has no chunks, so the cancel check never runs and the result
/// is [`ScanOutcome::Completed`] (nothing to scan) regardless of the flag.
///
/// `items` is a slice (not an iterator) so a yielded scan can be resumed with
/// `&items[processed..]`. `chunk_size` is floored at 1 — a zero chunk would
/// never make progress.
pub fn run_chunked_scan<T, F>(
    items: &[T],
    chunk_size: usize,
    cancel: &ScanCancel,
    mut work: F,
) -> ScanOutcome
where
    F: FnMut(&T),
{
    let chunk_size = chunk_size.max(1);
    let mut processed = 0;
    for chunk in items.chunks(chunk_size) {
        if cancel.is_cancelled() {
            return ScanOutcome::Yielded { processed };
        }
        for item in chunk {
            work(item);
            processed += 1;
        }
    }
    ScanOutcome::Completed
}

/// Per-[`WorktreeKey`] in-flight-work admission, layered over the IPC listener's
/// per-connection semaphore. Bounds how many `validate_paths` requests for a
/// single workspace may be in flight at once.
///
/// Cheap to clone — all clones share one slot table.
#[derive(Debug, Clone)]
pub struct WorkspaceAdmission {
    max_inflight: usize,
    slots: Arc<Mutex<HashMap<WorktreeKey, usize>>>,
}

impl WorkspaceAdmission {
    /// Construct an admission gate that allows at most `max_inflight` concurrent
    /// in-flight requests per workspace. A request of 0 is clamped to 1 — the
    /// daemon must always admit at least one unit of work.
    #[must_use]
    pub fn new(max_inflight: usize) -> Self {
        Self {
            max_inflight: max_inflight.max(1),
            slots: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Try to admit one unit of in-flight work for `key`. Returns an
    /// [`AdmissionGuard`] that frees the slot on drop, or `None` if the
    /// workspace is already at its cap (the caller maps `None` to a
    /// server-busy / stale response — it never silently drops the request).
    #[must_use]
    pub fn try_admit(&self, key: &WorktreeKey) -> Option<AdmissionGuard> {
        let mut slots = lock(&self.slots);
        let count = slots.entry(key.clone()).or_insert(0);
        if *count >= self.max_inflight {
            return None;
        }
        *count += 1;
        Some(AdmissionGuard {
            key: key.clone(),
            slots: Arc::clone(&self.slots),
        })
    }

    /// The per-workspace in-flight cap.
    #[must_use]
    pub fn max_inflight(&self) -> usize {
        self.max_inflight
    }
}

/// RAII guard for one admitted unit of in-flight work. Releases the slot when
/// dropped.
#[derive(Debug)]
pub struct AdmissionGuard {
    key: WorktreeKey,
    slots: Arc<Mutex<HashMap<WorktreeKey, usize>>>,
}

impl Drop for AdmissionGuard {
    fn drop(&mut self) {
        let mut slots = lock(&self.slots);
        // The key is always present here in correct usage — a guard cannot
        // exist without the matching increment, and entries are only removed
        // when the count reaches zero. `saturating_sub` keeps `Drop` panic-free
        // even if a poisoned-lock recovery ever handed us a stale zero count
        // (an underflow would otherwise abort the process from within `Drop`).
        debug_assert!(
            slots.contains_key(&self.key),
            "admission guard dropped for an absent workspace key"
        );
        if let Some(count) = slots.get_mut(&self.key) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                // Drop the entry so the table does not grow unbounded with
                // one zero-count entry per workspace ever seen.
                slots.remove(&self.key);
            }
        }
    }
}

/// Lock the slot table, recovering from a poisoned mutex rather than
/// propagating a panic (which, in `Drop`, would abort the process).
///
/// Recovery is sound here because, in release builds, both critical sections —
/// `try_admit` and `AdmissionGuard::drop` — perform only non-panicking
/// operations between acquiring and releasing the lock (integer arithmetic,
/// `HashMap` entry/get, and an `Arc::clone`), so a poisoned lock implies the
/// allocator already aborted rather than that the slot table holds logically
/// corrupt state. The one panic vector is the `debug_assert!` in
/// `AdmissionGuard::drop`, which is compiled out of release builds and fires
/// only on an internal invariant violation (a guard for an absent key); if it
/// ever does fire under test, recovering the guard here keeps the count
/// monotonic via `saturating_sub` instead of escalating to a process abort.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Per-workspace denial-of-service caps for the save-time path (DSV-006 / Task
/// 11, ADR-061 §4).
///
/// Three work vectors are unbounded by default and a hostile or merely
/// pathological workspace can drive any of them to starve the daemon:
///
/// - a single enormous file the verdict path would parse + antipattern-scan
///   ([`max_parse_bytes`](Self::max_parse_bytes));
/// - a deeply nested directory tree a background full scan would descend
///   ([`max_walk_depth`](Self::max_walk_depth)); and
/// - a directory tree with an enormous *file count* a full scan would
///   accumulate ([`max_walk_files`](Self::max_walk_files)).
///
/// The classic fourth vector — symlink cycles — is already dead on the verdict
/// read path via the Task 3 `openat2(RESOLVE_NO_SYMLINKS|RESOLVE_BENEATH)`
/// guard, and [`walk_capped`] does not follow symlinked directories, so it does
/// not need a separate cap here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DosCaps {
    /// Largest file (in bytes) the verdict path will parse + antipattern-scan.
    /// A file above this is skipped with a diagnostic and never parsed (the
    /// parse + scan are the expensive, super-linear work; see
    /// [`validate_paths`](crate::validate_paths)). A separate, larger
    /// memory-DoS ceiling on the *read* itself lives at
    /// [`path_safety::MAX_GUARDED_READ_BYTES`](crate::path_safety::MAX_GUARDED_READ_BYTES).
    pub max_parse_bytes: usize,
    /// Deepest directory nesting a background full scan will descend. Files
    /// directly in the workspace root are depth 1; a file deeper than this is
    /// not collected (see [`walk_capped`]).
    pub max_walk_depth: usize,
    /// Largest number of files a single background-scan walk will collect. The
    /// depth cap alone does not bound a *wide* tree (millions of files at a
    /// shallow depth), so the walk stops once this many files are gathered.
    pub max_walk_files: usize,
}

impl DosCaps {
    /// Default parse-size cap: 2 MiB. Source files the antipattern family
    /// targets sit far below this; a file above it is almost always generated,
    /// vendored, or hostile, and skipping its parse costs only a localised
    /// coverage gap (surfaced as a diagnostic), never a wrong verdict.
    pub const DEFAULT_MAX_PARSE_BYTES: usize = 2 * 1024 * 1024;

    /// Default walk-depth cap. Real source trees are nowhere near this deep;
    /// the cap exists to stop an adversarial or symlink-free-but-cyclic-by-name
    /// tree from making a full scan run unboundedly.
    pub const DEFAULT_MAX_WALK_DEPTH: usize = 64;

    /// Default walk file-count cap: 100k files. A real source workspace is well
    /// under this; the cap bounds the walk's result allocation against a
    /// pathologically wide tree.
    pub const DEFAULT_MAX_WALK_FILES: usize = 100_000;
}

impl Default for DosCaps {
    fn default() -> Self {
        Self {
            max_parse_bytes: Self::DEFAULT_MAX_PARSE_BYTES,
            max_walk_depth: Self::DEFAULT_MAX_WALK_DEPTH,
            max_walk_files: Self::DEFAULT_MAX_WALK_FILES,
        }
    }
}

/// Walk `root` collecting regular files, descending at most `max_depth`
/// directory levels and collecting at most `max_files` files (DSV-006 / Task 11
/// walk caps, ADR-061 §4).
///
/// Depth semantics: files directly in `root` are depth 1, files one directory
/// down are depth 2, and so on; a file deeper than `max_depth` is not
/// collected. `max_depth` and `max_files` are each floored at 1 so the call
/// always at least lists (some of) the root's own files. The walk stops as soon
/// as `max_files` files are gathered — the cap bounds the result allocation
/// against a pathologically wide tree.
///
/// The traversal uses an **explicit stack**, not recursion: a recursive form
/// would put the directory nesting on the thread stack, which a deep (or
/// operator-misconfigured) `max_depth` could overflow. The stack form is O(1)
/// in call frames at any depth.
///
/// Symlinked directories are **not** followed. A background scan that chased
/// symlinks would reintroduce the cycle `DoS` the verdict read path already
/// closes with `openat2(RESOLVE_NO_SYMLINKS)` (Task 3); the walk mirrors that
/// stance with a cheap `file_type()` check rather than a second realpath. A
/// directory that cannot be read (permissions, races) is skipped silently — a
/// best-effort scan never aborts the whole walk over one unreadable subtree.
///
/// This is the bounded primitive the background full-scan executor consumes; it
/// is allocation-simple (returns an owned `Vec`) because the executor chunks
/// the result through [`run_chunked_scan`].
#[must_use]
pub fn walk_capped(root: &Path, max_depth: usize, max_files: usize) -> Vec<PathBuf> {
    let max_depth = max_depth.max(1);
    let max_files = max_files.max(1);
    let mut out = Vec::new();
    // (directory, depth-of-the-entries-inside-it). Seeding at depth 1 makes the
    // root's own files depth 1.
    let mut stack: Vec<(PathBuf, usize)> = vec![(root.to_path_buf(), 1)];
    while let Some((dir, depth)) = stack.pop() {
        if depth > max_depth {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            // Never follow symlinks — a symlinked directory could form a cycle
            // or escape the workspace, the same DoS the read path forbids.
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                stack.push((entry.path(), depth + 1));
            } else if file_type.is_file() {
                out.push(entry.path());
                if out.len() >= max_files {
                    return out;
                }
            }
        }
    }
    out
}

/// Hard ceiling on how far past `max_files` a [`walk_gitignored`] count runs,
/// as a multiple of the collection cap. The walk collects at most `max_files`
/// paths but keeps *counting* to report `GitignoreWalk::total`; this bounds that
/// counting against a pathologically wide *non-ignored* tree (the residual `DoS`
/// the gitignore pre-filter does not remove) so a 50M-file directory cannot make
/// the walk run unboundedly. When hit, `total` is a lower bound — still strictly
/// greater than `max_files`, so the worktree still resolves to `Bounded`.
const WALK_COUNT_CEILING_FACTOR: usize = 8;

/// The outcome of a [`walk_gitignored`] full-scan walk: the collected file
/// paths (capped) plus the total count of gitignore-filtered files found.
#[derive(Debug, Clone)]
pub struct GitignoreWalk {
    /// Regular-file paths to scan, at most `max_files` (absolute, under `root`).
    pub files: Vec<PathBuf>,
    /// Total gitignore-filtered regular files the depth-capped walk found (a
    /// lower bound once [`WALK_COUNT_CEILING_FACTOR`] is hit). Equal to
    /// `files.len()` when the worktree fit under the cap; strictly greater when
    /// it was truncated.
    pub total: usize,
}

impl GitignoreWalk {
    /// `true` when the worktree exceeded the file-count cap after the gitignore
    /// pre-filter — i.e. the warm graph will be **bounded**, not complete
    /// (DSV-045 / ADR-085 Decision 5).
    #[must_use]
    pub fn truncated(&self) -> bool {
        self.total > self.files.len()
    }
}

/// Walk `root` collecting regular files **pre-filtered through `.gitignore`**,
/// descending at most `max_depth` levels and collecting at most `max_files`
/// paths, while reporting the total filtered file count for the truncation
/// decision (DSV-045 / ADR-085 Decision 5).
///
/// This is [`walk_capped`]'s gitignore-aware sibling, built for the full-scan
/// executor. The crucial difference from `walk_capped` is the ordering ADR-085
/// fixes: the gitignore filter is applied **before** files are counted against
/// `max_files`, so a repo whose bulk is `node_modules`/`target` (ignored) warms
/// a *complete* graph of its source instead of truncating on machine-generated
/// files. Like `walk_capped` it:
///
/// - never follows symlinks (`follow_links(false)`) — the same cycle/escape `DoS`
///   stance the read path enforces;
/// - bounds depth (`max_depth`, files directly in `root` are depth 1); and
/// - bounds the collected `Vec` at `max_files`.
///
/// It honours `.gitignore`/`.ignore`/parent-directory ignores **even outside a
/// git repository** (`require_git(false)`) and lets the `ignore` crate's
/// built-in `.git/`-directory skip keep VCS internals out. Dotfiles are *not*
/// skipped (`hidden(false)`): only gitignore-marked paths are filtered, matching
/// the ADR's "pre-filtered through gitignore" wording (a non-source dotfile the
/// parser cannot handle simply yields no symbols downstream). Unreadable entries
/// are skipped silently — a best-effort scan never aborts over one bad subtree.
///
/// `max_depth` and `max_files` are each floored at 1.
#[must_use]
pub fn walk_gitignored(root: &Path, max_depth: usize, max_files: usize) -> GitignoreWalk {
    let max_depth = max_depth.max(1);
    let max_files = max_files.max(1);
    let count_ceiling = max_files.saturating_mul(WALK_COUNT_CEILING_FACTOR);

    let mut files = Vec::new();
    let mut total = 0usize;

    let walker = ignore::WalkBuilder::new(root)
        .follow_links(false)
        .hidden(false)
        .parents(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .ignore(true)
        // Honour `.gitignore` even when `root` is not inside a git checkout — a
        // fresh MCP session may open a non-git directory that still carries one.
        .require_git(false)
        .max_depth(Some(max_depth))
        .build();

    for entry in walker {
        let Ok(entry) = entry else {
            // Permission / race error on one entry — skip it, never abort.
            continue;
        };
        // Depth 0 is `root` itself; collect only regular files beneath it. A
        // symlink's `file_type` is `is_symlink` (we do not follow), so this also
        // excludes symlinks — matching `walk_capped`.
        if entry.depth() == 0 {
            continue;
        }
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        total += 1;
        if files.len() < max_files {
            files.push(entry.into_path());
        }
        if total >= count_ceiling {
            // Bound the counting against a pathologically wide non-ignored
            // tree; `total` is now a lower bound (> max_files ⇒ still Bounded).
            break;
        }
    }

    GitignoreWalk { files, total }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Barrier;
    use std::sync::atomic::AtomicUsize;

    fn key(p: &str) -> WorktreeKey {
        WorktreeKey::from_canonical(PathBuf::from(p))
    }

    /// File-count cap large enough to be irrelevant for the depth/symlink tests.
    const UNCAPPED_FILES: usize = usize::MAX;

    fn names_of(paths: &[PathBuf]) -> std::collections::HashSet<String> {
        paths
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn dos_caps_defaults_are_sane() {
        let caps = DosCaps::default();
        assert_eq!(caps.max_parse_bytes, 2 * 1024 * 1024);
        assert_eq!(caps.max_walk_depth, 64);
        assert_eq!(caps.max_walk_files, 100_000);
    }

    #[test]
    fn walk_depth_capped() {
        // root/a.txt           depth 1  -> included
        // root/d1/b.txt        depth 2  -> included (cap = 2)
        // root/d1/d2/c.txt     depth 3  -> excluded
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let d1 = root.join("d1");
        let d2 = d1.join("d2");
        std::fs::create_dir_all(&d2).expect("nested dirs");
        std::fs::write(root.join("a.txt"), b"a").expect("a");
        std::fs::write(d1.join("b.txt"), b"b").expect("b");
        std::fs::write(d2.join("c.txt"), b"c").expect("c");

        let names = names_of(&walk_capped(root, 2, UNCAPPED_FILES));
        assert!(names.contains("a.txt"), "depth-1 file collected");
        assert!(
            names.contains("b.txt"),
            "depth-2 file collected (at the cap)"
        );
        assert!(
            !names.contains("c.txt"),
            "depth-3 file is past the cap and must be skipped: {names:?}"
        );
    }

    #[test]
    fn walk_depth_cap_floors_at_one() {
        // A zero cap is clamped to 1 — the root's own files are always listed.
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        std::fs::write(root.join("top.txt"), b"x").expect("top");
        std::fs::create_dir(root.join("sub")).expect("sub");
        std::fs::write(root.join("sub").join("deep.txt"), b"y").expect("deep");

        let names = names_of(&walk_capped(root, 0, UNCAPPED_FILES));
        assert!(names.contains("top.txt"), "root files listed even at cap 0");
        assert!(!names.contains("deep.txt"), "but nothing below depth 1");
    }

    #[test]
    fn walk_file_count_capped() {
        // Twelve files at the root; a cap of 5 returns exactly 5 (the cap bounds
        // the result allocation against a pathologically wide tree).
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        for i in 0..12 {
            std::fs::write(root.join(format!("f{i}.txt")), b"x").expect("file");
        }
        assert_eq!(
            walk_capped(root, 8, 5).len(),
            5,
            "walk stops at the file-count cap"
        );
    }

    #[test]
    fn walk_deep_tree_does_not_overflow_the_stack() {
        // A deep chain exercises the iterative walk's descent; the leaf sits
        // past the 256 depth cap below, proving descent terminates by the cap
        // (not by crashing or by following the chain to the leaf).
        //
        // Depth is bounded below `PATH_MAX` (~1024 on macOS, half of Linux's)
        // because the chain is built from absolute paths: a 512-deep `d/` chain
        // overflows macOS `PATH_MAX` during *setup*. 300 keeps the deepest path
        // well under the limit while still exceeding the 256 cap (the walk only
        // ever descends to the cap, so the chain need only be deeper than it).
        const DEPTH: usize = 300;
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut dir = tmp.path().to_path_buf();
        for _ in 0..DEPTH {
            dir = dir.join("d");
            std::fs::create_dir(&dir).expect("nested dir");
        }
        std::fs::write(dir.join("leaf.txt"), b"x").expect("leaf");
        let names = names_of(&walk_capped(tmp.path(), 256, UNCAPPED_FILES));
        assert!(!names.contains("leaf.txt"), "leaf is past the depth cap");
    }

    #[cfg(unix)]
    #[test]
    fn walk_does_not_follow_symlinked_dirs() {
        // A symlinked directory is never descended — chasing it would
        // reintroduce the cycle/escape DoS the read path forbids (Task 3).
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let real = root.join("real");
        std::fs::create_dir(&real).expect("real dir");
        std::fs::write(real.join("inside.txt"), b"z").expect("inside");
        std::os::unix::fs::symlink(&real, root.join("link")).expect("symlink");

        let found = walk_capped(root, 16, UNCAPPED_FILES);
        // `inside.txt` is reachable once (via `real/`), never twice (the
        // `link/` symlink is not followed).
        let inside_hits = found
            .iter()
            .filter(|p| p.file_name().is_some_and(|n| n == "inside.txt"))
            .count();
        assert_eq!(inside_hits, 1, "symlinked dir not followed: {found:?}");
    }

    // ---- DSV-045: gitignore-aware bounded walk ----

    #[test]
    fn walk_gitignored_filters_ignored_paths_before_the_cap() {
        // A repo whose bulk is gitignored (node_modules) must warm a *complete*
        // graph of its source — the gitignore filter applies BEFORE the cap.
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        std::fs::write(root.join(".gitignore"), b"node_modules/\ndist/\n").expect("gitignore");
        std::fs::write(root.join("a.ts"), b"export const a = 1;").expect("a");
        std::fs::write(root.join("b.ts"), b"export const b = 2;").expect("b");
        let nm = root.join("node_modules").join("dep");
        std::fs::create_dir_all(&nm).expect("node_modules");
        for i in 0..50 {
            std::fs::write(nm.join(format!("g{i}.js")), b"x").expect("gen");
        }
        std::fs::create_dir(root.join("dist")).expect("dist");
        std::fs::write(root.join("dist").join("bundle.js"), b"x").expect("bundle");

        let walk = walk_gitignored(root, 32, 1000);
        let names = names_of(&walk.files);
        assert!(
            names.contains("a.ts") && names.contains("b.ts"),
            "{names:?}"
        );
        assert!(
            !names.iter().any(|n| Path::new(n)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("js"))),
            "ignored node_modules/dist files must be filtered: {names:?}"
        );
        assert!(!walk.truncated(), "well under the cap after filtering");
        // The .gitignore file itself is a regular file and not ignored, so it is
        // counted — source filtering, not dotfile hiding.
        assert!(names.contains(".gitignore"));
    }

    #[test]
    fn walk_gitignored_reports_truncation_over_cap() {
        // 12 non-ignored files, cap 5 → collect 5, total 12, truncated.
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        for i in 0..12 {
            std::fs::write(root.join(format!("f{i}.ts")), b"x").expect("file");
        }
        let walk = walk_gitignored(root, 8, 5);
        assert_eq!(walk.files.len(), 5, "collection capped at max_files");
        assert_eq!(walk.total, 12, "total counts past the cap for the bound");
        assert!(walk.truncated());
    }

    #[test]
    fn walk_gitignored_respects_depth_and_skips_symlinks() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        std::fs::write(root.join("top.ts"), b"x").expect("top");
        let deep = root.join("d1").join("d2");
        std::fs::create_dir_all(&deep).expect("deep");
        std::fs::write(deep.join("deep.ts"), b"x").expect("deep file");

        let names = names_of(&walk_gitignored(root, 1, 1000).files);
        assert!(names.contains("top.ts"), "depth-1 file collected");
        assert!(!names.contains("deep.ts"), "depth-3 file past the cap");

        #[cfg(unix)]
        {
            let real = root.join("real");
            std::fs::create_dir(&real).expect("real");
            std::fs::write(real.join("inside.ts"), b"x").expect("inside");
            std::os::unix::fs::symlink(&real, root.join("link")).expect("symlink");
            let all = walk_gitignored(root, 16, 1000).files;
            let inside_hits = all
                .iter()
                .filter(|p| p.file_name().is_some_and(|n| n == "inside.ts"))
                .count();
            assert_eq!(inside_hits, 1, "symlinked dir not followed: {all:?}");
        }
    }

    #[test]
    fn pool_budget_holds_invariants_across_full_range() {
        // Exhaustive over the whole relevant domain, including 0 (reachable only
        // via a direct call — `from_host` floors at 1) and the 1→2→4 transition
        // zone the discrete spot-checks below would miss.
        for cores in 0usize..=128 {
            let b = PoolBudget::from_cores(cores);
            assert!(b.interactive >= 1, "interactive floor (cores={cores})");
            assert!(b.background >= 1, "background floor (cores={cores})");
            assert!(
                b.interactive <= PoolBudget::MAX_INTERACTIVE,
                "interactive stays small (cores={cores}, got {})",
                b.interactive
            );
            assert!(
                b.background >= b.interactive,
                "background >= interactive (cores={cores}: {b:?})"
            );
        }
    }

    #[test]
    fn pool_budget_interactive_grows_then_caps() {
        // Regression guards pinning the *arithmetic result* of the current
        // policy (half-cores host budget, interactive = half of that, capped at
        // 4). These are not the spec floor — the spec only fixes interactive at
        // ≈2–4 / "small"; if the policy is retuned within that envelope these
        // values change and that is fine. The invariant test above is the part
        // that must never break.
        assert_eq!(PoolBudget::from_cores(0).interactive, 1);
        assert_eq!(PoolBudget::from_cores(0).background, 1);
        assert_eq!(PoolBudget::from_cores(4).interactive, 1);
        assert_eq!(PoolBudget::from_cores(8).interactive, 2);
        assert_eq!(PoolBudget::from_cores(16).interactive, 4);
        // Above the cap, the interactive pool stays pinned while background grows.
        assert_eq!(PoolBudget::from_cores(64).interactive, 4);
        assert_eq!(PoolBudget::from_cores(64).background, 28);
    }

    /// A background pool saturated with blocked work must not prevent the
    /// interactive pool from running.
    ///
    /// Two barriers make this deterministic. `started` (size `background + 1`)
    /// is released only once *every* background thread has entered its task —
    /// the test thread waits on it before touching the interactive pool, so the
    /// background pool is provably fully occupied at the point of the interactive
    /// assertion. `finished` then releases the parked tasks. If the two pools
    /// shared threads, the interactive `install` would have no free thread and
    /// this test would deadlock (CI timeout) — so it genuinely pins the
    /// separation, not a scheduler-timing coincidence.
    #[test]
    fn interactive_pool_not_starved_by_background() {
        let budget = PoolBudget {
            interactive: 2,
            background: 2,
        };
        let sched = WorkScheduler::with_budget(budget).expect("build pools");

        let started = Arc::new(Barrier::new(budget.background + 1));
        let finished = Arc::new(Barrier::new(budget.background + 1));
        for _ in 0..budget.background {
            let started = Arc::clone(&started);
            let finished = Arc::clone(&finished);
            sched.background().spawn(move || {
                started.wait(); // signal: this background thread is running
                finished.wait(); // park until the test releases us
            });
        }

        // Block until every background thread is provably parked.
        started.wait();

        // With the background pool fully saturated, interactive work still runs.
        let answer = sched.interactive().install(|| 19 + 23);
        assert_eq!(
            answer, 42,
            "interactive pool ran despite saturated background"
        );

        // Release the parked background tasks so the pools drop cleanly.
        finished.wait();
    }

    #[test]
    fn per_workspace_token_bounds_inflight() {
        let adm = WorkspaceAdmission::new(2);
        let k = key("/tmp/ws-a");

        let g1 = adm.try_admit(&k).expect("first admit under cap");
        let g2 = adm.try_admit(&k).expect("second admit at cap");
        assert!(
            adm.try_admit(&k).is_none(),
            "third request over the per-workspace cap is refused"
        );

        // A different workspace has its own independent budget.
        let other = adm
            .try_admit(&key("/tmp/ws-b"))
            .expect("a second workspace is admitted independently");

        // Freeing a slot lets the next request in.
        drop(g1);
        let g3 = adm.try_admit(&k).expect("slot freed on guard drop");

        drop(g2);
        drop(g3);
        drop(other);
        // Fully drained: the workspace can be admitted again from empty.
        assert!(adm.try_admit(&k).is_some(), "cap resets once fully drained");
    }

    #[test]
    fn admission_cap_floors_at_one() {
        let adm = WorkspaceAdmission::new(0);
        assert_eq!(adm.max_inflight(), 1);
        let k = key("/tmp/ws-zero");
        let _g = adm
            .try_admit(&k)
            .expect("at least one unit always admitted");
        assert!(adm.try_admit(&k).is_none(), "but only one");
    }

    /// The core 10b guarantee: once cancellation is requested mid-chunk, the
    /// scan runs at most the rest of the current chunk and then yields at the
    /// next boundary — never the whole repo. Deterministic via a self-cancel:
    /// item 25 lives in chunk index 2 (items 20..30), so that chunk finishes
    /// (through 29) and the check before chunk 3 (items 30..) yields.
    #[test]
    fn background_scan_yields_within_one_chunk_on_cancel() {
        let items: Vec<usize> = (0..100).collect();
        let chunk = 10;
        let cancel = ScanCancel::new();
        let seen = Arc::new(Mutex::new(Vec::new()));

        let outcome = run_chunked_scan(&items, chunk, &cancel, |&i| {
            lock(&seen).push(i);
            if i == 25 {
                cancel.cancel();
            }
        });

        assert_eq!(outcome, ScanOutcome::Yielded { processed: 30 });
        let seen = lock(&seen);
        assert_eq!(seen.len(), 30, "only the cancelled chunk's worth ran on");
        assert_eq!(seen.last(), Some(&29), "stopped at the chunk boundary");
        // Cancel was requested at item 25; only items 26..=29 — the 4 remaining
        // items in chunk 2, fewer than one full chunk — ran after it, and no
        // item from chunk 3 onward started.
    }

    #[test]
    fn background_scan_runs_to_completion_without_cancel() {
        let items: Vec<usize> = (0..37).collect();
        let cancel = ScanCancel::new();
        let count = AtomicUsize::new(0);

        let outcome = run_chunked_scan(&items, 8, &cancel, |_| {
            count.fetch_add(1, Ordering::Relaxed);
        });

        assert_eq!(outcome, ScanOutcome::Completed);
        assert_eq!(count.load(Ordering::Relaxed), 37, "every item processed");
    }

    #[test]
    fn background_scan_yields_immediately_when_cancelled_before_start() {
        let items: Vec<usize> = (0..50).collect();
        let cancel = ScanCancel::new();
        cancel.cancel();
        let mut ran = 0usize;

        let outcome = run_chunked_scan(&items, 10, &cancel, |_| ran += 1);

        assert_eq!(outcome, ScanOutcome::Yielded { processed: 0 });
        assert_eq!(ran, 0, "work is never invoked once cancelled up front");
    }

    #[test]
    fn background_scan_cancelled_in_final_chunk_completes_not_yields() {
        // A cancel observed during the last chunk has no later boundary to act
        // on: the scan finishes that chunk and returns Completed, not Yielded.
        // This pins the doc contract that a mid-chunk cancel near the end does
        // not guarantee Yielded.
        let items: Vec<usize> = (0..20).collect();
        let chunk = 10;
        let cancel = ScanCancel::new();
        let count = AtomicUsize::new(0);

        let outcome = run_chunked_scan(&items, chunk, &cancel, |&i| {
            count.fetch_add(1, Ordering::Relaxed);
            // Fire during the second (final) chunk, items 10..20.
            if i == 15 {
                cancel.cancel();
            }
        });

        assert_eq!(outcome, ScanOutcome::Completed);
        assert_eq!(
            count.load(Ordering::Relaxed),
            20,
            "the final chunk still runs to its end"
        );
    }

    #[test]
    fn background_scan_over_empty_items_completes_even_if_cancelled() {
        // An empty corpus has no chunks, so the cancel check never runs: the
        // documented result is Completed (nothing to scan), not Yielded, even
        // with the flag pre-set.
        let items: [usize; 0] = [];
        let cancel = ScanCancel::new();
        cancel.cancel();
        let mut ran = 0usize;

        let outcome = run_chunked_scan(&items, 10, &cancel, |_| ran += 1);

        assert_eq!(outcome, ScanOutcome::Completed);
        assert_eq!(ran, 0);
    }

    #[test]
    fn background_scan_chunk_size_floors_at_one() {
        // A zero chunk size would never make progress; it is clamped to 1 so the
        // scan still runs (and still yields between every item).
        let items: Vec<usize> = (0..5).collect();
        let cancel = ScanCancel::new();
        let count = AtomicUsize::new(0);

        let outcome = run_chunked_scan(&items, 0, &cancel, |_| {
            count.fetch_add(1, Ordering::Relaxed);
        });

        assert_eq!(outcome, ScanOutcome::Completed);
        assert_eq!(count.load(Ordering::Relaxed), 5);
    }

    /// The real cross-thread scenario: a scan running on the background pool is
    /// asked to yield by another thread (standing in for the IPC listener
    /// admitting interactive work). It must observe the cancel via the shared
    /// flag and stop at a chunk boundary, well before completing the corpus.
    ///
    /// Made deterministic — independent of thread scheduling, so it cannot flake
    /// or race to `Completed` on a single-core / loaded host — by pinning the
    /// scan inside the first item until the cancel is provably visible: the
    /// `underway` barrier proves the scan reached item 0, then the work closure
    /// busy-waits (cooperatively, via `yield_now`) until `is_cancelled()` reads
    /// true. The scan therefore cannot advance past chunk 0 until the canceller
    /// has run, so the yield lands at exactly one chunk every time.
    #[test]
    fn background_scan_on_pool_yields_to_concurrent_canceller() {
        let sched = WorkScheduler::with_budget(PoolBudget {
            interactive: 1,
            background: 1,
        })
        .expect("build pools");

        let items: Vec<usize> = (0..100_000).collect();
        let chunk = 256;
        let cancel = ScanCancel::new();
        let started = AtomicUsize::new(0);
        let underway = Barrier::new(2);

        let outcome = std::thread::scope(|s| {
            let canceller = s.spawn(|| {
                underway.wait(); // the scan is provably at item 0
                cancel.cancel();
            });

            let out = sched.background().install(|| {
                run_chunked_scan(&items, chunk, &cancel, |_| {
                    // On the first item only: release the canceller, then hold
                    // here until its cancel is observable across the threads.
                    // yield_now keeps a single-core host from starving it.
                    if started.fetch_add(1, Ordering::Relaxed) == 0 {
                        underway.wait();
                        while !cancel.is_cancelled() {
                            std::thread::yield_now();
                        }
                    }
                })
            });

            canceller.join().expect("canceller thread");
            out
        });

        match outcome {
            ScanOutcome::Yielded { processed } => {
                // The flag was set during item 0, so chunk 0 finishes and the
                // check before chunk 1 yields — a chunk-aligned stop, far short
                // of the whole corpus. Asserting on the outcome's own counter
                // (not a per-item proxy) verifies the real invariant.
                assert_eq!(processed, chunk, "yields exactly one chunk after cancel");
                assert!(processed < items.len(), "stopped before the whole corpus");
            }
            ScanOutcome::Completed => panic!("a concurrently cancelled scan must yield"),
        }
    }
}
