// Architecture check parity validation (RENG-004)
//
// Validates that the kernel's policy engine invariants produce equivalent
// results to the current JS architecture check (ARCH-001..004).
//
// ## Parity Coverage
//
// | JS check          | Kernel invariant            | Parity status      |
// |--------------------|-----------------------------|--------------------|
// | ARCH-003 layer     | cross-layer-violation       | Equivalent         |
// | — (none)           | public-api-expansion        | Rust-only (new)    |
// | — (none)           | privilege-expansion         | Rust-only (new)    |
// | — (none)           | new-dependency-introduction | Rust-only (new)    |
// | ARCH-001 circular  | — (not in H1)               | Gap (by design)    |
// | ARCH-002 orphan    | — (not in H1)               | Gap (by design)    |
// | ARCH-004 other     | — (not in H1)               | Gap (by design)    |
//
// The kernel operates at symbol-level granularity with trust/visibility
// awareness, superseding the module-level dependency-cruiser approach.
// Circular and orphan detection are intentionally deferred — the kernel's
// graph structure supports them but they are not H1 priorities.

use std::collections::HashSet;

use anvil_kernel::graph::SymbolGraph;
use anvil_kernel::graph::incremental::GraphDelta;
use anvil_kernel::policy::config::ArchitectureConfig;
use anvil_kernel::policy::engine::{PolicyEngine, Severity};
use anvil_kernel::policy::invariants::cross_layer::CrossLayerViolation;
use anvil_kernel::policy::invariants::new_dependency::NewDependencyIntroduction;
use anvil_kernel::policy::invariants::privilege_expansion::PrivilegeExpansion;
use anvil_kernel::policy::invariants::public_api::PublicApiExpansion;
use anvil_kernel_types::{EdgeType, SymbolEdge, SymbolKind, SymbolNode, TrustLevel, Visibility};

fn layered_config() -> ArchitectureConfig {
    ArchitectureConfig::from_yaml(
        r#"
layers:
  - name: domain
    paths: ["src/domain/*"]
    allowed_imports: [domain]
  - name: application
    paths: ["src/app/*"]
    allowed_imports: [domain, application]
  - name: infrastructure
    paths: ["src/infra/*"]
    allowed_imports: [domain, application, infrastructure]
  - name: presentation
    paths: ["src/ui/*"]
    allowed_imports: [domain, application, infrastructure, presentation]
"#,
    )
    .unwrap()
}

fn sym(id: u64, name: &str, file: &str, vis: Visibility, trust: TrustLevel) -> SymbolNode {
    SymbolNode {
        id,
        kind: SymbolKind::Function,
        name: name.to_string(),
        visibility: vis,
        file: file.to_string(),
        trust_level: trust,
    }
}

fn external_sym(id: u64, name: &str, file: &str) -> SymbolNode {
    SymbolNode {
        id,
        kind: SymbolKind::Module,
        name: name.to_string(),
        visibility: Visibility::Public,
        file: file.to_string(),
        trust_level: TrustLevel::External,
    }
}

/// Build engine with all four invariants registered.
fn build_engine() -> PolicyEngine {
    let mut engine = PolicyEngine::new();
    engine.register(Box::new(CrossLayerViolation));
    engine.register(Box::new(PublicApiExpansion));
    engine.register(Box::new(PrivilegeExpansion));
    engine.register(Box::new(NewDependencyIntroduction));
    engine
}

// ── Cross-layer parity (JS ARCH-003 equivalent) ─────────────────────

/// Fixture: domain layer imports from infrastructure — forbidden.
/// JS equivalent: ARCH-003 layer/boundary violation.
#[test]
fn cross_layer_violation_detected() {
    let config = layered_config();
    let mut graph = SymbolGraph::new();

    graph
        .add_symbol(sym(
            1,
            "getUserById",
            "src/domain/user.ts",
            Visibility::Internal,
            TrustLevel::Internal,
        ))
        .unwrap();
    graph
        .add_symbol(sym(
            2,
            "dbQuery",
            "src/infra/db.ts",
            Visibility::Internal,
            TrustLevel::Internal,
        ))
        .unwrap();
    graph
        .add_edge(SymbolEdge {
            from: 1,
            to: 2,
            edge_type: EdgeType::Imports,
        })
        .unwrap();

    let delta = GraphDelta {
        added_symbols: vec![1],
        file: "src/domain/user.ts".to_string(),
        ..Default::default()
    };

    let mut engine = build_engine();
    let violations = engine.evaluate(&delta, &graph, &config);

    let cross_layer: Vec<_> = violations
        .iter()
        .filter(|v| v.policy_id == "cross-layer-violation")
        .collect();
    assert_eq!(cross_layer.len(), 1);
    assert_eq!(cross_layer[0].file, "src/domain/user.ts");
    assert_eq!(cross_layer[0].symbol, "getUserById");
    assert_eq!(cross_layer[0].severity, Severity::High);
    assert!(
        cross_layer[0]
            .message
            .contains("'domain' cannot import from layer 'infrastructure'")
    );
}

/// Fixture: application layer imports from presentation — forbidden
/// (application can only import domain + application).
#[test]
fn cross_layer_app_importing_from_presentation() {
    let config = layered_config();
    let mut graph = SymbolGraph::new();

    graph
        .add_symbol(sym(
            1,
            "appService",
            "src/app/service.ts",
            Visibility::Internal,
            TrustLevel::Internal,
        ))
        .unwrap();
    graph
        .add_symbol(sym(
            2,
            "Button",
            "src/ui/button.tsx",
            Visibility::Internal,
            TrustLevel::Internal,
        ))
        .unwrap();
    graph
        .add_edge(SymbolEdge {
            from: 1,
            to: 2,
            edge_type: EdgeType::Imports,
        })
        .unwrap();

    let delta = GraphDelta {
        added_symbols: vec![1],
        file: "src/app/service.ts".to_string(),
        ..Default::default()
    };

    let mut engine = build_engine();
    let violations = engine.evaluate(&delta, &graph, &config);

    let cross_layer: Vec<_> = violations
        .iter()
        .filter(|v| v.policy_id == "cross-layer-violation")
        .collect();
    assert_eq!(cross_layer.len(), 1);
    assert!(
        cross_layer[0]
            .message
            .contains("'application' cannot import from layer 'presentation'")
    );
}

/// Non-Import edge types (Calls, Inherits) must not trigger cross-layer.
#[test]
fn non_import_edge_does_not_trigger_cross_layer() {
    let config = layered_config();
    let mut graph = SymbolGraph::new();

    graph
        .add_symbol(sym(
            1,
            "domainFn",
            "src/domain/user.ts",
            Visibility::Internal,
            TrustLevel::Internal,
        ))
        .unwrap();
    graph
        .add_symbol(sym(
            2,
            "infraFn",
            "src/infra/db.ts",
            Visibility::Internal,
            TrustLevel::Internal,
        ))
        .unwrap();
    graph
        .add_edge(SymbolEdge {
            from: 1,
            to: 2,
            edge_type: EdgeType::Calls,
        })
        .unwrap();

    let delta = GraphDelta {
        added_symbols: vec![1],
        file: "src/domain/user.ts".to_string(),
        ..Default::default()
    };

    let mut engine = build_engine();
    let violations = engine.evaluate(&delta, &graph, &config);

    // All symbols are Internal/non-privileged, so no invariant should fire
    assert!(
        violations.is_empty(),
        "Calls edges should not trigger any violation (all symbols are Internal)"
    );
}

// ── Public API expansion (Rust-only, no JS equivalent) ──────────────

/// Fixture: new public export expands API surface.
#[test]
fn public_api_expansion_detected() {
    let config = layered_config();
    let mut graph = SymbolGraph::new();

    graph
        .add_symbol(sym(
            10,
            "createUser",
            "src/app/users.ts",
            Visibility::Public,
            TrustLevel::Internal,
        ))
        .unwrap();

    let delta = GraphDelta {
        added_symbols: vec![10],
        file: "src/app/users.ts".to_string(),
        ..Default::default()
    };

    let mut engine = build_engine();
    let violations = engine.evaluate(&delta, &graph, &config);

    let api_expansion: Vec<_> = violations
        .iter()
        .filter(|v| v.policy_id == "public-api-expansion")
        .collect();
    assert_eq!(api_expansion.len(), 1);
    assert_eq!(api_expansion[0].symbol, "createUser");
    assert_eq!(api_expansion[0].severity, Severity::Low);
}

// ── Privilege expansion (Rust-only, no JS equivalent) ───────────────

/// Fixture: new symbol with privileged access (fs, `child_process`).
#[test]
fn privilege_expansion_detected() {
    let config = layered_config();
    let mut graph = SymbolGraph::new();

    graph
        .add_symbol(sym(
            20,
            "deleteAllFiles",
            "src/infra/cleanup.ts",
            Visibility::Internal,
            TrustLevel::Privileged,
        ))
        .unwrap();

    let delta = GraphDelta {
        added_symbols: vec![20],
        file: "src/infra/cleanup.ts".to_string(),
        ..Default::default()
    };

    let mut engine = build_engine();
    let violations = engine.evaluate(&delta, &graph, &config);

    let priv_expansion: Vec<_> = violations
        .iter()
        .filter(|v| v.policy_id == "privilege-expansion")
        .collect();
    assert_eq!(priv_expansion.len(), 1);
    assert_eq!(priv_expansion[0].symbol, "deleteAllFiles");
    assert_eq!(priv_expansion[0].severity, Severity::Critical);
}

// ── New dependency introduction (Rust-only, no JS equivalent) ───────

/// Fixture: new external dependency (npm package) introduced.
#[test]
fn new_dependency_introduction_detected() {
    let config = layered_config();
    let mut graph = SymbolGraph::new();

    graph
        .add_symbol(sym(
            60,
            "fetchData",
            "src/app/api.ts",
            Visibility::Internal,
            TrustLevel::Internal,
        ))
        .unwrap();
    graph
        .add_symbol(external_sym(61, "axios", "axios"))
        .unwrap();
    graph
        .add_edge(SymbolEdge {
            from: 60,
            to: 61,
            edge_type: EdgeType::Imports,
        })
        .unwrap();

    let delta = GraphDelta {
        added_symbols: vec![60],
        added_edges: vec![(60, 61, EdgeType::Imports)],
        file: "src/app/api.ts".to_string(),
        ..Default::default()
    };

    let mut engine = build_engine();
    let violations = engine.evaluate(&delta, &graph, &config);

    let new_dep: Vec<_> = violations
        .iter()
        .filter(|v| v.policy_id == "new-dependency-introduction")
        .collect();
    assert_eq!(new_dep.len(), 1);
    assert_eq!(new_dep[0].symbol, "fetchData");
    assert_eq!(new_dep[0].severity, Severity::Medium);
}

/// Previously-imported external dependency should not fire.
#[test]
fn previously_imported_dependency_suppressed() {
    let config = layered_config();
    let mut graph = SymbolGraph::new();

    graph
        .add_symbol(sym(
            60,
            "fetchData",
            "src/app/api.ts",
            Visibility::Internal,
            TrustLevel::Internal,
        ))
        .unwrap();
    graph
        .add_symbol(external_sym(61, "axios", "axios"))
        .unwrap();
    graph
        .add_edge(SymbolEdge {
            from: 60,
            to: 61,
            edge_type: EdgeType::Imports,
        })
        .unwrap();

    let mut previously_imported = HashSet::new();
    previously_imported.insert("axios".to_string());

    let delta = GraphDelta {
        added_symbols: vec![60],
        added_edges: vec![(60, 61, EdgeType::Imports)],
        file: "src/app/api.ts".to_string(),
        previously_imported,
        ..Default::default()
    };

    let mut engine = build_engine();
    let violations = engine.evaluate(&delta, &graph, &config);

    assert!(
        violations
            .iter()
            .all(|v| v.policy_id != "new-dependency-introduction"),
        "re-added import should be suppressed by previously_imported"
    );
}

// ── Clean fixture — zero violations ─────────────────────────────────

/// Fixture: clean repo — all imports are allowed, no public symbols,
/// no privileged access. Should produce zero violations.
#[test]
fn clean_fixture_no_false_positives() {
    let config = layered_config();
    let mut graph = SymbolGraph::new();

    // infra importing from domain — allowed
    graph
        .add_symbol(sym(
            30,
            "saveUser",
            "src/infra/repo.ts",
            Visibility::Internal,
            TrustLevel::Internal,
        ))
        .unwrap();
    graph
        .add_symbol(sym(
            31,
            "User",
            "src/domain/types.ts",
            Visibility::Internal,
            TrustLevel::Internal,
        ))
        .unwrap();
    graph
        .add_edge(SymbolEdge {
            from: 30,
            to: 31,
            edge_type: EdgeType::Imports,
        })
        .unwrap();

    let delta = GraphDelta {
        added_symbols: vec![30],
        file: "src/infra/repo.ts".to_string(),
        ..Default::default()
    };

    let mut engine = build_engine();
    let violations = engine.evaluate(&delta, &graph, &config);

    assert!(
        violations.is_empty(),
        "clean fixture should have no violations"
    );
}

// ── Composite violations ────────────────────────────────────────────

/// Fixture: multiple violations in a single delta — cross-layer + privilege.
#[test]
fn multiple_violations_in_single_delta() {
    let config = layered_config();
    let mut graph = SymbolGraph::new();

    // domain function that is privileged AND imports from infra
    graph
        .add_symbol(sym(
            40,
            "dangerousFn",
            "src/domain/danger.ts",
            Visibility::Internal,
            TrustLevel::Privileged,
        ))
        .unwrap();
    graph
        .add_symbol(sym(
            41,
            "infraHelper",
            "src/infra/helper.ts",
            Visibility::Internal,
            TrustLevel::Internal,
        ))
        .unwrap();
    graph
        .add_edge(SymbolEdge {
            from: 40,
            to: 41,
            edge_type: EdgeType::Imports,
        })
        .unwrap();

    let delta = GraphDelta {
        added_symbols: vec![40],
        file: "src/domain/danger.ts".to_string(),
        ..Default::default()
    };

    let mut engine = build_engine();
    let violations = engine.evaluate(&delta, &graph, &config);

    let policy_ids: Vec<&str> = violations.iter().map(|v| v.policy_id.as_str()).collect();
    assert!(policy_ids.contains(&"cross-layer-violation"));
    assert!(policy_ids.contains(&"privilege-expansion"));
    assert_eq!(violations.len(), 2);
}

/// All four invariants fire simultaneously on a maximally-violating symbol.
#[test]
fn all_four_invariants_fire_on_maximum_violation() {
    let config = layered_config();
    let mut graph = SymbolGraph::new();

    // domain symbol: public + privileged + imports from infra + new external dep
    graph
        .add_symbol(sym(
            70,
            "unsafeDomainExport",
            "src/domain/danger.ts",
            Visibility::Public,
            TrustLevel::Privileged,
        ))
        .unwrap();
    graph
        .add_symbol(sym(
            71,
            "infraFn",
            "src/infra/db.ts",
            Visibility::Internal,
            TrustLevel::Internal,
        ))
        .unwrap();
    graph
        .add_symbol(external_sym(72, "danger", "danger"))
        .unwrap();
    graph
        .add_edge(SymbolEdge {
            from: 70,
            to: 71,
            edge_type: EdgeType::Imports,
        })
        .unwrap();
    graph
        .add_edge(SymbolEdge {
            from: 70,
            to: 72,
            edge_type: EdgeType::Imports,
        })
        .unwrap();

    let delta = GraphDelta {
        added_symbols: vec![70],
        added_edges: vec![(70, 72, EdgeType::Imports)],
        file: "src/domain/danger.ts".to_string(),
        ..Default::default()
    };

    let mut engine = build_engine();
    let violations = engine.evaluate(&delta, &graph, &config);

    let policy_ids: Vec<&str> = violations.iter().map(|v| v.policy_id.as_str()).collect();
    assert!(
        policy_ids.contains(&"cross-layer-violation"),
        "domain→infra import"
    );
    assert!(
        policy_ids.contains(&"public-api-expansion"),
        "new public symbol"
    );
    assert!(
        policy_ids.contains(&"privilege-expansion"),
        "new privileged symbol"
    );
    assert!(
        policy_ids.contains(&"new-dependency-introduction"),
        "new external dep"
    );
    // Exactly 4: the external sym's file ("danger") does not match any layer,
    // so cross-layer fires once (domain→infra, id 71 only).
    assert_eq!(violations.len(), 4);
}

// ── Baseline suppression (Rust `previously_*` = JS baseline) ────────

/// Previously-public symbols should not fire public-api-expansion.
/// This is the Rust equivalent of JS baseline filtering — the JS check
/// loads `.anvil/baseline.json` and filters out known violations.
#[test]
fn previously_public_symbol_suppressed() {
    let config = layered_config();
    let mut graph = SymbolGraph::new();

    let existing = sym(
        80,
        "existingExport",
        "src/app/service.ts",
        Visibility::Public,
        TrustLevel::Internal,
    );

    let mut previously_public = HashSet::new();
    previously_public.insert(GraphDelta::symbol_baseline_key(&existing));

    graph.add_symbol(existing).unwrap();

    let delta = GraphDelta {
        added_symbols: vec![80],
        file: "src/app/service.ts".to_string(),
        previously_public,
        ..Default::default()
    };

    let mut engine = build_engine();
    let violations = engine.evaluate(&delta, &graph, &config);

    assert!(
        violations
            .iter()
            .all(|v| v.policy_id != "public-api-expansion"),
        "previously public symbol should be suppressed"
    );
}

/// Previously-privileged symbols should not fire privilege-expansion.
#[test]
fn previously_privileged_symbol_suppressed() {
    let config = layered_config();
    let mut graph = SymbolGraph::new();

    let existing = sym(
        81,
        "existingPriv",
        "src/infra/legacy.ts",
        Visibility::Internal,
        TrustLevel::Privileged,
    );

    let mut previously_privileged = HashSet::new();
    previously_privileged.insert(GraphDelta::symbol_baseline_key(&existing));

    graph.add_symbol(existing).unwrap();

    let delta = GraphDelta {
        added_symbols: vec![81],
        file: "src/infra/legacy.ts".to_string(),
        previously_privileged,
        ..Default::default()
    };

    let mut engine = build_engine();
    let violations = engine.evaluate(&delta, &graph, &config);

    assert!(
        violations
            .iter()
            .all(|v| v.policy_id != "privilege-expansion"),
        "previously privileged symbol should be suppressed"
    );
}

/// Mixed baseline: some symbols suppressed, new ones still flagged.
#[test]
fn baseline_suppresses_known_but_flags_new() {
    let config = layered_config();
    let mut graph = SymbolGraph::new();

    let known = sym(
        82,
        "knownExport",
        "src/app/service.ts",
        Visibility::Public,
        TrustLevel::Internal,
    );
    // New public symbol
    graph
        .add_symbol(sym(
            83,
            "brandNewExport",
            "src/app/service.ts",
            Visibility::Public,
            TrustLevel::Internal,
        ))
        .unwrap();

    let mut previously_public = HashSet::new();
    previously_public.insert(GraphDelta::symbol_baseline_key(&known));

    graph.add_symbol(known).unwrap();

    let delta = GraphDelta {
        added_symbols: vec![82, 83],
        file: "src/app/service.ts".to_string(),
        previously_public,
        ..Default::default()
    };

    let mut engine = build_engine();
    let violations = engine.evaluate(&delta, &graph, &config);

    let api_violations: Vec<_> = violations
        .iter()
        .filter(|v| v.policy_id == "public-api-expansion")
        .collect();
    assert_eq!(api_violations.len(), 1);
    assert_eq!(api_violations[0].symbol, "brandNewExport");
}

/// Same-name public symbols in different files are distinct baseline entries.
#[test]
fn same_name_different_file_public_symbol_still_flags_new_export() {
    let config = layered_config();
    let mut graph = SymbolGraph::new();

    let known = sym(
        84,
        "sharedName",
        "src/app/legacy.ts",
        Visibility::Public,
        TrustLevel::Internal,
    );
    graph.add_symbol(known.clone()).unwrap();
    graph
        .add_symbol(sym(
            85,
            "sharedName",
            "src/app/new.ts",
            Visibility::Public,
            TrustLevel::Internal,
        ))
        .unwrap();

    let mut previously_public = HashSet::new();
    previously_public.insert(GraphDelta::symbol_baseline_key(&known));

    let delta = GraphDelta {
        added_symbols: vec![85],
        file: "src/app/new.ts".to_string(),
        previously_public,
        ..Default::default()
    };

    let mut engine = build_engine();
    let violations = engine.evaluate(&delta, &graph, &config);

    let api_violations: Vec<_> = violations
        .iter()
        .filter(|v| v.policy_id == "public-api-expansion")
        .collect();
    assert_eq!(api_violations.len(), 1);
    assert_eq!(api_violations[0].file, "src/app/new.ts");
    assert_eq!(api_violations[0].symbol, "sharedName");
}

/// Same-name privileged symbols in different files are distinct baseline entries.
#[test]
fn same_name_different_file_privileged_symbol_still_flags_new_access() {
    let config = layered_config();
    let mut graph = SymbolGraph::new();

    let known = sym(
        86,
        "sharedPrivilegedName",
        "src/infra/legacy.ts",
        Visibility::Internal,
        TrustLevel::Privileged,
    );
    graph.add_symbol(known.clone()).unwrap();
    graph
        .add_symbol(sym(
            87,
            "sharedPrivilegedName",
            "src/infra/new.ts",
            Visibility::Internal,
            TrustLevel::Privileged,
        ))
        .unwrap();

    let mut previously_privileged = HashSet::new();
    previously_privileged.insert(GraphDelta::symbol_baseline_key(&known));

    let delta = GraphDelta {
        added_symbols: vec![87],
        file: "src/infra/new.ts".to_string(),
        previously_privileged,
        ..Default::default()
    };

    let mut engine = build_engine();
    let violations = engine.evaluate(&delta, &graph, &config);

    let priv_violations: Vec<_> = violations
        .iter()
        .filter(|v| v.policy_id == "privilege-expansion")
        .collect();
    assert_eq!(priv_violations.len(), 1);
    assert_eq!(priv_violations[0].file, "src/infra/new.ts");
    assert_eq!(priv_violations[0].symbol, "sharedPrivilegedName");
}

/// Collision in a single delta: a baselined symbol and a brand-new symbol
/// that share the SAME name but live in DIFFERENT files are evaluated
/// together. A name-only baseline key would suppress both; the file/kind/name
/// key must suppress only the baselined identity and STILL flag the new file's
/// symbol. (CLAWP-026 — guards against name-collision suppression regression.)
#[test]
fn same_name_collision_in_single_delta_flags_only_new_file() {
    let config = layered_config();
    let mut graph = SymbolGraph::new();

    let baselined = sym(
        88,
        "collidingName",
        "src/app/old.ts",
        Visibility::Public,
        TrustLevel::Internal,
    );
    let newcomer = sym(
        89,
        "collidingName",
        "src/app/added.ts",
        Visibility::Public,
        TrustLevel::Internal,
    );

    // Only the original identity is baselined as previously public.
    let mut previously_public = HashSet::new();
    previously_public.insert(GraphDelta::symbol_baseline_key(&baselined));

    graph.add_symbol(baselined).unwrap();
    graph.add_symbol(newcomer).unwrap();

    // Both same-name symbols are present in the same delta.
    let delta = GraphDelta {
        added_symbols: vec![88, 89],
        file: "src/app/added.ts".to_string(),
        previously_public,
        ..Default::default()
    };

    let mut engine = build_engine();
    let violations = engine.evaluate(&delta, &graph, &config);

    let api_violations: Vec<_> = violations
        .iter()
        .filter(|v| v.policy_id == "public-api-expansion")
        .collect();
    // The baselined identity must be suppressed; the new file's identity must
    // NOT be silently suppressed by the shared name.
    assert_eq!(api_violations.len(), 1);
    assert_eq!(api_violations[0].file, "src/app/added.ts");
    assert_eq!(api_violations[0].symbol, "collidingName");
}

// ── Layer boundary edge cases ───────────────────────────────────────

/// File outside any layer should not trigger cross-layer.
#[test]
fn unmatched_file_no_cross_layer_violation() {
    let config = layered_config();
    let mut graph = SymbolGraph::new();

    graph
        .add_symbol(sym(
            50,
            "testHelper",
            "test/helpers.ts",
            Visibility::Internal,
            TrustLevel::Internal,
        ))
        .unwrap();
    graph
        .add_symbol(sym(
            51,
            "domainFn",
            "src/domain/core.ts",
            Visibility::Internal,
            TrustLevel::Internal,
        ))
        .unwrap();
    graph
        .add_edge(SymbolEdge {
            from: 50,
            to: 51,
            edge_type: EdgeType::Imports,
        })
        .unwrap();

    let delta = GraphDelta {
        added_symbols: vec![50],
        file: "test/helpers.ts".to_string(),
        ..Default::default()
    };

    let mut engine = build_engine();
    let violations = engine.evaluate(&delta, &graph, &config);

    assert!(
        violations
            .iter()
            .all(|v| v.policy_id != "cross-layer-violation"),
        "files outside layer boundaries should not trigger cross-layer"
    );
}

/// Import target outside any layer should not trigger cross-layer either.
#[test]
fn import_target_outside_layers_no_violation() {
    let config = layered_config();
    let mut graph = SymbolGraph::new();

    graph
        .add_symbol(sym(
            90,
            "domainFn",
            "src/domain/user.ts",
            Visibility::Internal,
            TrustLevel::Internal,
        ))
        .unwrap();
    graph
        .add_symbol(sym(
            91,
            "testUtil",
            "test/utils.ts",
            Visibility::Internal,
            TrustLevel::Internal,
        ))
        .unwrap();
    graph
        .add_edge(SymbolEdge {
            from: 90,
            to: 91,
            edge_type: EdgeType::Imports,
        })
        .unwrap();

    let delta = GraphDelta {
        added_symbols: vec![90],
        file: "src/domain/user.ts".to_string(),
        ..Default::default()
    };

    let mut engine = build_engine();
    let violations = engine.evaluate(&delta, &graph, &config);

    assert!(
        violations
            .iter()
            .all(|v| v.policy_id != "cross-layer-violation"),
        "importing from an unlayered target should not violate"
    );
}

/// Multiple forbidden imports from different symbols in the same delta.
/// Engine deduplicates by (`policy_id`, file, symbol), so two distinct
/// symbols each importing across layers produce two violations.
#[test]
fn multiple_cross_layer_violations_from_different_symbols() {
    let config = layered_config();
    let mut graph = SymbolGraph::new();

    graph
        .add_symbol(sym(
            100,
            "getUser",
            "src/domain/orchestrator.ts",
            Visibility::Internal,
            TrustLevel::Internal,
        ))
        .unwrap();
    graph
        .add_symbol(sym(
            101,
            "getOrder",
            "src/domain/orchestrator.ts",
            Visibility::Internal,
            TrustLevel::Internal,
        ))
        .unwrap();
    graph
        .add_symbol(sym(
            102,
            "dbQuery",
            "src/infra/db.ts",
            Visibility::Internal,
            TrustLevel::Internal,
        ))
        .unwrap();
    graph
        .add_symbol(sym(
            103,
            "cacheGet",
            "src/infra/cache.ts",
            Visibility::Internal,
            TrustLevel::Internal,
        ))
        .unwrap();
    graph
        .add_edge(SymbolEdge {
            from: 100,
            to: 102,
            edge_type: EdgeType::Imports,
        })
        .unwrap();
    graph
        .add_edge(SymbolEdge {
            from: 101,
            to: 103,
            edge_type: EdgeType::Imports,
        })
        .unwrap();

    let delta = GraphDelta {
        added_symbols: vec![100, 101],
        file: "src/domain/orchestrator.ts".to_string(),
        ..Default::default()
    };

    let mut engine = build_engine();
    let violations = engine.evaluate(&delta, &graph, &config);

    let cross_layer: Vec<_> = violations
        .iter()
        .filter(|v| v.policy_id == "cross-layer-violation")
        .collect();
    assert_eq!(
        cross_layer.len(),
        2,
        "two distinct symbols with forbidden imports should produce two violations"
    );
    let symbols: Vec<&str> = cross_layer.iter().map(|v| v.symbol.as_str()).collect();
    assert!(symbols.contains(&"getUser"));
    assert!(symbols.contains(&"getOrder"));
}

// ── Deduplication ───────────────────────────────────────────────────

/// Engine deduplicates violations by (`policy_id`, file, symbol) fingerprint
/// across multiple `evaluate()` calls for the same delta.
#[test]
fn engine_deduplicates_across_evaluations() {
    let config = layered_config();
    let mut graph = SymbolGraph::new();

    graph
        .add_symbol(sym(
            110,
            "newExport",
            "src/app/api.ts",
            Visibility::Public,
            TrustLevel::Internal,
        ))
        .unwrap();

    let delta = GraphDelta {
        added_symbols: vec![110],
        file: "src/app/api.ts".to_string(),
        ..Default::default()
    };

    let mut engine = build_engine();
    let first = engine.evaluate(&delta, &graph, &config);
    assert_eq!(first.len(), 1);

    let second = engine.evaluate(&delta, &graph, &config);
    assert!(
        second.is_empty(),
        "duplicate violation should be suppressed by fingerprint"
    );
}

// ── Gap analysis: JS features NOT ported (by design) ────────────────
//
// The following tests document intentional gaps between the JS
// architecture check and the kernel invariants. These are recorded
// as test cases so they serve as living documentation.

/// ARCH-001: Circular dependency detection is NOT in the kernel H1 set.
/// The kernel's `SymbolGraph` supports cycle detection via petgraph, but
/// a circular-dependency invariant has not been implemented. This is
/// intentional — the kernel focuses on symbol-level trust and layering
/// for H1, with cycle detection planned for a future invariant.
#[test]
fn gap_circular_dependency_not_detected() {
    let config = layered_config();
    let mut graph = SymbolGraph::new();

    // A → B → A cycle within the same layer (domain)
    graph
        .add_symbol(sym(
            200,
            "moduleA",
            "src/domain/a.ts",
            Visibility::Internal,
            TrustLevel::Internal,
        ))
        .unwrap();
    graph
        .add_symbol(sym(
            201,
            "moduleB",
            "src/domain/b.ts",
            Visibility::Internal,
            TrustLevel::Internal,
        ))
        .unwrap();
    graph
        .add_edge(SymbolEdge {
            from: 200,
            to: 201,
            edge_type: EdgeType::Imports,
        })
        .unwrap();
    graph
        .add_edge(SymbolEdge {
            from: 201,
            to: 200,
            edge_type: EdgeType::Imports,
        })
        .unwrap();

    // Graph must accept cycles — this is a prerequisite for future cycle detection
    assert_eq!(graph.edge_count(), 2, "graph must accept cycles");

    let delta = GraphDelta {
        added_symbols: vec![200],
        file: "src/domain/a.ts".to_string(),
        ..Default::default()
    };

    let mut engine = build_engine();
    let violations = engine.evaluate(&delta, &graph, &config);

    // Documenting the gap: no circular-dependency invariant exists
    assert!(
        violations
            .iter()
            .all(|v| v.policy_id != "circular-dependency"),
        "gap: circular dependency detection is not in H1 invariant set"
    );
}

/// ARCH-002: Orphaned module detection is NOT in the kernel H1 set.
/// The JS check uses dependency-cruiser's orphan detection to find
/// modules with no dependents. The kernel can compute this from
/// the graph but does not currently have an invariant for it.
#[test]
fn gap_orphaned_module_not_detected() {
    let config = layered_config();
    let mut graph = SymbolGraph::new();

    // Isolated symbol with no edges
    graph
        .add_symbol(sym(
            210,
            "orphanedHelper",
            "src/domain/orphan.ts",
            Visibility::Internal,
            TrustLevel::Internal,
        ))
        .unwrap();

    let delta = GraphDelta {
        added_symbols: vec![210],
        file: "src/domain/orphan.ts".to_string(),
        ..Default::default()
    };

    let mut engine = build_engine();
    let violations = engine.evaluate(&delta, &graph, &config);

    assert!(
        violations.iter().all(|v| v.policy_id != "orphaned-module"),
        "gap: orphaned module detection is not in H1 invariant set"
    );
}

/// ARCH-004: Generic "other" architecture violations are NOT in the kernel.
/// The JS check uses dependency-cruiser's catch-all rule category for
/// violations that don't match circular, orphan, or layer patterns. The
/// kernel has no equivalent — custom rules require new Invariant impls.
#[test]
fn gap_arch004_catch_all_not_detected() {
    let config = layered_config();
    let mut graph = SymbolGraph::new();

    // A symbol that imports from within the same layer — no violation from
    // any H1 invariant, but a custom dependency-cruiser rule could flag it
    // (e.g., "no-deprecated-imports"). The kernel has no catch-all for this.
    graph
        .add_symbol(sym(
            230,
            "caller",
            "src/domain/service.ts",
            Visibility::Internal,
            TrustLevel::Internal,
        ))
        .unwrap();
    graph
        .add_symbol(sym(
            231,
            "deprecated",
            "src/domain/legacy.ts",
            Visibility::Internal,
            TrustLevel::Internal,
        ))
        .unwrap();
    graph
        .add_edge(SymbolEdge {
            from: 230,
            to: 231,
            edge_type: EdgeType::Imports,
        })
        .unwrap();

    let delta = GraphDelta {
        added_symbols: vec![230],
        file: "src/domain/service.ts".to_string(),
        ..Default::default()
    };

    let mut engine = build_engine();
    let violations = engine.evaluate(&delta, &graph, &config);

    // Same-layer import, no visibility/trust flags — all invariants should pass.
    // Documents that custom dependency-cruiser rules have no kernel equivalent.
    assert!(
        violations.is_empty(),
        "gap: custom dependency-cruiser rules (ARCH-004) have no kernel equivalent"
    );
}

// ── Empty/edge cases ────────────────────────────────────────────────

/// Empty delta produces no violations from any invariant.
#[test]
fn empty_delta_no_violations() {
    let config = layered_config();
    let graph = SymbolGraph::new();
    let delta = GraphDelta::default();

    let mut engine = build_engine();
    let violations = engine.evaluate(&delta, &graph, &config);

    assert!(violations.is_empty(), "empty delta should produce nothing");
}

/// Empty config (no layers) should not panic and produce no cross-layer
/// violations, since no files match any layer.
#[test]
fn empty_config_no_panic() {
    let config = ArchitectureConfig { layers: Vec::new() };
    let mut graph = SymbolGraph::new();

    graph
        .add_symbol(sym(
            220,
            "fn",
            "src/domain/user.ts",
            Visibility::Public,
            TrustLevel::Privileged,
        ))
        .unwrap();

    let delta = GraphDelta {
        added_symbols: vec![220],
        file: "src/domain/user.ts".to_string(),
        ..Default::default()
    };

    let mut engine = build_engine();
    let violations = engine.evaluate(&delta, &graph, &config);

    // Should still fire public-api-expansion and privilege-expansion
    // (these don't depend on layer config), but not cross-layer
    assert!(
        violations
            .iter()
            .all(|v| v.policy_id != "cross-layer-violation")
    );
    assert!(
        violations
            .iter()
            .any(|v| v.policy_id == "public-api-expansion")
    );
    assert!(
        violations
            .iter()
            .any(|v| v.policy_id == "privilege-expansion")
    );
}
