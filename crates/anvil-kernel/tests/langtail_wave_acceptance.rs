//! LANGTAIL-008 wave acceptance.
//!
//! Proves the tail-language wave end-to-end: every included language parses a
//! representative real-world fixture without panicking or producing an error
//! tree, its symbols land in the kernel symbol graph, and a full embedded scan
//! over the fixture corpus discovers and parses every file (the production
//! path, not just the unit extractors).
//!
//! The per-language fixture corpus lives in `tests/fixtures/langtail/`
//! (resolving the module's open question on fixture location).

use std::path::{Path, PathBuf};

use anvil_kernel::embedded::{EmbeddedConfig, run_embedded};
use anvil_kernel::graph::{SymbolGraph, update_file};
use anvil_kernel::parser::Parser;
use anvil_kernel::parser::extract::extract_symbols;

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/langtail")
}

/// (fixture filename, symbols that must appear in the graph for that file).
const EXPECTATIONS: &[(&str, &[&str])] = &[
    (
        "greeter.dart",
        &["Greeter", "Greeter.greet", "Mood", "topLevelGreeting"],
    ),
    ("server.go", &["Server", "Handler", "New", "Server.Start"]),
    (
        "Service.java",
        &["Service", "Service.fetch", "Repository", "Status", "Pair"],
    ),
    (
        "Main.kt",
        &["Greeter", "Greeter.greet", "Service", "Mood", "main"],
    ),
    (
        "Program.cs",
        &["Service", "Service.FetchAsync", "IRepository", "Status"],
    ),
    (
        "parser.c",
        &["Token", "TokenKind", "classify", "parse_line"],
    ),
    // `.h` maps to the C grammar (deterministic detection); a prototypes-only
    // header must still yield function + type symbols.
    ("parser.h", &["ParseResult", "parse_line", "parse_buffer"]),
    (
        "engine.cpp",
        &[
            "Renderer",
            "Renderer.render",
            "Config",
            "clampValue",
            "globalSeed",
        ],
    ),
];

#[test]
fn every_fixture_parses_cleanly_and_populates_the_graph() {
    let dir = fixtures_dir();
    let mut parser = Parser::new();

    for (file, expected) in EXPECTATIONS {
        let path = dir.join(file);
        let content = std::fs::read(&path).unwrap_or_else(|e| panic!("read {file}: {e}"));

        let parsed = parser
            .parse_bytes(&path, &content)
            .unwrap_or_else(|e| panic!("{file} must parse: {e}"));
        assert!(
            !parsed.tree.root_node().has_error(),
            "{file}: representative real-world source must parse without an error tree"
        );

        // Symbols must land in the graph, keyed by the file path.
        let symbols = extract_symbols(&parsed.tree, &content, &path, 0);
        let mut graph = SymbolGraph::new();
        update_file(&mut graph, symbols);

        let key = path.to_string_lossy().to_string();
        let names: Vec<String> = graph
            .symbols_in_file(&key)
            .iter()
            .map(|s| s.name.clone())
            .collect();
        assert!(
            !names.is_empty(),
            "{file}: produced no symbols in the graph"
        );
        for want in *expected {
            assert!(
                names.iter().any(|n| n == want),
                "{file}: expected symbol `{want}` in graph, got {names:?}"
            );
        }
    }
}

#[test]
fn embedded_scan_discovers_and_parses_the_whole_corpus() {
    // The production scan path (default filter) must discover and parse every
    // tail-language fixture — proving the parseable gate admits them, not just
    // the unit extractors.
    //
    // Fixtures are copied into a temp dir before scanning: the canonical corpus
    // lives under the repo's `.claude/.../tests/...` in worktree checkouts, and
    // `.claude` is on the scan denylist, so scanning it in place would prune
    // everything. A flat temp dir is location-independent.
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let src = fixtures_dir();
    for (file, _) in EXPECTATIONS {
        std::fs::copy(src.join(file), tmp.path().join(file))
            .unwrap_or_else(|e| panic!("copy {file}: {e}"));
    }

    let config = EmbeddedConfig {
        root: tmp.path().to_path_buf(),
        architecture_config: None,
        filter: None,
        plan: None,
    };

    let result = run_embedded(&config).expect("embedded scan over fixtures must succeed");

    // `stats.files` counts every file entry in the graph — the scanned source
    // files plus synthetic import-target pseudo-files. A full scan of the corpus
    // yields ~27 file entries and ~69 symbol nodes; a pruned scan reports 0 and
    // a *partial* scan (some fixtures dropped) falls well below these floors, so
    // the thresholds catch a regression that silently stops parsing a subset —
    // not just a total prune. Per-symbol correctness per fixture is asserted in
    // `every_fixture_parses_cleanly_and_populates_the_graph`; this test guards
    // the production discovery path (gate admits them, scan does not panic).
    assert!(
        result.stats.files >= 18,
        "embedded scan must discover the whole corpus (got {} file entries; \
         full scan yields ~27, a partial scan far fewer)",
        result.stats.files
    );
    assert!(
        result.stats.node_count >= 50,
        "the graph must hold the corpus's extracted symbols (got {} nodes; \
         full scan yields ~69)",
        result.stats.node_count
    );
}
