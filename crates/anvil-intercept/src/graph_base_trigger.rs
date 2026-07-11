//! GBASE-003 (ADR-105 §6/§7): proactive pre-production of the shared base graph,
//! driven by **directory-level ref watches** rather than a save.
//!
//! The daemon warms a worktree from a shared base keyed by its merge-base commit
//! (GBASE-001/002). That base is only useful if it already exists when a worktree
//! restarts, so this module pre-produces it **when the merge-base moves** — a
//! fetch that advances `origin/HEAD`, a branch update, a checkout. Git ref updates
//! are **rename-based** (git writes a temp then `rename`s it into place), so a
//! file-inode watch misses the swap; we watch the **directories** ref updates land
//! in (`IN_MOVED_TO | IN_CREATE`) and treat any such event as "the merge-base may
//! have moved".
//!
//! # Shape (why a state machine + an executor)
//!
//! The heart is [`TriggerCore`] — a **pure, clock-injected state machine**. It
//! takes ref events + `poll(now)` ticks and returns [`TriggerAction`]s; it owns no
//! threads, no sockets, no git, and no wall clock, so debounce, the restart cap,
//! and the ENOSPC degrade are all asserted deterministically (the GBASE-002
//! lesson: single-shot green under real time lies). [`GraphBaseTrigger`] is the
//! thin executor that performs those actions — spawns the subprocess, signals a
//! cancel, emits the health envelope — against injected seams
//! ([`ProductionSpawner`], [`Signaller`], [`BaseTriggerNotifier`]).
//!
//! # Design decisions (ADR-105 §7)
//!
//! - **The daemon stays git-free.** Production runs as a detached
//!   `anvil graph-base build --repo <root>` subprocess with the merge-base
//!   **omitted** — the subprocess resolves the sha itself (`graph_base` already
//!   resolves from the default branch when `--merge-base` is absent). The daemon
//!   never runs `git`, never parses, never links tree-sitter (ADR-061/064 upheld).
//! - **Single-flight lives in the child, not the daemon.** The daemon does **not**
//!   pre-claim. The subprocess performs its own `base_store::claim()` internally;
//!   contention is resolved there (it prints a `claimed-elsewhere` JSON outcome and
//!   exits cleanly). A daemon-side claim would be a redundant *double*-claim and
//!   would force the daemon to resolve the sha (git in the daemon) just to key the
//!   lock — exactly what we avoid. So the daemon's job is only **debounce + spawn +
//!   reap**; the child's claim is the one source of single-flight truth.
//! - **The "sha-lineage" is tracked git-free as a ref-trigger lineage.** Because
//!   the daemon never resolves the sha, it cannot key the restart cap on the actual
//!   merge-base sha. Instead a *lineage* is the chain of successive debounced
//!   ref-change triggers for one repo **between quiescent periods**: a gap of
//!   [`DEFAULT_QUIESCENCE`] with no ref events starts a fresh lineage and resets the
//!   restart counter. This is the daemon-git-free proxy for §7's "per sha-lineage"
//!   cap, and it is strictly conservative — it never *under*-counts churn.
//! - **The background pool never blocks.** On a debounced trigger the background
//!   thread only spawns + records; a dedicated reaper `std::thread` owns
//!   `child.wait()` and clears the in-flight slot on exit ([`spawn_reaper`]).
//!
//! # Failure posture
//!
//! Every path is non-fatal (base absent ⇒ cold scan serves). A spawn error is
//! logged and dropped; an exceeded restart cap serves cold, logs, and emits an
//! **ADR-090 worktree-scoped health envelope per registered worktree** of the repo
//! through the real fan-out ([`BaseTriggerNotifier`]); on `ENOSPC` (or any
//! add-watch failure) the repo's ref watches degrade to a disabled state and the
//! fallback is CLI-invocation check-and-request (see the module note below).
//!
//! # Multi-worktree model & budget
//!
//! State is keyed by a repo's **common gitdir**, so every worktree of a repo
//! shares one lineage/debounce/in-flight. [`GraphBaseTrigger::reconcile_roots`]
//! groups the registered roots by common gitdir and watches the **shared** ref
//! dirs once per repo plus **one HEAD dir per registered worktree** — so a
//! `git checkout` in *any* worktree (each rewrites only its own HEAD) is caught,
//! not just the first registrant's. The honest per-repo descriptor invariant is
//! `O(1) per registered workspace`: shared (≤ [`MAX_SHARED_REF_WATCHES_PER_REPO`])
//! plus one per worktree ([`ref_watch_budget`]).
//!
//! # Known gap — no unregister removal (rides GBASE-010)
//!
//! [`TriggerCore`] currently **never removes** a repo/worktree or its watches: a
//! worktree that unregisters leaves its watches and latched lineage state resident
//! until the daemon restarts. This is bounded (the registered set is capped, watch
//! counts are `O(worktrees)`), and correctness is unaffected (a stale watch only
//! risks a redundant, single-flighted, cold-serving build). Real removal —
//! dropping the inotify watch and pruning `RepoState` when
//! `registered_worktrees()` no longer lists a worktree — is deferred to the
//! GBASE-010 graduation gate, alongside its lock-contention measurement.
//!
//! # Wiring
//!
//! Activation is gated on the **same** condition the save-time path gates
//! persistence on — `ANVIL_PERSIST_GRAPH` affirmative (see [`trigger_enabled`]) —
//! because pre-production is meaningless when persistence is off (ADR-105 §7:
//! proactive *when the feature is live*, never auto-enabled). `run_foreground`
//! calls [`activate`] on Linux: it builds the trigger (`current_exe` spawner,
//! `SIGTERM` signaller, the real broadcaster notifier), seeds the currently
//! registered worktree roots, and spawns a **dedicated** ref-watch `std::thread`
//! ([`run_refwatch_loop`]) that reconciles roots every ~1 s (picking up **late**
//! registrations — the daemon's `notify` watcher lives in the un-linked
//! `anvil-kernel` crate and its membership hook is already owned by the save-time
//! driver supervisor, so the trigger learns roots by polling
//! [`crate::registry::SessionRegistry::registered_worktrees`] every ~10 ticks —
//! not at the full 10 Hz loop rate, to spare the registry's hot lock), drains
//! inotify events into the debounce every tick,
//! and ticks `poll`. Clean shutdown is via
//! [`TriggerActivation::shutdown_and_join`], called on every daemon exit path.
//! The check-and-request degrade fallback (what `anvil start`/`watch`/`status`
//! do after an `ENOSPC` degrade) remains a documented CLI-surface follow-up.
//! Non-Linux `cfg(unix)` (macOS) needs a kqueue/FSEvents backend — the inherited
//! ADR-105 §8 platform gap.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use crate::broadcaster::TelemetryBroadcaster;
use crate::telemetry::{NotificationEnvelope, TelemetryCorrelation, TelemetryEmitter};

/// The **shared** ref-watch descriptor budget per repo (ADR-105 §6): the common
/// `refs` dir, the `packed-refs` parent, and the primary `HEAD` dir. In the
/// standard git layout the packed-refs parent and the primary HEAD both live
/// directly in the common gitdir, so these dedup to **2** dirs; the cap is 3 to be
/// honest about the three conceptual ref surfaces even where a layout keeps them
/// distinct. These are counted **once per repo** regardless of how many worktrees
/// share it.
///
/// The **full** per-repo budget is `O(1) per registered workspace` (the council
/// design): the shared dirs (≤ this) plus **one** per-worktree HEAD-dir watch per
/// registered worktree — see [`ref_watch_budget`]. The resident daemon owns no
/// other inotify/descriptor budget to extend (the recursive `notify` watcher lives
/// in the separate `anvil-kernel` crate the daemon does not link), so this is the
/// single authoritative ref-watch accounting.
pub const MAX_SHARED_REF_WATCHES_PER_REPO: usize = 3;

/// The honest per-repo descriptor invariant (ADR-105 §6): shared dirs
/// (≤ [`MAX_SHARED_REF_WATCHES_PER_REPO`]) plus one HEAD-dir watch per registered
/// worktree — `O(1) per registered workspace`. A repo's live watch count must
/// never exceed this. (The main worktree's HEAD dir *is* the common gitdir, so it
/// dedups into the shared set — this is an upper bound, not an exact count.)
#[must_use]
pub fn ref_watch_budget(registered_worktrees: usize) -> usize {
    MAX_SHARED_REF_WATCHES_PER_REPO + registered_worktrees
}

/// Debounce window: ref-event bursts within this window coalesce into a single
/// production trigger (ADR-105 §6, ~500 ms). A fetch touches many refs in a burst;
/// one trigger per burst, not one per ref.
pub const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(500);

/// Quiescence window: a gap of this long with no ref events ends a lineage, so the
/// next event starts a fresh one and re-arms the restart cap (ADR-105 §7,
/// "re-arm on next quiescence").
pub const DEFAULT_QUIESCENCE: Duration = Duration::from_secs(5);

/// Cancel-and-restart cap **per sha-lineage** (ADR-105 §7, `N=3`). Three restarts
/// are permitted within a lineage; a fourth is refused — the daemon serves cold,
/// logs, and emits the ADR-090 health envelope until the next quiescence re-arms.
pub const MAX_RESTARTS_PER_LINEAGE: u32 = 3;

// ---------------------------------------------------------------------------
// Ref-path resolution (git-free; never shells out)
// ---------------------------------------------------------------------------

/// The directory watch plan for a repo and its worktrees (ADR-105 §6). Resolved
/// from the on-disk git layout **without a subprocess** — a linked worktree's
/// `.git` is a *file* pointing at its gitdir, and the shared refs live under the
/// **common** gitdir; we read `.git` and `<gitdir>/commondir` directly. The plan
/// separates the **shared** dirs (watched once per repo) from the **per-worktree**
/// HEAD dirs (one per registered worktree), which is the `O(1) per registered
/// workspace` budget shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoWatchPlan {
    /// The resolved **common** gitdir (shared across every worktree of the repo).
    pub common_dir: PathBuf,
    /// Shared ref dirs watched **once per repo**: `<common>/refs` and `<common>`
    /// (the packed-refs parent **and** primary HEAD dir collapse to the common
    /// dir). ≤ [`MAX_SHARED_REF_WATCHES_PER_REPO`].
    pub shared_dirs: Vec<PathBuf>,
    /// Per-worktree HEAD dirs (each worktree's gitdir), **one per registered
    /// worktree**. The main worktree's gitdir equals [`Self::common_dir`] (it
    /// dedups into the shared watch); a linked worktree contributes its distinct
    /// `worktrees/<name>` gitdir.
    pub worktree_head_dirs: Vec<PathBuf>,
}

/// Resolve a repo root's gitdir (ADR-105 §6). Handles the linked-worktree case
/// where `.git` is a **file** containing `gitdir: <path>`. Never shells out to
/// `git`.
///
/// # Errors
/// An `io::Error` if `.git` is absent, unreadable, or a malformed gitdir file.
pub fn resolve_git_dir(repo_root: &Path) -> io::Result<PathBuf> {
    let dot_git = repo_root.join(".git");
    let meta = std::fs::symlink_metadata(&dot_git)?;
    if meta.is_dir() {
        return Ok(dot_git);
    }
    // Linked worktree: `.git` is a file `gitdir: <abs-or-relative-path>`.
    let contents = std::fs::read_to_string(&dot_git)?;
    let line = contents
        .lines()
        .find_map(|l| l.trim().strip_prefix("gitdir:"))
        .map(str::trim)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "`.git` file has no `gitdir:` pointer",
            )
        })?;
    let git_dir = Path::new(line);
    let git_dir = if git_dir.is_absolute() {
        git_dir.to_path_buf()
    } else {
        normalise(&repo_root.join(git_dir))
    };
    Ok(git_dir)
}

/// Resolve the **common** gitdir from a (possibly per-worktree) gitdir (ADR-105
/// §6). A linked worktree's gitdir carries a `commondir` file pointing at the
/// shared dir (often `../..`); a main worktree has none, so the gitdir *is* the
/// common dir. Never shells out.
#[must_use]
pub fn resolve_common_dir(git_dir: &Path) -> PathBuf {
    match std::fs::read_to_string(git_dir.join("commondir")) {
        Ok(raw) => {
            let rel = raw.trim();
            let p = Path::new(rel);
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                normalise(&git_dir.join(p))
            }
        }
        Err(_) => git_dir.to_path_buf(),
    }
}

/// Lexically normalise a path (resolve `..`/`.` components) without touching the
/// filesystem — the ref dirs may not all exist yet, so `canonicalize` is wrong
/// here. Keeps the descriptor set stable and dedup-friendly.
fn normalise(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Resolve the ref-watch plan for a repo given the primary root and any
/// linked-worktree roots that share it (ADR-105 §6): the shared dirs (once per
/// repo) plus one HEAD dir per worktree. Used for inspection/tests; the live
/// reconcile resolves incrementally per registered root.
///
/// # Errors
/// An `io::Error` if the primary root's gitdir cannot be resolved.
pub fn resolve_repo_watch_plan(
    primary_root: &Path,
    linked_worktree_roots: &[PathBuf],
) -> io::Result<RepoWatchPlan> {
    let primary_git_dir = resolve_git_dir(primary_root)?;
    let common_dir = resolve_common_dir(&primary_git_dir);

    // Per-worktree HEAD dirs = each worktree's own gitdir (deduped, ≤1 per
    // worktree). The main worktree's gitdir equals the common dir; a linked
    // worktree contributes its distinct `worktrees/<name>` gitdir.
    let mut worktree_head_dirs: Vec<PathBuf> = vec![primary_git_dir];
    for root in linked_worktree_roots {
        if let Ok(gd) = resolve_git_dir(root)
            && !worktree_head_dirs.contains(&gd)
        {
            worktree_head_dirs.push(gd);
        }
    }

    Ok(RepoWatchPlan {
        shared_dirs: resolve_shared_ref_dirs(&common_dir),
        common_dir,
        worktree_head_dirs,
    })
}

/// The **shared** ref-watch dirs for a repo (once per repo, ADR-105 §6): the
/// common `refs` dir and the common gitdir (which holds `packed-refs` **and** the
/// primary `HEAD`). Deduplicated, ≤ [`MAX_SHARED_REF_WATCHES_PER_REPO`].
#[must_use]
pub fn resolve_shared_ref_dirs(common_dir: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![common_dir.join("refs")];
    if !dirs.contains(&common_dir.to_path_buf()) {
        dirs.push(common_dir.to_path_buf());
    }
    debug_assert!(dirs.len() <= MAX_SHARED_REF_WATCHES_PER_REPO);
    dirs
}

// ---------------------------------------------------------------------------
// Watch backend seam (ENOSPC degrade injection point)
// ---------------------------------------------------------------------------

/// Opaque handle for one directory watch. The value is backend-defined; callers
/// only compare/store it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WatchDescriptor(pub u64);

/// Failure adding a directory watch (ADR-105 §6). [`Self::NoSpace`] is the box's
/// known inotify-exhaustion condition (`ENOSPC` — the per-user
/// `max_user_watches`/`max_user_instances` limit) and is the sole trigger for the
/// degrade-to-disabled path; anything else is a plain I/O error.
#[derive(Debug)]
pub enum WatchAddError {
    /// `ENOSPC`: the inotify instance/user watch limit is exhausted.
    NoSpace,
    /// Any other add-watch failure.
    Other(io::Error),
}

impl std::fmt::Display for WatchAddError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSpace => f.write_str("inotify watch limit exhausted (ENOSPC)"),
            Self::Other(err) => write!(f, "watch add failed: {err}"),
        }
    }
}

impl std::error::Error for WatchAddError {}

/// The seam over the OS watch facility, so the ENOSPC-degrade and budget paths are
/// testable without touching real inotify (the box's exhaustion makes real
/// registration flaky). The production implementation is [`InotifyRefWatchBackend`]
/// (Linux); tests inject a counting/failing double.
pub trait RefWatchBackend {
    /// Add a directory-level watch (`IN_MOVED_TO | IN_CREATE`) on `dir`, tagged
    /// with the owning `repo` (its common gitdir) so a later event can be mapped
    /// back to the repo whose merge-base may have moved. Returns a descriptor
    /// counted against the budget, or [`WatchAddError`].
    fn add_dir_watch(&mut self, repo: &Path, dir: &Path) -> Result<WatchDescriptor, WatchAddError>;
}

// ---------------------------------------------------------------------------
// Pure state machine (debounce + lineage cap + degrade)
// ---------------------------------------------------------------------------

/// An action the [`TriggerCore`] state machine asks the executor to perform. The
/// core itself never spawns, signals, or emits — it returns these so every
/// decision is asserted without side effects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriggerAction {
    /// Spawn a base-production subprocess for `repo`. `spawn_id` is the core's
    /// generation token; the executor must call [`TriggerCore::on_child_spawned`]
    /// with it once the pid is known.
    Spawn { repo: PathBuf, spawn_id: u64 },
    /// Cancel an in-flight production child by sending it `SIGTERM` (ADR-105 §7
    /// cancel-and-restart). The reaper handles the actual reap.
    Terminate { pid: u32 },
    /// The restart cap was exceeded for the repo's lineage — emit the ADR-090
    /// worktree-scoped health envelope for **every** currently-registered worktree
    /// of the repo (serve cold). ADR-090 delivers a health envelope by worktree
    /// ownership, so a repo shared by N worktrees emits N envelopes — each worktree's
    /// subscriber learns its base is not being warmed.
    EmitCapExceeded { worktrees: Vec<PathBuf> },
}

/// In-flight production for a repo: the core's generation token plus the child pid
/// once the executor reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InFlight {
    spawn_id: u64,
    pid: Option<u32>,
}

/// Per-repo trigger state. Keyed (in [`TriggerCore::repos`]) by the repo's common
/// gitdir, so every worktree of a repo shares one lineage/debounce/in-flight.
#[derive(Debug, Clone, Default)]
struct RepoState {
    /// Every currently-registered worktree of this repo, updated on reconcile.
    /// The cap-exceeded envelope is emitted for each (ADR-090 delivers by worktree
    /// ownership), so the attribution is faithful to the *current* worktree set,
    /// not pinned to the first registrant.
    worktrees: Vec<PathBuf>,
    /// Directories already watched for this repo (shared + per-worktree HEAD
    /// dirs), the dedup marker the reconcile uses so a sibling worktree adds
    /// **exactly** its HEAD-dir watch and never re-adds a shared one.
    watched_dirs: std::collections::HashSet<PathBuf>,
    /// `true` once the ref watches degraded (ENOSPC). A degraded repo ignores ref
    /// events; the fallback is CLI check-and-request.
    degraded: bool,
    /// The debounce deadline: `Some` while a coalescing window is open.
    pending_deadline: Option<Instant>,
    /// When the most recent ref event landed — drives quiescence/lineage reset.
    last_event: Option<Instant>,
    /// Cancel-and-restart count within the current lineage (ADR-105 §7).
    lineage_restarts: u32,
    /// Latched once the cap is exceeded; cleared when a quiescence gap starts a
    /// new lineage. While latched, debounced triggers serve cold (no spawn).
    over_cap: bool,
    /// The in-flight production child, if any.
    in_flight: Option<InFlight>,
}

/// The pure, clock-injected trigger state machine (ADR-105 §6/§7). Drive it with
/// [`Self::on_ref_event`] + [`Self::poll`]; it returns [`TriggerAction`]s. It owns
/// no threads, no git, and no wall clock.
#[derive(Debug)]
pub struct TriggerCore {
    repos: HashMap<PathBuf, RepoState>,
    debounce: Duration,
    quiescence: Duration,
    max_restarts: u32,
    next_spawn_id: u64,
}

impl Default for TriggerCore {
    fn default() -> Self {
        Self::new()
    }
}

impl TriggerCore {
    #[must_use]
    pub fn new() -> Self {
        Self::with_timings(
            DEFAULT_DEBOUNCE,
            DEFAULT_QUIESCENCE,
            MAX_RESTARTS_PER_LINEAGE,
        )
    }

    /// Construct with explicit timings/cap — used by tests to drive the machine
    /// deterministically.
    #[must_use]
    pub fn with_timings(debounce: Duration, quiescence: Duration, max_restarts: u32) -> Self {
        Self {
            repos: HashMap::new(),
            debounce,
            quiescence,
            max_restarts,
            next_spawn_id: 0,
        }
    }

    /// Ensure a repo (keyed by its common gitdir) is tracked. Idempotent — keeps
    /// existing lineage/worktree state.
    pub fn ensure_repo(&mut self, repo: impl Into<PathBuf>) {
        self.repos.entry(repo.into()).or_default();
    }

    /// Register a repo scoped to a single `worktree` (convenience for tests + the
    /// single-worktree case): `ensure_repo` + `add_worktree`.
    pub fn register_repo(&mut self, repo: impl Into<PathBuf>, worktree: impl Into<PathBuf>) {
        let repo = repo.into();
        self.ensure_repo(repo.clone());
        self.add_worktree(&repo, worktree.into());
    }

    /// Record that `worktree` is a live registered worktree of `repo`. Returns
    /// `true` if it was newly added (the reconcile then watches its HEAD dir).
    pub fn add_worktree(&mut self, repo: &Path, worktree: PathBuf) -> bool {
        match self.repos.get_mut(repo) {
            Some(state) if !state.worktrees.contains(&worktree) => {
                state.worktrees.push(worktree);
                true
            }
            _ => false,
        }
    }

    /// The currently-registered worktrees of `repo` (for envelope scoping).
    #[must_use]
    pub fn worktrees_of(&self, repo: &Path) -> Vec<PathBuf> {
        self.repos
            .get(repo)
            .map(|s| s.worktrees.clone())
            .unwrap_or_default()
    }

    /// Mark `dir` as watched for `repo`, returning `true` if it was **newly**
    /// inserted (so the reconcile issues an `add_dir_watch` for it exactly once).
    /// A shared dir or an already-watched sibling HEAD dir returns `false`.
    pub fn mark_dir_watched(&mut self, repo: &Path, dir: PathBuf) -> bool {
        self.repos
            .get_mut(repo)
            .is_some_and(|state| state.watched_dirs.insert(dir))
    }

    /// The count of live directory watches for `repo` — asserted against
    /// [`ref_watch_budget`] (`shared ≤3 + 1 per registered worktree`).
    #[must_use]
    pub fn descriptor_count(&self, repo: &Path) -> usize {
        self.repos.get(repo).map_or(0, |s| s.watched_dirs.len())
    }

    /// The count of registered worktrees for `repo`.
    #[must_use]
    pub fn worktree_count(&self, repo: &Path) -> usize {
        self.repos.get(repo).map_or(0, |s| s.worktrees.len())
    }

    /// Whether a repo is already tracked. The reconcile path uses this as the
    /// "already watched" marker so it never re-adds inotify watches for a repo a
    /// prior reconcile already registered.
    #[must_use]
    pub fn contains_repo(&self, repo_root: &Path) -> bool {
        self.repos.contains_key(repo_root)
    }

    /// Mark a repo's ref watches degraded (ENOSPC). Returns `true` if this flipped
    /// the state (so the caller logs **once**, structured). A degraded repo ignores
    /// ref events; the fallback is CLI check-and-request.
    pub fn mark_degraded(&mut self, repo_root: &Path) -> bool {
        match self.repos.get_mut(repo_root) {
            Some(state) if !state.degraded => {
                state.degraded = true;
                state.pending_deadline = None;
                true
            }
            _ => false,
        }
    }

    /// Whether a repo's trigger is degraded.
    #[must_use]
    pub fn is_degraded(&self, repo_root: &Path) -> bool {
        self.repos.get(repo_root).is_some_and(|s| s.degraded)
    }

    /// Record a ref-change event for `repo_root` at `now`. Opens/extends the
    /// debounce window. A gap of `≥ quiescence` since the last event starts a fresh
    /// lineage (re-arming the restart cap). A degraded repo ignores the event.
    pub fn on_ref_event(&mut self, repo_root: &Path, now: Instant) {
        let (debounce, quiescence) = (self.debounce, self.quiescence);
        let Some(state) = self.repos.get_mut(repo_root) else {
            return;
        };
        if state.degraded {
            return;
        }
        // Quiescence gap ⇒ new lineage (ADR-105 §7 re-arm).
        if let Some(last) = state.last_event
            && now.saturating_duration_since(last) >= quiescence
        {
            state.lineage_restarts = 0;
            state.over_cap = false;
        }
        state.last_event = Some(now);
        state.pending_deadline = Some(now + debounce);
    }

    /// Advance time to `now`, firing any repo whose debounce window has closed.
    /// Returns the actions the executor must perform.
    #[must_use]
    pub fn poll(&mut self, now: Instant) -> Vec<TriggerAction> {
        let mut actions = Vec::new();
        let max_restarts = self.max_restarts;
        // Collect due repos first to avoid borrow contention while minting ids.
        let due: Vec<PathBuf> = self
            .repos
            .iter()
            .filter(|(_, s)| s.pending_deadline.is_some_and(|d| now >= d))
            .map(|(k, _)| k.clone())
            .collect();
        for repo in due {
            let spawn_id = self.next_spawn_id;
            let mut minted = false;
            if let Some(state) = self.repos.get_mut(&repo) {
                state.pending_deadline = None;
                match Self::fire(state, &repo, spawn_id, max_restarts) {
                    FireOutcome::Idle => {}
                    FireOutcome::Actions { acts, minted_id } => {
                        actions.extend(acts);
                        minted = minted_id;
                    }
                }
            }
            if minted {
                self.next_spawn_id += 1;
            }
        }
        actions
    }

    /// The fire decision for one due repo (ADR-105 §7). Pure w.r.t. `state`.
    fn fire(state: &mut RepoState, repo: &Path, spawn_id: u64, max_restarts: u32) -> FireOutcome {
        // Cap already latched for this lineage ⇒ serve cold until quiescence.
        if state.over_cap {
            return FireOutcome::Idle;
        }
        match state.in_flight {
            // No child running: a fresh production (initial, or after the previous
            // completed). Not a "restart" — it does not consume the cap.
            None => {
                state.in_flight = Some(InFlight {
                    spawn_id,
                    pid: None,
                });
                FireOutcome::Actions {
                    acts: vec![TriggerAction::Spawn {
                        repo: repo.to_path_buf(),
                        spawn_id,
                    }],
                    minted_id: true,
                }
            }
            // A child is in flight and a newer trigger arrived ⇒ cancel-and-restart.
            Some(current) => {
                if state.lineage_restarts >= max_restarts {
                    // Over cap: serve cold, emit the ADR-090 envelope, latch until
                    // the next quiescence re-arms. Cancel the churning child.
                    state.over_cap = true;
                    state.in_flight = None;
                    let mut acts = Vec::new();
                    if let Some(pid) = current.pid {
                        acts.push(TriggerAction::Terminate { pid });
                    }
                    acts.push(TriggerAction::EmitCapExceeded {
                        worktrees: state.worktrees.clone(),
                    });
                    FireOutcome::Actions {
                        acts,
                        minted_id: false,
                    }
                } else {
                    state.lineage_restarts += 1;
                    state.in_flight = Some(InFlight {
                        spawn_id,
                        pid: None,
                    });
                    let mut acts = Vec::new();
                    if let Some(pid) = current.pid {
                        acts.push(TriggerAction::Terminate { pid });
                    }
                    acts.push(TriggerAction::Spawn {
                        repo: repo.to_path_buf(),
                        spawn_id,
                    });
                    FireOutcome::Actions {
                        acts,
                        minted_id: true,
                    }
                }
            }
        }
    }

    /// Record the pid of a spawned child so a later cancel can signal it. Ignored
    /// if the repo already moved on to a newer generation.
    pub fn on_child_spawned(&mut self, repo_root: &Path, spawn_id: u64, pid: u32) {
        if let Some(state) = self.repos.get_mut(repo_root)
            && let Some(inflight) = state.in_flight.as_mut()
            && inflight.spawn_id == spawn_id
        {
            inflight.pid = Some(pid);
        }
    }

    /// Report that the child for `spawn_id` exited (the reaper calls this). Clears
    /// the in-flight slot **only** if it is still the current generation — a
    /// cancelled child's late exit must not clear its replacement.
    pub fn on_child_exited(&mut self, spawn_id: u64) {
        for state in self.repos.values_mut() {
            if state
                .in_flight
                .is_some_and(|inflight| inflight.spawn_id == spawn_id)
            {
                state.in_flight = None;
                return;
            }
        }
    }

    /// Test/inspection: restart count for a repo's current lineage.
    #[must_use]
    pub fn lineage_restarts(&self, repo_root: &Path) -> u32 {
        self.repos.get(repo_root).map_or(0, |s| s.lineage_restarts)
    }

    /// Test/inspection: whether a repo has an in-flight production child.
    #[must_use]
    pub fn has_in_flight(&self, repo_root: &Path) -> bool {
        self.repos
            .get(repo_root)
            .is_some_and(|s| s.in_flight.is_some())
    }
}

/// Internal result of [`TriggerCore::fire`].
enum FireOutcome {
    Idle,
    Actions {
        acts: Vec<TriggerAction>,
        minted_id: bool,
    },
}

// ---------------------------------------------------------------------------
// Executor seams: spawn, reap, signal, envelope
// ---------------------------------------------------------------------------

/// A production child the reaper can block on. `wait` consumes the handle and
/// blocks until the child exits (owned by the dedicated reaper thread, never the
/// background pool).
pub trait ReapableChild: Send {
    /// Block until the child exits, reaping it. Consumes the handle.
    fn wait(self: Box<Self>) -> ChildExit;
}

/// The outcome of reaping a production child. `code` is `None` if the child was
/// terminated by a signal (e.g. our cancel `SIGTERM`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChildExit {
    pub code: Option<i32>,
}

/// A spawned base-production child: its pid (for cancel signalling) plus the
/// reapable handle the reaper owns.
pub struct SpawnedChild {
    pub pid: u32,
    pub child: Box<dyn ReapableChild>,
}

/// Spawns the detached `anvil graph-base build` subprocess (ADR-105 §7). Injected
/// so tests drive the spawn/reap seam with a fake child.
pub trait ProductionSpawner: Send + Sync {
    /// Spawn a production child for `repo`. The merge-base is **omitted** — the
    /// subprocess resolves the sha and single-flight-claims internally.
    ///
    /// # Errors
    /// An `io::Error` if the child could not be spawned (non-fatal — the daemon
    /// serves cold).
    fn spawn(&self, repo: &Path) -> io::Result<SpawnedChild>;
}

/// Sends a cancel signal to an in-flight child (ADR-105 §7). Injected so tests
/// record cancels without signalling real pids.
pub trait Signaller: Send + Sync {
    /// Best-effort `SIGTERM` to `pid`. An already-exited child is success.
    fn terminate(&self, pid: u32);
}

/// Production [`Signaller`] over the crate's existing `SIGTERM` helper.
pub struct SystemSignaller;

impl Signaller for SystemSignaller {
    fn terminate(&self, pid: u32) {
        // Best-effort; an already-dead child (`ESRCH`) is already success in the
        // helper. A genuine failure is logged, never fatal.
        if let Err(err) = crate::send_sigterm(pid) {
            tracing::debug!(
                target: "anvil_intercept::graph_base_trigger",
                pid,
                error = %err,
                "SIGTERM to base-production child failed (already exited?)",
            );
        }
    }
}

/// Production [`ProductionSpawner`]: re-exec the daemon's own `current_exe()` as
/// `anvil graph-base build --repo <root>` (merge-base omitted — the child
/// resolves it).
///
/// # Assumption
/// The daemon process **is** the `anvil` CLI binary (production starts it via
/// `anvil intercept start --foreground`), so `current_exe()` names a binary that
/// understands the hidden `graph-base build` subcommand — the same re-exec
/// contract [`crate::save_time_driver::CurrentExeDriverFactory`] relies on.
pub struct CurrentExeSpawner;

impl ProductionSpawner for CurrentExeSpawner {
    fn spawn(&self, repo: &Path) -> io::Result<SpawnedChild> {
        use std::process::{Command, Stdio};
        let exe = std::env::current_exe()?;
        let child = Command::new(exe)
            .arg("graph-base")
            .arg("build")
            .arg("--repo")
            .arg(repo)
            .stdin(Stdio::null())
            // stdout (the child's one-line JSON summary) is DISCARDED, not read:
            // piping it would hand the daemon a pipe to drain, and the reaper
            // only consumes the exit status — the store outcome is observable
            // via the artefact itself. Inherit stderr so a producer error lands
            // in the daemon log.
            .stdout(Stdio::null())
            .spawn()?;
        let pid = child.id();
        Ok(SpawnedChild {
            pid,
            child: Box::new(StdReapableChild { child }),
        })
    }
}

/// A [`ReapableChild`] over a real `std::process::Child`.
struct StdReapableChild {
    child: std::process::Child,
}

impl ReapableChild for StdReapableChild {
    fn wait(mut self: Box<Self>) -> ChildExit {
        match self.child.wait() {
            Ok(status) => ChildExit {
                code: status.code(),
            },
            Err(_) => ChildExit { code: None },
        }
    }
}

/// Spawn the dedicated reaper thread for a production child (ADR-105 §7). The
/// thread owns `child.wait()` and clears the in-flight slot on exit; the
/// background pool thread that spawned the child **never blocks**.
pub fn spawn_reaper(core: Arc<Mutex<TriggerCore>>, child: Box<dyn ReapableChild>, spawn_id: u64) {
    let builder = std::thread::Builder::new().name("anvil-gbase-reaper".to_owned());
    // A spawn failure here is non-fatal: without a reaper the child still runs to
    // completion and is reaped by the OS on daemon exit; we just miss the in-flight
    // clear. Log and move on.
    let spawned = builder.spawn(move || {
        let exit = child.wait();
        core.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .on_child_exited(spawn_id);
        tracing::debug!(
            target: "anvil_intercept::graph_base_trigger",
            spawn_id,
            code = ?exit.code,
            "base-production child reaped",
        );
    });
    if let Err(err) = spawned {
        tracing::warn!(
            target: "anvil_intercept::graph_base_trigger",
            spawn_id,
            error = %err,
            "failed to spawn base-production reaper thread; in-flight slot will not auto-clear",
        );
    }
}

// ---------------------------------------------------------------------------
// ADR-090 health-envelope emission (through the real fan-out)
// ---------------------------------------------------------------------------

/// Emits the ADR-090 worktree-scoped health envelope when the restart cap is
/// exceeded (ADR-105 §7), through the **real** [`TelemetryBroadcaster`] fan-out —
/// mirrors [`crate::save_time::SaveTimeState`]'s persist-failure notifier so the
/// signal is an envelope object an opted-in subscriber receives, not a log line.
#[derive(Clone)]
pub struct BaseTriggerNotifier {
    broadcaster: Arc<TelemetryBroadcaster>,
    emitter: Arc<Mutex<TelemetryEmitter>>,
}

impl BaseTriggerNotifier {
    #[must_use]
    pub fn new(broadcaster: Arc<TelemetryBroadcaster>) -> Self {
        Self {
            broadcaster,
            emitter: Arc::new(Mutex::new(TelemetryEmitter::new())),
        }
    }

    /// Build + broadcast the restart-cap-exceeded health envelope for `worktree`.
    /// Returns the envelope so callers/tests can assert on the emitted object.
    pub fn notify_cap_exceeded(&self, worktree: &Path) -> NotificationEnvelope {
        let envelope = {
            let mut emitter = self.emitter.lock().unwrap_or_else(PoisonError::into_inner);
            emitter.base_pre_production_health_envelope(
                TelemetryCorrelation::default(),
                worktree,
                "base pre-production restart cap exceeded; serving cold until quiescence",
            )
        };
        let _ = self.broadcaster.broadcast(&envelope);
        envelope
    }
}

// ---------------------------------------------------------------------------
// Executor
// ---------------------------------------------------------------------------

/// The thin executor over [`TriggerCore`] (ADR-105 §7). Owns the shared core and
/// the injected seams; performs the [`TriggerAction`]s the core returns — spawn +
/// launch a reaper, signal a cancel, emit the health envelope. Cheap to share.
#[derive(Clone)]
pub struct GraphBaseTrigger {
    core: Arc<Mutex<TriggerCore>>,
    spawner: Arc<dyn ProductionSpawner>,
    signaller: Arc<dyn Signaller>,
    notifier: Option<BaseTriggerNotifier>,
}

impl GraphBaseTrigger {
    /// Construct with the production seams. `notifier` is `None` when no
    /// broadcaster is wired (the health envelope then degrades to a `tracing`
    /// line — the cold-serve behaviour is identical either way).
    #[must_use]
    pub fn new(
        core: Arc<Mutex<TriggerCore>>,
        spawner: Arc<dyn ProductionSpawner>,
        signaller: Arc<dyn Signaller>,
        notifier: Option<BaseTriggerNotifier>,
    ) -> Self {
        Self {
            core,
            spawner,
            signaller,
            notifier,
        }
    }

    fn lock_core(&self) -> std::sync::MutexGuard<'_, TriggerCore> {
        self.core.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Record a ref event (the OS event loop / a CLI request calls this).
    pub fn on_ref_event(&self, repo_root: &Path, now: Instant) {
        self.lock_core().on_ref_event(repo_root, now);
    }

    /// Advance time and execute any due actions. Runs on the background pool; it
    /// spawns + launches reapers but never blocks on a child.
    pub fn poll(&self, now: Instant) {
        let actions = self.lock_core().poll(now);
        self.apply(actions);
    }

    fn apply(&self, actions: Vec<TriggerAction>) {
        for action in actions {
            match action {
                TriggerAction::Terminate { pid } => self.signaller.terminate(pid),
                TriggerAction::EmitCapExceeded { worktrees } => self.emit_cap_exceeded(&worktrees),
                TriggerAction::Spawn { repo, spawn_id } => self.spawn(&repo, spawn_id),
            }
        }
    }

    fn spawn(&self, repo: &Path, spawn_id: u64) {
        match self.spawner.spawn(repo) {
            Ok(SpawnedChild { pid, child }) => {
                self.lock_core().on_child_spawned(repo, spawn_id, pid);
                spawn_reaper(Arc::clone(&self.core), child, spawn_id);
            }
            Err(err) => {
                // Non-fatal (ADR-105 §6): serve cold. Clear the in-flight slot the
                // core optimistically set so a later trigger can spawn again.
                tracing::warn!(
                    target: "anvil_intercept::graph_base_trigger",
                    repo = %repo.display(),
                    error = %err,
                    "failed to spawn base-production subprocess; serving cold",
                );
                self.lock_core().on_child_exited(spawn_id);
            }
        }
    }

    /// Emit the ADR-090 health envelope for **every** currently-registered
    /// worktree of the capped repo. ADR-090 delivers a worktree-scoped health
    /// signal by worktree ownership, so a repo shared by N worktrees emits N
    /// envelopes — each worktree's subscriber learns its base is served cold.
    fn emit_cap_exceeded(&self, worktrees: &[PathBuf]) {
        match &self.notifier {
            Some(notifier) => {
                for worktree in worktrees {
                    notifier.notify_cap_exceeded(worktree);
                }
            }
            None => {
                tracing::warn!(
                    target: "anvil_intercept::graph_base_trigger",
                    worktrees = worktrees.len(),
                    "base pre-production restart cap exceeded; serving cold (no broadcaster wired)",
                );
            }
        }
    }

    /// Reconcile the watched set against the current registered worktree `roots`
    /// (ADR-105 §6). Roots are **grouped by their resolved common gitdir**, so a
    /// repo shared by N worktrees is reconciled against its FULL current worktree
    /// set in one pass: the shared ref dirs are watched **once per repo** and each
    /// worktree's own HEAD dir is watched **once per worktree**. This is what makes
    /// a `git checkout` in worktree #2+ (which rewrites only *its* HEAD) visible —
    /// the earlier "first-worktree-only" shape missed it.
    ///
    /// Idempotent per dir (via the core's `watched_dirs` marker), so it is safe to
    /// call repeatedly — which is exactly how **late registration** is picked up: a
    /// worktree (new repo *or* a new sibling of a tracked repo) that registers after
    /// startup is watched on the next reconcile. `ENOSPC` (or any add failure)
    /// degrades that repo (logged once) — the fallback is CLI check-and-request.
    pub fn reconcile_roots(&self, backend: &mut dyn RefWatchBackend, roots: &[PathBuf]) {
        // Group the registered roots by their resolved common gitdir (the repo id).
        let mut by_repo: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
        for root in roots {
            let Ok(git_dir) = resolve_git_dir(root) else {
                continue;
            };
            let common = resolve_common_dir(&git_dir);
            by_repo.entry(common).or_default().push(root.clone());
        }

        for (repo, group_roots) in by_repo {
            // Ensure the repo is tracked; watch its SHARED dirs exactly once.
            let new_repo = {
                let mut core = self.lock_core();
                let new_repo = !core.contains_repo(&repo);
                core.ensure_repo(repo.clone());
                new_repo
            };
            if new_repo {
                for dir in resolve_shared_ref_dirs(&repo) {
                    if !self.watch_dir(backend, &repo, &dir) {
                        break; // degraded — stop adding watches for this repo
                    }
                }
            }
            // Each worktree: register it (envelope scoping) + watch its HEAD dir.
            for root in &group_roots {
                self.lock_core().add_worktree(&repo, root.clone());
                if self.repo_degraded(&repo) {
                    continue;
                }
                if let Ok(head_dir) = resolve_git_dir(root) {
                    self.watch_dir(backend, &repo, &head_dir);
                }
            }
        }
    }

    /// Add one directory watch for `repo` if not already watched. Returns `false`
    /// if the add failed (the repo is then degraded); `true` otherwise (including
    /// the already-watched no-op).
    fn watch_dir(&self, backend: &mut dyn RefWatchBackend, repo: &Path, dir: &Path) -> bool {
        // `mark_dir_watched` inserts + returns whether it was new, so a shared dir
        // or an already-watched sibling HEAD dir is a no-op (never re-added).
        let is_new = self.lock_core().mark_dir_watched(repo, dir.to_path_buf());
        if !is_new {
            return true;
        }
        if let Err(err) = backend.add_dir_watch(repo, dir) {
            if self.lock_core().mark_degraded(repo) {
                tracing::warn!(
                    target: "anvil_intercept::graph_base_trigger",
                    repo = %repo.display(),
                    error = %err,
                    "ref watches disabled for repo (degraded); \
                     falling back to CLI check-and-request",
                );
            }
            return false;
        }
        true
    }

    fn repo_degraded(&self, repo: &Path) -> bool {
        self.lock_core().is_degraded(repo)
    }

    /// The shared core, for the reaper and tests.
    #[must_use]
    pub fn core(&self) -> Arc<Mutex<TriggerCore>> {
        Arc::clone(&self.core)
    }
}

/// Build the daemon-side proactive trigger **if** the persistence gate is on
/// (ADR-105 §7) — the seam `run_foreground` calls. Returns `None` unless BOTH the
/// `ANVIL_PERSIST_GRAPH` gate is affirmative **and** `state_dir_available` (a
/// resolvable base store dir): this mirrors the save-time path exactly, which only
/// enables persistence when the flag is on *and* a state dir resolves — without a
/// store dir the produced base would have nowhere to land, so triggering builds
/// would be pointless churn. Split from [`activate`] so the gate + construction are
/// tested without real inotify.
#[must_use]
pub fn build_activated_trigger(
    persist_graph_env: Option<&str>,
    state_dir_available: bool,
    spawner: Arc<dyn ProductionSpawner>,
    signaller: Arc<dyn Signaller>,
    notifier: Option<BaseTriggerNotifier>,
) -> Option<GraphBaseTrigger> {
    if !trigger_enabled(persist_graph_env) || !state_dir_available {
        return None;
    }
    let core = Arc::new(Mutex::new(TriggerCore::new()));
    Some(GraphBaseTrigger::new(core, spawner, signaller, notifier))
}

// ---------------------------------------------------------------------------
// Activation gate (mirrors the save-time persistence gate)
// ---------------------------------------------------------------------------

/// Whether the proactive trigger should activate (ADR-105 §7). Mirrors the
/// **exact** gate the save-time path uses to enable persistence — an affirmative
/// `ANVIL_PERSIST_GRAPH` — because pre-production is meaningless when persistence
/// is off, and the council said pre-production should be proactive *only when the
/// feature is live* (never auto-enable when persistence is off).
#[must_use]
pub fn trigger_enabled(persist_graph_env: Option<&str>) -> bool {
    anvil_graph_cache::snapshot::persist_graph_enabled(persist_graph_env)
}

// ---------------------------------------------------------------------------
// Production OS backend + event loop (documented wiring seam)
// ---------------------------------------------------------------------------

/// Linux inotify backend for directory-level ref watches (`IN_MOVED_TO |
/// IN_CREATE`, ADR-105 §6). Git ref updates are rename-based, so directory watches
/// catch the rename-into-place a file-inode watch would miss. `ENOSPC` maps to
/// [`WatchAddError::NoSpace`] — the degrade trigger.
///
/// This is the real OS surface; it is compiled and wired-ready but not
/// unit-tested against live inotify here (the box's known watch-limit exhaustion
/// makes real registration flaky — the [`RefWatchBackend`] seam is exercised via a
/// fake instead, per the item's test guidance). Non-Linux `cfg(unix)` targets
/// (macOS) need a kqueue/FSEvents backend — a documented follow-up, consistent
/// with ADR-105 §8's inherited `cfg(unix)` platform gap.
#[cfg(target_os = "linux")]
pub struct InotifyRefWatchBackend {
    inotify: nix::sys::inotify::Inotify,
    /// Maps each live inotify watch descriptor back to the repo (common gitdir)
    /// whose merge-base may have moved, so a drained event names the right repo.
    wd_to_repo: HashMap<nix::sys::inotify::WatchDescriptor, PathBuf>,
    next: u64,
}

#[cfg(target_os = "linux")]
impl InotifyRefWatchBackend {
    /// Create a non-blocking inotify instance for ref watches.
    ///
    /// # Errors
    /// An `io::Error` if the inotify instance could not be created (e.g. the
    /// per-user instance limit is exhausted).
    pub fn new() -> io::Result<Self> {
        use nix::sys::inotify::{InitFlags, Inotify};
        let inotify = Inotify::init(InitFlags::IN_NONBLOCK | InitFlags::IN_CLOEXEC)
            .map_err(io::Error::from)?;
        Ok(Self {
            inotify,
            wd_to_repo: HashMap::new(),
            next: 0,
        })
    }

    /// Drain pending inotify events (non-blocking) and return the **deduplicated**
    /// set of repos that saw a ref-relevant event. The event loop forwards one
    /// [`GraphBaseTrigger::on_ref_event`] per returned repo; the debounce coalesces
    /// a burst into one trigger.
    #[must_use]
    pub fn drain_repo_events(&self) -> Vec<PathBuf> {
        let events = match self.inotify.read_events() {
            Ok(events) => events,
            // `EAGAIN`/`EWOULDBLOCK`: nothing pending on the non-blocking instance.
            Err(nix::errno::Errno::EAGAIN) => return Vec::new(),
            Err(err) => {
                // Anything else is unexpected on a healthy inotify fd; dropping
                // it silently would leave the trigger blind with no operator
                // signal.
                tracing::warn!(
                    target: "graph_base_trigger",
                    error = %err,
                    "inotify read_events failed; ref events may be missed this tick",
                );
                return Vec::new();
            }
        };
        let mut repos: Vec<PathBuf> = Vec::new();
        for event in events {
            if let Some(repo) = self.wd_to_repo.get(&event.wd)
                && !repos.contains(repo)
            {
                repos.push(repo.clone());
            }
        }
        repos
    }
}

#[cfg(target_os = "linux")]
impl RefWatchBackend for InotifyRefWatchBackend {
    fn add_dir_watch(&mut self, repo: &Path, dir: &Path) -> Result<WatchDescriptor, WatchAddError> {
        use nix::sys::inotify::AddWatchFlags;
        // Directory-level watch: catch the rename-into-place (IN_MOVED_TO) and a
        // fresh ref file (IN_CREATE) git writes when it updates a ref.
        let flags = AddWatchFlags::IN_MOVED_TO | AddWatchFlags::IN_CREATE;
        match self.inotify.add_watch(dir, flags) {
            Ok(wd) => {
                self.wd_to_repo.insert(wd, repo.to_path_buf());
                let id = self.next;
                self.next += 1;
                Ok(WatchDescriptor(id))
            }
            Err(nix::errno::Errno::ENOSPC) => Err(WatchAddError::NoSpace),
            Err(err) => Err(WatchAddError::Other(io::Error::from(err))),
        }
    }
}

/// A live proactive-trigger activation: the dedicated ref-watch thread plus its
/// shutdown flag (ADR-105 §6/§7). `run_foreground` holds this and calls
/// [`Self::shutdown_and_join`] on every exit path so the thread never outlives the
/// daemon — matching the abort-on-drop discipline the other daemon tasks use.
#[cfg(target_os = "linux")]
pub struct TriggerActivation {
    shutdown: Arc<std::sync::atomic::AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

#[cfg(target_os = "linux")]
impl TriggerActivation {
    /// Signal the ref-watch thread to stop and join it. Idempotent-safe on drop.
    pub fn shutdown_and_join(mut self) {
        self.shutdown
            .store(true, std::sync::atomic::Ordering::Release);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(target_os = "linux")]
impl Drop for TriggerActivation {
    fn drop(&mut self) {
        // Backstop: if the daemon dropped us without an explicit join (an early
        // `?`), still signal the thread so it exits promptly. We do not join in
        // Drop (it could block an async destructor); the thread is a short poll
        // loop that observes the flag within one tick and exits on its own.
        self.shutdown
            .store(true, std::sync::atomic::Ordering::Release);
    }
}

/// Activate the proactive trigger on the daemon (ADR-105 §6/§7) — the seam
/// `run_foreground` calls. Returns `None` when the persistence gate is off (see
/// [`build_activated_trigger`]) or the inotify instance cannot be created (both
/// non-fatal — the daemon serves cold). Otherwise it builds the real seams
/// (`current_exe` spawner, `SIGTERM` signaller, the real broadcaster notifier),
/// seeds the initial registered roots, and spawns a **dedicated** ref-watch
/// `std::thread` (never the background pool) that reconciles roots (picking up
/// late registrations), drains inotify events into the debounce, and ticks
/// `poll`. Clean shutdown is via [`TriggerActivation::shutdown_and_join`].
#[cfg(target_os = "linux")]
#[must_use]
pub fn activate(
    registry: Arc<crate::registry::SessionRegistry>,
    broadcaster: Option<Arc<TelemetryBroadcaster>>,
    persist_graph_env: Option<&str>,
) -> Option<TriggerActivation> {
    let notifier = broadcaster.map(BaseTriggerNotifier::new);
    // Mirror the save-time gate: require a resolvable base store dir, not just the
    // flag — the base store dir is `<graph-cache>/base`, so a resolvable
    // graph-cache dir means the produced base has somewhere to land.
    let state_dir_available = crate::snapshot_io::base_store::default_base_dir().is_some();
    let trigger = build_activated_trigger(
        persist_graph_env,
        state_dir_available,
        Arc::new(CurrentExeSpawner),
        Arc::new(SystemSignaller),
        notifier,
    )?;
    let mut backend = match InotifyRefWatchBackend::new() {
        Ok(backend) => backend,
        Err(err) => {
            tracing::warn!(
                target: "anvil_intercept::graph_base_trigger",
                error = %err,
                "could not create the inotify instance for ref watches; \
                 proactive base pre-production disabled (serving cold)",
            );
            return None;
        }
    };
    // Seed the watches for the roots already registered at startup.
    trigger.reconcile_roots(&mut backend, &registry.registered_worktrees());

    let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let thread_shutdown = Arc::clone(&shutdown);
    let handle = match std::thread::Builder::new()
        .name("anvil-gbase-refwatch".to_owned())
        .spawn(move || run_refwatch_loop(&trigger, &mut backend, &registry, &thread_shutdown))
    {
        Ok(handle) => handle,
        Err(err) => {
            // Without this log a spawn failure would make proactive
            // pre-production silently disappear (cold path serves) — hard to
            // diagnose.
            tracing::warn!(
                target: "graph_base_trigger",
                error = %err,
                "ref-watch thread spawn failed; base pre-production disabled (cold path serves)",
            );
            return None;
        }
    };
    Some(TriggerActivation {
        shutdown,
        handle: Some(handle),
    })
}

/// How many 100 ms loop ticks between registry reconciles. `registered_worktrees`
/// shares the registry's `Mutex<Inner>` with the hot `attribute_path` verdict
/// path, so we do **not** poll it at the full 10 Hz loop rate: reconciling every
/// 10th tick (~1 s) keeps late-registration latency at ≤1 s (fine — a base warms
/// on the order of seconds anyway) while touching the shared lock only ~1 Hz.
/// Inotify draining + `poll` still run every tick so debounced triggers stay
/// prompt. GBASE-010's graduation gate should measure this lock contention.
#[cfg(target_os = "linux")]
const RECONCILE_EVERY_N_TICKS: u64 = 10;

/// The dedicated ref-watch loop (ADR-105 §6). Runs on its own `std::thread` — the
/// blocking-free poll never touches the daemon's single-thread tokio runtime or
/// the background pool. Each tick: drain inotify events into the debounce and tick
/// `poll` so debounced triggers fire; every [`RECONCILE_EVERY_N_TICKS`] ticks also
/// reconcile roots (late-registration pickup) off the shared registry lock. Exits
/// within one tick of the shutdown flag.
#[cfg(target_os = "linux")]
fn run_refwatch_loop(
    trigger: &GraphBaseTrigger,
    backend: &mut InotifyRefWatchBackend,
    registry: &crate::registry::SessionRegistry,
    shutdown: &std::sync::atomic::AtomicBool,
) {
    // 100 ms is well inside the 500 ms debounce, so a burst still coalesces while
    // a debounced trigger fires within ~one tick of its deadline.
    let tick = Duration::from_millis(100);
    let mut ticks: u64 = 0;
    while !shutdown.load(std::sync::atomic::Ordering::Acquire) {
        // Reconcile the registry only every Nth tick — see RECONCILE_EVERY_N_TICKS.
        // The first iteration (ticks == 0) reconciles immediately so a worktree that
        // registered between `activate`'s seed and the loop start is not missed.
        if ticks.is_multiple_of(RECONCILE_EVERY_N_TICKS) {
            trigger.reconcile_roots(backend, &registry.registered_worktrees());
        }
        for repo in backend.drain_repo_events() {
            trigger.on_ref_event(&repo, Instant::now());
        }
        trigger.poll(Instant::now());
        ticks = ticks.wrapping_add(1);
        std::thread::sleep(tick);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn t0() -> Instant {
        Instant::now()
    }

    // ---- (a) ref-rename detection fires a trigger ----

    #[test]
    fn ref_event_then_debounce_fires_a_single_spawn() {
        // A ref rename-into-place (modelled as one ref event) in a watched dir
        // debounces and drives exactly one production spawn.
        let mut core = TriggerCore::with_timings(
            Duration::from_millis(500),
            Duration::from_secs(5),
            MAX_RESTARTS_PER_LINEAGE,
        );
        let repo = PathBuf::from("/repo");
        core.register_repo(&repo, "/repo");

        let t = t0();
        core.on_ref_event(&repo, t);
        // Before the window closes: nothing.
        assert!(
            core.poll(t + Duration::from_millis(499)).is_empty(),
            "no spawn before the debounce window closes",
        );
        // After the window: exactly one Spawn.
        let actions = core.poll(t + Duration::from_millis(500));
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], TriggerAction::Spawn { repo: r, .. } if r == &repo));
    }

    // ---- (b) debounce coalescing ----

    #[test]
    fn burst_of_ref_events_coalesces_into_one_trigger() {
        let mut core = TriggerCore::with_timings(
            Duration::from_millis(500),
            Duration::from_secs(5),
            MAX_RESTARTS_PER_LINEAGE,
        );
        let repo = PathBuf::from("/repo");
        core.register_repo(&repo, "/repo");

        let t = t0();
        // Ten rapid events inside the window (each extends the deadline).
        for i in 0..10 {
            core.on_ref_event(&repo, t + Duration::from_millis(i * 10));
            assert!(
                core.poll(t + Duration::from_millis(i * 10)).is_empty(),
                "no flush mid-burst",
            );
        }
        // Last event at t+90ms ⇒ deadline t+590ms. One trigger, not ten.
        assert!(core.poll(t + Duration::from_millis(589)).is_empty());
        let actions = core.poll(t + Duration::from_millis(590));
        assert_eq!(actions.len(), 1, "the whole burst collapses to one spawn");
        assert!(matches!(actions[0], TriggerAction::Spawn { .. }));
    }

    // ---- (c) restart cap + re-arm after quiescence ----

    /// Drive a debounced trigger to completion, filling the child pid so a
    /// subsequent preempt can Terminate it. Returns the spawn id.
    fn drive_spawn(core: &mut TriggerCore, repo: &Path, at: Instant, pid: u32) -> u64 {
        core.on_ref_event(repo, at);
        let actions = core.poll(at + Duration::from_millis(500));
        let Some(spawn_id) = actions.iter().find_map(|a| match a {
            TriggerAction::Spawn { spawn_id, .. } => Some(*spawn_id),
            _ => None,
        }) else {
            panic!("expected a Spawn action, got {actions:?}");
        };
        core.on_child_spawned(repo, spawn_id, pid);
        spawn_id
    }

    #[test]
    fn fourth_restart_in_a_lineage_serves_cold_and_emits_envelope() {
        let mut core = TriggerCore::with_timings(
            Duration::from_millis(500),
            Duration::from_secs(5),
            MAX_RESTARTS_PER_LINEAGE, // = 3
        );
        let repo = PathBuf::from("/repo");
        // TWO worktrees share this repo — the cap-exceeded envelope must cover BOTH
        // (ADR-090 delivers by worktree ownership), not just the first registrant.
        core.register_repo(&repo, "/wt/a");
        core.add_worktree(&repo, PathBuf::from("/wt/b"));

        let mut t = t0();
        // Initial spawn (not a restart).
        drive_spawn(&mut core, &repo, t, 100);
        assert_eq!(core.lineage_restarts(&repo), 0);
        assert!(core.has_in_flight(&repo));

        // Restarts 1..=3: each preempts the in-flight child and respawns. Keep the
        // events inside one lineage (gap < quiescence).
        for restart in 1..=3u32 {
            t += Duration::from_millis(600);
            core.on_ref_event(&repo, t);
            let actions = core.poll(t + Duration::from_millis(500));
            assert!(
                actions
                    .iter()
                    .any(|a| matches!(a, TriggerAction::Terminate { .. })),
                "restart {restart} cancels the in-flight child",
            );
            let spawn_id = actions
                .iter()
                .find_map(|a| match a {
                    TriggerAction::Spawn { spawn_id, .. } => Some(*spawn_id),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("restart {restart} must respawn"));
            core.on_child_spawned(&repo, spawn_id, 200 + restart);
            assert_eq!(core.lineage_restarts(&repo), restart);
        }

        // The 4th restart is refused: no Spawn, a Terminate of the churning child,
        // and the ADR-090 EmitCapExceeded carrying BOTH registered worktrees.
        t += Duration::from_millis(600);
        core.on_ref_event(&repo, t);
        let actions = core.poll(t + Duration::from_millis(500));
        let cap_worktrees = actions
            .iter()
            .find_map(|a| match a {
                TriggerAction::EmitCapExceeded { worktrees } => Some(worktrees.clone()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("the 4th restart emits the health envelope: {actions:?}"));
        assert_eq!(
            cap_worktrees,
            vec![PathBuf::from("/wt/a"), PathBuf::from("/wt/b")],
            "the cap-exceeded envelope must be scoped to every registered worktree",
        );
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, TriggerAction::Spawn { .. })),
            "the 4th restart must NOT spawn: {actions:?}",
        );
        assert!(!core.has_in_flight(&repo), "cold: no in-flight child");

        // Further triggers within the same lineage stay cold (no spawn).
        t += Duration::from_millis(600);
        core.on_ref_event(&repo, t);
        assert!(
            core.poll(t + Duration::from_millis(500)).is_empty(),
            "still cold within the capped lineage",
        );

        // Re-arm: a quiescence gap starts a fresh lineage; the next trigger spawns.
        t += Duration::from_secs(6); // > quiescence
        core.on_ref_event(&repo, t);
        let actions = core.poll(t + Duration::from_millis(500));
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, TriggerAction::Spawn { .. })),
            "a quiescence gap re-arms the lineage and the next trigger spawns: {actions:?}",
        );
        assert_eq!(
            core.lineage_restarts(&repo),
            0,
            "fresh lineage resets the count"
        );
    }

    // ---- (d) ENOSPC degrade ----

    /// A backend that fails the Nth add with `ENOSPC`.
    struct FailingBackend {
        adds: usize,
        fail_at: usize,
    }
    impl RefWatchBackend for FailingBackend {
        fn add_dir_watch(
            &mut self,
            _repo: &Path,
            _dir: &Path,
        ) -> Result<WatchDescriptor, WatchAddError> {
            self.adds += 1;
            if self.adds >= self.fail_at {
                Err(WatchAddError::NoSpace)
            } else {
                Ok(WatchDescriptor(self.adds as u64))
            }
        }
    }

    /// Build a real git-worktree layout: a main worktree at `<root>/main` (`.git`
    /// is a dir with `refs/` + HEAD + packed-refs) plus one linked worktree per
    /// `linked_names` (`.git` is a FILE pointing at `<common>/worktrees/<name>`,
    /// each with a `commondir` back-pointer). Returns `(main, common_dir, linked)`.
    fn make_repo_fixture(root: &Path, linked_names: &[&str]) -> (PathBuf, PathBuf, Vec<PathBuf>) {
        let main = root.join("main");
        let common = main.join(".git");
        std::fs::create_dir_all(common.join("refs")).unwrap();
        std::fs::write(common.join("HEAD"), b"ref: refs/heads/main\n").unwrap();
        std::fs::write(common.join("packed-refs"), b"").unwrap();
        std::fs::create_dir_all(&main).unwrap();
        let mut linked = Vec::new();
        for name in linked_names {
            let gitdir = common.join("worktrees").join(name);
            std::fs::create_dir_all(&gitdir).unwrap();
            std::fs::write(gitdir.join("HEAD"), b"ref: refs/heads/feature\n").unwrap();
            std::fs::write(gitdir.join("commondir"), b"../..\n").unwrap();
            let wt = root.join(name);
            std::fs::create_dir_all(&wt).unwrap();
            std::fs::write(wt.join(".git"), format!("gitdir: {}\n", gitdir.display())).unwrap();
            linked.push(wt);
        }
        (main, common, linked)
    }

    fn enabled_trigger(
        notifier: Option<BaseTriggerNotifier>,
    ) -> (GraphBaseTrigger, Arc<RecordingSeams>) {
        let s = seams();
        let trigger = build_activated_trigger(
            Some("1"),
            true,
            Arc::clone(&s) as Arc<dyn ProductionSpawner>,
            Arc::clone(&s) as Arc<dyn Signaller>,
            notifier,
        )
        .expect("gate on + state dir ⇒ trigger");
        (trigger, s)
    }

    #[test]
    fn enospc_degrades_the_repo_and_ignores_events_no_panic() {
        // reconcile surfaces ENOSPC via the backend; the repo degrades (logged
        // once) and then ignores ref events (fallback = CLI check-and-request).
        let tmp = tempfile::tempdir().unwrap();
        let (main, common, _linked) = make_repo_fixture(tmp.path(), &[]);
        let (trigger, s) = enabled_trigger(None);

        // The FIRST add (the shared refs dir) fails ENOSPC ⇒ the repo degrades.
        let mut backend = FailingBackend {
            adds: 0,
            fail_at: 1,
        };
        trigger.reconcile_roots(&mut backend, std::slice::from_ref(&main));
        assert!(
            trigger.core().lock().unwrap().is_degraded(&common),
            "ENOSPC degrades the repo",
        );

        // A degraded repo ignores ref events — no spawn.
        let t = t0();
        trigger.on_ref_event(&common, t);
        trigger.poll(t + Duration::from_secs(1));
        assert_eq!(
            s.spawns.load(Ordering::SeqCst),
            0,
            "a degraded trigger produces no spawn (CLI check-and-request fallback)",
        );
    }

    // ---- (e) descriptor budget: O(1) per registered workspace ----

    #[test]
    fn ref_watch_budget_is_o1_per_registered_workspace() {
        // A main worktree + two linked worktrees, all sharing one common dir. The
        // live watch count must be ≤ shared(≤3) + 1 per registered worktree, and
        // EVERY worktree's HEAD dir must be watched (the multi-worktree fix).
        let tmp = tempfile::tempdir().unwrap();
        let (main, common, linked) = make_repo_fixture(tmp.path(), &["wt-a", "wt-b"]);

        // Plan sanity: shared ≤ cap.
        let plan = resolve_repo_watch_plan(&main, &linked).unwrap();
        assert!(plan.shared_dirs.len() <= MAX_SHARED_REF_WATCHES_PER_REPO);
        assert_eq!(plan.common_dir, common);

        let (trigger, _s) = enabled_trigger(None);
        let mut backend = RecordingBackend {
            watches: Vec::new(),
        };
        let mut roots = vec![main.clone()];
        roots.extend(linked.clone());
        trigger.reconcile_roots(&mut backend, &roots);

        let core = trigger.core();
        let core = core.lock().unwrap();
        let worktrees = core.worktree_count(&common);
        assert_eq!(worktrees, 3, "main + 2 linked registered");
        assert!(
            core.descriptor_count(&common) <= ref_watch_budget(worktrees),
            "descriptor count {} exceeds O(1)-per-workspace budget {}",
            core.descriptor_count(&common),
            ref_watch_budget(worktrees),
        );
        // Shared entries watched once.
        assert!(
            backend
                .watches
                .iter()
                .any(|(_, d)| d == &common.join("refs"))
        );
        assert!(backend.watches.iter().any(|(_, d)| d == &common));
        // EVERY linked worktree's own HEAD dir is watched, tagged to the repo —
        // the sibling-checkout case the first-worktree-only shape missed.
        for l in &linked {
            let head = resolve_git_dir(l).unwrap();
            assert!(
                backend
                    .watches
                    .iter()
                    .any(|(r, d)| r == &common && d == &head),
                "sibling HEAD dir {head:?} must be watched: {:?}",
                backend.watches,
            );
        }
    }

    #[test]
    fn linked_worktree_dot_git_file_resolves_to_common_dir() {
        // A linked worktree's `.git` FILE resolves to its per-worktree gitdir, and
        // `commondir` folds it back to the shared common dir.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let common = root.join("repo").join(".git");
        std::fs::create_dir_all(common.join("worktrees").join("wt")).unwrap();
        let gitdir = common.join("worktrees").join("wt");
        std::fs::write(gitdir.join("commondir"), b"../..\n").unwrap();
        let wt = root.join("wt");
        std::fs::create_dir_all(&wt).unwrap();
        std::fs::write(wt.join(".git"), format!("gitdir: {}\n", gitdir.display())).unwrap();

        let resolved = resolve_git_dir(&wt).unwrap();
        assert_eq!(
            resolved, gitdir,
            "linked `.git` file resolves to the gitdir"
        );
        assert_eq!(
            resolve_common_dir(&resolved),
            common,
            "commondir folds the per-worktree gitdir back to the shared common dir",
        );
    }

    // ---- (f) ADR-090 envelope emission through the fan-out ----

    #[test]
    fn cap_exceeded_emits_worktree_scoped_envelope_through_broadcaster() {
        use crate::broadcaster::TelemetryBroadcaster;
        use crate::fanout::{Fanout, OwnershipResolver, SubscriberId};

        // A resolver that authorises a subscriber for exactly one worktree.
        struct WtResolver {
            owner: SubscriberId,
            worktree: String,
        }
        impl OwnershipResolver for WtResolver {
            fn is_authorised(&self, _sub: &SubscriberId, _sess: &str) -> bool {
                false
            }
            fn is_authorised_for_worktree(&self, sub: &SubscriberId, wt: &str) -> bool {
                sub == &self.owner && wt == self.worktree
            }
        }

        let worktree = "/worktrees/W";
        let owner = SubscriberId::new("owner");
        let resolver = WtResolver {
            owner: owner.clone(),
            worktree: worktree.to_owned(),
        };
        let fanout = Arc::new(Fanout::new(Box::new(resolver)));
        let broadcaster = Arc::new(TelemetryBroadcaster::new(fanout));
        let mut rx = broadcaster.register(owner, None);

        let notifier = BaseTriggerNotifier::new(Arc::clone(&broadcaster));
        let envelope = notifier.notify_cap_exceeded(Path::new(worktree));

        // The emitted object is a daemon-health, worktree-scoped envelope (ADR-090)
        // — not merely a log line.
        assert!(
            envelope.daemon_worktree_health,
            "restart-cap envelope must be flagged daemon-health (ADR-090)",
        );
        assert_eq!(
            envelope.correlation.originating_session_id, None,
            "a daemon-health envelope carries no session",
        );
        assert_eq!(
            envelope.correlation.worktree.as_deref(),
            Some(worktree),
            "the envelope is scoped to the affected worktree",
        );
        // It routed through the real fan-out to the worktree-owning subscriber.
        let frame = rx
            .try_recv()
            .expect("the owning subscriber receives the envelope");
        assert!(frame.contains("telemetry.event"));
    }

    // ---- (g) reaper: child exit reaps off the pool thread ----

    /// A fake child whose `wait` blocks until released, so the test can prove the
    /// reaper runs on its own thread (not the caller's).
    struct BlockingChild {
        release: Arc<std::sync::Mutex<bool>>,
        cond: Arc<std::sync::Condvar>,
    }
    impl ReapableChild for BlockingChild {
        fn wait(self: Box<Self>) -> ChildExit {
            let mut released = self.release.lock().unwrap();
            while !*released {
                released = self.cond.wait(released).unwrap();
            }
            ChildExit { code: Some(0) }
        }
    }

    #[test]
    fn reaper_reaps_child_exit_without_blocking_the_caller() {
        let core = Arc::new(Mutex::new(TriggerCore::new()));
        let repo = PathBuf::from("/repo");
        {
            let mut c = core.lock().unwrap();
            c.register_repo(&repo, "/repo");
            // Simulate the core having spawned generation 7 with a known pid.
            c.on_ref_event(&repo, t0());
            let actions = c.poll(t0() + Duration::from_secs(1));
            let spawn_id = actions
                .iter()
                .find_map(|a| match a {
                    TriggerAction::Spawn { spawn_id, .. } => Some(*spawn_id),
                    _ => None,
                })
                .expect("spawn");
            c.on_child_spawned(&repo, spawn_id, 4242);
            assert!(c.has_in_flight(&repo));
            drop(c);

            let release = Arc::new(std::sync::Mutex::new(false));
            let cond = Arc::new(std::sync::Condvar::new());
            let child = Box::new(BlockingChild {
                release: Arc::clone(&release),
                cond: Arc::clone(&cond),
            });
            spawn_reaper(Arc::clone(&core), child, spawn_id);

            // The caller did NOT block: the in-flight slot is still set because the
            // child has not been released to exit yet.
            assert!(
                core.lock().unwrap().has_in_flight(&repo),
                "reaper must run on its own thread; caller is not blocked on wait()",
            );

            // Release the child; the reaper wakes, reaps, and clears the slot.
            *release.lock().unwrap() = true;
            cond.notify_all();

            // Poll the core until the reaper has cleared the slot (bounded).
            let deadline = Instant::now() + Duration::from_secs(5);
            while core.lock().unwrap().has_in_flight(&repo) {
                assert!(Instant::now() < deadline, "reaper failed to clear the slot");
                std::thread::yield_now();
            }
        }
    }

    // ---- executor wiring: spawn + reap + cancel through fakes ----

    /// A recording spawner + signaller pair, backed by an immediately-exiting fake
    /// child, to exercise the executor end-to-end without a real subprocess.
    struct RecordingSeams {
        spawns: AtomicUsize,
        terminates: Arc<std::sync::Mutex<Vec<u32>>>,
    }

    struct InstantChild;
    impl ReapableChild for InstantChild {
        fn wait(self: Box<Self>) -> ChildExit {
            ChildExit { code: Some(0) }
        }
    }

    impl ProductionSpawner for RecordingSeams {
        fn spawn(&self, _repo: &Path) -> io::Result<SpawnedChild> {
            let n = self.spawns.fetch_add(1, Ordering::SeqCst);
            Ok(SpawnedChild {
                pid: 9000 + u32::try_from(n).unwrap_or(0),
                child: Box::new(InstantChild),
            })
        }
    }
    impl Signaller for RecordingSeams {
        fn terminate(&self, pid: u32) {
            self.terminates.lock().unwrap().push(pid);
        }
    }

    #[test]
    fn executor_spawns_and_records_pid_via_core() {
        let core = Arc::new(Mutex::new(TriggerCore::with_timings(
            Duration::from_millis(500),
            Duration::from_secs(5),
            MAX_RESTARTS_PER_LINEAGE,
        )));
        core.lock().unwrap().register_repo("/repo", "/repo");
        let seams = Arc::new(RecordingSeams {
            spawns: AtomicUsize::new(0),
            terminates: Arc::new(std::sync::Mutex::new(Vec::new())),
        });
        let trigger = GraphBaseTrigger::new(
            Arc::clone(&core),
            Arc::clone(&seams) as Arc<dyn ProductionSpawner>,
            Arc::clone(&seams) as Arc<dyn Signaller>,
            None,
        );

        let t = t0();
        let repo = Path::new("/repo");
        trigger.on_ref_event(repo, t);
        trigger.poll(t + Duration::from_millis(500));
        assert_eq!(seams.spawns.load(Ordering::SeqCst), 1, "one spawn executed");
    }

    // ---- daemon activation seam (the function run_foreground calls) ----

    /// A backend that records every `(repo, dir)` watch it is asked to add.
    struct RecordingBackend {
        watches: Vec<(PathBuf, PathBuf)>,
    }
    impl RefWatchBackend for RecordingBackend {
        fn add_dir_watch(
            &mut self,
            repo: &Path,
            dir: &Path,
        ) -> Result<WatchDescriptor, WatchAddError> {
            self.watches.push((repo.to_path_buf(), dir.to_path_buf()));
            Ok(WatchDescriptor(self.watches.len() as u64))
        }
    }

    fn seams() -> Arc<RecordingSeams> {
        Arc::new(RecordingSeams {
            spawns: AtomicUsize::new(0),
            terminates: Arc::new(std::sync::Mutex::new(Vec::new())),
        })
    }

    #[test]
    fn build_activated_trigger_returns_none_when_gate_off() {
        // The daemon-side seam constructs NOTHING when persistence (the gate) is
        // off — pre-production is inert unless the feature is live (ADR-105 §7).
        let s = seams();
        // Gate flag off (with a state dir available) ⇒ no trigger.
        for env in [None, Some("0"), Some("off"), Some("garbage")] {
            assert!(
                build_activated_trigger(
                    env,
                    true,
                    Arc::clone(&s) as Arc<dyn ProductionSpawner>,
                    Arc::clone(&s) as Arc<dyn Signaller>,
                    None,
                )
                .is_none(),
                "gate {env:?} must not construct a trigger",
            );
        }
        // Gate flag ON but NO resolvable state dir ⇒ still no trigger (mirrors the
        // save-time persistence gate — a base needs somewhere to land).
        assert!(
            build_activated_trigger(
                Some("1"),
                false,
                Arc::clone(&s) as Arc<dyn ProductionSpawner>,
                Arc::clone(&s) as Arc<dyn Signaller>,
                None,
            )
            .is_none(),
            "gate on but no state dir must not construct a trigger",
        );
    }

    #[test]
    fn activated_trigger_registers_roots_and_events_flow_to_core() {
        // Gate ON ⇒ the seam builds a trigger. Reconciling a registered root
        // registers its repo + adds ref-dir watches, and a ref event on that repo
        // flows through the debounce to a spawn — proving the wired path fires.
        let (trigger, s) = enabled_trigger(None);

        // A real main-worktree git layout so `resolve_git_dir` succeeds.
        let tmp = tempfile::tempdir().unwrap();
        let (wt, common, _linked) = make_repo_fixture(tmp.path(), &[]);

        let mut backend = RecordingBackend {
            watches: Vec::new(),
        };
        // First reconcile registers the repo + adds watches.
        trigger.reconcile_roots(&mut backend, std::slice::from_ref(&wt));
        assert!(
            trigger.core().lock().unwrap().contains_repo(&common),
            "reconcile registers the repo (keyed by its common gitdir)",
        );
        assert!(
            !backend.watches.is_empty() && backend.watches.len() <= ref_watch_budget(1),
            "reconcile adds shared + one per-worktree watch, within budget: {:?}",
            backend.watches,
        );
        assert!(
            backend.watches.iter().all(|(repo, _)| repo == &common),
            "every watch is tagged with the repo's common gitdir",
        );

        // Idempotent: a second reconcile (a later tick) adds no new watches.
        let before = backend.watches.len();
        trigger.reconcile_roots(&mut backend, std::slice::from_ref(&wt));
        assert_eq!(
            backend.watches.len(),
            before,
            "already-tracked repo skipped"
        );

        // A ref event on the repo flows through the debounce to a spawn.
        let t = t0();
        trigger.on_ref_event(&common, t);
        trigger.poll(t + Duration::from_millis(500));
        assert_eq!(
            s.spawns.load(Ordering::SeqCst),
            1,
            "a ref event on a registered repo drives one production spawn",
        );
    }

    #[test]
    fn sibling_worktree_head_is_watched_and_a_sibling_event_triggers() {
        // BLOCKING-fix coverage: a repo with a main + a linked worktree must watch
        // BOTH HEAD dirs. A ref update landing in the SIBLING's HEAD dir (which the
        // backend maps to the shared repo) drives a production trigger — the case
        // the first-worktree-only reconcile silently dropped.
        let tmp = tempfile::tempdir().unwrap();
        let (main, common, linked) = make_repo_fixture(tmp.path(), &["wt-a"]);
        let sibling = linked[0].clone();
        let sibling_head = resolve_git_dir(&sibling).unwrap();
        assert_ne!(
            sibling_head, common,
            "the sibling's HEAD dir is its own gitdir, distinct from the common dir",
        );

        let (trigger, s) = enabled_trigger(None);
        let mut backend = RecordingBackend {
            watches: Vec::new(),
        };
        trigger.reconcile_roots(&mut backend, &[main.clone(), sibling.clone()]);

        // The sibling's own HEAD dir is watched, tagged to the shared repo.
        assert!(
            backend
                .watches
                .iter()
                .any(|(r, d)| r == &common && d == &sibling_head),
            "the sibling worktree's HEAD dir must be watched: {:?}",
            backend.watches,
        );
        assert_eq!(
            trigger.core().lock().unwrap().worktree_count(&common),
            2,
            "both worktrees registered under the one repo",
        );

        // A rename-into-place in the SIBLING's HEAD dir maps to the repo (the
        // backend's wd→repo mapping tags every dir of the repo to `common`) and
        // drives one production spawn.
        let t = t0();
        trigger.on_ref_event(&common, t);
        trigger.poll(t + Duration::from_millis(500));
        assert_eq!(
            s.spawns.load(Ordering::SeqCst),
            1,
            "a sibling-HEAD ref event drives a production spawn",
        );
    }

    #[test]
    fn cap_exceeded_emits_one_envelope_per_registered_worktree_through_fanout() {
        use crate::broadcaster::TelemetryBroadcaster;
        use crate::fanout::{Fanout, OwnershipResolver, SubscriberId};

        // Two subscribers, each owning one of the repo's two worktrees.
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
        let sub_a = SubscriberId::new("owner-a");
        let sub_b = SubscriberId::new("owner-b");
        let resolver = WtResolver {
            owners: vec![
                (sub_a.clone(), "/wt/a".to_owned()),
                (sub_b.clone(), "/wt/b".to_owned()),
            ],
        };
        let broadcaster = Arc::new(TelemetryBroadcaster::new(Arc::new(Fanout::new(Box::new(
            resolver,
        )))));
        let mut rx_a = broadcaster.register(sub_a, None);
        let mut rx_b = broadcaster.register(sub_b, None);
        let notifier = BaseTriggerNotifier::new(Arc::clone(&broadcaster));

        // Drive a shared core to the cap edge directly (no reaper threads), then
        // fire the cap-exceeding trigger THROUGH the executor so the envelope fan-out
        // runs for both worktrees.
        let core = Arc::new(Mutex::new(TriggerCore::with_timings(
            Duration::from_millis(500),
            Duration::from_secs(5),
            MAX_RESTARTS_PER_LINEAGE,
        )));
        let repo = PathBuf::from("/repo");
        {
            let mut c = core.lock().unwrap();
            c.register_repo(&repo, "/wt/a");
            c.add_worktree(&repo, PathBuf::from("/wt/b"));
            let mut t = t0();
            drive_spawn(&mut c, &repo, t, 100);
            for restart in 1..=3u32 {
                t += Duration::from_millis(600);
                c.on_ref_event(&repo, t);
                let actions = c.poll(t + Duration::from_millis(500));
                let spawn_id = actions
                    .iter()
                    .find_map(|a| match a {
                        TriggerAction::Spawn { spawn_id, .. } => Some(*spawn_id),
                        _ => None,
                    })
                    .expect("respawn");
                c.on_child_spawned(&repo, spawn_id, 200 + restart);
            }
            // Arm the cap-exceeding event; the executor's poll below fires it.
            t += Duration::from_millis(600);
            c.on_ref_event(&repo, t);
        }

        let s = seams();
        let trigger = GraphBaseTrigger::new(
            Arc::clone(&core),
            Arc::clone(&s) as Arc<dyn ProductionSpawner>,
            Arc::clone(&s) as Arc<dyn Signaller>,
            Some(notifier),
        );
        // The next poll (well past the ~2.9 s pending deadline) crosses the cap →
        // EmitCapExceeded for BOTH worktrees.
        trigger.poll(Instant::now() + Duration::from_secs(30));

        assert_eq!(
            s.spawns.load(Ordering::SeqCst),
            0,
            "cap-exceed serves cold — no new spawn",
        );
        // Each worktree's subscriber received its own health envelope.
        assert!(
            rx_a.try_recv().is_ok(),
            "worktree A subscriber gets an envelope"
        );
        assert!(
            rx_b.try_recv().is_ok(),
            "worktree B subscriber gets an envelope"
        );
    }
}
