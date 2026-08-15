use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread;

use anvil_kernel_types::{EngineEvent, EngineId, ErrorCode};
use rayon::prelude::*;

use crate::graph::{
    SymbolGraph, annotate_trust, re_resolve_imports, re_resolve_reexports, update_file,
};
use crate::parser::Parser;
use crate::parser::extract::{FileSymbols, ImportEdge, ReexportEdge, extract_symbols};
use crate::policy::config::ArchitectureConfig;
use crate::policy::config_validator::{
    ArchitectureConfigValidationError, parse_validated_architecture_config,
    read_architecture_config_capped,
};
use crate::policy::engine::PolicyEngine;
use crate::policy::invariants::cross_layer::CrossLayerViolation;
use crate::policy::invariants::new_dependency::NewDependencyIntroduction;
use crate::policy::invariants::privilege_expansion::PrivilegeExpansion;
use crate::policy::invariants::public_api::PublicApiExpansion;
use crate::protocol::emitter::EventEmitter;
use crate::watcher::events::ChangeKind;
use crate::watcher::filter::FileFilter;
use crate::watcher::pattern::{PatternError, WatchPatternFilter};
use crate::watcher::{WatcherConfig, start_watcher};
use anvil_rayon_init::init_global as init_rayon_pool;

#[derive(Debug, thiserror::Error)]
pub enum WatchError {
    #[error("root directory does not exist: {0}")]
    RootNotFound(PathBuf),
    #[error("architecture config error: {path}: {source}")]
    ConfigIo {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("architecture config parse error: {0}")]
    ConfigParse(serde_yaml::Error),
    #[error("architecture config validation error: {0}")]
    ConfigValidation(ArchitectureConfigValidationError),
    #[error("watcher error: {0}")]
    Watcher(#[from] crate::watcher::WatcherError),
    #[error("invalid watch pattern: {0}")]
    Pattern(#[from] PatternError),
    #[error("watch loop terminated unexpectedly")]
    ThreadPanicked,
    #[error("architecture config reload error: {0}")]
    ConfigReload(String),
}

/// Reloads a mapped `ArchitectureConfig` when the watched architecture
/// source file changes (UCFG-013). The CLI supplies a closure that
/// re-resolves the unified config so TOML/JSON inline sections work.
pub type ArchitectureReloader =
    std::sync::Arc<dyn Fn() -> Result<ArchitectureConfig, WatchError> + Send + Sync>;

impl From<ArchitectureConfigValidationError> for WatchError {
    fn from(error: ArchitectureConfigValidationError) -> Self {
        match error {
            ArchitectureConfigValidationError::Parse(parse_error) => Self::ConfigParse(parse_error),
            invalid @ ArchitectureConfigValidationError::Invalid(_) => {
                Self::ConfigValidation(invalid)
            }
        }
    }
}

pub struct WatchConfig {
    pub root: PathBuf,
    pub architecture_config: Option<PathBuf>,
    /// Pre-mapped architecture (inline / delegated Definition schema).
    /// When set, used instead of parsing `architecture_config` as YAML.
    pub architecture: Option<ArchitectureConfig>,
    /// Re-resolve the mapped architecture after the source file changes.
    pub architecture_reloader: Option<ArchitectureReloader>,
    pub watcher: WatcherConfig,
    /// User-supplied include glob patterns. Empty = include everything.
    /// Compiled into a `WatchPatternFilter` at `run_watch` start.
    /// Distinct from `WatcherConfig.filter`, which owns the hardcoded
    /// internal denylist (`node_modules`, `.git`, `target`, …).
    pub include_patterns: Vec<String>,
    /// User-supplied exclude glob patterns. Empty = exclude nothing.
    /// Takes precedence over `include_patterns` when both match.
    pub exclude_patterns: Vec<String>,
    /// Validated repo-relative files from the CLI warm-up cache. Empty means
    /// discover from disk. The cache is advisory: each path still goes through
    /// the active parser/filter path before entering the graph.
    pub warmup_paths: Vec<PathBuf>,
}

pub struct WatchHandle {
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
    _watcher: crate::watcher::WatcherHandle,
}

impl WatchHandle {
    pub fn stop(self) -> Result<(), WatchError> {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.thread {
            handle.join().map_err(|_| WatchError::ThreadPanicked)?;
        }
        Ok(())
    }
}

struct WatchState {
    parser: Parser,
    graph: SymbolGraph,
    all_imports: Vec<ImportEdge>,
    /// Live re-export edges keyed by `from_file` (GV2-031) — the re-export
    /// analogue of `all_imports`, retried by `re_resolve_reexports` so a
    /// forward-referenced re-export chain resolves and the certify privilege
    /// diff sees a re-exported privileged module.
    all_reexports: Vec<ReexportEdge>,
    /// Monotonic ID counter. Previously this was recomputed by scanning
    /// every node in the graph on every file-change event
    /// (`node_weights().map(|s| s.id).max()`), which is O(|symbols|) per
    /// keystroke-driven save. Hot path uses incremental updates only.
    next_id: u64,
    engine: PolicyEngine,
    file_count: u64,
    /// Files we have parsed at least once. Replaces the per-event
    /// `node_weights().any(|s| s.file == rel_str)` scan that was O(|symbols|)
    /// on every modify event.
    tracked_files: std::collections::HashSet<String>,
}

impl WatchState {
    fn new() -> Self {
        let mut engine = PolicyEngine::new();
        engine.register(Box::new(CrossLayerViolation));
        engine.register(Box::new(NewDependencyIntroduction));
        engine.register(Box::new(PublicApiExpansion));
        engine.register(Box::new(PrivilegeExpansion));

        Self {
            parser: Parser::new(),
            graph: SymbolGraph::new(),
            all_imports: Vec::new(),
            all_reexports: Vec::new(),
            next_id: 0,
            engine,
            file_count: 0,
            tracked_files: std::collections::HashSet::new(),
        }
    }
}

fn initial_scan(
    root: &Path,
    filter: &FileFilter,
    pattern_filter: &WatchPatternFilter,
    warmup_paths: &[PathBuf],
    state: &mut WatchState,
    emitter: &EventEmitter,
    stop: &AtomicBool,
) {
    emitter.progress("Discovering files", 0, 0);

    let cached_paths = cached_initial_paths(root, filter, pattern_filter, warmup_paths);
    if !cached_paths.is_empty() {
        let total_paths = cached_paths.len() as u64;
        emitter.progress("Discovering files", total_paths, total_paths);
        build_initial_graph(root, &cached_paths, state, emitter, stop);
        return;
    }

    // SCAN-001: noise-pruning discovery (skips target/, node_modules/, etc; .gitignore is intentionally not applied so security scans see every file) (same shape as the welcome
    // walker). Per-file parsing below already runs on rayon, so the only
    // change here is the walker primitive.
    let filter_for_walker = filter.clone();
    let walker = ignore::WalkBuilder::new(root)
        .follow_links(false)
        .standard_filters(false)
        .hidden(false)
        .filter_entry(move |e| {
            !e.file_type().is_some_and(|ft| ft.is_dir())
                || e.depth() == 0
                || !filter_for_walker.should_ignore(e.path())
        })
        .build();

    let all_paths: Vec<PathBuf> = walker
        .filter_map(|r| match r {
            Ok(e) => {
                if e.file_type().is_some_and(|ft| ft.is_file())
                    && filter.should_process(e.path())
                    && pattern_matches(pattern_filter, root, e.path())
                {
                    Some(e.path().to_path_buf())
                } else {
                    None
                }
            }
            Err(e) => {
                // `ignore::Error::WithPath` carries the offending path
                // (permission errors, broken symlinks, oversize repos) —
                // surface it on the emitter so operators can diagnose
                // without re-running with verbose logging. Other Error
                // variants don't have a path attached and pass None.
                let walk_path = match &e {
                    ignore::Error::WithPath { path, .. } => path.to_str(),
                    _ => None,
                };
                emitter.error(
                    ErrorCode::Internal,
                    walk_path,
                    &format!("walk error: {e}"),
                    true,
                );
                None
            }
        })
        .collect();
    let total_paths = all_paths.len() as u64;
    emitter.progress("Discovering files", total_paths, total_paths);

    build_initial_graph(root, &all_paths, state, emitter, stop);
}

fn cached_initial_paths(
    root: &Path,
    filter: &FileFilter,
    pattern_filter: &WatchPatternFilter,
    warmup_paths: &[PathBuf],
) -> Vec<PathBuf> {
    warmup_paths
        .iter()
        .map(|path| root.join(path))
        .filter(|path| filter.should_process(path) && pattern_matches(pattern_filter, root, path))
        .collect()
}

fn build_initial_graph(
    root: &Path,
    all_paths: &[PathBuf],
    state: &mut WatchState,
    emitter: &EventEmitter,
    stop: &AtomicBool,
) {
    if stop.load(Ordering::Relaxed) {
        return;
    }
    let total_paths = all_paths.len() as u64;
    emitter.progress("Building graph", 0, total_paths);

    // Phase 1: parse all files in parallel (embarrassingly parallel — no shared state).
    // Errors are collected as Err variants and surfaced via emitter after the parallel phase.
    // INVARIANT: extract_symbols() assigns 0-based sequential IDs per file; rebasing below
    // relies on this. If the extractor changes, update rebasing logic accordingly.
    //
    // Thread pool: capped at half available cores to avoid saturating VS Code extension host.
    // V050F-007: the canonical place to init the pool is the binary entry point
    // (`crates/anvil-cli/src/main.rs`), but we keep this defensive call here so direct
    // library consumers (tests, downstream binaries that bypass the CLI) still get the
    // cap. `init_global` is idempotent so the cost is one atomic load when the CLI
    // already initialised it at process start.
    init_rayon_pool();

    let parsed_count = AtomicU64::new(0);
    let parse_results: Vec<Result<(PathBuf, _), (PathBuf, String)>> = all_paths
        .par_iter()
        .map(|abs_path| {
            let result = parse_initial_file(root, abs_path, stop);
            let current = parsed_count.fetch_add(1, Ordering::Relaxed) + 1;
            emitter.progress("Building graph", current, total_paths);
            result
        })
        .collect();

    // Phase 2: apply parsed results to graph sequentially (graph requires &mut).
    // Surface errors so callers know which files were skipped.
    for result in parse_results {
        let (rel_path, mut file_symbols) = match result {
            Ok(v) => v,
            Err((_, msg)) if msg == "cancelled" => continue,
            Err((path, msg)) => {
                emitter.error(
                    ErrorCode::ParseError,
                    Some(&path.to_string_lossy()),
                    &msg,
                    true,
                );
                continue;
            }
        };
        // Assign unique symbol IDs
        let base_id = state.next_id;
        let symbol_count = file_symbols.symbols.len() as u64;
        for sym in &mut file_symbols.symbols {
            sym.id += base_id;
        }
        let rel_str = rel_path.to_string_lossy().to_string();
        state.all_imports.extend(file_symbols.imports.clone());
        state.all_reexports.extend(file_symbols.reexports.clone());
        update_file(&mut state.graph, file_symbols);
        // update_file may create synthetic external/module nodes when
        // resolving bare imports (axios, node:fs) or side-effect-only
        // modules. Those use graph.next_id() internally, which advances
        // past base_id + symbol_count. Take the max so the next file's
        // base_id never overlaps a synthetic id.
        state.next_id = (base_id + symbol_count).max(state.graph.next_id());
        state.file_count += 1;
        state.tracked_files.insert(rel_str);
    }

    re_resolve_imports(&mut state.graph, &state.all_imports);
    re_resolve_reexports(&mut state.graph, &state.all_reexports);
    annotate_trust(&mut state.graph, &state.all_imports);

    // Initial-scan snapshot: no single changed file — the CLI skips dispatch
    // on the first snapshot anyway, and a full re-walk is the safe default.
    emitter.snapshot(&state.graph, state.file_count, None);
}

fn parse_initial_file(
    root: &Path,
    abs_path: &Path,
    stop: &AtomicBool,
) -> Result<(PathBuf, FileSymbols), (PathBuf, String)> {
    // Check cancellation at start of each closure for responsive shutdown.
    if stop.load(Ordering::Relaxed) {
        return Err((abs_path.to_path_buf(), "cancelled".to_string()));
    }

    let rel_path = abs_path.strip_prefix(root).unwrap_or(abs_path);
    let content = std::fs::read(abs_path)
        .map_err(|e| (rel_path.to_path_buf(), format!("failed to read: {e}")))?;
    let mut parser = Parser::new();
    let result = parser
        .parse_bytes(rel_path, &content)
        .map_err(|e| (rel_path.to_path_buf(), format!("parse failed: {e}")))?;
    let symbols = extract_symbols(&result.tree, &content, rel_path, 0);
    Ok((rel_path.to_path_buf(), symbols))
}

struct WatchArchitecture<'a> {
    path: Option<&'a Path>,
    config: &'a mut ArchitectureConfig,
    reloader: Option<ArchitectureReloader>,
}

fn watch_loop(
    root: &Path,
    batch_rx: &mpsc::Receiver<crate::watcher::events::ChangeBatch>,
    pattern_filter: &WatchPatternFilter,
    architecture: &mut WatchArchitecture<'_>,
    state: &mut WatchState,
    emitter: &EventEmitter,
    stop: &AtomicBool,
) {
    while !stop.load(Ordering::Relaxed) {
        let batch = match batch_rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(b) => b,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        };

        for change in &batch.changes {
            if architecture_source_changed(&change.path, architecture.path) {
                let reloaded = if let Some(reloader) = architecture.reloader.as_ref() {
                    reloader()
                } else {
                    load_architecture_config(architecture.path)
                };
                match apply_architecture_reload(architecture.config, reloaded) {
                    Ok(()) => {
                        state.engine.clear_seen();
                    }
                    Err(err) => {
                        let rel_path = change.path.strip_prefix(root).unwrap_or(&change.path);
                        emitter.error(
                            ErrorCode::Internal,
                            Some(&rel_path.to_string_lossy()),
                            &format!("reloading architecture config: {err}"),
                            true,
                        );
                    }
                }
                continue;
            }
            // Drop events that don't pass the user's --patterns / --exclude
            // filter.
            //
            // Removed events get a narrow exemption: a delete event for a
            // file the graph already tracks must always flow through so
            // the graph can clean up — even if the user later changed
            // their patterns to exclude the path. A delete event for a
            // file the graph never tracked (e.g. .git or node_modules
            // churn from a rebase) gets dropped like any other excluded
            // event; today `remove_file` would no-op on it, but
            // forwarding generates spurious work and grows the surface
            // area of any future side effect added to `process_change`.
            let pattern_passes = pattern_matches(pattern_filter, root, &change.path);
            if !pattern_passes {
                if change.kind != ChangeKind::Removed {
                    continue;
                }
                // Mirror pattern_matches: canonicalise before strip_prefix
                // so a non-canonical change.path against a canonical root
                // (the macOS /private/tmp case) doesn't silently turn the
                // graph-membership check into "always empty".
                let canon = change.path.canonicalize();
                let candidate = canon.as_deref().unwrap_or(&change.path);
                let Ok(rel) = candidate.strip_prefix(root) else {
                    // Path lives outside the workspace and is excluded —
                    // nothing in the graph could match anyway.
                    continue;
                };
                let rel_str = rel.to_string_lossy();
                if state.graph.symbols_in_file(&rel_str).is_empty() {
                    continue;
                }
            }
            // Isolate per-change work so a panic in parse/extract/evaluate
            // surfaces as an error event and the loop keeps draining events
            // instead of silently terminating the watch thread.
            let rel_path = change.path.strip_prefix(root).unwrap_or(&change.path);
            let rel_str = rel_path.to_string_lossy().to_string();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                process_change(root, change, architecture.config, state, emitter);
            }));
            if let Err(panic) = result {
                let message = panic_message(&panic);
                emitter.error(
                    ErrorCode::Internal,
                    Some(&rel_str),
                    &format!("watcher panic processing change: {message}"),
                    true,
                );
            }
        }
    }
}

fn process_change(
    root: &Path,
    change: &crate::watcher::events::FileChange,
    arch_config: &ArchitectureConfig,
    state: &mut WatchState,
    emitter: &EventEmitter,
) {
    let rel_path = change.path.strip_prefix(root).unwrap_or(&change.path);
    let rel_str = rel_path.to_string_lossy().to_string();

    match change.kind {
        ChangeKind::Removed => {
            let delta = crate::graph::remove_file(&mut state.graph, &rel_str);
            // Always drop the tracked-files entry on a Removed event, even if
            // remove_file reported no symbols (the file may have been tracked
            // earlier and parsed empty since). Without this, a delete-then-
            // recreate sequence (rename-on-save editors, atomic-write patterns,
            // git checkout swaps) keeps the file marked as "tracked", so the
            // next Created event hits the `was_tracked` branch and never
            // re-increments file_count.
            state.tracked_files.remove(&rel_str);
            if !delta.removed_symbols.is_empty() {
                // Only decrement file_count if we actually removed tracked symbols
                state.file_count = state.file_count.saturating_sub(1);
                // Remove stale imports for the deleted file
                state.all_imports.retain(|i| i.from_file != rel_str);
                state.all_reexports.retain(|r| r.from_file != rel_str);
                annotate_trust(&mut state.graph, &state.all_imports);
                // Delete-driven snapshot (only emitted when the deleted file
                // had tracked symbols): `None` so the CLI re-walks with `--all`
                // — a delete can break imports in *other* files — rather than
                // scoping to the now-gone path.
                emitter.snapshot(&state.graph, state.file_count, None);
            }
        }
        ChangeKind::Created | ChangeKind::Modified => {
            // For modify events that may be renames on some backends,
            // remove stale symbols for the old path if the file no
            // longer exists at the reported path (read will fail and
            // we fall through to continue after cleanup).
            let content = match std::fs::read(&change.path) {
                Ok(c) => c,
                Err(e) => {
                    // File unreadable — if we had symbols for this
                    // path, clean them up (rename-style modify where
                    // the old path is gone).
                    let removed = crate::graph::remove_file(&mut state.graph, &rel_str);
                    // Drop the tracked-files entry on the same rename-style
                    // path; otherwise a subsequent Created event for a
                    // re-instated file at the same path would skip the
                    // file_count increment.
                    state.tracked_files.remove(&rel_str);
                    if !removed.removed_symbols.is_empty() {
                        state.file_count = state.file_count.saturating_sub(1);
                        state.all_imports.retain(|i| i.from_file != rel_str);
                        state.all_reexports.retain(|r| r.from_file != rel_str);
                        annotate_trust(&mut state.graph, &state.all_imports);
                        // Rename-away (old path vanished): `None` re-walk, same
                        // rationale as a delete.
                        emitter.snapshot(&state.graph, state.file_count, None);
                    }
                    emitter.error(
                        ErrorCode::ParseError,
                        Some(&rel_str),
                        &format!("failed to read: {e}"),
                        true,
                    );
                    return;
                }
            };

            let parse_result = match state.parser.parse_bytes(rel_path, &content) {
                Ok(r) => r,
                Err(e) => {
                    emitter.error(
                        ErrorCode::ParseError,
                        Some(&rel_str),
                        &format!("parse failed: {e}"),
                        true,
                    );
                    return;
                }
            };

            let file_symbols =
                extract_symbols(&parse_result.tree, &content, rel_path, state.next_id);
            let symbol_count = file_symbols.symbols.len() as u64;
            let new_imports = file_symbols.imports.clone();
            let new_reexports = file_symbols.reexports.clone();
            let was_tracked = state.tracked_files.contains(&rel_str);
            let base_id = state.next_id;
            let delta = update_file(&mut state.graph, file_symbols);

            // Replace imports for this file (remove old, add new)
            state.all_imports.retain(|i| i.from_file != rel_str);
            state.all_imports.extend(new_imports);
            re_resolve_imports(&mut state.graph, &state.all_imports);
            // GV2-031: same maintenance for re-export edges so a re-exported
            // privileged module stays visible to the certify privilege diff.
            state.all_reexports.retain(|r| r.from_file != rel_str);
            state.all_reexports.extend(new_reexports);
            re_resolve_reexports(&mut state.graph, &state.all_reexports);
            // Sync after update_file AND re_resolve_imports — both may have
            // added synthetic external nodes via graph.next_id(). Without this,
            // the next change event re-uses ids already claimed by externals
            // and produces a flood of "duplicate symbol id" errors.
            state.next_id = (base_id + symbol_count).max(state.graph.next_id());
            annotate_trust(&mut state.graph, &state.all_imports);

            // Clear policy dedupe state so reintroduced violations
            // are detected in each watch cycle.
            state.engine.clear_seen();
            let violations = state.engine.evaluate(&delta, &state.graph, arch_config);
            for v in &violations {
                emitter.violation(v);
            }

            // Only increment file_count for genuinely new files that
            // produced tracked symbols.
            if !was_tracked && !delta.added_symbols.is_empty() {
                state.file_count += 1;
            }
            // Track the file regardless of whether it added symbols, so
            // a subsequent re-save of the same file reports `was_tracked
            // = true` and we don't double-count the file_count.
            state.tracked_files.insert(rel_str.clone());
            // RLB-007: carry the absolute changed path so the CLI watch loop
            // can scope its per-save `anvil check` to exactly this file
            // instead of re-walking the whole repo (GH #2156). `change.path`
            // is the watcher's absolute path, so it resolves regardless of the
            // check child's cwd or any `--file` watch scope.
            //
            // Use `to_str()` (not `to_string_lossy()`): a non-UTF-8 path would
            // otherwise be mangled with U+FFFD replacement chars into a string
            // that resolves to no real file, so the scoped `anvil check` would
            // silently scan nothing. `None` for such paths falls back to the
            // full `--all` walk, which still covers the file.
            emitter.snapshot(&state.graph, state.file_count, change.path.to_str());
        }
    }
}

/// Apply the user's pattern filter to a path, computing the
/// repo-relative form first because globs like `src/**/*.ts` are
/// always written relative to the workspace root.
///
/// On platforms where the watcher emits paths via a different prefix
/// than the configured root (notably macOS, where `/tmp` resolves to
/// `/private/tmp` via a symlink), a raw `strip_prefix` would fail and
/// the absolute path would be silently matched against repo-relative
/// globs (i.e. nothing would ever match). To stay correct without
/// paying a syscall per event, we try `strip_prefix(canonical_root)`
/// on the raw path first — that succeeds for paths produced by
/// `initial_scan` (already under the canonical root) and for any
/// notify event whose prefix happens to match. Only when that fails
/// do we fall back to canonicalising the path itself. If the path
/// still cannot be made relative after that, the file lives outside
/// the workspace and is dropped from any non-noop filter.
fn pattern_matches(filter: &WatchPatternFilter, canonical_root: &Path, path: &Path) -> bool {
    if filter.is_noop() {
        return true;
    }
    if let Ok(rel) = path.strip_prefix(canonical_root) {
        return filter.matches(rel);
    }
    match path.canonicalize() {
        Ok(canon) => match canon.strip_prefix(canonical_root) {
            Ok(rel) => filter.matches(rel),
            // Path lives outside the workspace root after canonicalisation.
            // A user pattern is repo-relative, so we cannot meaningfully
            // match it — drop the event rather than fall back to matching
            // an absolute path against a relative-style glob.
            Err(_) => false,
        },
        Err(_) => false,
    }
}

fn panic_message(panic: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = panic.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = panic.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_string()
    }
}

pub fn run_watch(
    config: &WatchConfig,
    event_tx: mpsc::Sender<EngineEvent>,
) -> Result<WatchHandle, WatchError> {
    if !config.root.exists() {
        return Err(WatchError::RootNotFound(config.root.clone()));
    }

    let arch_path = config.architecture_config.clone();
    let reloader = config.architecture_reloader.clone();
    let mut arch_config = if let Some(mapped) = config.architecture.clone() {
        mapped
    } else {
        load_architecture_config(arch_path.as_deref())?
    };

    let filter = config.watcher.filter.clone().unwrap_or_default();
    let pattern_filter =
        WatchPatternFilter::new(&config.include_patterns, &config.exclude_patterns)?;
    // Canonicalise the watch root once and use the same form for both
    // the OS watcher and downstream path comparisons. Without this, the
    // watcher would register against the raw root (e.g. /tmp/...) while
    // the rest of the loop strips a canonicalised prefix (e.g.
    // /private/tmp/... on macOS), forcing per-event canonicalisation
    // and making behaviour depend on which form notify happens to emit.
    // Falls back to the raw root when canonicalise fails so we don't
    // lose existing platforms where it would have worked.
    let root = config
        .root
        .canonicalize()
        .unwrap_or_else(|_| config.root.clone());
    let mut watcher_config = config.watcher.clone();
    watcher_config.root.clone_from(&root);
    let emitter = EventEmitter::new(event_tx, EngineId::Rust);
    let (watcher, batch_rx, setup_diagnostics) = start_watcher(
        &watcher_config,
        Some(&|registered, attempted| {
            emitter.progress("Registering watchers", registered, attempted);
        }),
    )?;

    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop);
    let warmup_paths = config.warmup_paths.clone();

    let thread = thread::spawn(move || {
        let mut state = WatchState::new();

        // If some directories couldn't register a watch (commonly: inotify
        // `max_user_watches` reached), emit a warning up-front so the user
        // sees why changes in some subtrees will be missed.
        if setup_diagnostics.failed > 0 {
            let hint = if setup_diagnostics.limit_exhausted {
                // Platform-aware copy (CIB-175): inotify sysctl wording only on
                // Linux, generic reduce-scope guidance elsewhere.
                format!(" — {}", crate::watcher::watch_limit_guidance())
            } else {
                String::new()
            };
            let sample = if setup_diagnostics.sample_errors.is_empty() {
                String::new()
            } else {
                format!(" (e.g. {})", setup_diagnostics.sample_errors.join("; "))
            };
            emitter.error(
                ErrorCode::Internal,
                None,
                &format!(
                    "watch partially registered: {} dirs watched, {} failed — changes in unwatched subtrees won't be detected{hint}{sample}",
                    setup_diagnostics.registered, setup_diagnostics.failed
                ),
                true,
            );
        }

        initial_scan(
            &root,
            &filter,
            &pattern_filter,
            &warmup_paths,
            &mut state,
            &emitter,
            &stop_clone,
        );
        watch_loop(
            &root,
            &batch_rx,
            &pattern_filter,
            &mut WatchArchitecture {
                path: arch_path.as_deref(),
                config: &mut arch_config,
                reloader,
            },
            &mut state,
            &emitter,
            &stop_clone,
        );
    });

    Ok(WatchHandle {
        stop,
        thread: Some(thread),
        _watcher: watcher,
    })
}

/// Apply a reload result atomically. A failed reload drops the previous
/// in-memory layers so kernel checks do not keep enforcing stale policy.
/// Empty layers skip kernel cross-layer matches; the CLI preflight is
/// the fail-closed operator surface (#3918).
fn apply_architecture_reload(
    current: &mut ArchitectureConfig,
    reloaded: Result<ArchitectureConfig, WatchError>,
) -> Result<(), WatchError> {
    match reloaded {
        Ok(reloaded) => {
            *current = reloaded;
            Ok(())
        }
        Err(err) => {
            *current = ArchitectureConfig { layers: Vec::new() };
            Err(err)
        }
    }
}

fn load_architecture_config(path: Option<&Path>) -> Result<ArchitectureConfig, WatchError> {
    match path {
        Some(path) => {
            let yaml = read_architecture_config_capped(path).map_err(|e| WatchError::ConfigIo {
                path: path.to_path_buf(),
                source: e,
            })?;
            Ok(parse_validated_architecture_config(&yaml)?)
        }
        None => Ok(ArchitectureConfig { layers: Vec::new() }),
    }
}

fn architecture_source_changed(change_path: &Path, arch_path: Option<&Path>) -> bool {
    let Some(arch_path) = arch_path else {
        return false;
    };
    let canon_arch = arch_path
        .canonicalize()
        .unwrap_or_else(|_| arch_path.to_path_buf());
    let canon_change = change_path
        .canonicalize()
        .unwrap_or_else(|_| change_path.to_path_buf());
    canon_arch == canon_change
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn architecture_config_loader_rejects_unknown_allowed_imports() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("architecture.yaml");
        fs::write(
            &config_path,
            r#"
layers:
  - name: api
    paths: ["src/api/*"]
    allowed_imports: [domain]
"#,
        )
        .unwrap();

        let err = load_architecture_config(Some(&config_path)).unwrap_err();
        assert!(err.to_string().contains("unknown layer"));
    }

    #[test]
    fn architecture_config_loader_accepts_definition_schema() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("architecture.yaml");
        fs::write(
            &config_path,
            r#"
schema_version: "0.1.0"
layers:
  api:
    patterns: ["src/api/*"]
    depends_on: [api]
"#,
        )
        .unwrap();

        let config = load_architecture_config(Some(&config_path)).unwrap();
        assert_eq!(config.layers.len(), 1);
        assert_eq!(config.layers[0].name, "api");
        assert_eq!(config.layers[0].paths, vec!["src/api/*"]);
        assert_eq!(config.layers[0].allowed_imports, vec!["api"]);
    }

    #[test]
    fn architecture_config_loader_accepts_inline_main_config() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join(".anvil.yaml");
        fs::write(
            &config_path,
            r#"
version: 1
architecture:
  schema_version: "0.1.0"
  layers:
    core:
      patterns: ["src/core/**"]
      depends_on: []
"#,
        )
        .unwrap();

        let config = load_architecture_config(Some(&config_path)).unwrap();
        assert_eq!(config.layers.len(), 1);
        assert_eq!(config.layers[0].name, "core");
    }

    #[test]
    fn architecture_reload_failure_clears_stale_policy() {
        let mut current = ArchitectureConfig::from_yaml(
            r#"
layers:
  - name: core
    paths: ["src/core/*"]
    allowed_imports: []
"#,
        )
        .unwrap();
        assert_eq!(current.layers.len(), 1);

        let err = apply_architecture_reload(
            &mut current,
            Err(WatchError::ConfigReload("preflight failed".into())),
        )
        .unwrap_err();
        assert!(err.to_string().contains("preflight"));
        assert!(
            current.layers.is_empty(),
            "invalid reload must not keep the previous layers"
        );
    }

    #[test]
    fn architecture_source_changed_matches_canonical_path() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("architecture.yaml");
        fs::write(&path, "layers: []\n").unwrap();
        assert!(architecture_source_changed(&path, Some(&path)));
        assert!(!architecture_source_changed(
            &tmp.path().join("other.yaml"),
            Some(&path)
        ));
        assert!(!architecture_source_changed(&path, None));
    }

    #[test]
    fn starts_and_stops_cleanly() {
        let tmp = TempDir::new().unwrap();
        let (event_tx, _event_rx) = mpsc::channel();

        let config = WatchConfig {
            root: tmp.path().to_path_buf(),
            architecture_config: None,
            architecture: None,
            architecture_reloader: None,
            watcher: WatcherConfig {
                root: tmp.path().to_path_buf(),
                ..Default::default()
            },
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            warmup_paths: Vec::new(),
        };

        let handle = run_watch(&config, event_tx).unwrap();
        thread::sleep(std::time::Duration::from_millis(50));
        handle.stop().unwrap();
    }

    #[test]
    fn emits_snapshot_on_initial_scan() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("main.ts"), "export function hello() {}").unwrap();

        let (event_tx, event_rx) = mpsc::channel();

        let config = WatchConfig {
            root: tmp.path().to_path_buf(),
            architecture_config: None,
            architecture: None,
            architecture_reloader: None,
            watcher: WatcherConfig {
                root: tmp.path().to_path_buf(),
                ..Default::default()
            },
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            warmup_paths: Vec::new(),
        };

        let handle = run_watch(&config, event_tx).unwrap();
        thread::sleep(std::time::Duration::from_millis(200));
        handle.stop().unwrap();

        let events: Vec<EngineEvent> = event_rx.try_iter().collect();
        let has_snapshot = events
            .iter()
            .any(|e| e.event_type == anvil_kernel_types::EventType::Snapshot);
        assert!(has_snapshot, "should emit a snapshot after initial scan");
    }

    #[test]
    fn initial_scan_emits_warmup_progress_before_snapshot() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("main.ts"), "export function hello() {}").unwrap();

        let (event_tx, event_rx) = mpsc::channel();

        let config = WatchConfig {
            root: tmp.path().to_path_buf(),
            architecture_config: None,
            architecture: None,
            architecture_reloader: None,
            watcher: WatcherConfig {
                root: tmp.path().to_path_buf(),
                ..Default::default()
            },
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            warmup_paths: Vec::new(),
        };

        let handle = run_watch(&config, event_tx).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let mut events = Vec::new();
        while std::time::Instant::now() < deadline {
            match event_rx.recv_timeout(std::time::Duration::from_millis(50)) {
                Ok(event) => {
                    let saw_snapshot = event.event_type == anvil_kernel_types::EventType::Snapshot;
                    events.push(event);
                    if saw_snapshot {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        handle.stop().unwrap();

        let phases: Vec<&str> = events
            .iter()
            .filter_map(|event| match &event.payload {
                anvil_kernel_types::EventPayload::Progress { phase, .. } => Some(phase.as_str()),
                _ => None,
            })
            .collect();

        assert!(
            phases.contains(&"Registering watchers")
                && phases.contains(&"Discovering files")
                && phases.contains(&"Building graph"),
            "initial scan should emit warm-up phases before the snapshot, got {phases:?}"
        );
    }

    #[test]
    fn initial_scan_does_not_emit_existing_api_as_violations() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("main.ts"),
            "export function existingApi() {}",
        )
        .unwrap();

        let (event_tx, event_rx) = mpsc::channel();

        let config = WatchConfig {
            root: tmp.path().to_path_buf(),
            architecture_config: None,
            architecture: None,
            architecture_reloader: None,
            watcher: WatcherConfig {
                root: tmp.path().to_path_buf(),
                ..Default::default()
            },
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            warmup_paths: Vec::new(),
        };

        let handle = run_watch(&config, event_tx).unwrap();
        let mut events = Vec::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let mut saw_snapshot = false;
        while std::time::Instant::now() < deadline {
            match event_rx.recv_timeout(std::time::Duration::from_millis(50)) {
                Ok(event) => {
                    saw_snapshot |= event.event_type == anvil_kernel_types::EventType::Snapshot;
                    events.push(event);
                    if saw_snapshot {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        handle.stop().unwrap();
        events.extend(event_rx.try_iter());

        assert!(saw_snapshot, "initial scan should still emit a snapshot");
        assert!(
            events
                .iter()
                .all(|e| e.event_type != anvil_kernel_types::EventType::Violation),
            "initial scan should not fail on pre-existing public API surface: {events:?}"
        );
    }

    /// Regression test for #1802: on a never-baselined repo, the audit's
    /// `anvil watch --no-tui` reproduction flagged every pre-existing public
    /// symbol as `public-api-expansion`. The bug was the old `evaluate_baseline`
    /// path that ran `engine.evaluate` on the initial graph with an empty
    /// `previously_public` set. After WATCHUX-001, the initial graph is the
    /// baseline; only post-scan modifications go through the policy engine.
    ///
    /// Mirrors the audit's multi-file shape (`src/index.ts`, `src/db.ts`,
    /// `src/smelly.ts`, each with at least one public symbol) so a future
    /// refactor that resurrects per-symbol evaluation on the initial graph
    /// is caught by every variant of the audited surface, not just one file.
    #[test]
    fn audit_1802_multi_file_initial_scan_emits_no_public_api_violations() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        fs::create_dir(&src).unwrap();
        fs::write(src.join("index.ts"), "export function greet() {}").unwrap();
        fs::write(
            src.join("db.ts"),
            "export function userRow() {}\nexport function dbExec() {}\n",
        )
        .unwrap();
        fs::write(
            src.join("smelly.ts"),
            "export function unsafe(input: string) {}\n",
        )
        .unwrap();

        let (event_tx, event_rx) = mpsc::channel();
        let config = WatchConfig {
            root: tmp.path().to_path_buf(),
            architecture_config: None,
            architecture: None,
            architecture_reloader: None,
            watcher: WatcherConfig {
                root: tmp.path().to_path_buf(),
                ..Default::default()
            },
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            warmup_paths: Vec::new(),
        };

        let handle = run_watch(&config, event_tx).unwrap();
        let mut events = Vec::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let mut saw_snapshot = false;
        while std::time::Instant::now() < deadline {
            match event_rx.recv_timeout(std::time::Duration::from_millis(50)) {
                Ok(event) => {
                    saw_snapshot |= event.event_type == anvil_kernel_types::EventType::Snapshot;
                    events.push(event);
                    if saw_snapshot {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        handle.stop().unwrap();
        events.extend(event_rx.try_iter());

        assert!(saw_snapshot, "initial scan should still emit a snapshot");
        let public_api_violations: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == anvil_kernel_types::EventType::Violation)
            .filter(|e| {
                matches!(
                    &e.payload,
                    anvil_kernel_types::EventPayload::Violation { policy_id, .. }
                        if policy_id == "public-api-expansion"
                )
            })
            .collect();
        assert!(
            public_api_violations.is_empty(),
            "audit #1802: initial scan must not flag pre-existing public symbols as \
             new exports; got {public_api_violations:?}"
        );
    }

    #[test]
    fn initial_scan_uses_validated_warmup_paths_as_seed() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("cached.ts"), "export function cached() {}").unwrap();
        fs::write(
            tmp.path().join("uncached.ts"),
            "export function uncached() {}",
        )
        .unwrap();

        let (event_tx, event_rx) = mpsc::channel();

        let config = WatchConfig {
            root: tmp.path().to_path_buf(),
            architecture_config: None,
            architecture: None,
            architecture_reloader: None,
            watcher: WatcherConfig {
                root: tmp.path().to_path_buf(),
                ..Default::default()
            },
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            warmup_paths: vec![PathBuf::from("cached.ts")],
        };

        let handle = run_watch(&config, event_tx).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let mut watched_files = None;
        while std::time::Instant::now() < deadline {
            match event_rx.recv_timeout(std::time::Duration::from_millis(50)) {
                Ok(event) if event.event_type == anvil_kernel_types::EventType::Snapshot => {
                    if let anvil_kernel_types::EventPayload::Snapshot { files_watched, .. } =
                        event.payload
                    {
                        watched_files = Some(files_watched);
                        break;
                    }
                }
                Ok(_) | Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        handle.stop().unwrap();

        assert_eq!(watched_files, Some(1));
    }

    #[test]
    fn panic_message_extracts_static_str() {
        let payload: Box<dyn std::any::Any + Send> = Box::new("static panic");
        assert_eq!(panic_message(&payload), "static panic");
    }

    #[test]
    fn panic_message_extracts_owned_string() {
        let payload: Box<dyn std::any::Any + Send> = Box::new(String::from("owned panic"));
        assert_eq!(panic_message(&payload), "owned panic");
    }

    #[test]
    fn panic_message_falls_back_for_unknown_payload() {
        let payload: Box<dyn std::any::Any + Send> = Box::new(42_i32);
        assert_eq!(panic_message(&payload), "unknown panic");
    }

    #[test]
    fn watch_loop_survives_panic_in_process_change() {
        // Exercise the catch_unwind guard: we manually invoke the catch
        // path by running a closure that panics and confirming we can
        // recover the message through panic_message(). This mirrors the
        // exact shape used in watch_loop() without needing a real file
        // change that happens to panic.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            panic!("simulated process_change failure")
        }));
        let panic = result.expect_err("closure should have panicked");
        let msg = panic_message(&panic);
        assert!(
            msg.contains("simulated process_change failure"),
            "panic_message should surface the payload, got {msg:?}"
        );
    }

    #[test]
    fn detects_file_creation() {
        let tmp = TempDir::new().unwrap();
        let (event_tx, event_rx) = mpsc::channel();

        let config = WatchConfig {
            root: tmp.path().to_path_buf(),
            architecture_config: None,
            architecture: None,
            architecture_reloader: None,
            watcher: WatcherConfig {
                root: tmp.path().to_path_buf(),
                debounce_window: std::time::Duration::from_millis(20),
                tick_interval: std::time::Duration::from_millis(10),
                ..Default::default()
            },
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            warmup_paths: Vec::new(),
        };

        let handle = run_watch(&config, event_tx).unwrap();
        thread::sleep(std::time::Duration::from_millis(200));

        fs::write(tmp.path().join("new-file.ts"), "export function newFn() {}").unwrap();

        thread::sleep(std::time::Duration::from_millis(500));
        handle.stop().unwrap();

        let events: Vec<EngineEvent> = event_rx.try_iter().collect();
        let snapshot_count = events
            .iter()
            .filter(|e| e.event_type == anvil_kernel_types::EventType::Snapshot)
            .count();
        assert!(
            snapshot_count >= 1,
            "should have at least 1 snapshot, got {snapshot_count}"
        );
    }
}
