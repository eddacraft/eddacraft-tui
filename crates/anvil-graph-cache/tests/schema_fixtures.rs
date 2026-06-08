//! GV2-010 schema fixtures — deterministic node/edge output for the semantic
//! code graph v2 schema, including the first-class `Reexports` edge.
//!
//! `anvil-graph-cache` deliberately does not link the parser (ADR-064 —
//! `anvil-kernel` depends on this crate, so a dev-dependency back would cycle).
//! These fixtures therefore construct [`FileSymbols`] exactly as the TS / Rust
//! extractors emit them; the extractor *walk* is covered by the unit tests in
//! `anvil-kernel`'s `parser::extract::{typescript,rust}`. Here we pin the
//! graph-schema shape the daemon and downstream consumers (GV2-011/027) read.

use anvil_graph_cache::{SymbolGraph, update_file};
use anvil_kernel_types::{
    ByteRange, EdgeType, FileSymbols, ImportEdge, ReexportEdge, SymbolEdge, SymbolKind, SymbolNode,
    TrustLevel, Visibility,
};

fn sym(id: u64, kind: SymbolKind, name: &str, vis: Visibility, file: &str) -> SymbolNode {
    SymbolNode {
        id,
        kind,
        name: name.to_string(),
        visibility: vis,
        file: file.to_string(),
        trust_level: TrustLevel::default(),
    }
}

/// [`FileSymbols`] as the TypeScript extractor emits for:
/// ```ts
/// export { Button } from './button';
/// export * from './icons';
/// ```
fn ts_reexport_fixture() -> FileSymbols {
    FileSymbols {
        file: "src/index.ts".to_string(),
        // The export clause with no local declaration yields an `Export` node
        // (the extractor's surface-tracking behaviour).
        symbols: vec![sym(
            0,
            SymbolKind::Export,
            "Button",
            Visibility::Public,
            "src/index.ts",
        )],
        imports: vec![
            ImportEdge {
                from_file: "src/index.ts".to_string(),
                to_source: "./button".to_string(),
                line: 1,
            },
            ImportEdge {
                from_file: "src/index.ts".to_string(),
                to_source: "./icons".to_string(),
                line: 2,
            },
        ],
        reexports: vec![
            ReexportEdge {
                from_file: "src/index.ts".to_string(),
                exported_name: "Button".to_string(),
                to_source: "./button".to_string(),
                line: 1,
            },
            ReexportEdge {
                from_file: "src/index.ts".to_string(),
                exported_name: "*".to_string(),
                to_source: "./icons".to_string(),
                line: 2,
            },
        ],
    }
}

#[test]
fn schema_fixtures_ts_reexport_edges_are_structured_and_deterministic() {
    let fs = ts_reexport_fixture();

    // A re-export is first-class and distinct from a plain import: every
    // re-exported name has a matching dependency import, but the re-export
    // additionally records the widened public surface.
    let reexport_names: Vec<&str> = fs
        .reexports
        .iter()
        .map(|r| r.exported_name.as_str())
        .collect();
    assert_eq!(reexport_names, ["Button", "*"]);
    assert!(
        fs.reexports.iter().any(|r| r.exported_name == "*"),
        "wildcard `export * from` is captured as a re-export"
    );

    // Schema consistency: every re-export's module is also a dependency import
    // (a re-export pulls from a module it depends on) — a re-export is the
    // import edge *plus* the re-published name, never a free-floating edge.
    for re in &fs.reexports {
        assert!(
            fs.imports.iter().any(|i| i.to_source == re.to_source),
            "re-export of `{}` from `{}` must have a matching dependency import",
            re.exported_name,
            re.to_source
        );
    }
}

#[test]
fn schema_fixtures_graph_node_output_is_deterministic() {
    // Feed the fixture through the real graph: the node output (the part this
    // crate owns) must be deterministic. Re-export *consumption* into graph
    // edges is GV2-011's dependency/impact work; GV2-010 lands the schema.
    let fs = ts_reexport_fixture();
    let mut graph = SymbolGraph::new();
    let _delta = update_file(&mut graph, fs);

    let names: Vec<&str> = graph
        .symbols_in_file("src/index.ts")
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(
        names,
        ["Button"],
        "deterministic symbol nodes from the fixture"
    );
}

/// Future-language fixture (Rust), as the Rust extractor emits for:
/// ```rust
/// pub use crate::internal::Widget;
/// ```
#[test]
fn schema_fixtures_rust_pub_use_reexport_future_language() {
    let fs = FileSymbols {
        file: "src/lib.rs".to_string(),
        symbols: Vec::new(),
        imports: vec![ImportEdge {
            from_file: "src/lib.rs".to_string(),
            to_source: "crate::internal::Widget".to_string(),
            line: 1,
        }],
        reexports: vec![ReexportEdge {
            from_file: "src/lib.rs".to_string(),
            // Rust re-export: name is the last `::` segment, `to_source` the
            // full path (Rust's ImportEdge convention).
            exported_name: "Widget".to_string(),
            to_source: "crate::internal::Widget".to_string(),
            line: 1,
        }],
    };
    assert_eq!(fs.reexports.len(), 1);
    assert_eq!(fs.reexports[0].exported_name, "Widget");
}

#[test]
fn schema_fixtures_reexport_edge_type_is_first_class() {
    // The Reexports edge variant is distinct from Imports, so impact analysis
    // can tell a re-export (widens the surface) from a plain dependency.
    let re = SymbolEdge {
        from: 1,
        to: 2,
        edge_type: EdgeType::Reexports,
    };
    assert_ne!(re.edge_type, EdgeType::Imports);
    assert_ne!(re.edge_type, EdgeType::Calls);
}

#[test]
fn schema_fixtures_byte_range_span_is_no_text() {
    // GV2-010 freezes the no-text span shape (privacy verdict PV-7(e)); span
    // *population* onto nodes lands in v0.9. The type carries only offsets —
    // there is structurally no text field to leak.
    let span = ByteRange { start: 4, end: 16 };
    assert_eq!(span.len(), 12);
    assert!(!span.is_empty());
}
