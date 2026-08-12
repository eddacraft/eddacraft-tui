//! DSV-003 Task 5 (ADR-061 §5): the default-deny invalidation taxonomy.
//!
//! This is the **taxonomy half** of the assurance module; the workspace
//! assurance *state machine* (Clean/Stale/Pending/Running/Unavailable
//! transitions + the `workspace_status` / `request_full_scan` verbs) lands
//! with the `validate_paths` orchestration in DSV-005.
//!
//! [`taxonomy_reason`] decides, for one classified change in its file-role
//! context, whether the change is *potentially certifiable* (returns `None`)
//! or carries a concrete [`StaleReason`]. The contract is **default-deny**:
//! `None` is returned for exactly one case — a plain content modify of an
//! ordinary file — and every other class, plus an explicitly unclassifiable
//! change, maps to a reason. An unknown change is stale, never clean.
//!
//! Note this taxonomy owns only the *change-classification* reasons. Four
//! `StaleReason` variants are raised elsewhere and are intentionally **not**
//! produced here: `ImpactSetOverflow` (the certify closure, DSV-004/Task 6),
//! `WarmStateEvicted` (the cache, DSV-004/Task 7), `ScanTimeout` (the scan
//! orchestration, DSV-005/-006), and `DaemonAbsent` (the client-side
//! fallback, DSV-007). The wire-level `StaleReason::Unknown` `#[serde(other)]`
//! fallback is a deserialisation affordance, not a taxonomy output.

use std::path::Path;

use anvil_intercept_proto::protocol::{
    AssuranceState, ScanCoverage, StaleReason, WorkspaceAssurance,
};

use crate::change_class::CanonicalChange;

/// File-role context for a change, used to override the raw change class.
///
/// Editing (or deleting) certain files invalidates more than the file itself:
/// a `.gitignore` edit changes which paths are even in scope; a config /
/// policy / boundary edit changes the rule surface; a symlink retarget moves
/// the resolution target out from under the warm graph. These take precedence
/// over the underlying [`CanonicalChange`].
// Four independent boolean facts about one change; they are not a state enum
// (a change can be, e.g., both a symlink retarget and under `.anvil/`), so the
// flat-flags shape is the honest model.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default, Clone, Copy)]
pub struct ChangeCtx {
    /// The upstream classifier could not determine the change. Default-deny:
    /// this forces [`StaleReason::UnknownClass`].
    pub unclassifiable: bool,
    /// The change retargeted a symlink in the resolution path.
    pub symlink_retarget: bool,
    /// The changed path is a `.gitignore`.
    pub gitignore: bool,
    /// The changed path is an Anvil config / boundary / policy file.
    pub config_or_policy: bool,
}

impl ChangeCtx {
    /// Derive the role context for a root-relative path. `symlink_retarget` is
    /// supplied by the caller (it is a property of the change, observed at
    /// classification time, not of the path string).
    #[must_use]
    pub fn for_path(rel: &str, symlink_retarget: bool) -> Self {
        Self {
            unclassifiable: false,
            symlink_retarget,
            gitignore: is_gitignore(rel),
            config_or_policy: is_config_or_policy(rel),
        }
    }
}

/// `true` for a `.gitignore` at the root or any subdirectory.
#[must_use]
fn is_gitignore(rel: &str) -> bool {
    rel == ".gitignore" || rel.ends_with("/.gitignore")
}

/// `true` for an Anvil config / boundary / policy file: a `.anvil.<ext>` config
/// (`yaml`/`yml`/`json`/`toml`) or anything under a `.anvil/` directory.
///
/// The config-file set is kept **lock-step with the canonical recogniser**
/// `rule_cache::is_anvil_config_file` (and `anvil_config::discover`): stem
/// `.anvil`, extension in `{yaml, yml, json, toml}`, lowercase only. A false
/// negative here would wrongly certify a boundary change as clean, so the two
/// must not drift. The legacy `.anvilrc` is a separate MLP2-040 migration
/// concern, not in the discover precedence, and is intentionally excluded.
#[must_use]
fn is_config_or_policy(rel: &str) -> bool {
    let basename = rel.rsplit('/').next().unwrap_or(rel);
    let path = Path::new(basename);
    let is_anvil_config = path.file_stem().and_then(|s| s.to_str()) == Some(".anvil")
        && path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| matches!(ext, "yaml" | "yml" | "json" | "toml"));
    is_anvil_config || rel == ".anvil" || rel.starts_with(".anvil/") || rel.contains("/.anvil/")
}

/// Map a classified change + its file-role context to an invalidation reason,
/// or `None` if the change is potentially certifiable.
///
/// Default-deny precedence: an unclassifiable change is stale first; then the
/// file-role overrides (symlink retarget → gitignore → config/policy); then
/// the raw change class. Only a plain content modify of an ordinary file is
/// `None` (potentially certifiable — the certify closure decides the rest).
#[must_use]
pub fn taxonomy_reason(change: &CanonicalChange, ctx: &ChangeCtx) -> Option<StaleReason> {
    if ctx.unclassifiable {
        return Some(StaleReason::UnknownClass);
    }
    if ctx.symlink_retarget {
        return Some(StaleReason::SymlinkRetarget);
    }
    if ctx.gitignore {
        return Some(StaleReason::GitignoreScopeChange);
    }
    if ctx.config_or_policy {
        return Some(StaleReason::ConfigBoundaryPolicyEdit);
    }
    match change {
        CanonicalChange::Delete => Some(StaleReason::Deleted),
        CanonicalChange::Rename { .. } => Some(StaleReason::Renamed),
        // A new file needs cross-file resolution before its impact is known;
        // conservatively stale until the certify closure runs.
        CanonicalChange::Create => Some(StaleReason::CrossFileResolutionNeeded),
        // The single potentially-certifiable case.
        CanonicalChange::ContentModify => None,
    }
}

/// Priority of a requested full scan (DSV-005 Task 9 `request_full_scan`): an
/// interactive (client-blocking) scan jumps the queue ahead of a background one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanPriority {
    /// Client is waiting on the result; run on the interactive pool.
    Interactive,
    /// Opportunistic warm-up; run on the background pool.
    Background,
}

/// How a full scan finished, returned by [`AssuranceMachine::complete_scan`] /
/// [`AssuranceMachine::complete_scan_bounded`] so the executor knows whether to
/// re-queue (DSV-045 / ADR-085 Decision 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanCompletion {
    /// The scan completed with full coverage and was never raced by a save →
    /// the workspace is `Clean`.
    Clean,
    /// The scan completed but the worktree exceeded the post-gitignore walk cap
    /// → the workspace is `Bounded` (warm-but-incomplete), never `Clean`.
    Bounded,
    /// A save (or any other `apply_delta`) landed for this key *during* the
    /// `Running` scan, so the completed graph may not reflect it: the workspace
    /// is marked `Stale(CrossFileResolutionNeeded)` and the executor MUST
    /// re-queue a fresh scan. This is the phantom-`Clean` guard (Decision 4).
    Dirtied,
    /// The completion did not match the active scan token (timed out, superseded
    /// by a newer `start_scan`, or never started). The machine state is unchanged.
    Ignored,
}

/// The per-worktree workspace-assurance state machine (DSV-005 Task 9,
/// ADR-061 §9). Drives the lifecycle `validate_paths` reports and renders the
/// wire [`WorkspaceAssurance`].
///
/// Lifecycle (contract §6 + council B6): a fresh / cold-key worktree starts
/// **`Stale(CrossFileResolutionNeeded)`** — never `Clean` — and reaches `Clean`
/// only via a completed full scan
/// (`request_full_scan` → `Pending` → [`start_scan`](Self::start_scan) →
/// `Running` → [`complete_scan`](Self::complete_scan) → `Clean`). It drops back
/// to `Stale` on any uncertifiable delta ([`record_verdict`](Self::record_verdict)
/// / [`mark_stale`](Self::mark_stale)), a [`scan_timeout`](Self::scan_timeout),
/// or a daemon [`restart`](Self::on_restart) that abandons an in-flight scan.
///
/// Invariant (mirrors the wire): `reason` is `Some` exactly when the state is
/// `Stale`/`Unavailable`; [`snapshot`](Self::snapshot) enforces it. `generation`
/// is the opaque turnover token that bumps on warm-state eviction / cold
/// rebuild. `scan_started_at` is internal lifecycle bookkeeping (it has no wire
/// field — it rides the tracing mirror per Task 9) and is `Some` only while
/// `Running`.
#[derive(Debug, Clone)]
pub struct AssuranceMachine {
    state: AssuranceState,
    reason: Option<StaleReason>,
    generation: u64,
    last_full_scan: Option<String>,
    scan_started_at: Option<String>,
    /// DSV-045 (ADR-085 Decision 4): set by [`note_apply_delta`](Self::note_apply_delta)
    /// when **any** `apply_delta` lands for this key while a scan is `Running` —
    /// origin-agnostic (an interactive `validate_paths` save *and* a GCTX
    /// on-demand re-warm both set it). [`complete_scan`](Self::complete_scan)
    /// reads-and-clears it under the same machine lock as the terminal
    /// transition (compare-and-clear), so a raced save can never be lost to a
    /// phantom-`Clean`.
    dirty_during_scan: bool,
    /// DSV-045 (ADR-085 Decision 5c): the walk coverage of the last
    /// `Bounded`-completing scan. `Some` only while the state is `Bounded`;
    /// every other transition clears it.
    scan_coverage: Option<ScanCoverage>,
    /// Monotonic scan identity. Bumped by every [`start_scan`](Self::start_scan).
    /// Completions and timeouts must present the token they observed at start so
    /// a late worker cannot settle a newer scan or restore a timed-out workspace
    /// to `Clean`.
    scan_seq: u64,
    /// Token of the scan currently allowed to complete or time out. Cleared when
    /// that scan finishes, times out, is abandoned on restart, or is superseded
    /// by a fresh enqueue via [`request_full_scan`](Self::request_full_scan).
    active_scan: Option<u64>,
}

impl AssuranceMachine {
    /// A fresh worktree: `Stale(CrossFileResolutionNeeded)` (B6 — never clean
    /// on first contact; cross-file imports are unresolved until a scan).
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: AssuranceState::Stale,
            reason: Some(StaleReason::CrossFileResolutionNeeded),
            generation: 0,
            last_full_scan: None,
            scan_started_at: None,
            dirty_during_scan: false,
            scan_coverage: None,
            scan_seq: 0,
            active_scan: None,
        }
    }

    /// The current coarse state.
    #[must_use]
    pub fn state(&self) -> AssuranceState {
        self.state
    }

    /// The current staleness cause, if any.
    #[must_use]
    pub fn reason(&self) -> Option<StaleReason> {
        self.reason
    }

    /// The opaque warm-state turnover token.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The RFC 3339 start time of the in-flight scan, if `Running`. Internal
    /// bookkeeping — no wire field; rides the tracing mirror (Task 9).
    #[must_use]
    pub fn scan_started_at(&self) -> Option<&str> {
        self.scan_started_at.as_deref()
    }

    /// The current scan coverage, if the state is `Bounded`.
    #[must_use]
    pub fn scan_coverage(&self) -> Option<ScanCoverage> {
        self.scan_coverage
    }

    /// Whether a save raced the in-flight scan (DSV-045). Test/diagnostic
    /// accessor — the authoritative compare-and-clear is in
    /// [`complete_scan`](Self::complete_scan).
    #[must_use]
    pub fn is_dirty_during_scan(&self) -> bool {
        self.dirty_during_scan
    }

    /// Render the wire snapshot, enforcing the `reason`-iff-`Stale`/`Unavailable`
    /// invariant regardless of any stale `reason` left on a lifecycle state, and
    /// carrying `scan_coverage` only on a `Bounded` snapshot (DSV-045).
    #[must_use]
    pub fn snapshot(&self) -> WorkspaceAssurance {
        let reason = match self.state {
            AssuranceState::Stale | AssuranceState::Unavailable => self.reason,
            // `Bounded` (a completed-but-truncated scan) and the lifecycle
            // states carry no staleness cause; `Unknown` is never produced
            // locally but must be handled for exhaustiveness (fail-safe: no
            // reason emitted, the wire fallback is a deser-only affordance).
            AssuranceState::Clean
            | AssuranceState::Pending
            | AssuranceState::Running
            | AssuranceState::Bounded
            | AssuranceState::Unknown => None,
        };
        let scan_coverage = match self.state {
            AssuranceState::Bounded => self.scan_coverage,
            _ => None,
        };
        WorkspaceAssurance {
            state: self.state,
            reason,
            generation: self.generation,
            last_full_scan: self.last_full_scan.clone(),
            scan_coverage,
        }
    }

    /// An uncertifiable delta makes the workspace stale with `reason`. Clears
    /// any in-flight scan bookkeeping (DSV-045): a stale workspace has no
    /// trustworthy coverage and no pending dirty-race to resolve.
    pub fn mark_stale(&mut self, reason: StaleReason) {
        self.state = AssuranceState::Stale;
        self.reason = Some(reason);
        self.scan_started_at = None;
        self.scan_coverage = None;
        // NB: the dirty flag is deliberately NOT cleared here. An uncertifiable
        // verdict that lands *during* a `Running` scan transitions the machine
        // to `Stale` while the scan loop is still in flight; the dirty flag must
        // survive so the scan's eventual `complete_scan` still sees the race and
        // re-queues (DSV-045). It is cleared only by `request_full_scan` (a fresh
        // enqueue) and `finish_scan` (a completion's compare-and-clear).
    }

    /// Record that an `apply_delta` landed for this key (DSV-045 / ADR-085
    /// Decision 4). If a scan is `Running`, this flags the scan dirty so its
    /// eventual [`complete_scan`](Self::complete_scan) fails safe to `Stale`
    /// rather than certifying a graph that may not reflect the just-applied
    /// delta. Origin-agnostic: an interactive `validate_paths` save and a GCTX
    /// on-demand re-warm both call it. A no-op outside `Running` — a delta
    /// applied while `Clean`/`Stale`/etc. has no scan to invalidate.
    pub fn note_apply_delta(&mut self) {
        if self.state == AssuranceState::Running {
            self.dirty_during_scan = true;
        }
    }

    /// Fold a `validate_paths` verdict into the workspace state.
    ///
    /// A `certified` verdict leaves the state unchanged — it keeps a `Clean`
    /// workspace clean, but does **not** by itself clear a `Stale` workspace
    /// (only a completed full scan does: stale→pending→running→clean). An
    /// uncertifiable verdict makes the workspace stale with `stale_reason`
    /// (defaulting to `CrossFileResolutionNeeded` if none was supplied).
    pub fn record_verdict(&mut self, certified: bool, stale_reason: Option<StaleReason>) {
        if certified {
            return;
        }
        self.mark_stale(stale_reason.unwrap_or(StaleReason::CrossFileResolutionNeeded));
    }

    /// Queue a full scan. Idempotent while a scan is already `Running`. Returns
    /// `priority` so the caller can build the job handle.
    pub fn request_full_scan(&mut self, priority: ScanPriority) -> ScanPriority {
        if self.state != AssuranceState::Running {
            self.state = AssuranceState::Pending;
            self.reason = None;
            self.scan_started_at = None;
            self.scan_coverage = None;
            self.dirty_during_scan = false;
            // A fresh enqueue abandons any prior in-flight token so a straggler
            // completion from the previous attempt cannot settle this queue.
            self.active_scan = None;
        }
        priority
    }

    /// Begin a queued scan (`now` = RFC 3339 start time). Clears any prior
    /// coverage but NOT the dirty flag: a continuation segment (after a
    /// cooperative yield) re-enters `start_scan`, and a save that raced the
    /// already-processed portion must still be honoured at completion (DSV-045).
    /// A fresh enqueue clears the flag via [`request_full_scan`](Self::request_full_scan).
    ///
    /// Returns a scan token the caller must present to
    /// [`complete_scan`](Self::complete_scan) / [`scan_timeout`](Self::scan_timeout).
    /// Each call issues a new token, so a late completion from a prior segment or
    /// timed-out attempt cannot settle a newer run.
    pub fn start_scan(&mut self, now: String) -> u64 {
        self.scan_seq = self.scan_seq.wrapping_add(1);
        let token = self.scan_seq;
        self.active_scan = Some(token);
        self.state = AssuranceState::Running;
        self.reason = None;
        self.scan_started_at = Some(now);
        self.scan_coverage = None;
        token
    }

    /// A full-coverage scan completed (`now` = RFC 3339 completion). Reads-and-
    /// clears the dirty flag under the caller's machine lock (DSV-045 / ADR-085
    /// Decision 4): if a save raced the scan the workspace fails safe to
    /// `Stale(CrossFileResolutionNeeded)` ([`ScanCompletion::Dirtied`], the
    /// executor re-queues); otherwise it transitions to `Clean`.
    ///
    /// `token` must be the value returned by the matching
    /// [`start_scan`](Self::start_scan). A mismatched or already-settled token
    /// yields [`ScanCompletion::Ignored`] and leaves the machine unchanged.
    pub fn complete_scan(&mut self, now: String, token: u64) -> ScanCompletion {
        self.finish_scan(now, None, token)
    }

    /// A scan completed but the worktree exceeded the post-gitignore walk cap:
    /// the workspace is `Bounded` (warm-but-incomplete), carrying `coverage`,
    /// never `Clean` (DSV-045 / ADR-085 Decision 5). The same dirty-race guard
    /// as [`complete_scan`](Self::complete_scan) applies — a raced save still
    /// wins (`Dirtied`), because a bounded-but-raced graph is no more
    /// trustworthy than a full-but-raced one.
    pub fn complete_scan_bounded(
        &mut self,
        now: String,
        coverage: ScanCoverage,
        token: u64,
    ) -> ScanCompletion {
        self.finish_scan(now, Some(coverage), token)
    }

    /// Shared terminal transition for both completion paths: require a matching
    /// active scan token, then compare-and-clear the dirty flag and settle to
    /// `Clean`/`Bounded` (or fail safe to `Stale` on a raced save). `coverage`
    /// selects the terminal state.
    fn finish_scan(
        &mut self,
        now: String,
        coverage: Option<ScanCoverage>,
        token: u64,
    ) -> ScanCompletion {
        if self.active_scan != Some(token) {
            // Timed out, superseded, or never the active scan — do not move the
            // machine (in particular, do not restore a ScanTimeout to Clean).
            return ScanCompletion::Ignored;
        }
        self.active_scan = None;
        let was_dirty = std::mem::take(&mut self.dirty_during_scan);
        self.scan_started_at = None;
        if was_dirty {
            // A save landed mid-scan — the completed graph may not reflect it.
            // Fail safe and let the executor re-queue. `last_full_scan` does NOT
            // advance: no trustworthy scan completed.
            self.state = AssuranceState::Stale;
            self.reason = Some(StaleReason::CrossFileResolutionNeeded);
            self.scan_coverage = None;
            return ScanCompletion::Dirtied;
        }
        self.reason = None;
        self.last_full_scan = Some(now);
        if let Some(coverage) = coverage {
            self.state = AssuranceState::Bounded;
            self.scan_coverage = Some(coverage);
            ScanCompletion::Bounded
        } else {
            self.state = AssuranceState::Clean;
            self.scan_coverage = None;
            ScanCompletion::Clean
        }
    }

    /// A scan exceeded its time budget ⇒ `Stale(ScanTimeout)`.
    ///
    /// `token` must match the active scan; a stale or superseded token is a
    /// no-op so a timed-out straggler cannot overwrite a newer scan's state.
    pub fn scan_timeout(&mut self, token: u64) {
        if self.active_scan != Some(token) {
            return;
        }
        self.active_scan = None;
        self.mark_stale(StaleReason::ScanTimeout);
    }

    /// A daemon restart abandons any in-flight scan and loses warm state: a
    /// `Pending`/`Running` workspace becomes `Stale(WarmStateEvicted)`. A
    /// terminal `Clean`/`Stale` is left as-is (a restart does not invent a
    /// verdict for an already-settled workspace).
    pub fn on_restart(&mut self) {
        if matches!(
            self.state,
            AssuranceState::Pending | AssuranceState::Running
        ) {
            // Abandon the in-flight token so a worker that survives the restart
            // cannot complete into Clean over WarmStateEvicted.
            self.active_scan = None;
            self.mark_stale(StaleReason::WarmStateEvicted);
        }
    }

    /// Bump the opaque turnover token on warm-state eviction / cold rebuild.
    pub fn bump_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }
}

impl Default for AssuranceMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn plain() -> ChangeCtx {
        ChangeCtx::default()
    }

    #[test]
    fn plain_content_modify_is_certifiable() {
        assert_eq!(
            taxonomy_reason(&CanonicalChange::ContentModify, &plain()),
            None,
            "a plain content modify is the one potentially-certifiable case"
        );
    }

    #[test]
    fn delete_maps_to_deleted() {
        assert_eq!(
            taxonomy_reason(&CanonicalChange::Delete, &plain()),
            Some(StaleReason::Deleted)
        );
    }

    #[test]
    fn rename_maps_to_renamed() {
        assert_eq!(
            taxonomy_reason(
                &CanonicalChange::Rename {
                    from: PathBuf::from("src/old.rs")
                },
                &plain()
            ),
            Some(StaleReason::Renamed)
        );
    }

    #[test]
    fn create_maps_to_cross_file_resolution_needed() {
        assert_eq!(
            taxonomy_reason(&CanonicalChange::Create, &plain()),
            Some(StaleReason::CrossFileResolutionNeeded)
        );
    }

    #[test]
    fn symlink_retarget_overrides_even_a_content_modify() {
        let ctx = ChangeCtx {
            symlink_retarget: true,
            ..ChangeCtx::default()
        };
        assert_eq!(
            taxonomy_reason(&CanonicalChange::ContentModify, &ctx),
            Some(StaleReason::SymlinkRetarget)
        );
    }

    #[test]
    fn gitignore_edit_maps_to_gitignore_scope_change() {
        let ctx = ChangeCtx::for_path(".gitignore", false);
        assert!(ctx.gitignore);
        assert_eq!(
            taxonomy_reason(&CanonicalChange::ContentModify, &ctx),
            Some(StaleReason::GitignoreScopeChange)
        );
        // Nested .gitignore too.
        assert!(ChangeCtx::for_path("crates/foo/.gitignore", false).gitignore);
    }

    #[test]
    fn config_or_policy_edit_maps_to_config_boundary_policy_edit() {
        // All four `.anvil.<ext>` forms the canonical recogniser accepts,
        // nested configs, and the `.anvil/` directory — lock-step with
        // rule_cache::is_anvil_config_file.
        for path in [
            ".anvil.yaml",
            ".anvil.yml",
            ".anvil.json",
            ".anvil.toml",
            "crates/foo/.anvil.toml",
            ".anvil/policy.toml",
        ] {
            let ctx = ChangeCtx::for_path(path, false);
            assert!(
                ctx.config_or_policy,
                "{path} should be a config/policy file"
            );
            assert_eq!(
                taxonomy_reason(&CanonicalChange::ContentModify, &ctx),
                Some(StaleReason::ConfigBoundaryPolicyEdit),
                "{path}"
            );
        }
        // Not config/policy: an ordinary source file, the dotless `anvil.toml`
        // (not a discover-precedence name), and the legacy `.anvilrc`.
        for path in ["src/lib.rs", "anvil.toml", ".anvilrc"] {
            assert!(
                !ChangeCtx::for_path(path, false).config_or_policy,
                "{path} must NOT be treated as a config/policy file"
            );
        }
    }

    #[test]
    fn file_role_takes_precedence_over_rename() {
        // Renaming `src/.ignore` → `src/.gitignore` is a scope change, not a
        // plain Renamed; same for renaming into a config path.
        let gitignore = ChangeCtx::for_path("src/.gitignore", false);
        assert_eq!(
            taxonomy_reason(
                &CanonicalChange::Rename {
                    from: PathBuf::from("src/.ignore")
                },
                &gitignore
            ),
            Some(StaleReason::GitignoreScopeChange)
        );
        let config = ChangeCtx::for_path(".anvil.toml", false);
        assert_eq!(
            taxonomy_reason(
                &CanonicalChange::Rename {
                    from: PathBuf::from(".anvil.toml.bak")
                },
                &config
            ),
            Some(StaleReason::ConfigBoundaryPolicyEdit)
        );
    }

    #[test]
    fn unknown_class_defaults_to_stale_not_clean() {
        let ctx = ChangeCtx {
            unclassifiable: true,
            ..ChangeCtx::default()
        };
        // Even for a content modify (the otherwise-certifiable case), an
        // unclassifiable change fails closed to stale.
        let reason = taxonomy_reason(&CanonicalChange::ContentModify, &ctx);
        assert_eq!(reason, Some(StaleReason::UnknownClass));
        assert!(reason.is_some(), "unknown class must be stale, never clean");
    }

    #[test]
    fn file_role_takes_precedence_over_delete() {
        // Deleting a .gitignore is a scope change, not a plain Deleted.
        let ctx = ChangeCtx::for_path(".gitignore", false);
        assert_eq!(
            taxonomy_reason(&CanonicalChange::Delete, &ctx),
            Some(StaleReason::GitignoreScopeChange)
        );
    }

    // ---- AssuranceMachine (Task 9 state machine) ----

    #[test]
    fn initial_workspace_state_is_stale_not_clean() {
        // B6: a fresh / cold-key workspace must NOT start clean.
        let m = AssuranceMachine::new();
        assert_eq!(m.state(), AssuranceState::Stale);
        assert_eq!(m.reason(), Some(StaleReason::CrossFileResolutionNeeded));
        let snap = m.snapshot();
        assert_eq!(snap.state, AssuranceState::Stale);
        assert_eq!(snap.reason, Some(StaleReason::CrossFileResolutionNeeded));
    }

    #[test]
    fn stale_requires_reason() {
        // Stale/Unavailable snapshots carry a reason; lifecycle states do not.
        let mut m = AssuranceMachine::new();
        m.mark_stale(StaleReason::ImpactSetOverflow);
        assert_eq!(m.snapshot().reason, Some(StaleReason::ImpactSetOverflow));

        let token = m.start_scan("2026-06-03T00:00:00Z".to_string());
        assert_eq!(
            m.snapshot().reason,
            None,
            "a Running snapshot has no staleness reason"
        );

        m.complete_scan("2026-06-03T00:00:01Z".to_string(), token);
        assert_eq!(
            m.snapshot().reason,
            None,
            "a Clean snapshot has no staleness reason"
        );
    }

    #[test]
    fn running_carries_scan_started_at() {
        let mut m = AssuranceMachine::new();
        assert_eq!(m.scan_started_at(), None);
        m.request_full_scan(ScanPriority::Interactive);
        assert_eq!(m.state(), AssuranceState::Pending);
        assert_eq!(m.scan_started_at(), None, "pending has no start time");

        m.start_scan("2026-06-03T12:00:00Z".to_string());
        assert_eq!(m.state(), AssuranceState::Running);
        assert_eq!(m.scan_started_at(), Some("2026-06-03T12:00:00Z"));
    }

    #[test]
    fn full_scan_lifecycle_reaches_clean() {
        let mut m = AssuranceMachine::new();
        m.request_full_scan(ScanPriority::Background);
        let token = m.start_scan("t0".to_string());
        m.complete_scan("t1".to_string(), token);
        assert_eq!(m.state(), AssuranceState::Clean);
        assert_eq!(m.snapshot().last_full_scan.as_deref(), Some("t1"));
        assert_eq!(m.scan_started_at(), None);
    }

    #[test]
    fn certified_verdict_keeps_clean_but_does_not_clear_stale() {
        let mut m = AssuranceMachine::new();
        // Cold/stale: a certified single-path verdict does NOT clear it — only
        // a full scan can (stale→pending→running→clean).
        m.record_verdict(true, None);
        assert_eq!(m.state(), AssuranceState::Stale);

        // Once clean (via a scan), a certified verdict keeps it clean...
        m.request_full_scan(ScanPriority::Interactive);
        let token = m.start_scan("t0".to_string());
        m.complete_scan("t1".to_string(), token);
        m.record_verdict(true, None);
        assert_eq!(m.state(), AssuranceState::Clean);

        // ...and an uncertifiable verdict makes it stale with the reason.
        m.record_verdict(false, Some(StaleReason::ImpactSetOverflow));
        assert_eq!(m.state(), AssuranceState::Stale);
        assert_eq!(m.reason(), Some(StaleReason::ImpactSetOverflow));
    }

    #[test]
    fn scan_timeout_to_stale() {
        let mut m = AssuranceMachine::new();
        m.request_full_scan(ScanPriority::Interactive);
        let token = m.start_scan("t0".to_string());
        m.scan_timeout(token);
        assert_eq!(m.state(), AssuranceState::Stale);
        assert_eq!(m.reason(), Some(StaleReason::ScanTimeout));
        assert_eq!(m.scan_started_at(), None);
    }

    #[test]
    fn restart_running_becomes_stale() {
        let mut m = AssuranceMachine::new();
        m.request_full_scan(ScanPriority::Interactive);
        m.start_scan("t0".to_string());
        m.on_restart();
        assert_eq!(m.state(), AssuranceState::Stale);
        assert_eq!(m.reason(), Some(StaleReason::WarmStateEvicted));

        // A restart does not disturb an already-settled Clean workspace.
        let mut clean = AssuranceMachine::new();
        clean.request_full_scan(ScanPriority::Interactive);
        let token = clean.start_scan("t0".to_string());
        clean.complete_scan("t1".to_string(), token);
        clean.on_restart();
        assert_eq!(clean.state(), AssuranceState::Clean);
    }

    // ---- DSV-045: dirty-during-scan race guard + Bounded completion ----

    #[test]
    fn note_apply_delta_sets_dirty_only_while_running() {
        let mut m = AssuranceMachine::new();
        // Stale (cold): a delta has no scan to invalidate.
        m.note_apply_delta();
        assert!(!m.is_dirty_during_scan(), "no dirty flag outside Running");

        m.request_full_scan(ScanPriority::Background);
        m.note_apply_delta();
        assert!(!m.is_dirty_during_scan(), "Pending is not Running");

        m.start_scan("t0".to_string());
        m.note_apply_delta();
        assert!(
            m.is_dirty_during_scan(),
            "a delta during Running flags dirty"
        );
    }

    #[test]
    fn complete_scan_after_dirty_is_stale_not_clean_and_clears_flag() {
        let mut m = AssuranceMachine::new();
        m.request_full_scan(ScanPriority::Background);
        let token = m.start_scan("t0".to_string());
        m.note_apply_delta(); // a save raced the scan

        let completion = m.complete_scan("t1".to_string(), token);
        assert_eq!(completion, ScanCompletion::Dirtied);
        assert_eq!(m.state(), AssuranceState::Stale);
        assert_eq!(m.reason(), Some(StaleReason::CrossFileResolutionNeeded));
        // Read-and-clear: the flag is cleared by the completion, so a fresh
        // (re-queued) scan does not inherit the prior race.
        assert!(
            !m.is_dirty_during_scan(),
            "dirty flag cleared on completion"
        );
        // last_full_scan did NOT advance — no trustworthy scan completed.
        assert_eq!(m.snapshot().last_full_scan, None);
    }

    #[test]
    fn clean_completion_when_never_dirtied() {
        let mut m = AssuranceMachine::new();
        m.request_full_scan(ScanPriority::Background);
        let token = m.start_scan("t0".to_string());
        let completion = m.complete_scan("t1".to_string(), token);
        assert_eq!(completion, ScanCompletion::Clean);
        assert_eq!(m.state(), AssuranceState::Clean);
        assert_eq!(m.snapshot().last_full_scan.as_deref(), Some("t1"));
    }

    #[test]
    fn bounded_completion_carries_coverage_and_no_reason() {
        let mut m = AssuranceMachine::new();
        m.request_full_scan(ScanPriority::Background);
        let token = m.start_scan("t0".to_string());
        let coverage = ScanCoverage {
            scanned_files: 100,
            total_files: 250,
        };
        let completion = m.complete_scan_bounded("t1".to_string(), coverage, token);
        assert_eq!(completion, ScanCompletion::Bounded);
        assert_eq!(m.state(), AssuranceState::Bounded);
        let snap = m.snapshot();
        assert_eq!(snap.state, AssuranceState::Bounded);
        assert_eq!(snap.reason, None, "Bounded is a lifecycle state, no reason");
        assert_eq!(snap.scan_coverage, Some(coverage));
    }

    #[test]
    fn bounded_completion_after_dirty_still_fails_safe_to_stale() {
        let mut m = AssuranceMachine::new();
        m.request_full_scan(ScanPriority::Background);
        let token = m.start_scan("t0".to_string());
        m.note_apply_delta();
        let completion = m.complete_scan_bounded(
            "t1".to_string(),
            ScanCoverage {
                scanned_files: 100,
                total_files: 250,
            },
            token,
        );
        assert_eq!(completion, ScanCompletion::Dirtied);
        assert_eq!(m.state(), AssuranceState::Stale);
        assert_eq!(
            m.snapshot().scan_coverage,
            None,
            "no coverage on a stale race"
        );
    }

    #[test]
    fn scan_coverage_clears_on_a_later_clean_scan() {
        let mut m = AssuranceMachine::new();
        m.request_full_scan(ScanPriority::Background);
        let token = m.start_scan("t0".to_string());
        m.complete_scan_bounded(
            "t1".to_string(),
            ScanCoverage {
                scanned_files: 100,
                total_files: 250,
            },
            token,
        );
        assert!(m.scan_coverage().is_some());
        // A later full scan that completes clean drops the bounded coverage.
        m.request_full_scan(ScanPriority::Background);
        let token = m.start_scan("t2".to_string());
        m.complete_scan("t3".to_string(), token);
        assert_eq!(m.state(), AssuranceState::Clean);
        assert_eq!(m.snapshot().scan_coverage, None);
    }

    #[test]
    fn workspace_status_reports_state_and_generation() {
        let mut m = AssuranceMachine::new();
        m.bump_generation();
        m.bump_generation();
        let snap = m.snapshot();
        assert_eq!(snap.state, AssuranceState::Stale);
        assert_eq!(
            snap.generation, 2,
            "generation bumps on warm-state turnover"
        );
    }

    // ---- Scan-token correlation (late complete after timeout) ----

    #[test]
    fn late_complete_after_timeout_stays_stale_scan_timeout() {
        // Reproduction: start → timeout → complete must NOT restore Clean.
        let mut m = AssuranceMachine::new();
        m.request_full_scan(ScanPriority::Interactive);
        let token = m.start_scan("t0".to_string());
        m.scan_timeout(token);
        assert_eq!(m.state(), AssuranceState::Stale);
        assert_eq!(m.reason(), Some(StaleReason::ScanTimeout));

        let completion = m.complete_scan("t1".to_string(), token);
        assert_eq!(
            completion,
            ScanCompletion::Ignored,
            "timed-out scan completion must be ignored"
        );
        assert_eq!(m.state(), AssuranceState::Stale);
        assert_eq!(
            m.reason(),
            Some(StaleReason::ScanTimeout),
            "must remain Stale(ScanTimeout), not Clean"
        );
        assert_eq!(
            m.snapshot().last_full_scan,
            None,
            "a timed-out scan must not advance last_full_scan"
        );
    }

    #[test]
    fn late_bounded_complete_after_timeout_is_ignored() {
        let mut m = AssuranceMachine::new();
        m.request_full_scan(ScanPriority::Background);
        let token = m.start_scan("t0".to_string());
        m.scan_timeout(token);
        let completion = m.complete_scan_bounded(
            "t1".to_string(),
            ScanCoverage {
                scanned_files: 1,
                total_files: 2,
            },
            token,
        );
        assert_eq!(completion, ScanCompletion::Ignored);
        assert_eq!(m.state(), AssuranceState::Stale);
        assert_eq!(m.reason(), Some(StaleReason::ScanTimeout));
        assert_eq!(m.snapshot().scan_coverage, None);
    }

    #[test]
    fn old_completion_cannot_settle_a_newer_running_scan() {
        let mut m = AssuranceMachine::new();
        m.request_full_scan(ScanPriority::Interactive);
        let old = m.start_scan("t0".to_string());
        m.scan_timeout(old);
        assert_eq!(m.reason(), Some(StaleReason::ScanTimeout));

        // A subsequent scan starts after the timeout.
        m.request_full_scan(ScanPriority::Interactive);
        let newer = m.start_scan("t1".to_string());
        assert_eq!(m.state(), AssuranceState::Running);
        assert_ne!(old, newer, "each start_scan issues a fresh token");

        // The timed-out worker's late completion must not settle the new scan.
        let completion = m.complete_scan("t2".to_string(), old);
        assert_eq!(completion, ScanCompletion::Ignored);
        assert_eq!(
            m.state(),
            AssuranceState::Running,
            "newer scan must remain Running"
        );

        // The matching completion still settles Clean.
        let completion = m.complete_scan("t3".to_string(), newer);
        assert_eq!(completion, ScanCompletion::Clean);
        assert_eq!(m.state(), AssuranceState::Clean);
    }

    #[test]
    fn stale_timeout_token_is_a_noop() {
        let mut m = AssuranceMachine::new();
        m.request_full_scan(ScanPriority::Interactive);
        let first = m.start_scan("t0".to_string());
        let second = m.start_scan("t1".to_string()); // continuation / re-arm
        assert_ne!(first, second);

        // Timeout for the superseded segment must not mark the active scan stale.
        m.scan_timeout(first);
        assert_eq!(m.state(), AssuranceState::Running);

        m.scan_timeout(second);
        assert_eq!(m.state(), AssuranceState::Stale);
        assert_eq!(m.reason(), Some(StaleReason::ScanTimeout));
    }
}
