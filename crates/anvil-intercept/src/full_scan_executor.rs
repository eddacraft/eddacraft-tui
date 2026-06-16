//! DSV-045 (ADR-085): the daemon full-scan executor.
//!
//! `anvil/request_full_scan` and a fresh MCP session's first contact both want a
//! *populated* warm graph without the user having to save a file. The verb and
//! the assurance state machine (DSV-005/-006) only ever set the per-worktree
//! state to `Pending` — nothing dequeued it. This module is the loop that drives
//! `Pending → Running → Clean` (or `Bounded`), populating the
//! [`KernelGraphCache`] by walking the worktree, parsing each file with the
//! injected [`SymbolParser`], and feeding the symbols through `apply_delta`.
//!
//! # Where the work runs (ADR-084 C2 / ADR-031)
//!
//! The whole walk + parse + apply runs as a job on the **background** rayon pool
//! (the caller spawns [`PreparedScan::run`] there), so it never touches the
//! small interactive pool that protects the `validate_paths` latency budget. The
//! per-key [`AssuranceMachine`] lock is held **only** for the brief
//! `start_scan` / `complete_scan` transitions — never across the walk/parse/
//! apply — so a concurrent `validate_paths` for the same worktree is never
//! blocked behind a scan.
//!
//! # The guarantees this module owns
//!
//! - **No phantom `Clean` on a raced save (Decision 4).** Any `apply_delta` for
//!   the key while the scan is `Running` flags the machine dirty; the scan's
//!   `complete_scan` compare-and-clears that flag *under the same lock* as the
//!   `Clean` transition, so a save that raced the scan fails safe to `Stale` and
//!   re-queues instead of being certified away.
//! - **No phantom `Clean` without a parser (Decision 3).** A daemon with no
//!   injected parser (e.g. Windows today) marks the worktree `Stale` and never
//!   starts a scan — it never produces an empty graph that reads as complete.
//! - **Honest truncation (Decision 5).** A worktree still over the walk
//!   file-count cap *after* the gitignore pre-filter completes to `Bounded`
//!   (carrying [`ScanCoverage`]), never `Clean`.
//! - **DoS-safe coalescing (Decision 10).** A per-key `scan-enqueued` CAS flag
//!   means N concurrent `request_full_scan` calls drive one scan; the flag
//!   resets via an RAII guard on **any** job exit — completion, panic, or
//!   cancellation — so a panicked job never wedges the verb permanently inert.
//! - **Cooperative yield (Decision 9).** An interactive `validate_paths` can
//!   [`ScanCoordinator::cancel`] the in-flight scan; the chunked loop yields at
//!   the next chunk boundary, keeps the deltas it already applied, and resumes
//!   the continuation from the processed offset.
#![cfg(any(unix, windows))]

use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use anvil_graph_cache::certify::ChangeKind;
use anvil_intercept_proto::protocol::{AssuranceState, ScanCoverage, StaleReason};

use crate::assurance::{AssuranceMachine, ScanCompletion, ScanPriority};
use crate::kernel_cache::KernelGraphCache;
use crate::rule_cache::WorktreeKey;
use crate::save_time::{SymbolParser, trace_machine_transition};
use crate::workspace_anchor::WorkspaceAnchor;
use crate::workspace_pool::{DosCaps, ScanCancel, ScanOutcome, run_chunked_scan, walk_gitignored};

/// Files processed between cancel checks (ADR-061 §4 cooperative yield). Small
/// enough that an interactive `validate_paths` cancel is honoured promptly,
/// large enough that the per-chunk overhead is negligible against the parse.
const SCAN_CHUNK: usize = 64;

/// Maximum consecutive dirty-race re-scans before the executor gives up and
/// leaves the worktree `Stale` (a later trigger — first contact, eviction, an
/// explicit `request_full_scan` — re-warms it). Bounds a pathological save-storm
/// busy-loop; in practice a user editing continuously already keeps the cache
/// warm through each save's own `apply_delta`, so the cap is rarely approached.
const MAX_DIRTY_RETRIES: u32 = 4;

/// RFC 3339 wall-clock now, for the machine's scan timestamps (diagnostic only —
/// never load-bearing for a verdict).
fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Per-[`WorktreeKey`] concurrency coordination for the executor: the
/// `scan-enqueued` CAS flags (coalescing) and the in-flight scans' cancel
/// handles (cooperative yield). Cheap to clone — all clones share one inner.
#[derive(Clone, Default)]
pub struct ScanCoordinator {
    inner: Arc<CoordinatorInner>,
}

#[derive(Default)]
struct CoordinatorInner {
    /// Per-key "a background scan job is enqueued or running" flag. Set by a
    /// `false → true` CAS in [`ScanCoordinator::try_enqueue`]; reset on any job
    /// exit by [`EnqueuedGuard`]. The `Arc` survives map churn so a guard always
    /// resets the exact flag it set.
    enqueued: Mutex<HashMap<WorktreeKey, Arc<AtomicBool>>>,
    /// Per-key cancel handle for the in-flight scan, so an interactive
    /// `validate_paths` can preempt it (Decision 9).
    active: Mutex<HashMap<WorktreeKey, ScanCancel>>,
}

impl ScanCoordinator {
    /// A fresh coordinator with no enqueued scans.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn enqueued_flag(&self, key: &WorktreeKey) -> Arc<AtomicBool> {
        let mut map = lock(&self.inner.enqueued);
        Arc::clone(map.entry(key.clone()).or_default())
    }

    /// Try to claim the single scan slot for `key`. Returns an [`EnqueuedGuard`]
    /// (which resets the slot on drop) on a `false → true` CAS, or `None` when a
    /// scan is already enqueued/running for the key — the coalescing contract
    /// (Decision 10): N concurrent claims yield one job.
    #[must_use]
    fn try_enqueue(&self, key: &WorktreeKey) -> Option<EnqueuedGuard> {
        let flag = self.enqueued_flag(key);
        flag.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| EnqueuedGuard {
                flag,
                key: key.clone(),
                inner: Arc::clone(&self.inner),
            })
    }

    /// Whether a scan is currently enqueued/running for `key` (test/diagnostic).
    #[must_use]
    pub fn is_enqueued(&self, key: &WorktreeKey) -> bool {
        lock(&self.inner.enqueued)
            .get(key)
            .is_some_and(|f| f.load(Ordering::Acquire))
    }

    fn register_cancel(&self, key: &WorktreeKey, cancel: ScanCancel) {
        lock(&self.inner.active).insert(key.clone(), cancel);
    }

    fn clear_cancel(&self, key: &WorktreeKey) {
        lock(&self.inner.active).remove(key);
    }

    /// Request the in-flight scan for `key` (if any) to yield at its next chunk
    /// boundary (Decision 9). A no-op when no scan is running. Called by an
    /// interactive `validate_paths` to hand cores back from a background scan.
    pub fn cancel(&self, key: &WorktreeKey) {
        if let Some(cancel) = lock(&self.inner.active).get(key) {
            cancel.cancel();
        }
    }
}

/// RAII slot release for one enqueued scan job. Resets the `scan-enqueued` CAS
/// flag on **any** drop — normal completion, a cooperative yield's terminal
/// exit, or an unwinding panic — so a crashed scan never leaves
/// `request_full_scan` permanently inert for the key (Decision 10's liveness
/// hole). Also drops the key's cancel registration.
pub struct EnqueuedGuard {
    flag: Arc<AtomicBool>,
    key: WorktreeKey,
    inner: Arc<CoordinatorInner>,
}

impl Drop for EnqueuedGuard {
    fn drop(&mut self) {
        // Clear the cancel registration first, then release the slot, so a
        // claimant that wins the freed slot never observes a stale cancel handle.
        lock(&self.inner.active).remove(&self.key);
        self.flag.store(false, Ordering::Release);
    }
}

/// The shared, `'static`-capturable collaborators a spawned scan job reads. Cheap
/// to clone (all `Arc`/`Copy`), so a job owns its own copy and never borrows the
/// per-connection [`SaveTimeConn`](crate::save_time::SaveTimeConn).
#[derive(Clone)]
pub struct ScanContext {
    cache: Arc<KernelGraphCache>,
    parser: Option<Arc<dyn SymbolParser>>,
    caps: DosCaps,
    coordinator: ScanCoordinator,
}

impl ScanContext {
    /// Assemble a scan context from the daemon's shared collaborators.
    #[must_use]
    pub fn new(
        cache: Arc<KernelGraphCache>,
        parser: Option<Arc<dyn SymbolParser>>,
        caps: DosCaps,
        coordinator: ScanCoordinator,
    ) -> Self {
        Self {
            cache,
            parser,
            caps,
            coordinator,
        }
    }

    /// The coordinator (so a caller can preempt an in-flight scan).
    #[must_use]
    pub fn coordinator(&self) -> &ScanCoordinator {
        &self.coordinator
    }
}

/// A scan the executor has decided to run and has claimed the slot for. The
/// caller chooses the thread: production spawns `job.run()` on the background
/// pool; tests call it inline for determinism.
pub struct PreparedScan {
    ctx: ScanContext,
    machine: Arc<Mutex<AssuranceMachine>>,
    key: WorktreeKey,
    root: PathBuf,
    guard: EnqueuedGuard,
}

impl PreparedScan {
    /// Drive the scan to a terminal state. Catches a panic in the scan body so a
    /// crashing parse marks the worktree `Stale` (fail-safe) and resets the
    /// enqueued slot via the guard's drop, rather than aborting the daemon or
    /// wedging the verb (Decision 10).
    pub fn run(self) {
        let PreparedScan {
            ctx,
            machine,
            key,
            root,
            guard,
        } = self;

        let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| {
            run_scan_loop(&ctx, &machine, &key, &root);
        }));

        if outcome.is_err() {
            // The scan panicked mid-flight (e.g. a parser bug). The graph is
            // half-built and untrustworthy → fail safe to `Stale`, needing a
            // re-scan. `CrossFileResolutionNeeded` is the honest "this needs a
            // (re)scan" cause in the frozen vocabulary.
            with_locked_machine_trace(&machine, &root, |m| {
                m.mark_stale(StaleReason::CrossFileResolutionNeeded);
            });
            tracing::warn!(
                target: "anvil_intercept::full_scan",
                workspace_root = %root.display(),
                "full scan job panicked; worktree marked stale and slot released",
            );
        }

        // Belt-and-braces: the guard's drop also clears this, but make the
        // post-run state explicit. Then the guard drop releases the slot.
        ctx.coordinator.clear_cancel(&key);
        drop(guard);
    }
}

/// Decide whether `key` needs a scan and, if so, claim the slot and return a
/// runnable [`PreparedScan`]. Returns `None` when the worktree is already warm
/// (a `Background` first-contact / eviction trigger on a `Clean`/`Bounded`,
/// still-cached key is a no-op) or when a scan is already enqueued (coalesced).
///
/// `priority` distinguishes an explicit client `request_full_scan`
/// ([`ScanPriority::Interactive`] — always (re)scans) from an opportunistic
/// first-contact / eviction warm-up ([`ScanPriority::Background`] — scans only
/// when the warm graph is missing or stale).
///
/// Eviction (Decision: a `WarmStateEvicted` re-queues): if the machine still
/// believes the worktree `Clean`/`Bounded` but the cache no longer holds its
/// warm pair (an LRU eviction dropped it), the machine is marked
/// `Stale(WarmStateEvicted)` and a re-scan is enqueued. Detection is lazy — at
/// the next trigger — which is lock-safe (no callback fires under the cache
/// lock) and cheap (a cold cache only matters when something reads it).
#[must_use]
pub fn prepare_scan(
    ctx: &ScanContext,
    machine: &Arc<Mutex<AssuranceMachine>>,
    key: &WorktreeKey,
    root: &Path,
    priority: ScanPriority,
) -> Option<PreparedScan> {
    let needs_scan = {
        let mut m = lock(machine);
        let before = m.snapshot();

        // Lazy eviction detection: a warm-believed state whose cache pair is
        // gone was evicted — fail safe and re-warm.
        let evicted = matches!(m.state(), AssuranceState::Clean | AssuranceState::Bounded)
            && !ctx.cache.contains(key);
        if evicted {
            m.mark_stale(StaleReason::WarmStateEvicted);
        }

        let already_warm = matches!(m.state(), AssuranceState::Clean | AssuranceState::Bounded)
            && ctx.cache.contains(key);
        // Explicit requests always (re)scan; opportunistic warm-ups skip an
        // already-warm worktree.
        let needs = priority == ScanPriority::Interactive || !already_warm;
        if needs {
            m.request_full_scan(priority);
        }
        let after = m.snapshot();
        drop(m);
        trace_machine_transition(root, &before, &after);
        needs
    };

    if !needs_scan {
        return None;
    }

    // Coalesce: one job per key. A loser leaves the (now `Pending`) machine for
    // the winning job to drive.
    let guard = ctx.coordinator.try_enqueue(key)?;
    Some(PreparedScan {
        ctx: ctx.clone(),
        machine: Arc::clone(machine),
        key: key.clone(),
        root: root.to_path_buf(),
        guard,
    })
}

/// Drive one scan to a terminal state, handling cooperative-yield continuations
/// and dirty-race re-scans in a single job (so the slot guard's ownership is
/// never handed off and the CAS flag is never raced).
fn run_scan_loop(
    ctx: &ScanContext,
    machine: &Arc<Mutex<AssuranceMachine>>,
    key: &WorktreeKey,
    root: &Path,
) {
    // Decision 3: no parser → abort to `Stale`, never start a scan / produce an
    // empty `Clean` graph.
    let Some(parser) = ctx.parser.clone() else {
        with_locked_machine_trace(machine, root, |m| {
            m.mark_stale(StaleReason::CrossFileResolutionNeeded);
        });
        tracing::debug!(
            target: "anvil_intercept::full_scan",
            workspace_root = %root.display(),
            "no parser injected; full scan aborted to stale (never phantom-clean)",
        );
        return;
    };

    // Open the executor's own anchor on the admitted canonical root (Decision 7)
    // — reads survive a client disconnect.
    let anchor = match WorkspaceAnchor::open(root) {
        Ok(anchor) => anchor,
        Err(err) => {
            with_locked_machine_trace(machine, root, |m| {
                m.mark_stale(StaleReason::CrossFileResolutionNeeded);
            });
            tracing::warn!(
                target: "anvil_intercept::full_scan",
                workspace_root = %root.display(),
                error = %err,
                "full scan could not open the workspace anchor; marked stale",
            );
            return;
        }
    };

    let mut walk = walk_gitignored(root, ctx.caps.max_walk_depth, ctx.caps.max_walk_files);
    let mut dirty_retries: u32 = 0;

    loop {
        let files = &walk.files;
        let truncated = walk.truncated();
        let coverage = ScanCoverage {
            scanned_files: files.len() as u64,
            total_files: walk.total as u64,
        };

        // Run the segments (start..) until the file list is exhausted, yielding
        // and resuming on cancel. Returns when every file has been applied.
        let mut start = 0usize;
        loop {
            with_locked_machine_trace(machine, root, |m| m.start_scan(now_rfc3339()));

            let cancel = ScanCancel::new();
            ctx.coordinator.register_cancel(key, cancel.clone());

            // The walk + parse + apply run WITHOUT the machine lock (C2): only
            // `start_scan` above and `complete_scan` below take it, briefly.
            let outcome = run_chunked_scan(&files[start..], SCAN_CHUNK, &cancel, |path| {
                apply_file(
                    &ctx.cache,
                    key,
                    &anchor,
                    parser.as_ref(),
                    ctx.caps.max_parse_bytes,
                    root,
                    path,
                );
            });
            ctx.coordinator.clear_cancel(key);

            match outcome {
                ScanOutcome::Yielded { processed } => {
                    // Keep the applied deltas; resume the continuation from the
                    // processed offset. The machine stays `Running` — a scan is
                    // genuinely still in flight (we deviate from ADR-085's
                    // "set Stale + re-queue a separate job" framing: the
                    // two-pool model lets the continuation run in the same job,
                    // so `Running` is the accurate state and there is no slot
                    // hand-off to race). A cancel only ever lands before a
                    // not-yet-started chunk, so `processed` advances by whole
                    // chunks and the loop terminates.
                    start += processed;
                    if start >= files.len() {
                        break;
                    }
                }
                ScanOutcome::Completed => break,
            }
        }

        // Terminal transition for this walk, under the lock (brief): the
        // compare-and-clear of the dirty flag happens here (Decision 4).
        let completion = with_locked_machine_trace(machine, root, |m| {
            if truncated {
                m.complete_scan_bounded(now_rfc3339(), coverage)
            } else {
                m.complete_scan(now_rfc3339())
            }
        });

        match completion {
            ScanCompletion::Clean | ScanCompletion::Bounded => return,
            ScanCompletion::Dirtied => {
                dirty_retries += 1;
                if dirty_retries > MAX_DIRTY_RETRIES {
                    // Give up: the machine is already `Stale` from the dirtied
                    // completion. A later trigger re-warms.
                    tracing::debug!(
                        target: "anvil_intercept::full_scan",
                        workspace_root = %root.display(),
                        retries = dirty_retries,
                        "full scan exceeded dirty-race retries; left stale for a later trigger",
                    );
                    return;
                }
                // A save raced the scan; re-walk to pick up creates/deletes and
                // re-scan from the start.
                walk = walk_gitignored(root, ctx.caps.max_walk_depth, ctx.caps.max_walk_files);
            }
        }
    }
}

/// Read, parse, and apply one walked file into the warm cache. Best-effort: an
/// unreadable file, an over-cap file, or an unparseable file is skipped (the scan
/// never aborts over one bad file). The root-relative path is built with forward
/// slashes so it matches the key `validate_paths` parses under (the parser
/// assigns path-stable symbol ids from it — the load-bearing fact behind the
/// scan-vs-save graph equivalence).
fn apply_file(
    cache: &KernelGraphCache,
    key: &WorktreeKey,
    anchor: &WorkspaceAnchor,
    parser: &dyn SymbolParser,
    max_parse_bytes: usize,
    root: &Path,
    abs_path: &Path,
) {
    let Ok(rel_path) = abs_path.strip_prefix(root) else {
        return;
    };
    // Forward-slash, structurally-clean relative path (the platform anchor
    // refuses backslashes/escape forms; the parser keys symbols on this string).
    let rel: String = rel_path
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    if rel.is_empty() {
        return;
    }

    let Ok(bytes) = anchor.read_rel(&rel) else {
        return;
    };
    // DoS parse-size cap (DSV-006 / Task 11): skip a file too large to parse.
    if bytes.len() > max_parse_bytes {
        return;
    }
    let Some(symbols) = parser.parse(Path::new(&rel), &bytes) else {
        return;
    };
    cache.apply_delta(key, ChangeKind::Create, symbols);
}

/// Run `f` against the per-key machine under its lock, emitting the standard
/// tracing transition. The lock is held only for `f` (a brief
/// `start_scan`/`complete_scan`/`mark_stale`) and released before the trace I/O.
fn with_locked_machine_trace<R>(
    machine: &Arc<Mutex<AssuranceMachine>>,
    root: &Path,
    f: impl FnOnce(&mut AssuranceMachine) -> R,
) -> R {
    let mut m = lock(machine);
    let before = m.snapshot();
    let result = f(&mut m);
    let after = m.snapshot();
    drop(m);
    trace_machine_transition(root, &before, &after);
    result
}

/// Lock a mutex, recovering a poisoned guard rather than propagating the panic
/// (mirrors `workspace_pool`/`kernel_cache`): the executor's critical sections
/// are short and panic-free, so a poison implies an unrelated abort, not a
/// corrupt invariant.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::sync::Barrier;
    use std::sync::atomic::AtomicUsize;

    use anvil_kernel_types::{
        FileSymbols, ImportEdge, SymbolKind, SymbolNode, TrustLevel, Visibility,
    };

    use crate::assurance::AssuranceMachine;

    // ---- A tiny, path-stable test parser ---------------------------------

    /// A trivial line-oriented parser for tests: `export NAME` lines become
    /// public symbols; `import ./spec` lines become import edges. Symbol ids are
    /// derived from a path-stable hash of `(file, name)` so re-parsing a file
    /// yields identical ids (the `SymbolParser` contract), which is what lets a
    /// scan-driven graph match a save-driven one.
    #[derive(Debug, Default)]
    struct LineParser;

    fn stable_id(file: &str, name: &str) -> u64 {
        // FNV-1a over file\0name — deterministic and collision-resistant enough
        // for the small test corpora.
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in file.bytes().chain(std::iter::once(0)).chain(name.bytes()) {
            h ^= u64::from(byte);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        h
    }

    impl SymbolParser for LineParser {
        fn parse(&self, path: &Path, bytes: &[u8]) -> Option<FileSymbols> {
            let text = std::str::from_utf8(bytes).ok()?;
            let file = path.to_string_lossy().into_owned();
            let mut symbols = Vec::new();
            let mut imports = Vec::new();
            for line in text.lines() {
                let line = line.trim();
                if let Some(name) = line.strip_prefix("export ") {
                    let name = name.trim();
                    symbols.push(SymbolNode {
                        id: stable_id(&file, name),
                        kind: SymbolKind::Function,
                        name: name.to_string(),
                        visibility: Visibility::Public,
                        file: file.clone(),
                        trust_level: TrustLevel::Unknown,
                    });
                } else if let Some(spec) = line.strip_prefix("import ") {
                    imports.push(ImportEdge {
                        from_file: file.clone(),
                        to_source: spec.trim().to_string(),
                        line: 0,
                    });
                }
            }
            Some(FileSymbols {
                file,
                symbols,
                imports,
                reexports: Vec::new(),
            })
        }
    }

    /// A parser that always panics — drives the panic-resilience test.
    #[derive(Debug, Default)]
    struct PanicParser;
    impl SymbolParser for PanicParser {
        fn parse(&self, _path: &Path, _bytes: &[u8]) -> Option<FileSymbols> {
            panic!("parser blew up mid-scan");
        }
    }

    /// A parser that blocks on a barrier the first time it is called, so a test
    /// can prove the scan is in its apply loop (machine unlocked) before making
    /// an assertion. Subsequent calls behave like [`LineParser`].
    #[derive(Debug)]
    struct GatedParser {
        entered: Arc<Barrier>,
        release: Arc<Barrier>,
        calls: Arc<AtomicUsize>,
    }
    impl SymbolParser for GatedParser {
        fn parse(&self, path: &Path, bytes: &[u8]) -> Option<FileSymbols> {
            if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
                self.entered.wait();
                self.release.wait();
            }
            LineParser.parse(path, bytes)
        }
    }

    // ---- Helpers ----------------------------------------------------------

    fn ctx_with(parser: Option<Arc<dyn SymbolParser>>, caps: DosCaps) -> ScanContext {
        ScanContext::new(
            Arc::new(KernelGraphCache::new()),
            parser,
            caps,
            ScanCoordinator::new(),
        )
    }

    fn line_ctx() -> ScanContext {
        ctx_with(Some(Arc::new(LineParser)), DosCaps::default())
    }

    fn machine() -> Arc<Mutex<AssuranceMachine>> {
        Arc::new(Mutex::new(AssuranceMachine::new()))
    }

    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(path, body).expect("write file");
    }

    fn key_for(root: &Path) -> WorktreeKey {
        WorktreeKey::from_canonical(root.to_path_buf())
    }

    /// Drive a scan synchronously to a terminal state and return the machine
    /// state after.
    fn scan_to_completion(ctx: &ScanContext, root: &Path) -> AssuranceState {
        let key = key_for(root);
        let machine = machine();
        let job = prepare_scan(ctx, &machine, &key, root, ScanPriority::Background)
            .expect("a cold worktree must enqueue a scan");
        job.run();
        lock(&machine).state()
    }

    /// A normalised, order-independent summary of the warm graph for `key`:
    /// each file maps to its sorted public symbol names and sorted resolved
    /// dependency files. Two equivalent graphs produce equal summaries.
    fn graph_summary(
        cache: &KernelGraphCache,
        key: &WorktreeKey,
    ) -> BTreeMap<String, (Vec<String>, Vec<String>)> {
        cache
            .with_graphs(key, |sym, dep| {
                let mut out: BTreeMap<String, (Vec<String>, Vec<String>)> = BTreeMap::new();
                for node in sym.inner().node_weights() {
                    let entry = out.entry(node.file.clone()).or_default();
                    entry.0.push(node.name.clone());
                }
                for (file, names_deps) in &mut out {
                    names_deps.0.sort();
                    names_deps.0.dedup();
                    let mut deps: Vec<String> = dep
                        .dependencies_of(file)
                        .into_iter()
                        .map(str::to_string)
                        .collect();
                    deps.sort();
                    deps.dedup();
                    names_deps.1 = deps;
                }
                out
            })
            .unwrap_or_default()
    }

    // ---- Tests ------------------------------------------------------------

    #[test]
    fn scan_driven_graph_equivalent_to_save_driven_baseline() {
        // Corpus: an import cycle (a -> b -> a), a diamond (d -> e, d -> f,
        // e -> g, f -> g), and >=10 cross-file-import files, so order
        // independence is genuinely exercised.
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let files: Vec<(&str, &str)> = vec![
            ("a.ts", "export a\nimport ./b"),
            ("b.ts", "export b\nimport ./a"),
            ("d.ts", "export d\nimport ./e\nimport ./f"),
            ("e.ts", "export e\nimport ./g"),
            ("f.ts", "export f\nimport ./g"),
            ("g.ts", "export g"),
            ("h.ts", "export h\nimport ./a"),
            ("i.ts", "export i\nimport ./h"),
            ("j.ts", "export j\nimport ./i"),
            ("k.ts", "export k\nimport ./j"),
            ("l.ts", "export l\nimport ./d"),
            ("m.ts", "export m\nimport ./l"),
        ];
        for (name, body) in &files {
            write(root, name, body);
        }

        // Scan-driven graph (walk order).
        let scan_ctx = line_ctx();
        let scan_state = scan_to_completion(&scan_ctx, root);
        assert_eq!(scan_state, AssuranceState::Clean);
        let scan_summary = graph_summary(&scan_ctx.cache, &key_for(root));

        // Save-driven baseline: apply each file as Create in a DIFFERENT
        // (reverse) order directly through the cache.
        let baseline = KernelGraphCache::new();
        let key = key_for(root);
        let parser = LineParser;
        for (name, body) in files.iter().rev() {
            let symbols = parser
                .parse(Path::new(name), body.as_bytes())
                .expect("parse");
            baseline.apply_delta(&key, ChangeKind::Create, symbols);
        }
        let baseline_summary = graph_summary(&baseline, &key);

        assert_eq!(
            scan_summary, baseline_summary,
            "a scan-driven graph must equal a save-driven baseline regardless of order"
        );
        // Sanity: the cycle and diamond resolved (a depends on b, d on e+f).
        assert_eq!(scan_summary["a.ts"].1, vec!["b.ts".to_string()]);
        assert_eq!(
            scan_summary["d.ts"].1,
            vec!["e.ts".to_string(), "f.ts".to_string()]
        );
    }

    #[test]
    fn no_parser_marks_stale_never_starts_scan() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        write(root, "a.ts", "export a");

        let ctx = ctx_with(None, DosCaps::default());
        let state = scan_to_completion(&ctx, root);
        assert_eq!(state, AssuranceState::Stale, "no parser → stale, not clean");
        assert!(
            !ctx.cache.contains(&key_for(root)),
            "a no-parser scan must never populate the cache (no phantom graph)"
        );
    }

    #[test]
    fn over_walk_cap_after_gitignore_resolves_bounded_not_clean() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        for i in 0..8 {
            write(root, &format!("f{i}.ts"), &format!("export s{i}"));
        }
        // Cap the walk at 3 files: 8 source files > 3 → Bounded.
        let caps = DosCaps {
            max_walk_files: 3,
            ..DosCaps::default()
        };
        let ctx = ctx_with(Some(Arc::new(LineParser)), caps);
        let key = key_for(root);
        let machine = machine();
        prepare_scan(&ctx, &machine, &key, root, ScanPriority::Background)
            .expect("enqueue")
            .run();

        let snap = lock(&machine).snapshot();
        assert_eq!(snap.state, AssuranceState::Bounded, "over-cap → Bounded");
        assert_eq!(snap.reason, None, "Bounded is a lifecycle state");
        let coverage = snap.scan_coverage.expect("Bounded carries coverage");
        assert_eq!(coverage.scanned_files, 3);
        assert_eq!(coverage.total_files, 8);
    }

    #[test]
    fn apply_delta_during_running_scan_marks_stale_not_clean() {
        // A save that races a Running scan must not be certified away: the scan
        // completes Dirtied → Stale, and the executor re-queues. Driven at the
        // machine boundary the executor relies on: note_apply_delta during
        // Running, then complete.
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        write(root, "a.ts", "export a");

        let ctx = line_ctx();
        let key = key_for(root);
        let machine = machine();
        // Put the machine into Running as a scan would.
        {
            let mut m = lock(&machine);
            m.request_full_scan(ScanPriority::Background);
            m.start_scan(now_rfc3339());
            // A save lands mid-scan (origin: validate_paths).
            m.note_apply_delta();
            let completion = m.complete_scan(now_rfc3339());
            assert_eq!(completion, ScanCompletion::Dirtied);
            assert_eq!(m.state(), AssuranceState::Stale);
            assert_eq!(m.reason(), Some(StaleReason::CrossFileResolutionNeeded));
        }
        // After the dirtied completion the worktree is Stale → a re-warm trigger
        // re-queues and completes Clean (the racing save has settled).
        let job = prepare_scan(&ctx, &machine, &key, root, ScanPriority::Background)
            .expect("dirtied → stale re-queues");
        job.run();
        assert_eq!(lock(&machine).state(), AssuranceState::Clean);
    }

    #[test]
    fn non_validate_paths_apply_delta_during_running_also_sets_dirty() {
        // The dirty flag is origin-agnostic: a GCTX on-demand re-warm (any
        // apply path) during Running sets it just like a validate_paths save.
        let mut m = AssuranceMachine::new();
        m.request_full_scan(ScanPriority::Background);
        m.start_scan(now_rfc3339());
        m.note_apply_delta(); // stands in for a non-validate_paths apply
        assert!(m.is_dirty_during_scan());
        assert_eq!(m.complete_scan(now_rfc3339()), ScanCompletion::Dirtied);
    }

    #[test]
    fn repeated_request_full_scan_drives_one_scan() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        write(root, "a.ts", "export a");

        let ctx = line_ctx();
        let key = key_for(root);
        let machine = machine();

        // First claim wins the slot.
        let job = prepare_scan(&ctx, &machine, &key, root, ScanPriority::Interactive)
            .expect("first request enqueues");
        // Concurrent repeats while the slot is held coalesce to nothing.
        for _ in 0..5 {
            assert!(
                prepare_scan(&ctx, &machine, &key, root, ScanPriority::Interactive).is_none(),
                "a repeat request while one is enqueued must coalesce"
            );
        }
        // Running it releases the slot; a later request can enqueue again.
        job.run();
        assert!(
            !ctx.coordinator.is_enqueued(&key),
            "slot released after run"
        );
        assert_eq!(lock(&machine).state(), AssuranceState::Clean);
    }

    #[test]
    fn executor_panic_resets_scan_enqueued_flag() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        write(root, "a.ts", "export a");

        let ctx = ctx_with(Some(Arc::new(PanicParser)), DosCaps::default());
        let key = key_for(root);
        let machine = machine();

        let job =
            prepare_scan(&ctx, &machine, &key, root, ScanPriority::Background).expect("enqueue");
        // The panic is caught inside run(); the slot must be released and the
        // worktree left stale (fail-safe), not wedged.
        job.run();
        assert!(
            !ctx.coordinator.is_enqueued(&key),
            "a panicked scan must reset the enqueued flag (no permanent inertness)"
        );
        assert_eq!(lock(&machine).state(), AssuranceState::Stale);
        // And a subsequent request can still enqueue.
        assert!(
            prepare_scan(&ctx, &machine, &key, root, ScanPriority::Background).is_some(),
            "request_full_scan must work again after a panicked job"
        );
    }

    #[test]
    fn yield_keeps_applied_deltas_and_resumes_from_processed_offset() {
        // A cancel mid-scan yields at a chunk boundary; the continuation resumes
        // from the processed offset and the final graph is complete. With a
        // chunk of 64 and >64 files, a cancel after the first chunk forces a
        // continuation. We drive scan segments directly to assert resumption.
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let n = SCAN_CHUNK + 10;
        for i in 0..n {
            write(root, &format!("f{i}.ts"), &format!("export s{i}"));
        }
        let ctx = line_ctx();
        let key = key_for(root);
        let machine = machine();

        // Run the full job, but cancel once after the first chunk lands. We
        // simulate the interactive preemption by cancelling the active scan from
        // a watcher thread as soon as the first file is applied.
        let job =
            prepare_scan(&ctx, &machine, &key, root, ScanPriority::Background).expect("enqueue");
        let coordinator = ctx.coordinator.clone();
        let key_for_cancel = key.clone();
        let cache_for_watch = Arc::clone(&ctx.cache);
        let canceller = std::thread::spawn(move || {
            // Wait until the scan has applied at least one file, then preempt.
            for _ in 0..10_000 {
                if !cache_for_watch.is_empty() {
                    coordinator.cancel(&key_for_cancel);
                    return;
                }
                std::thread::yield_now();
            }
        });
        job.run();
        canceller.join().expect("canceller");

        // Despite the mid-scan yield, every file resolved into the warm graph.
        let summary = graph_summary(&ctx.cache, &key);
        assert_eq!(
            summary.len(),
            n,
            "all {n} files applied across the continuation"
        );
        assert_eq!(lock(&machine).state(), AssuranceState::Clean);
    }

    #[test]
    fn evicted_warm_state_requeues_and_rewarms() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        write(root, "a.ts", "export a");

        let ctx = line_ctx();
        let key = key_for(root);
        let machine = machine();

        // Warm it.
        prepare_scan(&ctx, &machine, &key, root, ScanPriority::Background)
            .expect("enqueue")
            .run();
        assert_eq!(lock(&machine).state(), AssuranceState::Clean);
        assert!(ctx.cache.contains(&key));

        // Evict the warm pair (LRU eviction is modelled by an invalidate).
        ctx.cache.invalidate(&key);
        assert!(!ctx.cache.contains(&key));

        // A background trigger detects the eviction (Clean but not cached),
        // marks it WarmStateEvicted, and re-warms.
        let job = prepare_scan(&ctx, &machine, &key, root, ScanPriority::Background)
            .expect("eviction must re-queue a scan");
        job.run();
        assert_eq!(lock(&machine).state(), AssuranceState::Clean);
        assert!(
            ctx.cache.contains(&key),
            "the cache is re-warmed after eviction"
        );
    }

    #[test]
    fn already_warm_background_trigger_is_a_noop() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        write(root, "a.ts", "export a");

        let ctx = line_ctx();
        let key = key_for(root);
        let machine = machine();
        prepare_scan(&ctx, &machine, &key, root, ScanPriority::Background)
            .expect("enqueue")
            .run();
        assert_eq!(lock(&machine).state(), AssuranceState::Clean);

        // A first-contact background trigger on an already-warm worktree does
        // nothing (no redundant scan).
        assert!(
            prepare_scan(&ctx, &machine, &key, root, ScanPriority::Background).is_none(),
            "a warm worktree must not re-scan on an opportunistic trigger"
        );
    }

    #[test]
    fn validate_paths_can_lock_the_machine_while_a_scan_is_running() {
        // C2: the per-key machine lock is NOT held across walk+parse+apply, so a
        // concurrent validate_paths can take it promptly while a scan runs. The
        // gated parser parks inside the apply loop (machine unlocked); the test
        // thread must acquire the machine lock without waiting on the scan.
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        for i in 0..5 {
            write(root, &format!("f{i}.ts"), &format!("export s{i}"));
        }
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let parser = Arc::new(GatedParser {
            entered: Arc::clone(&entered),
            release: Arc::clone(&release),
            calls: Arc::new(AtomicUsize::new(0)),
        });
        let ctx = ctx_with(Some(parser), DosCaps::default());
        let key = key_for(root);
        let machine = machine();

        let job =
            prepare_scan(&ctx, &machine, &key, root, ScanPriority::Background).expect("enqueue");
        let machine_for_job = Arc::clone(&machine);
        let scan_thread = std::thread::spawn(move || {
            let _ = &machine_for_job;
            job.run();
        });

        // The parser is parked inside the apply loop → the scan is provably mid
        // walk+parse+apply, machine unlocked.
        entered.wait();
        {
            // This must NOT block on the scan: the machine lock is free.
            let m = lock(&machine);
            assert_eq!(m.state(), AssuranceState::Running, "a scan is in flight");
        }
        // Let the scan finish.
        release.wait();
        scan_thread.join().expect("scan thread");
        assert_eq!(lock(&machine).state(), AssuranceState::Clean);
    }
}
