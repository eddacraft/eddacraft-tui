// Architecture check parity validation (RENG-004)
//
// Validates that the kernel's H1 invariants produce equivalent results
// to the current JS architecture check. Uses fixture data representing
// repos with known violations.

use anvil_kernel::graph::SymbolGraph;
use anvil_kernel::graph::incremental::GraphDelta;
use anvil_kernel::policy::config::ArchitectureConfig;
use anvil_kernel::policy::engine::{PolicyEngine, Severity};
use anvil_kernel::policy::invariants::cross_layer::CrossLayerViolation;
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

fn build_engine() -> PolicyEngine {
    let mut engine = PolicyEngine::new();
    engine.register(Box::new(CrossLayerViolation));
    engine.register(Box::new(PublicApiExpansion));
    engine.register(Box::new(PrivilegeExpansion));
    engine
}

/// Fixture: domain layer imports from infrastructure — forbidden.
#[test]
fn cross_layer_violation_detected() {
    let config = layered_config();
    let mut graph = SymbolGraph::new();

    // domain function importing from infrastructure
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
}

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

/// Fixture: file outside any layer — should not trigger cross-layer.
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
