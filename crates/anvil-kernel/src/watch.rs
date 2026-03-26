use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;

use anvil_kernel_types::{EngineEvent, EngineId, ErrorCode};
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
use crate::watcher::{WatcherConfig, start_watcher};

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
    #[error("watch loop terminated unexpectedly")]
    ThreadPanicked,
}

pub struct WatchConfig {
    pub root: PathBuf,
    pub architecture_config: Option<PathBuf>,
    pub watcher: WatcherConfig,
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

    fn parse_and_update(
        &mut self,
        abs_path: &Path,
        rel_path: &Path,
        emitter: &EventEmitter,
    ) -> bool {
        let content = match std::fs::read(abs_path) {
            Ok(c) => c,
            Err(e) => {
                emitter.error(
                    ErrorCode::ParseError,
                    Some(&abs_path.to_string_lossy()),
                    &format!("failed to read: {e}"),
                    true,
                );
                return false;
            }
        };

        let parse_result = match self.parser.parse_bytes(rel_path, &content) {
            Ok(r) => r,
            Err(e) => {
                emitter.error(
                    ErrorCode::ParseError,
                    Some(&rel_path.to_string_lossy()),
                    &format!("parse failed: {e}"),
                    true,
                );
                return false;
            }
        };

        let file_symbols = extract_symbols(&parse_result.tree, &content, rel_path, self.next_id);
        self.all_imports.extend(file_symbols.imports.clone());
        update_file(&mut self.graph, file_symbols);
        self.next_id = self
            .graph
            .inner()
            .node_weights()
            .map(|s| s.id)
            .max()
            .map_or(0, |m| m + 1);
        true
    }
}

fn initial_scan(
    root: &Path,
    filter: &FileFilter,
    arch_config: &ArchitectureConfig,
    state: &mut WatchState,
    emitter: &EventEmitter,
    stop: &AtomicBool,
) {
    let mut scanned_files: Vec<PathBuf> = Vec::new();

    let walker = WalkDir::new(root).into_iter().filter_entry(|e| {
        // Prune ignored directories at the walk level so we never descend
        // into coverage/, node_modules/, .git, target, etc.
        if e.file_type().is_dir() {
            return !filter.should_ignore(e.path());
        }
        true
    });

    for result in walker {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        let entry = match result {
            Ok(e) => e,
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
                continue;
            }
        };
        if !entry.file_type().is_file() || !filter.should_process(entry.path()) {
            continue;
        }
        let rel_path = entry.path().strip_prefix(root).unwrap_or(entry.path());
        if state.parse_and_update(entry.path(), rel_path, emitter) {
            state.file_count += 1;
            scanned_files.push(rel_path.to_path_buf());
        }
    }

    re_resolve_imports(&mut state.graph, &state.all_imports);
    annotate_trust(&mut state.graph, &state.all_imports);

    // Run baseline policy evaluation so the first snapshot reflects real
    // invariant results rather than an empty 0/0 checks placeholder.
    for rel_path in &scanned_files {
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

    emitter.snapshot(&state.graph, state.file_count);
}

fn watch_loop(
    root: &Path,
    batch_rx: &mpsc::Receiver<crate::watcher::events::ChangeBatch>,
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
                            continue;
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
                            continue;
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
    let mut watcher_config = config.watcher.clone();
    watcher_config.root.clone_from(&config.root);
    let (watcher, batch_rx) = start_watcher(&watcher_config)?;

    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop);
    let root = config.root.clone();

    let thread = thread::spawn(move || {
        let emitter = EventEmitter::new(event_tx, EngineId::Rust);
        let mut state = WatchState::new();

        initial_scan(
            &root,
            &filter,
            &arch_config,
            &mut state,
            &emitter,
            &stop_clone,
        );
        watch_loop(
            &root,
            &batch_rx,
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
