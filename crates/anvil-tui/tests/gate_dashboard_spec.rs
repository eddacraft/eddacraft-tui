//! End-to-end test for the shipped `gate-summary` dashboard spec (#2237).
//!
//! Proves the shipped gate-summary spec (crate assets, seeded to
//! `.anvil/dashboards/` by `anvil init`) parses,
//! validates against the Anvil catalogue, and renders through the engine when
//! bound to a `gates.*` data context matching the shape `anvil gate` persists
//! to `.anvil/gates.json` (#2242, `GateSnapshot`). If the spec's `$data` paths
//! or the persisted shape drift apart, this test fails.

use anvil_tui::dashboard_catalog::{anvil_catalog, anvil_registry};
use eddacraft_tui::json_render::{DataContext, bind, parse, render_spec, validate};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

/// The shipped spec — the same constant `anvil init` seeds from, so the test
/// tracks the real artefact (ADR-073/CIB-053 moved it into crate assets).
const GATE_SPEC: &str = anvil_tui::dashboard_catalog::GATE_SUMMARY_SPEC;

/// A representative `.anvil/gates.json`, mirroring `GateSnapshot`'s camelCase
/// shape (status/statusLabel/checksRun/warnings/durationSeconds/checkRows/
/// warningList). Wrapped under the `gates` key, matching how `load_context`
/// keys `.anvil/<stem>.json`.
fn gates_context() -> DataContext {
    DataContext::new(serde_json::json!({
        "gates": {
            "status": "fail",
            "statusLabel": "FAILED — score 50/100",
            "score": 50.0,
            "checksRun": "2",
            "warnings": "1",
            "durationSeconds": "4",
            "checkRows": [
                ["lint", "passed", "100", "clean"],
                ["secret", "failed", "0", "leak found"]
            ],
            "warningList": [
                { "severity": "error", "message": "secret: leak found" }
            ]
        }
    }))
}

#[test]
fn shipped_gate_summary_spec_parses_and_validates() {
    let spec = parse(GATE_SPEC).expect("gate-summary spec parses");
    // Every component the spec names must be in the Anvil catalogue (base +
    // domain), so nothing degrades to a placeholder.
    validate(&spec, &anvil_catalog())
        .expect("gate-summary spec validates against the Anvil catalogue");
}

#[test]
fn shipped_gate_summary_renders_bound_data() {
    let spec = parse(GATE_SPEC).expect("parse");
    let bound = bind(&spec, &gates_context());
    let registry = anvil_registry();

    let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("backend");
    terminal
        .draw(|frame| render_spec(&bound, &registry, frame, frame.area()))
        .expect("draw");
    let text: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();

    // Bound values from the data context appear...
    assert!(
        text.contains("FAILED — score 50/100"),
        "status label bound: {text:?}"
    );
    assert!(
        text.contains("lint") && text.contains("secret"),
        "check rows bound"
    );
    assert!(text.contains("leak found"), "warning list bound");
    // ...and no component fell through to a placeholder.
    assert!(
        !text.contains("not available in terminal"),
        "every component in the spec is mapped"
    );
}
