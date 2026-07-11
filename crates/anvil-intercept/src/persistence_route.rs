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

        // 3. `merge-base HEAD <default>` — the default-branch key.
        let base_key = match classify_merge_base(run_git(
            &self.git_bin,
            worktree,
            &["merge-base", "--end-of-options", "HEAD", &default],
        )) {
            MergeBaseCall::Found(sha) => Some(sha),
            MergeBaseCall::NoBase => None,
            MergeBaseCall::Unavailable => return RouteMergeBase::Unavailable,
        };

        // 4. `@{upstream}` refinement WINS when tracked (the producer returns the
        // refined key), so the route composes from the same sha the producer built.
        let Ok(refined) = self.upstream_refinement(worktree) else {
            return RouteMergeBase::Unavailable;
        };

        // 5. Primary = refinement when present, else the default-branch key.
        match refined.or(base_key) {
            Some(primary) => RouteMergeBase::Resolved(primary),
            None => RouteMergeBase::Uncovered(UncoveredKind::NoMergeBase),
        }
    }
}

/// The re-entrant persistence router (ADR-105 §8). Holds the injected
/// [`RouteResolver`] and each worktree's **last route** so a re-evaluation can
/// classify transitions (merge-base movement, covered↔uncovered flips).
///
/// Routing is **time-free and deterministic**: the same resolver answer always
/// yields the same route, and the transition classification is a pure function of
/// `(previous route, new route)`. The determinism seam is the injected resolver
/// (a fake in tests) — mirroring GBASE-008's `KeepSetResolver`; there is no clock
/// to inject because a route decision depends on git state alone, never on time.
pub struct PersistenceRouter {
    resolver: Arc<dyn RouteResolver>,
    /// Each worktree's last computed route, for transition detection. Behind a
    /// `Mutex` so the daemon's concurrent warm-start contacts and the low-cadence
    /// re-route pass share one re-entrant memory.
    last: Mutex<HashMap<WorktreeKey, PersistenceRoute>>,
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
            last: Mutex::new(HashMap::new()),
        }
    }

    /// Route `key` (rooted at `worktree`) between the base and per-worktree paths,
    /// re-entrantly: re-evaluate the topology, classify any transition against the
    /// last route, emit the structured `persistence.route{route, reason}` event,
    /// and remember the new route. Returns the [`RouteDecision`] (the same
    /// `route` + `reason` the event carries).
    ///
    /// Deterministic and side-effect-free beyond the one memory update and the
    /// structured event.
    #[must_use]
    pub fn route(&self, key: &WorktreeKey, worktree: &Path) -> RouteDecision {
        let (route, topology_reason) = match self.resolver.resolve(worktree) {
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
        };

        // Classify the decision against the last route (re-entrancy) and commit
        // the new route under the same lock, so a concurrent re-evaluation sees a
        // consistent (previous, new) pair.
        let reason = {
            let mut last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
            let reason = transition_reason(last.get(key), &route, topology_reason);
            last.insert(key.clone(), route.clone());
            reason
        };

        emit_event(worktree, &route, reason);
        RouteDecision { route, reason }
    }
}

impl Default for PersistenceRouter {
    fn default() -> Self {
        Self::new()
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
/// `Base` decision also carries `merge_base_sha` so an operator can correlate the
/// route with the base-production/GC events keyed by that sha.
fn emit_event(worktree: &Path, route: &PersistenceRoute, reason: RouteReason) {
    match route {
        PersistenceRoute::Base { merge_base_sha } => tracing::info!(
            target: EVENT_TARGET,
            route = route.label(),
            reason = reason.as_str(),
            workspace_root = %worktree.display(),
            merge_base_sha = %merge_base_sha,
            "persistence.route",
        ),
        PersistenceRoute::PerWorktree { .. } => tracing::info!(
            target: EVENT_TARGET,
            route = route.label(),
            reason = reason.as_str(),
            workspace_root = %worktree.display(),
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
}
