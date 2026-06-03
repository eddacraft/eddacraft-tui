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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Barrier;
    use std::sync::atomic::AtomicUsize;

    fn key(p: &str) -> WorktreeKey {
        WorktreeKey::from_canonical(PathBuf::from(p))
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
