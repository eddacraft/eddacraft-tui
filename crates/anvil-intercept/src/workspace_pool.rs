//! DSV-006a / Sub-phase A Task 10a — the daemon's two cooperating work pools
//! and per-workspace in-flight admission token (ADR-061 §4 resource model).
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
//! interactive load) is the chunked cancel/yield loop — that is Task 10b and
//! lives elsewhere. This module owns only the **construction** of the pools
//! (the hard predecessor of Task 8, which runs the antipattern check on the
//! interactive pool) plus the admission token.
//!
//! # Admission token
//!
//! [`WorkspaceAdmission`] bounds how many `validate_paths` requests for a single
//! [`WorktreeKey`] may be in flight at once. It layers over the IPC listener's
//! per-connection semaphore (`ipc.rs`): the semaphore caps total connections,
//! the admission token stops one busy workspace from monopolising the shared
//! interactive pool.

use std::collections::HashMap;
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
}
