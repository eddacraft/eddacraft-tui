//! GBASE-009 (ADR-105 §7/§8): the daemon-side **re-entrant persistence route**.
//!
//! Every worktree the daemon warms is routed to one of two persistence paths:
//!
//! - [`PersistenceRoute::Base`] — the **shared write-once base** for the
//!   worktree's merge-base commit is composed with a live overlay (GBASE-006's
//!   [`crate::graph_base_warm_start::compose_worktree_from_base`]). This is the
//!   ADR-105 layout win: O(distinct merge-bases) on disk, siblings reuse
//!   parse/resolve.
//! - [`PersistenceRoute::PerWorktree`] — the **inherited per-worktree snapshot**
//!   path (ADR-069, [`crate::save_time`]). Kept **permanently** for the
//!   topologies a shared base cannot cover (ADR-105 §8): a detached HEAD, a repo
//!   with no resolvable default branch, or a HEAD that shares no merge-base with
//!   its default branch.
//!
//! # Which merge-base key is primary (producer parity)
//!
//! When the topology is covered the route carries **the one sha the graph-base
//! producer builds the base under**: the merge-base of `HEAD` against the default
//! branch, **refined by `@{upstream}` when the branch tracks one** (ADR-105 §6).
//! [`GitRouteResolver`] resolves it with the **exact** precedence
//! `anvil_cli::graph_base_producer::resolve_base_commit` uses, so the sha the
//! route composes *from* equals the sha the producer produced *under* — the
//! composed graph reads the base that actually exists on disk, never a
//! neighbouring key.
//!
//! **Default-branch parity is load-bearing.** The producer resolves the default
//! branch as `origin/HEAD` **only** (else an explicit `--default-branch`, which
//! the daemon's trigger never passes) — so a repo whose `origin/HEAD` is unset
//! has **no producible base**. The route resolver therefore probes `origin/HEAD`
//! **only** ([`resolve_default_branch_origin_head`]) and routes such a repo
//! **per-worktree** (`Uncovered(NoDefaultBranch)`), preserving today's behaviour
//! honestly. It deliberately does **not** reuse
//! [`base_gc::resolve_default_branch`](crate::snapshot_io::base_gc), whose
//! `origin/main`/`origin/master` fallback is a conservative **superset for the GC
//! keep-set** (over-retention is safe there). Using that fallback for *routing*
//! would classify an `origin/HEAD`-unset-but-`origin/main`-present repo (mirrors,
//! CI checkouts, hand-added remotes) **covered** and route `Base{sha}` for a sha
//! the producer can never produce — losing warm-start entirely (base absent ⇒
//! cold every contact) **and** never using the per-worktree path. The two
//! resolvers intentionally diverge on exactly this fallback.
//!
//! This is also the routing counterpart of GBASE-008's keep-set resolver on the
//! `@{upstream}` axis, which keeps a *superset* of both plausible keys; a route
//! needs the *single* producing key, so it lives here rather than reusing
//! [`crate::snapshot_io::base_gc::MergeBase`].
//!
//! # Re-entrancy
//!
//! [`PersistenceRouter::route`] is **re-entrant on the same ref-change trigger**
//! (ADR-105 §8). It remembers each worktree's last route and, on every
//! re-evaluation, classifies the decision as a first evaluation, a steady state,
//! or a **transition**:
//!
//! - **merge-base movement** (covered → covered, sha changed) ⇒
//!   [`RouteReason::MergeBaseMoved`] — the next composition uses the new sha;
//! - **covered ↔ uncovered** flips ⇒ [`RouteReason::BecameCovered`] /
//!   [`RouteReason::BecameUncovered`].
//!
//! It is driven by the **same debounced ref events** the GBASE-003 trigger
//! already watches (a rebase moves the merge-base; a `git checkout --detach`
//! flips coverage) via the post-admission warm-start contacts and the daemon's
//! low-cadence re-route pass — it adds **no new watches** of its own.
//!
//! # Uncovered permanence (ADR-105 §8 reading)
//!
//! An uncovered topology routes per-worktree and **no base composition is
//! attempted while it stays uncovered** — that is the "permanently" of ADR-105
//! §8: no retry churn within a *stable* topology. It is **not** an irreversible
//! latch: the re-entrant loop MAY flip a worktree back to [`PersistenceRoute::Base`]
//! if its topology later becomes covered (a detached HEAD is re-attached, a
//! default branch appears). This matches §8's "re-evaluates on
//! covered↔uncovered transitions".
//!
//! # Failure posture — never wedge
//!
//! A **transient** resolver failure ([`RouteMergeBase::Unavailable`]: `git` could
//! not be spawned, or the repo was mid-operation) routes per-worktree **this
//! pass** with its own [`RouteReason::ResolverUnavailable`] and is **re-evaluated
//! later** — it never latches a worktree off the base path (ADR-105 §6 non-fatal
//! posture). Only the *deterministic* uncovered topologies are stable.
//!
//! # Structured observability
//!
//! Every decision emits the structured event `persistence.route{route, reason}`
//! (tracing target [`EVENT_TARGET`]). The returned [`RouteDecision`] carries the
//! **same** `route` + `reason` fields the event emits, so a caller (and a test)
//! reads the decision directly rather than scraping logs.
//!
//! # Admission contract (security — the GBASE-006 compose seam)
//!
//! The `sha` a [`PersistenceRoute::Base`] carries is a **trust boundary**
//! (GBASE-006 council note). This module derives it **by construction** from the
//! worktree's **own git state** ([`GitRouteResolver`] shells `git` against the
//! worktree root), never from a wire/IPC value. The wired caller
//! ([`crate::save_time::SaveTimeState::spawn_route_restore`]) only ever routes a
//! worktree **after it is admitted** (`AdmittedRoots`), so a client can never key
//! a base on an attacker-chosen sha and materialise an attacker-chosen graph.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use crate::rule_cache::WorktreeKey;
use crate::snapshot_io::base_gc::{
    GitRun, MergeBaseCall, RefCall, classify_merge_base, classify_ref, run_git,
};

/// The tracing target for the structured `persistence.route{route, reason}` event.
pub const EVENT_TARGET: &str = "anvil_intercept::persistence_route";

/// Where a worktree's warm-start persistence is routed (ADR-105 §8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistenceRoute {
    /// Compose the worktree's resident graph from the **shared base** for
    /// `merge_base_sha` (the producer's key) plus a live overlay (GBASE-006).
    Base {
        /// The merge-base commit sha, resolved with producer parity (default
        /// branch, `@{upstream}`-refined when set). Daemon-derived — never a wire
        /// value.
        merge_base_sha: String,
    },
    /// Warm the worktree from its **private per-worktree snapshot** (ADR-069) —
    /// the permanent path for uncovered topologies (and the transient-failure
    /// fallback).
    PerWorktree {
        /// The admitted canonical worktree root.
        canonical_root: PathBuf,
    },
}

impl PersistenceRoute {
    /// The stable `route` label the structured event carries (`"base"` /
    /// `"per_worktree"`).
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Base { .. } => "base",
            Self::PerWorktree { .. } => "per_worktree",
        }
    }
}

/// Why a route decision was reached — the structured `reason` field of the
/// `persistence.route{route, reason}` event. First-evaluation / steady-state
/// reasons name the **topology**; the transition reasons name the **movement**
/// the re-entrant loop observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteReason {
    // ---- topology (first evaluation / unchanged steady state) --------------
    /// Covered: a merge-base resolved to the producer's primary key.
    BaseResolved,
    /// Uncovered: `HEAD` is detached (not a branch) — no stable base identity.
    UncoveredDetachedHead,
    /// Uncovered: no default branch is resolvable (`origin/HEAD` unset and no
    /// `origin/main`/`origin/master`), including a non-git root.
    UncoveredNoDefaultBranch,
    /// Uncovered: `HEAD` shares no merge-base with the default branch (nor with an
    /// `@{upstream}`, when tracked).
    UncoveredNoMergeBase,
    /// Transient: the merge-base could not be resolved this pass (a `git` spawn
    /// failure or a repo mid-operation). Routes per-worktree **this pass**, to be
    /// re-evaluated later — never a latch.
    ResolverUnavailable,
    // ---- transitions (re-entrancy) -----------------------------------------
    /// Covered → covered, but the merge-base **sha moved** (a rebase). The next
    /// composition uses the new sha.
    MergeBaseMoved,
    /// Uncovered/unavailable → covered: the topology became coverable (a detached
    /// HEAD re-attached, a default branch appeared).
    BecameCovered,
    /// Covered → uncovered: the topology stopped being coverable.
    BecameUncovered,
}

impl RouteReason {
    /// The stable string the structured event emits.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BaseResolved => "base_resolved",
            Self::UncoveredDetachedHead => "uncovered_detached_head",
            Self::UncoveredNoDefaultBranch => "uncovered_no_default_branch",
            Self::UncoveredNoMergeBase => "uncovered_no_merge_base",
            Self::ResolverUnavailable => "resolver_unavailable",
            Self::MergeBaseMoved => "merge_base_moved",
            Self::BecameCovered => "became_covered",
            Self::BecameUncovered => "became_uncovered",
        }
    }
}

/// A single route decision: the chosen [`PersistenceRoute`] and the
/// [`RouteReason`] behind it — the exact `route` + `reason` pair the structured
/// event emits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteDecision {
    /// The chosen persistence path.
    pub route: PersistenceRoute,
    /// Why this path was chosen (topology or transition).
    pub reason: RouteReason,
}

/// Why a worktree is not coverable by a shared base — the deterministic uncovered
/// topologies (ADR-105 §8). Distinct kinds give the route distinct reasons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UncoveredKind {
    /// `HEAD` is detached (not on a branch).
    DetachedHead,
    /// No default branch resolves (also a non-git root).
    NoDefaultBranch,
    /// `HEAD` shares no merge-base with the default branch (nor upstream).
    NoMergeBase,
}

impl UncoveredKind {
    /// The stable string the structured event's `uncovered_kind` field carries.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DetachedHead => "detached_head",
            Self::NoDefaultBranch => "no_default_branch",
            Self::NoMergeBase => "no_merge_base",
        }
    }
}

/// The merge-base resolution for one worktree, as a **route** sees it: the single
/// producing key, a deterministic uncovered topology, or a transient failure.
///
/// This differs from GBASE-008's [`crate::snapshot_io::base_gc::MergeBase`] on
/// purpose: GC keeps a **superset** of every plausible key (over-retention is
/// safe), whereas a route needs the **one** key the producer built under and the
/// **specific** uncovered kind (for distinct reasons).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteMergeBase {
    /// The producer's primary merge-base key (default-branch base, `@{upstream}`
    /// refined when tracked).
    Resolved(String),
    /// A deterministic uncovered topology (ADR-105 §8).
    Uncovered(UncoveredKind),
    /// A transient failure this pass (fail-safe, re-evaluated later).
    Unavailable,
}

/// Resolves a worktree's primary merge-base key for routing. Injected so the
/// router's tests run hermetically (no `git`, no real process state) — mirrors
/// the [`KeepSetResolver`](crate::snapshot_io::base_gc::KeepSetResolver)
/// discipline in GBASE-008.
pub trait RouteResolver: Send + Sync {
    /// Resolve `worktree`'s primary merge-base key (producer parity), or classify
    /// it uncovered / unavailable.
    fn resolve(&self, worktree: &Path) -> RouteMergeBase;
}

/// Production [`RouteResolver`]: a bounded `git` probe per worktree that mirrors
/// `anvil_cli::graph_base_producer::resolve_base_commit`'s precedence and reuses
/// GBASE-008's shared, fail-safe `git`-run plumbing
/// ([`run_git`]/[`classify_merge_base`]/[`resolve_default_branch`]) — the single
/// classification table for the whole daemon.
///
/// ADR-105 §7 keeps the resident daemon **git-free on the hot path**: this
/// resolver only ever runs on the **background** pool (via
/// [`crate::save_time::SaveTimeState::spawn_route_restore`]), never a connection
/// thread, so the sha a route composes from is daemon-derived *and* off the hot
/// path by construction.
///
/// # Probe order (producer parity + uncovered classification)
///
/// | probe | outcome | → |
/// | ----- | ------- | - |
/// | `symbolic-ref -q HEAD` exit 1 | HEAD detached | `Uncovered(DetachedHead)` |
/// | `symbolic-ref -q HEAD` spawn-failed | git unrunnable | `Unavailable` |
/// | `rev-parse origin/HEAD` unset (**only** probe — no origin/main fallback) | no producible base | `Uncovered(NoDefaultBranch)` |
/// | `rev-parse origin/HEAD` unexpected fatal | unexpected git failure | `Unavailable` |
/// | `merge-base HEAD origin/HEAD` found | + `@{upstream}` refine when tracked | `Resolved(primary)` |
/// | `merge-base` exit >1 / signal | unexpected | `Unavailable` |
/// | no base + no upstream base | share no history | `Uncovered(NoMergeBase)` |
pub struct GitRouteResolver {
    git_bin: std::ffi::OsString,
}

impl GitRouteResolver {
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

    /// Resolve the `@{upstream}`-refined merge-base when the branch tracks an
    /// upstream, mirroring the producer. `Ok(Some(sha))` refines the primary key;
    /// `Ok(None)` keeps the default-branch key; `Err(())` is an unexpected failure
    /// (abort → `Unavailable`).
    fn upstream_refinement(&self, worktree: &Path) -> Result<Option<String>, ()> {
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
            // Spawn/signal failure is unexpected.
            GitRun::Failed => return Err(()),
            GitRun::Exited {
                code: 0,
                stdout: Some(up),
                ..
            } => up,
            // Any clean non-zero ⇒ the branch tracks no upstream (the producer
            // falls back identically) — a deterministic skip, not uncertainty.
            GitRun::Exited { .. } => return Ok(None),
        };
        match classify_merge_base(run_git(
            &self.git_bin,
            worktree,
            &["merge-base", "--end-of-options", "HEAD", &upstream],
        )) {
            MergeBaseCall::Found(refined) => Ok(Some(refined)),
            MergeBaseCall::NoBase => Ok(None),
            MergeBaseCall::Unavailable => Err(()),
        }
    }

    /// Resolve the default branch with **exact producer parity**: `origin/HEAD`
    /// **only** (`anvil_cli::graph_base_producer::resolve_base_commit` →
    /// `resolve_default_branch`, whose only other source is an explicit
    /// `--default-branch` the daemon's trigger never supplies). A repo with
    /// `origin/HEAD` unset thus has **no producible base**, so routing it
    /// per-worktree is honest.
    ///
    /// This deliberately does **not** reuse
    /// [`base_gc::resolve_default_branch`](crate::snapshot_io::base_gc), whose
    /// `origin/main`/`origin/master` fallback is a conservative superset for the GC
    /// keep-set (over-retention is safe there); the same fallback in a route would
    /// send an `origin/HEAD`-unset repo to `Base{sha}` for a sha the producer can
    /// never produce (see the module-level default-branch-parity note).
    ///
    /// - `Ok(Some(ref))` — `origin/HEAD` resolved to a branch.
    /// - `Ok(None)` — `origin/HEAD` is deterministically unset ⇒
    ///   `Uncovered(NoDefaultBranch)`.
    /// - `Err(())` — an unexpected git failure ⇒ `Unavailable`.
    fn resolve_default_branch_origin_head(&self, worktree: &Path) -> Result<Option<String>, ()> {
        match classify_ref(run_git(
            &self.git_bin,
            worktree,
            &[
                "rev-parse",
                "--abbrev-ref",
                "--end-of-options",
                "origin/HEAD",
            ],
        )) {
            // A resolved symref prints the branch (e.g. `origin/main`); when
            // `origin/HEAD` is unset git may echo the literal `origin/HEAD` at
            // exit 0 — treat that as unset, matching the producer's `!empty` guard
            // (the producer only trusts a name that differs from the query).
            RefCall::Resolved(head) if head != "origin/HEAD" => Ok(Some(head)),
            // Deterministic unset (echoed literal, or a `unknown revision`/non-git
            // fatal) ⇒ no producible base ⇒ per-worktree.
            RefCall::Resolved(_) | RefCall::Missing => Ok(None),
            // An unexpected git failure must never wedge a route — re-evaluate later.
            RefCall::Unavailable => Err(()),
        }
    }
}

impl Default for GitRouteResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl RouteResolver for GitRouteResolver {
    fn resolve(&self, worktree: &Path) -> RouteMergeBase {
        // 1. Detached HEAD is an ADR-105 §8 uncovered topology regardless of
        // whether a merge-base *could* be computed: a detached HEAD tracks no
        // branch, so it has no stable base identity. `symbolic-ref -q HEAD` exits
        // 1 (quiet) exactly when HEAD is detached; a non-git root fatals (128,
        // "not a git repository") and falls through to the default-branch probe,
        // which classifies it `NoDefaultBranch`.
        match run_git(&self.git_bin, worktree, &["symbolic-ref", "-q", "HEAD"]) {
            GitRun::Failed => return RouteMergeBase::Unavailable,
            GitRun::Exited { code: 1, .. } => {
                return RouteMergeBase::Uncovered(UncoveredKind::DetachedHead);
            }
            // Exit 0 (attached) or any other exit (non-git root / unexpected) ⇒
            // let the default-branch probe make the deterministic-vs-unexpected call.
            GitRun::Exited { .. } => {}
        }

        // 2. Default branch — `origin/HEAD` ONLY (exact producer parity; NOT
        // base_gc's origin/main|master keep-set superset). See
        // `resolve_default_branch_origin_head`.
        let default = match self.resolve_default_branch_origin_head(worktree) {
            Ok(Some(branch)) => branch,
            Ok(None) => return RouteMergeBase::Uncovered(UncoveredKind::NoDefaultBranch),
            Err(()) => return RouteMergeBase::Unavailable,
        };

        // 3. `merge-base HEAD <default>` — the default-branch key. **Producer parity
        // short-circuit:** the producer computes this FIRST and bails
        // `NoBasePossible` when it fails, BEFORE its `@{upstream}` block — so a route
        // must NOT reach for an upstream refinement when the default-branch base does
        // not exist (that would key `Base{refined}` on a sha the producer never
        // builds). NoBase ⇒ `Uncovered(NoMergeBase)` immediately.
        let base_key = match classify_merge_base(run_git(
            &self.git_bin,
            worktree,
            &["merge-base", "--end-of-options", "HEAD", &default],
        )) {
            MergeBaseCall::Found(sha) => sha,
            MergeBaseCall::NoBase => {
                return RouteMergeBase::Uncovered(UncoveredKind::NoMergeBase);
            }
            MergeBaseCall::Unavailable => return RouteMergeBase::Unavailable,
        };

        // 4. `@{upstream}` refinement WINS when tracked (the producer returns the
        // refined key), so the route composes from the same sha the producer built.
        // Only reached because the default-branch base resolved (step 3).
        let Ok(refined) = self.upstream_refinement(worktree) else {
            return RouteMergeBase::Unavailable;
        };

        // 5. Primary = the upstream refinement when tracked, else the default-branch
        // key (which is guaranteed present here).
        RouteMergeBase::Resolved(refined.unwrap_or(base_key))
    }
}

/// After this many **consecutive same-route** evaluations a worktree is
/// considered stable and its tick-driven re-evaluation is decimated (ADR-105 §8
/// re-entrancy, bounded). Chosen small so a genuine flip is still caught within a
/// few passes.
const STABILITY_THRESHOLD: u32 = 4;

/// Once decimated, a worktree is re-evaluated on the tick only every Nth pass.
/// With the daemon's 30 s re-route tick that is ~5 min of steady-state cadence —
/// ample for a rebase (whose warm effect lands at the next composition anyway),
/// negligible git cost across a fleet.
const DECIMATION_TICKS: u32 = 10;

/// After this many **consecutive `Unavailable`** resolutions a worktree's
/// tick-driven re-evaluation is decimated the same way (failure backoff), so a
/// persistently-unresolvable repo (git mid-operation, permission loss) does not
/// burn a `git` shell-out every tick. Reset on any successful resolution.
const FAILURE_THRESHOLD: u32 = 4;

/// Per-worktree re-entrant router state (ADR-105 §8): the last route (transition
/// detection) plus the bounded-evaluation backoff counters.
struct RouteState {
    /// The last computed route — the transition-detection anchor.
    last: PersistenceRoute,
    /// Consecutive evaluations that produced the **same** route (stability).
    stable_count: u32,
    /// Consecutive `Unavailable` resolutions (failure backoff).
    unavailable_streak: u32,
    /// Whether we have already logged the "entering failure backoff" warning, so a
    /// persistently-failing worktree warns **once**, not every pass.
    failure_logged: bool,
    /// Remaining tick passes to **skip** before the next tick-driven evaluation
    /// (decimation countdown). `0` ⇒ evaluate on the next tick. Reset to `0` on any
    /// route change (re-arm) so a flip is picked up promptly.
    skip_ticks: u32,
}

/// The re-entrant persistence router (ADR-105 §8). Holds the injected
/// [`RouteResolver`] and each worktree's [`RouteState`] (last route + bounded
/// re-evaluation backoff) so a re-evaluation can classify transitions (merge-base
/// movement, covered↔uncovered flips) and the daemon's re-route tick stays cheap
/// on a stable fleet.
///
/// Routing is **deterministic**: the same resolver answer always yields the same
/// route, and the transition classification is a pure function of
/// `(previous route, new route)`. The determinism seam is the injected resolver
/// (a fake in tests) — mirroring GBASE-008's `KeepSetResolver`. The backoff is
/// **tick-count based** (no wall-clock): [`Self::route`] (contacts) always
/// evaluates and re-arms; only [`Self::route_on_tick`] honours the decimation, so
/// an active worktree is never starved (every cold contact re-routes) while an
/// idle fleet is bounded.
pub struct PersistenceRouter {
    resolver: Arc<dyn RouteResolver>,
    /// Each worktree's re-entrant state. Behind a `Mutex` so the daemon's
    /// concurrent warm-start contacts and the low-cadence re-route pass share one
    /// memory.
    states: Mutex<HashMap<WorktreeKey, RouteState>>,
}

impl PersistenceRouter {
    /// Build the router with the production [`GitRouteResolver`].
    #[must_use]
    pub fn new() -> Self {
        Self::with_resolver(Arc::new(GitRouteResolver::new()))
    }

    /// Build the router with an injected resolver (the daemon uses the production
    /// git resolver; tests inject a hermetic fake).
    #[must_use]
    pub fn with_resolver(resolver: Arc<dyn RouteResolver>) -> Self {
        Self {
            resolver,
            states: Mutex::new(HashMap::new()),
        }
    }

    /// Route `key` (rooted at `worktree`) between the base and per-worktree paths,
    /// re-entrantly: re-evaluate the topology, classify any transition against the
    /// last route, emit the structured `persistence.route{route, reason}` event,
    /// update the backoff counters, and remember the new route. Returns the
    /// [`RouteDecision`].
    ///
    /// This **always** evaluates (a `git` resolution) — it is the contact path
    /// (post-admission warm-start), where an active worktree must re-route
    /// promptly. The tick path ([`Self::route_on_tick`]) honours the backoff.
    #[must_use]
    pub fn route(&self, key: &WorktreeKey, worktree: &Path) -> RouteDecision {
        let resolved = self.resolver.resolve(worktree);
        let unavailable = matches!(resolved, RouteMergeBase::Unavailable);
        let uncovered_kind = match &resolved {
            RouteMergeBase::Uncovered(kind) => Some(*kind),
            _ => None,
        };
        let (route, topology_reason) = classify_route(resolved, worktree);

        // Classify the decision against the last route and commit the new state
        // under one lock, so a concurrent re-evaluation sees a consistent
        // (previous, new) pair. Also carry the previous Base sha (for a
        // `merge_base_moved` event) out of the lock.
        let (reason, previous_sha) = {
            let mut states = self.states.lock().unwrap_or_else(PoisonError::into_inner);
            let previous = states.get(key).map(|s| &s.last);
            let reason = transition_reason(previous, &route, topology_reason);
            let previous_sha = match (previous, &route) {
                (
                    Some(PersistenceRoute::Base {
                        merge_base_sha: old,
                    }),
                    PersistenceRoute::Base {
                        merge_base_sha: new,
                    },
                ) if old != new => Some(old.clone()),
                _ => None,
            };
            let changed = previous != Some(&route);
            update_state(&mut states, key, route.clone(), changed, unavailable);
            (reason, previous_sha)
        };

        emit_event(
            worktree,
            &route,
            reason,
            uncovered_kind,
            previous_sha.as_deref(),
        );
        RouteDecision { route, reason }
    }

    /// The **tick-driven** re-evaluation (ADR-105 §8 re-entrancy): re-route `key`
    /// like [`Self::route`], but honour the per-worktree decimation backoff — a
    /// stable or persistently-failing worktree is skipped on most ticks (no `git`
    /// shell-out, no event), so the daemon's re-route pass stays cheap on a large
    /// idle fleet. Returns `None` when the worktree was skipped this tick.
    ///
    /// The backoff is re-armed (skip counter cleared) whenever [`Self::route`]
    /// observes a change, so an actively-contacted worktree is never starved.
    #[must_use]
    pub fn route_on_tick(&self, key: &WorktreeKey, worktree: &Path) -> Option<RouteDecision> {
        {
            let mut states = self.states.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(state) = states.get_mut(key)
                && state.skip_ticks > 0
            {
                state.skip_ticks -= 1;
                return None;
            }
        }
        Some(self.route(key, worktree))
    }

    /// Drop `key`'s re-entrant state (ADR-105 §8). Wired into the ACTMO
    /// unregister hook so an unregistered worktree does not leak router state for
    /// the daemon's lifetime (the map would otherwise grow unbounded with churn).
    pub fn forget(&self, key: &WorktreeKey) {
        self.states
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(key);
    }

    /// Test-only: whether the state map is empty (leak assertions).
    #[cfg(test)]
    fn forget_is_empty(&self) -> bool {
        self.states
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_empty()
    }

    /// Test-only: whether the state map holds exactly `key` (no leak / no tear).
    #[cfg(test)]
    fn forget_is_single_key(&self, key: &WorktreeKey) -> bool {
        let states = self.states.lock().unwrap_or_else(PoisonError::into_inner);
        states.len() == 1 && states.contains_key(key)
    }
}

impl Default for PersistenceRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Map a resolver outcome to a route + its topology reason.
fn classify_route(resolved: RouteMergeBase, worktree: &Path) -> (PersistenceRoute, RouteReason) {
    match resolved {
        RouteMergeBase::Resolved(sha) => (
            PersistenceRoute::Base {
                merge_base_sha: sha,
            },
            RouteReason::BaseResolved,
        ),
        RouteMergeBase::Uncovered(kind) => (
            PersistenceRoute::PerWorktree {
                canonical_root: worktree.to_path_buf(),
            },
            match kind {
                UncoveredKind::DetachedHead => RouteReason::UncoveredDetachedHead,
                UncoveredKind::NoDefaultBranch => RouteReason::UncoveredNoDefaultBranch,
                UncoveredKind::NoMergeBase => RouteReason::UncoveredNoMergeBase,
            },
        ),
        RouteMergeBase::Unavailable => (
            PersistenceRoute::PerWorktree {
                canonical_root: worktree.to_path_buf(),
            },
            RouteReason::ResolverUnavailable,
        ),
    }
}

/// Update `key`'s [`RouteState`] after an evaluation: refresh the last route, the
/// stability / failure streaks, and the tick-decimation skip counter (ADR-105
/// §8, bounded re-evaluation). A change re-arms (skip cleared); sustained
/// stability or sustained failure decimates.
fn update_state(
    states: &mut HashMap<WorktreeKey, RouteState>,
    key: &WorktreeKey,
    route: PersistenceRoute,
    changed: bool,
    unavailable: bool,
) {
    let state = states.entry(key.clone()).or_insert_with(|| RouteState {
        last: route.clone(),
        stable_count: 0,
        unavailable_streak: 0,
        failure_logged: false,
        skip_ticks: 0,
    });
    state.last = route;

    // Stability streak.
    if changed {
        state.stable_count = 1;
        state.skip_ticks = 0; // re-arm: a flip is picked up on the next tick
    } else {
        state.stable_count = state.stable_count.saturating_add(1);
    }

    // Failure streak (independent of route value: `Unavailable` always routes
    // per-worktree, which may be "unchanged").
    if unavailable {
        state.unavailable_streak = state.unavailable_streak.saturating_add(1);
        if state.unavailable_streak >= FAILURE_THRESHOLD && !state.failure_logged {
            tracing::warn!(
                target: EVENT_TARGET,
                worktree = %key.as_path().display(),
                streak = state.unavailable_streak,
                "persistence route resolver Unavailable for a sustained streak; \
                 decimating re-evaluation (per-worktree path serves meanwhile)",
            );
            state.failure_logged = true;
        }
    } else {
        if state.failure_logged {
            tracing::info!(
                target: EVENT_TARGET,
                worktree = %key.as_path().display(),
                "persistence route resolver recovered from the Unavailable streak",
            );
        }
        state.unavailable_streak = 0;
        state.failure_logged = false;
    }

    // Decimate once stable OR persistently failing.
    let decimate =
        state.stable_count >= STABILITY_THRESHOLD || state.unavailable_streak >= FAILURE_THRESHOLD;
    if decimate && state.skip_ticks == 0 {
        state.skip_ticks = DECIMATION_TICKS - 1;
    }
}

/// Classify a route decision against the previous route (re-entrancy, ADR-105 §8):
/// a first evaluation and an unchanged steady state carry the **topology** reason;
/// a change carries the **transition** reason.
fn transition_reason(
    previous: Option<&PersistenceRoute>,
    new: &PersistenceRoute,
    topology_reason: RouteReason,
) -> RouteReason {
    match (previous, new) {
        // First time we see this worktree ⇒ the topology reason.
        (None, _) => topology_reason,
        (
            Some(PersistenceRoute::Base {
                merge_base_sha: old,
            }),
            PersistenceRoute::Base {
                merge_base_sha: new,
            },
        ) => {
            if old == new {
                topology_reason // unchanged covered
            } else {
                RouteReason::MergeBaseMoved
            }
        }
        (Some(PersistenceRoute::Base { .. }), PersistenceRoute::PerWorktree { .. }) => {
            RouteReason::BecameUncovered
        }
        (Some(PersistenceRoute::PerWorktree { .. }), PersistenceRoute::Base { .. }) => {
            RouteReason::BecameCovered
        }
        // Still uncovered (possibly a different uncovered kind, or a transient
        // failure): the current topology reason describes it.
        (Some(PersistenceRoute::PerWorktree { .. }), PersistenceRoute::PerWorktree { .. }) => {
            topology_reason
        }
    }
}

/// Emit the structured `persistence.route{route, reason}` event (ADR-105 §8). A
/// `Base` decision carries `merge_base_sha` (and `previous_merge_base_sha` on a
/// `merge_base_moved` transition) so an operator can correlate the route with the
/// base-production/GC events keyed by that sha; a `PerWorktree` decision carries
/// the specific `uncovered_kind` (or `unavailable`) so a `became_uncovered`
/// transition names *which* topology it fell to, not just that it fell.
///
/// The `EVENT_TARGET` (`anvil_intercept::persistence_route`) mirrors the sibling
/// base-event scoping (`anvil_intercept::graph_base_trigger` /
/// `::base_gc` / `::snapshot`): one `anvil_intercept::<area>` target per subsystem,
/// so an operator filters the whole shared-base story by target prefix.
fn emit_event(
    worktree: &Path,
    route: &PersistenceRoute,
    reason: RouteReason,
    uncovered_kind: Option<UncoveredKind>,
    previous_merge_base_sha: Option<&str>,
) {
    match route {
        PersistenceRoute::Base { merge_base_sha } => tracing::info!(
            target: EVENT_TARGET,
            route = route.label(),
            reason = reason.as_str(),
            workspace_root = %worktree.display(),
            merge_base_sha = %merge_base_sha,
            previous_merge_base_sha = previous_merge_base_sha.unwrap_or(""),
            "persistence.route",
        ),
        PersistenceRoute::PerWorktree { .. } => tracing::info!(
            target: EVENT_TARGET,
            route = route.label(),
            reason = reason.as_str(),
            workspace_root = %worktree.display(),
            uncovered_kind = uncovered_kind.map_or("unavailable", UncoveredKind::as_str),
            "persistence.route",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // ---- a scripted, hermetic RouteResolver (no git) -----------------------

    /// A resolver that returns a fixed answer per worktree, or replays a scripted
    /// sequence of answers for one worktree (to drive re-entrancy). No git.
    #[derive(Default)]
    struct FakeResolver {
        fixed: Mutex<HashMap<PathBuf, RouteMergeBase>>,
        scripted: Mutex<VecDeque<RouteMergeBase>>,
        calls: AtomicUsize,
    }

    impl FakeResolver {
        fn fixed(worktree: impl Into<PathBuf>, answer: RouteMergeBase) -> Self {
            let me = Self::default();
            me.fixed.lock().unwrap().insert(worktree.into(), answer);
            me
        }

        fn scripted(answers: impl IntoIterator<Item = RouteMergeBase>) -> Self {
            let me = Self::default();
            *me.scripted.lock().unwrap() = answers.into_iter().collect();
            me
        }
    }

    impl RouteResolver for FakeResolver {
        fn resolve(&self, worktree: &Path) -> RouteMergeBase {
            self.calls.fetch_add(1, Ordering::Relaxed);
            // A scripted queue (re-entrancy tests) takes precedence; when it drains
            // it repeats its last answer so steady-state polling is stable.
            {
                let mut q = self.scripted.lock().unwrap();
                if q.len() > 1 {
                    return q.pop_front().unwrap();
                }
                if let Some(only) = q.front().cloned() {
                    return only;
                }
            }
            self.fixed
                .lock()
                .unwrap()
                .get(worktree)
                .cloned()
                .unwrap_or(RouteMergeBase::Uncovered(UncoveredKind::NoDefaultBranch))
        }
    }

    fn key(root: &Path) -> WorktreeKey {
        WorktreeKey::from_canonical(root.to_path_buf())
    }

    fn sha(seed: char) -> String {
        seed.to_string().repeat(40)
    }

    // ---- (a) covered ⇒ Base with the resolver's primary sha ----------------

    #[test]
    fn covered_worktree_routes_base_with_primary_sha() {
        let wt = PathBuf::from("/wt/covered");
        let router = PersistenceRouter::with_resolver(Arc::new(FakeResolver::fixed(
            &wt,
            RouteMergeBase::Resolved(sha('a')),
        )));
        let decision = router.route(&key(&wt), &wt);
        assert_eq!(
            decision.route,
            PersistenceRoute::Base {
                merge_base_sha: sha('a')
            }
        );
        assert_eq!(decision.reason, RouteReason::BaseResolved);
        assert_eq!(decision.route.label(), "base");
    }

    // ---- (b) uncovered topologies ⇒ PerWorktree, distinct reasons ----------

    #[test]
    fn uncovered_topologies_route_per_worktree_with_distinct_reasons() {
        for (kind, expected_reason) in [
            (
                UncoveredKind::DetachedHead,
                RouteReason::UncoveredDetachedHead,
            ),
            (
                UncoveredKind::NoDefaultBranch,
                RouteReason::UncoveredNoDefaultBranch,
            ),
            (
                UncoveredKind::NoMergeBase,
                RouteReason::UncoveredNoMergeBase,
            ),
        ] {
            let wt = PathBuf::from(format!("/wt/{}", expected_reason.as_str()));
            let router = PersistenceRouter::with_resolver(Arc::new(FakeResolver::fixed(
                &wt,
                RouteMergeBase::Uncovered(kind),
            )));
            let decision = router.route(&key(&wt), &wt);
            assert_eq!(
                decision.route,
                PersistenceRoute::PerWorktree {
                    canonical_root: wt.clone()
                },
                "uncovered ({kind:?}) routes per-worktree",
            );
            assert_eq!(decision.reason, expected_reason, "distinct reason per kind");
            assert_eq!(decision.route.label(), "per_worktree");
        }
    }

    // ---- (c) Unavailable ⇒ PerWorktree this pass, its own reason, can flip --

    #[test]
    fn unavailable_routes_per_worktree_this_pass_then_can_flip() {
        let wt = PathBuf::from("/wt/transient");
        // First pass: transient failure. Next pass: it resolves ⇒ flips to Base.
        let router = PersistenceRouter::with_resolver(Arc::new(FakeResolver::scripted([
            RouteMergeBase::Unavailable,
            RouteMergeBase::Resolved(sha('b')),
        ])));
        let k = key(&wt);

        let first = router.route(&k, &wt);
        assert_eq!(
            first.route,
            PersistenceRoute::PerWorktree {
                canonical_root: wt.clone()
            },
            "a transient failure never wedges — it serves per-worktree this pass",
        );
        assert_eq!(first.reason, RouteReason::ResolverUnavailable);

        let second = router.route(&k, &wt);
        assert_eq!(
            second.route,
            PersistenceRoute::Base {
                merge_base_sha: sha('b')
            },
            "the very next evaluation may flip a transiently-failed worktree to Base",
        );
        assert_eq!(
            second.reason,
            RouteReason::BecameCovered,
            "uncovered/unavailable → covered is a became_covered transition",
        );
    }

    // ---- (d) re-entrancy: merge-base movement + coverage transitions -------

    #[test]
    fn reentrant_merge_base_movement_reflects_the_new_sha() {
        let wt = PathBuf::from("/wt/rebased");
        let router = PersistenceRouter::with_resolver(Arc::new(FakeResolver::scripted([
            RouteMergeBase::Resolved(sha('a')),
            RouteMergeBase::Resolved(sha('c')), // a rebase moved the merge-base
        ])));
        let k = key(&wt);

        let first = router.route(&k, &wt);
        assert_eq!(
            first.route,
            PersistenceRoute::Base {
                merge_base_sha: sha('a')
            }
        );
        assert_eq!(first.reason, RouteReason::BaseResolved);

        let moved = router.route(&k, &wt);
        assert_eq!(
            moved.route,
            PersistenceRoute::Base {
                merge_base_sha: sha('c')
            },
            "after a rebase the next route reflects the NEW merge-base sha",
        );
        assert_eq!(
            moved.reason,
            RouteReason::MergeBaseMoved,
            "a moved sha is reported as a merge_base_moved transition",
        );
    }

    #[test]
    fn reentrant_coverage_flip_becomes_uncovered_then_covered() {
        let wt = PathBuf::from("/wt/flipping");
        let router = PersistenceRouter::with_resolver(Arc::new(FakeResolver::scripted([
            RouteMergeBase::Resolved(sha('a')),
            RouteMergeBase::Uncovered(UncoveredKind::DetachedHead), // git checkout --detach
            RouteMergeBase::Resolved(sha('a')),                     // re-attached
        ])));
        let k = key(&wt);

        assert_eq!(router.route(&k, &wt).reason, RouteReason::BaseResolved);

        let became_uncovered = router.route(&k, &wt);
        assert_eq!(
            became_uncovered.route,
            PersistenceRoute::PerWorktree {
                canonical_root: wt.clone()
            }
        );
        assert_eq!(
            became_uncovered.reason,
            RouteReason::BecameUncovered,
            "covered → detached is a became_uncovered transition",
        );

        let became_covered = router.route(&k, &wt);
        assert_eq!(
            became_covered.route,
            PersistenceRoute::Base {
                merge_base_sha: sha('a')
            },
            "the uncovered route is NOT an irreversible latch — re-attaching flips it back",
        );
        assert_eq!(became_covered.reason, RouteReason::BecameCovered);
    }

    #[test]
    fn steady_state_covered_is_not_reported_as_a_transition() {
        // Re-evaluating an unchanged covered worktree stays base_resolved (no
        // spurious merge_base_moved).
        let wt = PathBuf::from("/wt/steady");
        let router = PersistenceRouter::with_resolver(Arc::new(FakeResolver::scripted([
            RouteMergeBase::Resolved(sha('a')),
            RouteMergeBase::Resolved(sha('a')),
        ])));
        let k = key(&wt);
        assert_eq!(router.route(&k, &wt).reason, RouteReason::BaseResolved);
        assert_eq!(router.route(&k, &wt).reason, RouteReason::BaseResolved);
    }

    // ---- (f)/(g) determinism + concurrency ---------------------------------

    #[test]
    fn routing_is_deterministic_same_input_same_route() {
        // The determinism seam is the injected resolver: identical answers ⇒
        // identical decisions, run to run.
        for _ in 0..20 {
            let wt = PathBuf::from("/wt/deterministic");
            let router = PersistenceRouter::with_resolver(Arc::new(FakeResolver::fixed(
                &wt,
                RouteMergeBase::Resolved(sha('a')),
            )));
            let decision = router.route(&key(&wt), &wt);
            assert_eq!(
                decision,
                RouteDecision {
                    route: PersistenceRoute::Base {
                        merge_base_sha: sha('a')
                    },
                    reason: RouteReason::BaseResolved,
                }
            );
        }
    }

    #[test]
    fn concurrent_distinct_worktrees_route_independently() {
        // Campaign lesson: a single green run lies. Hammer the shared re-entrant
        // memory from many threads over many rounds; distinct worktrees must never
        // cross-contaminate each other's route.
        use std::sync::Arc as StdArc;
        for _round in 0..20 {
            // Each worktree gets its own fixed answer; the resolver dispatches by
            // path, so concurrent routes must land independently.
            let mut answers = HashMap::new();
            for i in 0..8u8 {
                answers.insert(
                    PathBuf::from(format!("/wt/c{i}")),
                    RouteMergeBase::Resolved(sha((b'a' + i) as char)),
                );
            }
            let resolver = FakeResolver::default();
            *resolver.fixed.lock().unwrap() = answers;
            let router = StdArc::new(PersistenceRouter::with_resolver(StdArc::new(resolver)));

            std::thread::scope(|scope| {
                for i in 0..8u8 {
                    let router = StdArc::clone(&router);
                    scope.spawn(move || {
                        let wt = PathBuf::from(format!("/wt/c{i}"));
                        for _ in 0..50 {
                            let decision = router.route(&key(&wt), &wt);
                            assert_eq!(
                                decision.route,
                                PersistenceRoute::Base {
                                    merge_base_sha: sha((b'a' + i) as char)
                                },
                                "worktree c{i} must always route its own sha",
                            );
                        }
                    });
                }
            });
        }
    }

    // ---- GitRouteResolver classification against a real `git` (fake-git) ----

    /// A `#!/bin/sh` fake-git whose behaviour is `body` (a `case "$*"` over the
    /// invocation args) — the same discipline GBASE-008's `base_gc` tests use to
    /// exercise the REAL `run_git` plumbing against controlled outcomes.
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

    /// Resolve, tolerating a transient `Unavailable`. Exec'ing a just-written
    /// fake-git under the full suite's fork/exec pressure can transiently fail
    /// (`ETXTBSY` / fork `EAGAIN`) ⇒ `run_git::Failed` ⇒ `Unavailable`; these
    /// fixtures never *legitimately* yield `Unavailable`, so a few retries make the
    /// classification assertions robust under parallel load (the campaign lesson: a
    /// single green run lies). Tests that *expect* `Unavailable` call `resolve`
    /// directly.
    fn resolve_stable(resolver: &GitRouteResolver, worktree: &Path) -> RouteMergeBase {
        for _ in 0..10 {
            let outcome = resolver.resolve(worktree);
            if outcome != RouteMergeBase::Unavailable {
                return outcome;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        resolver.resolve(worktree)
    }

    #[test]
    fn git_resolver_detached_head_is_uncovered() {
        // `symbolic-ref -q HEAD` exit 1 ⇒ detached ⇒ Uncovered(DetachedHead),
        // even though a merge-base could be computed.
        let (_g, bin) = fake_git(
            "case \"$*\" in \
               *symbolic-ref*) exit 1 ;; \
               *) echo deadbeef; exit 0 ;; \
             esac",
        );
        let resolver = GitRouteResolver::with_git_bin(bin);
        assert_eq!(
            resolve_stable(&resolver, Path::new("/wt/detached")),
            RouteMergeBase::Uncovered(UncoveredKind::DetachedHead),
        );
    }

    #[test]
    fn git_resolver_upstream_refinement_is_the_primary_key() {
        // Producer parity: when the branch tracks an upstream, the upstream-refined
        // merge-base WINS over the default-branch merge-base.
        let (_g, bin) = fake_git(
            "case \"$*\" in \
               *symbolic-ref*) echo refs/heads/feature; exit 0 ;; \
               *origin/HEAD*) echo origin/main; exit 0 ;; \
               *@{upstream}*) echo origin/feature; exit 0 ;; \
               *merge-base*HEAD*origin/feature*) echo upstreamsha; exit 0 ;; \
               *merge-base*HEAD*origin/main*) echo defaultsha; exit 0 ;; \
               *) exit 0 ;; \
             esac",
        );
        let resolver = GitRouteResolver::with_git_bin(bin);
        assert_eq!(
            resolve_stable(&resolver, Path::new("/wt/tracking")),
            RouteMergeBase::Resolved("upstreamsha".to_owned()),
            "the @{{upstream}}-refined key is the producer's primary key",
        );
    }

    #[test]
    fn git_resolver_default_branch_key_when_no_upstream() {
        // No upstream ⇒ the default-branch merge-base is the primary key.
        let (_g, bin) = fake_git(
            "case \"$*\" in \
               *symbolic-ref*) echo refs/heads/local; exit 0 ;; \
               *origin/HEAD*) echo origin/main; exit 0 ;; \
               *@{upstream}*) echo 'fatal: no upstream configured' >&2; exit 128 ;; \
               *merge-base*origin/main*) echo defaultsha; exit 0 ;; \
               *) exit 0 ;; \
             esac",
        );
        let resolver = GitRouteResolver::with_git_bin(bin);
        assert_eq!(
            resolve_stable(&resolver, Path::new("/wt/local")),
            RouteMergeBase::Resolved("defaultsha".to_owned()),
        );
    }

    #[test]
    fn git_resolver_no_default_branch_is_uncovered() {
        // origin/HEAD unset + no origin/main|master ⇒ Uncovered(NoDefaultBranch).
        let (_g, bin) = fake_git(
            "case \"$*\" in \
               *symbolic-ref*) echo refs/heads/main; exit 0 ;; \
               *origin/HEAD*) echo \"fatal: ambiguous argument 'origin/HEAD': unknown revision\" >&2; exit 128 ;; \
               *rev-parse*) echo 'fatal: Needed a single revision' >&2; exit 128 ;; \
               *) exit 0 ;; \
             esac",
        );
        let resolver = GitRouteResolver::with_git_bin(bin);
        assert_eq!(
            resolve_stable(&resolver, Path::new("/wt/no-default")),
            RouteMergeBase::Uncovered(UncoveredKind::NoDefaultBranch),
        );
    }

    #[test]
    fn git_resolver_origin_head_unset_never_falls_back_to_origin_main() {
        // Producer-parity regression (kernel BLOCKING finding): `origin/HEAD` is
        // unset but `origin/main` IS resolvable (mirrors / CI checkouts /
        // hand-added remotes). The producer would return NoBasePossible (it probes
        // origin/HEAD only), so this repo has NO producible base and MUST route
        // per-worktree. base_gc's keep-set resolver would fall back to origin/main
        // and wrongly classify it covered — the route resolver must NOT.
        let (_g, bin) = fake_git(
            "case \"$*\" in \
               *symbolic-ref*) echo refs/heads/main; exit 0 ;; \
               *origin/HEAD*) echo \"fatal: ambiguous argument 'origin/HEAD': unknown revision\" >&2; exit 128 ;; \
               *rev-parse*origin/main*) echo 1111111111111111111111111111111111111111; exit 0 ;; \
               *rev-parse*origin/master*) echo 2222222222222222222222222222222222222222; exit 0 ;; \
               *merge-base*) echo 3333333333333333333333333333333333333333; exit 0 ;; \
               *) exit 0 ;; \
             esac",
        );
        let resolver = GitRouteResolver::with_git_bin(bin);
        assert_eq!(
            resolve_stable(&resolver, Path::new("/wt/mirror")),
            RouteMergeBase::Uncovered(UncoveredKind::NoDefaultBranch),
            "origin/HEAD unset ⇒ per-worktree; the resolver must NOT fall back to \
             origin/main (that sha the producer never produces)",
        );
    }

    #[test]
    fn git_resolver_no_merge_base_is_uncovered() {
        // A default branch exists but HEAD shares no history (merge-base exit 1)
        // and no upstream ⇒ Uncovered(NoMergeBase).
        let (_g, bin) = fake_git(
            "case \"$*\" in \
               *symbolic-ref*) echo refs/heads/orphan; exit 0 ;; \
               *origin/HEAD*) echo origin/main; exit 0 ;; \
               *@{upstream}*) echo 'fatal: no upstream' >&2; exit 128 ;; \
               *merge-base*) exit 1 ;; \
               *) exit 0 ;; \
             esac",
        );
        let resolver = GitRouteResolver::with_git_bin(bin);
        assert_eq!(
            resolve_stable(&resolver, Path::new("/wt/orphan")),
            RouteMergeBase::Uncovered(UncoveredKind::NoMergeBase),
        );
    }

    #[test]
    fn git_resolver_spawn_failure_is_unavailable() {
        // A missing git binary ⇒ spawn failure ⇒ Unavailable (never wedges as a
        // false uncovered).
        let resolver = GitRouteResolver::with_git_bin("/nonexistent/definitely-not-git");
        assert_eq!(
            resolver.resolve(Path::new("/wt/any")),
            RouteMergeBase::Unavailable,
        );
    }

    #[test]
    fn git_resolver_merge_base_unexpected_exit_is_unavailable() {
        // `merge-base` exit >1 is unexpected (not the documented exit-1 no-base) ⇒
        // Unavailable, so a route is re-evaluated later rather than wrongly latched.
        let (_g, bin) = fake_git(
            "case \"$*\" in \
               *symbolic-ref*) echo refs/heads/main; exit 0 ;; \
               *origin/HEAD*) echo origin/main; exit 0 ;; \
               *merge-base*) echo 'fatal: unexpected' >&2; exit 3 ;; \
               *) exit 0 ;; \
             esac",
        );
        let resolver = GitRouteResolver::with_git_bin(bin);
        assert_eq!(
            resolver.resolve(Path::new("/wt/broken")),
            RouteMergeBase::Unavailable,
        );
    }

    #[test]
    fn git_resolver_default_merge_base_fails_shortcircuits_before_upstream() {
        // Mechanism 1 (producer-parity short-circuit): the DEFAULT-branch merge-base
        // fails (exit 1) while the `@{upstream}` merge-base WOULD succeed. The
        // producer bails NoBasePossible before its upstream block, so the route must
        // short-circuit to Uncovered(NoMergeBase) and NEVER key Base on the upstream
        // sha (which the producer never builds).
        let (_g, bin) = fake_git(
            "case \"$*\" in \
               *symbolic-ref*) echo refs/heads/feature; exit 0 ;; \
               *origin/HEAD*) echo origin/main; exit 0 ;; \
               *merge-base*HEAD*origin/main*) exit 1 ;; \
               *@{upstream}*) echo origin/feature; exit 0 ;; \
               *merge-base*HEAD*origin/feature*) echo upstreamsha; exit 0 ;; \
               *) exit 0 ;; \
             esac",
        );
        let resolver = GitRouteResolver::with_git_bin(bin);
        assert_eq!(
            resolve_stable(&resolver, Path::new("/wt/split")),
            RouteMergeBase::Uncovered(UncoveredKind::NoMergeBase),
            "a failed default-branch merge-base short-circuits BEFORE the upstream \
             refinement — never Base{{upstreamsha}}",
        );
    }

    // ---- Mechanism 3: bounded tick re-evaluation (backoff) -----------------

    #[test]
    fn route_on_tick_decimates_a_stable_worktree() {
        // A settled worktree (same route every eval) is decimated: after
        // STABILITY_THRESHOLD evals the tick skips DECIMATION-1 passes before the
        // next eval, so git resolutions are a small fraction of ticks.
        let wt = PathBuf::from("/wt/stable");
        let resolver = Arc::new(FakeResolver::fixed(&wt, RouteMergeBase::Resolved(sha('a'))));
        let router = PersistenceRouter::with_resolver(resolver.clone());
        let k = key(&wt);

        // Drive enough ticks to cross the threshold and one decimation window.
        let ticks = STABILITY_THRESHOLD + DECIMATION_TICKS; // 4 + 10 = 14
        let mut evaluated = 0usize;
        for _ in 0..ticks {
            if router.route_on_tick(&k, &wt).is_some() {
                evaluated += 1;
            }
        }
        // The first STABILITY_THRESHOLD ticks all evaluate; then a full decimation
        // window (DECIMATION_TICKS-1 skips) before exactly one more eval.
        assert_eq!(
            evaluated,
            STABILITY_THRESHOLD as usize + 1,
            "a stable worktree is decimated after the stability threshold",
        );
        assert_eq!(
            resolver.calls.load(Ordering::Relaxed),
            STABILITY_THRESHOLD as usize + 1,
            "the decimated ticks did NOT shell git",
        );
    }

    #[test]
    fn route_on_tick_decimates_a_persistently_unavailable_worktree() {
        // Failure backoff: a worktree whose resolver keeps returning Unavailable is
        // decimated the same way, so a wedged repo does not burn a git probe per tick.
        let wt = PathBuf::from("/wt/wedged");
        let resolver = Arc::new(FakeResolver::fixed(&wt, RouteMergeBase::Unavailable));
        let router = PersistenceRouter::with_resolver(resolver.clone());
        let k = key(&wt);

        let ticks = FAILURE_THRESHOLD + DECIMATION_TICKS;
        for _ in 0..ticks {
            let _ = router.route_on_tick(&k, &wt);
        }
        assert_eq!(
            resolver.calls.load(Ordering::Relaxed),
            FAILURE_THRESHOLD as usize + 1,
            "a persistently-Unavailable worktree is decimated (git probed sparingly)",
        );
    }

    #[test]
    fn route_change_rearms_tick_evaluation() {
        // A route change resets the backoff: after decimation, a flip (driven via a
        // contact `route`) re-arms so the next tick evaluates promptly.
        let wt = PathBuf::from("/wt/rearm");
        let resolver = Arc::new(FakeResolver::default());
        *resolver.fixed.lock().unwrap() =
            std::iter::once((wt.clone(), RouteMergeBase::Resolved(sha('a')))).collect();
        let router = PersistenceRouter::with_resolver(resolver.clone());
        let k = key(&wt);

        // Settle into decimation.
        for _ in 0..(STABILITY_THRESHOLD + 2) {
            let _ = router.route_on_tick(&k, &wt);
        }
        // A contact observes a FLIP (merge-base moved) — re-arms the backoff.
        *resolver.fixed.lock().unwrap() =
            std::iter::once((wt.clone(), RouteMergeBase::Resolved(sha('b')))).collect();
        let moved = router.route(&k, &wt);
        assert_eq!(moved.reason, RouteReason::MergeBaseMoved);
        // The very next tick evaluates (not skipped), because the change re-armed.
        assert!(
            router.route_on_tick(&k, &wt).is_some(),
            "a route change re-arms the tick backoff (next tick evaluates)",
        );
    }

    // ---- Mechanism 3: same-key flapping stress (concurrency) ---------------

    #[test]
    fn route_flapping_same_key_stays_consistent() {
        // Campaign lesson: hammer ONE key from many threads with a resolver that
        // flaps Resolved/Uncovered/Unavailable. Every returned decision must be
        // internally consistent (route ⟺ reason family) and the shared state must
        // never tear or deadlock.
        use std::sync::Arc as StdArc;

        /// A resolver that rotates through the three outcome classes.
        struct FlapResolver {
            n: AtomicUsize,
        }
        impl RouteResolver for FlapResolver {
            fn resolve(&self, _wt: &Path) -> RouteMergeBase {
                match self.n.fetch_add(1, Ordering::Relaxed) % 3 {
                    0 => RouteMergeBase::Resolved(sha('a')),
                    1 => RouteMergeBase::Uncovered(UncoveredKind::DetachedHead),
                    _ => RouteMergeBase::Unavailable,
                }
            }
        }

        for _round in 0..20 {
            let router = StdArc::new(PersistenceRouter::with_resolver(StdArc::new(
                FlapResolver {
                    n: AtomicUsize::new(0),
                },
            )));
            let wt = PathBuf::from("/wt/flap");
            std::thread::scope(|scope| {
                for _ in 0..8 {
                    let router = StdArc::clone(&router);
                    let wt = wt.clone();
                    scope.spawn(move || {
                        for _ in 0..100 {
                            let d = router.route(&key(&wt), &wt);
                            // Route ⟺ reason family must always agree.
                            match d.route {
                                PersistenceRoute::Base { ref merge_base_sha } => {
                                    assert!(!merge_base_sha.is_empty());
                                    assert!(matches!(
                                        d.reason,
                                        RouteReason::BaseResolved
                                            | RouteReason::MergeBaseMoved
                                            | RouteReason::BecameCovered
                                    ));
                                }
                                PersistenceRoute::PerWorktree { .. } => {
                                    assert!(matches!(
                                        d.reason,
                                        RouteReason::UncoveredDetachedHead
                                            | RouteReason::UncoveredNoDefaultBranch
                                            | RouteReason::UncoveredNoMergeBase
                                            | RouteReason::ResolverUnavailable
                                            | RouteReason::BecameUncovered
                                    ));
                                }
                            }
                        }
                    });
                }
            });
            // The map holds exactly the one flapped key — no leak, no tear.
            assert!(router.forget_is_single_key(&key(&wt)));
        }
    }

    #[test]
    fn forget_drops_router_state() {
        let wt = PathBuf::from("/wt/forget");
        let router = PersistenceRouter::with_resolver(Arc::new(FakeResolver::fixed(
            &wt,
            RouteMergeBase::Resolved(sha('a')),
        )));
        let k = key(&wt);
        let _ = router.route(&k, &wt);
        assert!(!router.forget_is_empty(), "state recorded after a route");
        router.forget(&k);
        assert!(
            router.forget_is_empty(),
            "forget drops the worktree's state"
        );
    }
}
