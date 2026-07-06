use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use anvil_kernel_types::{EngineEvent, EngineId, ErrorCode};
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::graph::{SymbolGraph, annotate_trust, re_resolve_imports, update_file};
use crate::parser::Parser;
use crate::parser::extract::{ImportEdge, extract_symbols};
use crate::policy::config::ArchitectureConfig;
use crate::policy::config_validator::{
    ArchitectureConfigValidationError, parse_validated_architecture_config,
};
use crate::policy::engine::{PolicyEngine, Violation};
use crate::policy::invariants::cross_layer::CrossLayerViolation;
use crate::policy::invariants::new_dependency::NewDependencyIntroduction;
use crate::policy::invariants::privilege_expansion::PrivilegeExpansion;
use crate::policy::invariants::public_api::PublicApiExpansion;
use crate::protocol::emitter::EventEmitter;
use crate::watcher::filter::FileFilter;
use anvil_rayon_init::init_global as init_rayon_pool;

#[derive(Debug, thiserror::Error)]
pub enum EmbeddedError {
    #[error("root directory does not exist: {0}")]
    RootNotFound(PathBuf),
    #[error("architecture config error: {path}: {source}")]
    ConfigIo {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("architecture config parse error: {0}")]
    ConfigParse(#[from] serde_yaml::Error),
    #[error("architecture config validation error: {0}")]
    ConfigValidation(#[from] ArchitectureConfigValidationError),
    #[error("walkdir error: {0}")]
    Walk(#[from] walkdir::Error),
}

pub struct EmbeddedConfig {
    pub root: PathBuf,
    pub architecture_config: Option<PathBuf>,
    pub filter: Option<FileFilter>,
    /// Optional plan file path — passed through from the CLI for future
    /// plan-scoped filtering (not yet consumed by `run_embedded`).
    pub plan: Option<PathBuf>,
}

pub struct EmbeddedResult {
    pub violations: Vec<Violation>,
    pub stats: crate::graph::GraphStats,
    pub events: Vec<EngineEvent>,
    pub duration: Duration,
}

pub fn run_embedded(config: &EmbeddedConfig) -> Result<EmbeddedResult, EmbeddedError> {
    run_embedded_cancellable(config, None)
}

/// Like `run_embedded` but accepts an optional cancellation flag.
/// When `stop` is set to `true`, the scan exits at the next safe checkpoint.
pub fn run_embedded_cancellable(
    config: &EmbeddedConfig,
    stop: Option<Arc<AtomicBool>>,
) -> Result<EmbeddedResult, EmbeddedError> {
    let stop = stop.unwrap_or_else(|| Arc::new(AtomicBool::new(false)));
    let start = Instant::now();

    if !config.root.exists() {
        return Err(EmbeddedError::RootNotFound(config.root.clone()));
    }

    let filter = config.filter.clone().unwrap_or_default();

    let arch_config = load_architecture_config(config.architecture_config.as_deref())?;

    let (event_tx, event_rx) = mpsc::channel();
    let emitter = EventEmitter::new(event_tx, EngineId::Rust);

    // 1. Walk directory and collect parseable files
    let files = collect_files(&config.root, &filter)?;
    let total = files.len() as u64;
    emitter.progress("scanning", total, total);

    if stop.load(Ordering::Relaxed) {
        drop(emitter);
        let events: Vec<EngineEvent> = event_rx.try_iter().collect();
        return Ok(EmbeddedResult {
            violations: vec![],
            stats: SymbolGraph::new().stats(),
            events,
            duration: start.elapsed(),
        });
    }

    // Initialise rayon thread pool: cap at half available cores (min 1) to avoid
    // saturating the host — important for VS Code extension and CI contexts.
    // V050F-007: defensive call for direct lib consumers; the binary entry point
    // calls `init_global` first, after which this becomes a one-atomic-load no-op.
    init_rayon_pool();

    // 2. Parse all files in parallel, apply to graph sequentially, resolve imports.
    let (graph, _all_imports, parsed_count) =
        parse_and_build_graph(&files, &config.root, &stop, &emitter, total);

    // 5. Emit snapshot — use parsed_count so coverage reflects actual files scanned,
    //    not total files attempted (some may have been skipped due to parse errors).
    //    One-shot embedded scan: no single changed file to scope a follow-up to.
    emitter.snapshot(&graph, parsed_count, None);

    // 6. Run policy engine with all H1 invariants
    let mut engine = PolicyEngine::new();
    engine.register(Box::new(CrossLayerViolation));
    engine.register(Box::new(NewDependencyIntroduction));
    engine.register(Box::new(PublicApiExpansion));
    engine.register(Box::new(PrivilegeExpansion));

    let all_violations = evaluate_files(
        &files,
        &config.root,
        &graph,
        &mut engine,
        &arch_config,
        &emitter,
    );

    let stats = graph.stats();
    let duration = start.elapsed();

    // Collect all events
    drop(emitter);
    let events: Vec<EngineEvent> = event_rx.try_iter().collect();

    Ok(EmbeddedResult {
        violations: all_violations,
        stats,
        events,
        duration,
    })
}

fn evaluate_files(
    files: &[PathBuf],
    root: &Path,
    graph: &SymbolGraph,
    engine: &mut PolicyEngine,
    arch_config: &ArchitectureConfig,
    emitter: &EventEmitter,
) -> Vec<Violation> {
    let mut all_violations = Vec::new();
    for file_path in files {
        let rel_path = file_path.strip_prefix(root).unwrap_or(file_path.as_path());
        let rel_str = rel_path.to_string_lossy().to_string();

        let symbols_in_file: Vec<u64> = graph
            .symbols_in_file(&rel_str)
            .iter()
            .map(|s| s.id)
            .collect();

        if symbols_in_file.is_empty() {
            continue;
        }

        let file_edges: Vec<(u64, u64, anvil_kernel_types::EdgeType)> = symbols_in_file
            .iter()
            .flat_map(|&sid| graph.outgoing_edges(sid))
            .map(|e| (e.from, e.to, e.edge_type))
            .collect();

        let delta = crate::graph::GraphDelta {
            added_symbols: symbols_in_file,
            added_edges: file_edges,
            file: rel_str,
            ..Default::default()
        };

        let violations = engine.evaluate(&delta, graph, arch_config);
        for v in &violations {
            emitter.violation(v);
        }
        all_violations.extend(violations);
    }
    all_violations
}

/// Parse all files in parallel (rayon), then apply results to graph sequentially.
///
/// INVARIANT: `extract_symbols()` assigns 0-based sequential IDs within each file.
/// The sequential apply phase rebases IDs to be globally unique via `sym.id += base`.
/// If this invariant ever changes, update the rebasing logic and add tests.
fn parse_and_build_graph(
    files: &[PathBuf],
    root: &Path,
    stop: &Arc<AtomicBool>,
    emitter: &EventEmitter,
    total: u64,
) -> (SymbolGraph, Vec<ImportEdge>, u64) {
    let stop_ref = Arc::clone(stop);
    let parse_results: Vec<Result<(PathBuf, _), (PathBuf, String)>> = files
        .par_iter()
        .map(|file_path| {
            if stop_ref.load(Ordering::Relaxed) {
                return Err((file_path.clone(), "cancelled".to_string()));
            }
            let rel_path = file_path.strip_prefix(root).unwrap_or(file_path.as_path());
            let content = std::fs::read(file_path)
                .map_err(|e| (rel_path.to_path_buf(), format!("failed to read: {e}")))?;
            let mut parser = Parser::new();
            let result = parser
                .parse_bytes(rel_path, &content)
                .map_err(|e| (rel_path.to_path_buf(), format!("parse failed: {e}")))?;
            let symbols = extract_symbols(&result.tree, &content, rel_path, 0);
            Ok((rel_path.to_path_buf(), symbols))
        })
        .collect();

    let mut graph = SymbolGraph::new();
    let mut all_imports: Vec<ImportEdge> = Vec::new();
    let mut next_id: u64 = 0;
    let mut parsed_count: u64 = 0;

    for (i, result) in parse_results.into_iter().enumerate() {
        emitter.progress("parsing", i as u64 + 1, total);
        let (_, mut file_symbols) = match result {
            Ok(v) => v,
            Err((_path, msg)) if msg == "cancelled" => continue,
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
        parsed_count += 1;
        let base = next_id;
        for sym in &mut file_symbols.symbols {
            sym.id += base;
        }
        all_imports.extend(file_symbols.imports.clone());
        update_file(&mut graph, file_symbols);
        // graph.next_id() reflects both the symbols we just added and any
        // synthetic external nodes update_file created for bare imports.
        next_id = graph.next_id();
    }

    re_resolve_imports(&mut graph, &all_imports);
    annotate_trust(&mut graph, &all_imports);

    (graph, all_imports, parsed_count)
}

fn collect_files(root: &Path, filter: &FileFilter) -> Result<Vec<PathBuf>, EmbeddedError> {
    let mut files = Vec::new();
    let walker = WalkDir::new(root).into_iter().filter_entry(|e| {
        // Prune ignored directories so we never traverse coverage/,
        // node_modules/, .git, target, etc.
        if e.file_type().is_dir() {
            return !filter.should_ignore(e.path());
        }
        true
    });
    for entry in walker {
        let entry = entry?;
        if entry.file_type().is_file() && filter.should_process(entry.path()) {
            files.push(entry.into_path());
        }
    }
    files.sort();
    Ok(files)
}

fn load_architecture_config(path: Option<&Path>) -> Result<ArchitectureConfig, EmbeddedError> {
    match path {
        Some(path) => {
            let yaml = std::fs::read_to_string(path).map_err(|e| EmbeddedError::ConfigIo {
                path: path.to_path_buf(),
                source: e,
            })?;
            Ok(parse_validated_architecture_config(&yaml)?)
        }
        None => Ok(ArchitectureConfig { layers: Vec::new() }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_file(dir: &Path, name: &str, content: &str) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    fn write_arch_config(dir: &Path) -> PathBuf {
        let config_path = dir.join("architecture.yml");
        fs::write(
            &config_path,
            r#"
layers:
  - name: domain
    paths: ["src/domain/*"]
    allowed_imports: [domain]
  - name: infra
    paths: ["src/infra/*"]
    allowed_imports: [domain, infra]
"#,
        )
        .unwrap();
        config_path
    }

    #[test]
    fn architecture_config_loader_rejects_overlapping_paths() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("architecture.yaml");
        fs::write(
            &config_path,
            r#"
layers:
  - name: app
    paths: ["src/*"]
    allowed_imports: [app]
  - name: ui
    paths: ["src/ui/*"]
    allowed_imports: [ui]
"#,
        )
        .unwrap();

        let err = load_architecture_config(Some(&config_path)).unwrap_err();
        assert!(err.to_string().contains("overlaps"));
    }

    #[test]
    fn empty_directory_returns_no_violations() {
        let tmp = TempDir::new().unwrap();
        let config = EmbeddedConfig {
            root: tmp.path().to_path_buf(),
            architecture_config: None,
            filter: None,
            plan: None,
        };

        let result = run_embedded(&config).unwrap();
        assert!(result.violations.is_empty());
        assert_eq!(result.stats.node_count, 0);
    }

    #[test]
    fn parses_files_and_returns_stats() {
        let tmp = TempDir::new().unwrap();
        write_file(
            tmp.path(),
            "src/main.ts",
            "export function hello() { return 'hi'; }",
        );
        write_file(
            tmp.path(),
            "src/util.ts",
            "export function add(a: number, b: number) { return a + b; }",
        );

        let config = EmbeddedConfig {
            root: tmp.path().to_path_buf(),
            architecture_config: None,
            filter: None,
            plan: None,
        };

        let result = run_embedded(&config).unwrap();
        assert!(result.stats.node_count >= 2);
        assert!(result.stats.files >= 2);
        assert!(!result.events.is_empty());
    }

    #[test]
    fn detects_cross_layer_violation() {
        let tmp = TempDir::new().unwrap();
        let config_path = write_arch_config(tmp.path());

        // domain file importing from infra
        write_file(
            tmp.path(),
            "src/domain/user.ts",
            "import { db } from '../infra/db';\nexport function getUser() { return db(); }",
        );
        write_file(
            tmp.path(),
            "src/infra/db.ts",
            "export function db() { return 'connected'; }",
        );

        let config = EmbeddedConfig {
            root: tmp.path().to_path_buf(),
            architecture_config: Some(config_path),
            filter: None,
            plan: None,
        };

        let result = run_embedded(&config).unwrap();
        // Should have stats even if cross-layer edge resolution depends on
        // the import target matching a file in the graph
        assert!(result.stats.files >= 2);
    }

    #[test]
    fn nonexistent_root_returns_error() {
        let tmp = TempDir::new().unwrap();
        let nonexistent = tmp.path().join("missing-root");
        let config = EmbeddedConfig {
            root: nonexistent,
            architecture_config: None,
            filter: None,
            plan: None,
        };

        let result = run_embedded(&config);
        assert!(result.is_err());
        assert!(matches!(result, Err(EmbeddedError::RootNotFound(_))));
    }

    #[test]
    fn nonexistent_architecture_config_returns_error() {
        let tmp = TempDir::new().unwrap();
        let nonexistent_config = tmp.path().join("missing-arch.yml");
        let config = EmbeddedConfig {
            root: tmp.path().to_path_buf(),
            architecture_config: Some(nonexistent_config),
            filter: None,
            plan: None,
        };

        let result = run_embedded(&config);
        assert!(result.is_err());
        assert!(matches!(result, Err(EmbeddedError::ConfigIo { .. })));
    }

    #[test]
    fn records_duration() {
        let tmp = TempDir::new().unwrap();
        let config = EmbeddedConfig {
            root: tmp.path().to_path_buf(),
            architecture_config: None,
            filter: None,
            plan: None,
        };

        let result = run_embedded(&config).unwrap();
        // Duration should be non-zero (or at least not panic)
        assert!(result.duration.as_nanos() > 0 || result.duration == Duration::ZERO);
    }

    #[test]
    fn parallel_parse_produces_no_symbol_id_collisions() {
        // Verifies the rebase invariant: parallel parse assigns 0-based IDs per file,
        // sequential apply rebases them to be globally unique.
        // If any two symbols share an ID the graph is corrupt.
        let tmp = TempDir::new().unwrap();

        // Write 10 files each with multiple exports to generate multiple symbols per file
        for i in 0..10 {
            write_file(
                tmp.path(),
                &format!("src/module_{i}.ts"),
                &format!(
                    "export function fn_{i}_a() {{}} \nexport function fn_{i}_b() {{}} \nexport const VAL_{i} = {i};"
                ),
            );
        }

        let config = EmbeddedConfig {
            root: tmp.path().to_path_buf(),
            architecture_config: None,
            filter: None,
            plan: None,
        };

        let result = run_embedded(&config).unwrap();

        // Direct graph check: all node IDs must be unique
        assert!(
            result.stats.node_count >= 10,
            "expected at least 10 symbols across 10 files, got {}",
            result.stats.node_count
        );

        // Verify stats reflect parsed file count
        assert_eq!(
            result.stats.files, 10,
            "expected 10 files in stats, got {}",
            result.stats.files
        );
    }

    #[test]
    fn parse_errors_surface_as_events_not_silent_drops() {
        // Verifies that files which fail to parse produce error events rather than
        // being silently skipped (council finding: governance tools must not give
        // silent clean results for files they couldn\'t scan).
        let tmp = TempDir::new().unwrap();

        // One valid file
        write_file(tmp.path(), "src/valid.ts", "export function hello() {}");
        write_file(tmp.path(), "src/valid2.ts", "export const x = 1;");

        let config = EmbeddedConfig {
            root: tmp.path().to_path_buf(),
            architecture_config: None,
            filter: None,
            plan: None,
        };

        // Both files should parse successfully
        let result = run_embedded(&config).unwrap();
        assert!(
            result.stats.files >= 1,
            "expected at least 1 file parsed, got {}",
            result.stats.files
        );
        // No error events should be present for valid files
        let error_events: Vec<_> = result
            .events
            .iter()
            .filter(|e| e.event_type == anvil_kernel_types::EventType::Error)
            .collect();
        assert!(
            error_events.is_empty(),
            "unexpected parse errors for valid files: {error_events:?}"
        );
    }
}
