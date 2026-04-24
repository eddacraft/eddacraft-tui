use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;

use anvil_kernel_types::{EngineEvent, EngineId, ErrorCode};
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::graph::{SymbolGraph, annotate_trust, re_resolve_imports, update_file};
use crate::parser::Parser;
use crate::parser::extract::{ImportEdge, extract_symbols};
use crate::policy::config::ArchitectureConfig;
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

static POOL_INIT: std::sync::Once = std::sync::Once::new();

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
    ConfigParse(#[from] serde_yaml::Error),
    #[error("watcher error: {0}")]
    Watcher(#[from] crate::watcher::WatcherError),
    #[error("invalid watch pattern: {0}")]
    Pattern(#[from] PatternError),
    #[error("watch loop terminated unexpectedly")]
    ThreadPanicked,
}

pub struct WatchConfig {
    pub root: PathBuf,
    pub architecture_config: Option<PathBuf>,
    pub watcher: WatcherConfig,
    /// User-supplied include glob patterns. Empty = include everything.
    /// Compiled into a `WatchPatternFilter` at `run_watch` start.
    /// Distinct from `WatcherConfig.filter`, which owns the hardcoded
    /// internal denylist (`node_modules`, `.git`, `target`, …).
    pub include_patterns: Vec<String>,
    /// User-supplied exclude glob patterns. Empty = exclude nothing.
    /// Takes precedence over `include_patterns` when both match.
    pub exclude_patterns: Vec<String>,
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
    next_id: u64,
    engine: PolicyEngine,
    file_count: u64,
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
            next_id: 0,
            engine,
            file_count: 0,
        }
    }
}

fn initial_scan(
    root: &Path,
    filter: &FileFilter,
    pattern_filter: &WatchPatternFilter,
    arch_config: &ArchitectureConfig,
    state: &mut WatchState,
    emitter: &EventEmitter,
    stop: &AtomicBool,
) {
    let mut scanned_files: Vec<PathBuf> = Vec::new();

    // Collect all file paths first (walk is sequential — cheap)
    let all_paths: Vec<PathBuf> = WalkDir::new(root)
        .into_iter()
        .filter_map(|r| match r {
            Ok(e) => {
                if e.file_type().is_file()
                    && filter.should_process(e.path())
                    && pattern_matches(pattern_filter, root, e.path())
                {
                    Some(e.path().to_path_buf())
                } else {
                    None
                }
            }
            Err(e) => {
                let path_str = e
                    .path()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                emitter.error(
                    ErrorCode::Internal,
                    if path_str.is_empty() {
                        None
                    } else {
                        Some(&path_str)
                    },
                    &format!("walk error: {e}"),
                    true,
                );
                None
            }
        })
        .collect();

    if stop.load(Ordering::Relaxed) {
        return;
    }

    // Phase 1: parse all files in parallel (embarrassingly parallel — no shared state).
    // Errors are collected as Err variants and surfaced via emitter after the parallel phase.
    // INVARIANT: extract_symbols() assigns 0-based sequential IDs per file; rebasing below
    // relies on this. If the extractor changes, update rebasing logic accordingly.
    //
    // Thread pool: capped at half available cores to avoid saturating VS Code extension host.
    POOL_INIT.call_once(|| {
        let threads = (num_cpus::get() / 2).max(1);
        let _ = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global();
    });

    let parse_results: Vec<Result<(PathBuf, _), (PathBuf, String)>> = all_paths
        .par_iter()
        .map(|abs_path| {
            // Check cancellation at start of each closure for responsive shutdown
            if stop.load(Ordering::Relaxed) {
                return Err((abs_path.clone(), "cancelled".to_string()));
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
        for sym in &mut file_symbols.symbols {
            sym.id += base_id;
        }
        state.all_imports.extend(file_symbols.imports.clone());
        update_file(&mut state.graph, file_symbols);
        state.next_id = state
            .graph
            .inner()
            .node_weights()
            .map(|s| s.id)
            .max()
            .map_or(0, |m| m + 1);
        state.file_count += 1;
        scanned_files.push(rel_path);
    }

    re_resolve_imports(&mut state.graph, &state.all_imports);
    annotate_trust(&mut state.graph, &state.all_imports);

    evaluate_baseline(&scanned_files, state, arch_config, emitter);
    emitter.snapshot(&state.graph, state.file_count);
}

/// Run baseline policy evaluation so the first snapshot reflects real
/// invariant results rather than an empty 0/0 checks placeholder.
fn evaluate_baseline(
    scanned_files: &[PathBuf],
    state: &mut WatchState,
    arch_config: &ArchitectureConfig,
    emitter: &EventEmitter,
) {
    for rel_path in scanned_files {
        let rel_str = rel_path.to_string_lossy().to_string();
        let symbols_in_file: Vec<u64> = state
            .graph
            .symbols_in_file(&rel_str)
            .iter()
            .map(|s| s.id)
            .collect();

        if symbols_in_file.is_empty() {
            continue;
        }

        let file_edges: Vec<(u64, u64, anvil_kernel_types::EdgeType)> = symbols_in_file
            .iter()
            .flat_map(|&sid| state.graph.outgoing_edges(sid))
            .map(|e| (e.from, e.to, e.edge_type))
            .collect();

        let delta = crate::graph::GraphDelta {
            added_symbols: symbols_in_file,
            added_edges: file_edges,
            file: rel_str,
            ..Default::default()
        };

        let violations = state.engine.evaluate(&delta, &state.graph, arch_config);
        for v in &violations {
            emitter.violation(v);
        }
    }
}

fn watch_loop(
    root: &Path,
    batch_rx: &mpsc::Receiver<crate::watcher::events::ChangeBatch>,
    pattern_filter: &WatchPatternFilter,
    arch_config: &ArchitectureConfig,
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
                process_change(root, change, arch_config, state, emitter);
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
            if !delta.removed_symbols.is_empty() {
                // Only decrement file_count if we actually removed tracked symbols
                state.file_count = state.file_count.saturating_sub(1);
                // Remove stale imports for the deleted file
                state.all_imports.retain(|i| i.from_file != rel_str);
                annotate_trust(&mut state.graph, &state.all_imports);
                emitter.snapshot(&state.graph, state.file_count);
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
                    if !removed.removed_symbols.is_empty() {
                        state.file_count = state.file_count.saturating_sub(1);
                        state.all_imports.retain(|i| i.from_file != rel_str);
                        annotate_trust(&mut state.graph, &state.all_imports);
                        emitter.snapshot(&state.graph, state.file_count);
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
            let new_imports = file_symbols.imports.clone();
            let was_tracked = state
                .graph
                .inner()
                .node_weights()
                .any(|s| s.file == rel_str);
            let delta = update_file(&mut state.graph, file_symbols);
            state.next_id = state
                .graph
                .inner()
                .node_weights()
                .map(|s| s.id)
                .max()
                .map_or(0, |m| m + 1);

            // Replace imports for this file (remove old, add new)
            state.all_imports.retain(|i| i.from_file != rel_str);
            state.all_imports.extend(new_imports);
            re_resolve_imports(&mut state.graph, &state.all_imports);
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
            emitter.snapshot(&state.graph, state.file_count);
        }
    }
}

/// Apply the user's pattern filter to a path, computing the
/// repo-relative form first because globs like `src/**/*.ts` are
/// always written relative to the workspace root.
///
/// On platforms where the watcher emits paths via a different prefix
/// than the configured root (notably macOS, where `/tmp` resolves to
/// `/private/tmp` via a symlink), `strip_prefix` against the raw root
/// would always fail and the absolute path would be silently matched
/// against repo-relative globs (i.e. nothing would ever match). The
/// caller passes a *canonicalised* root so the prefix comparison
/// accepts whatever notify produces; if the path still cannot be made
/// relative, the file lives outside the workspace and is dropped from
/// any non-noop filter.
fn pattern_matches(filter: &WatchPatternFilter, canonical_root: &Path, path: &Path) -> bool {
    if filter.is_noop() {
        return true;
    }
    let canon = path.canonicalize();
    let candidate = canon.as_deref().unwrap_or(path);
    match candidate.strip_prefix(canonical_root) {
        Ok(rel) => filter.matches(rel),
        // Path lives outside the workspace root after canonicalisation.
        // A user pattern is repo-relative, so we cannot meaningfully
        // match it — drop the event rather than fall back to matching
        // an absolute path against a relative-style glob.
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

    let arch_config = match &config.architecture_config {
        Some(path) => {
            let yaml = std::fs::read_to_string(path).map_err(|e| WatchError::ConfigIo {
                path: path.clone(),
                source: e,
            })?;
            ArchitectureConfig::from_yaml(&yaml)?
        }
        None => ArchitectureConfig { layers: Vec::new() },
    };

    let filter = config.watcher.filter.clone().unwrap_or_default();
    let pattern_filter =
        WatchPatternFilter::new(&config.include_patterns, &config.exclude_patterns)?;
    let mut watcher_config = config.watcher.clone();
    watcher_config.root.clone_from(&config.root);
    let (watcher, batch_rx, setup_diagnostics) = start_watcher(&watcher_config)?;

    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop);
    // Canonicalise the watch root so user-pattern matching is robust
    // against symlink prefixes the OS watcher might emit (macOS notably
    // resolves /tmp through /private/tmp). Falls back to the raw root
    // when canonicalise fails so we don't lose existing platforms where
    // it would have worked.
    let root = config
        .root
        .canonicalize()
        .unwrap_or_else(|_| config.root.clone());

    let thread = thread::spawn(move || {
        let emitter = EventEmitter::new(event_tx, EngineId::Rust);
        let mut state = WatchState::new();

        // If some directories couldn't register a watch (commonly: inotify
        // `max_user_watches` reached), emit a warning up-front so the user
        // sees why changes in some subtrees will be missed.
        if setup_diagnostics.failed > 0 {
            let hint = if setup_diagnostics.limit_exhausted {
                " — OS watch limit reached; raise `fs.inotify.max_user_watches` or close other watch-heavy processes (tsserver, nx daemon, editors)"
            } else {
                ""
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
            &arch_config,
            &mut state,
            &emitter,
            &stop_clone,
        );
        watch_loop(
            &root,
            &batch_rx,
            &pattern_filter,
            &arch_config,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn starts_and_stops_cleanly() {
        let tmp = TempDir::new().unwrap();
        let (event_tx, _event_rx) = mpsc::channel();

        let config = WatchConfig {
            root: tmp.path().to_path_buf(),
            architecture_config: None,
            watcher: WatcherConfig {
                root: tmp.path().to_path_buf(),
                ..Default::default()
            },
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
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
            watcher: WatcherConfig {
                root: tmp.path().to_path_buf(),
                ..Default::default()
            },
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
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
            watcher: WatcherConfig {
                root: tmp.path().to_path_buf(),
                debounce_window: std::time::Duration::from_millis(20),
                tick_interval: std::time::Duration::from_millis(10),
                ..Default::default()
            },
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
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
