use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;

use anvil_kernel_types::{EngineEvent, EngineId, ErrorCode};
use walkdir::WalkDir;

use crate::graph::{SymbolGraph, annotate_trust, update_file};
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
    _watcher: notify::RecommendedWatcher,
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
        self.next_id += file_symbols.symbols.len() as u64;
        self.all_imports.extend(file_symbols.imports.clone());
        update_file(&mut self.graph, file_symbols);
        true
    }
}

fn initial_scan(
    root: &Path,
    filter: &FileFilter,
    state: &mut WatchState,
    emitter: &EventEmitter,
    stop: &AtomicBool,
) {
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        if !entry.file_type().is_file() || !filter.should_process(entry.path()) {
            continue;
        }
        let rel_path = entry.path().strip_prefix(root).unwrap_or(entry.path());
        if state.parse_and_update(entry.path(), rel_path, emitter) {
            state.file_count += 1;
        }
    }

    annotate_trust(&mut state.graph, &state.all_imports);
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
                    if !delta.is_empty() {
                        emitter.snapshot(&state.graph, state.file_count);
                    }
                    state.file_count = state.file_count.saturating_sub(1);
                }
                ChangeKind::Created | ChangeKind::Modified => {
                    let content = match std::fs::read(&change.path) {
                        Ok(c) => c,
                        Err(e) => {
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
                    state.next_id += file_symbols.symbols.len() as u64;
                    let new_imports = file_symbols.imports.clone();
                    let delta = update_file(&mut state.graph, file_symbols);

                    state.all_imports.extend(new_imports);
                    annotate_trust(&mut state.graph, &state.all_imports);

                    let violations = state.engine.evaluate(&delta, &state.graph, arch_config);
                    for v in &violations {
                        emitter.violation(v);
                    }

                    if change.kind == ChangeKind::Created {
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
    let (watcher, batch_rx) = start_watcher(&config.watcher)?;

    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop);
    let root = config.root.clone();

    let thread = thread::spawn(move || {
        let emitter = EventEmitter::new(event_tx, EngineId::Rust);
        let mut state = WatchState::new();

        initial_scan(&root, &filter, &mut state, &emitter, &stop_clone);
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
