//! `anvil dashboard architecture` — native architecture-health dashboard
//! (TDASH-002). Loads `.anvil/architecture.json` via the architecture crate's
//! baseline reader and renders it: `--json` emits a structured snapshot,
//! non-TTY prints a plain summary, TTY runs the Ratatui surface. A missing
//! baseline is a legitimate empty state, not an error.

use std::io::IsTerminal;

use anvil_architecture::baseline::load_baseline;
use anvil_architecture::types::ArchitectureBaseline;
use anvil_tui::surfaces::dashboard::architecture::{
    ArchViolationRow, ArchitectureDashboardState, ArchitectureView,
};
use serde::Serialize;

use crate::{GlobalArgs, tui, util};

/// Serializable architecture snapshot for `--json`.
#[derive(Debug, Serialize)]
struct ArchitectureSnapshot {
    created_at: String,
    updated_at: String,
    module_count: u32,
    layer_count: usize,
    boundary_count: usize,
    entry_point_count: usize,
    violation_count: usize,
    violations: Vec<ViolationRecord>,
}

#[derive(Debug, Serialize)]
struct ViolationRecord {
    from_layer: String,
    to_layer: String,
    from_file: String,
    /// Imported file. Carried in `--json` for completeness; the compact TUI
    /// table omits it (it shows `from_file`, the actionable location).
    to_file: String,
    import_line: u32,
    rule: Option<String>,
}

/// Stable `--json` envelope. `baseline_present` lets consumers distinguish "no
/// baseline" from a present-but-empty one without the top-level value flipping
/// between an object and `null`.
#[derive(Debug, Serialize)]
struct ArchitectureJson {
    baseline_present: bool,
    snapshot: Option<ArchitectureSnapshot>,
}

/// Run the architecture dashboard. Returns how the surface exited so the
/// picker can return to itself on [`SurfaceExit::Back`]. Non-interactive
/// branches (`--json`, no-TTY) print and report `Quit`.
pub fn run(global: &GlobalArgs) -> anyhow::Result<tui::SurfaceExit> {
    let root = util::workspace_root()?;
    // Propagate load errors (corrupt/unreadable baseline); `Ok(None)` is the
    // legitimate no-baseline empty state, not a failure.
    let baseline = load_baseline(&root)?;

    if global.json {
        // Stable envelope so the top-level shape never flips between an object
        // and `null`: `baseline_present` flags absence, `snapshot` carries data.
        let snapshot = baseline.as_ref().map(snapshot_from);
        let payload = ArchitectureJson {
            baseline_present: snapshot.is_some(),
            snapshot,
        };
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(tui::SurfaceExit::Quit);
    }

    if global.no_tui || !std::io::stdout().is_terminal() || !std::io::stdin().is_terminal() {
        print_summary(baseline.as_ref().map(snapshot_from).as_ref());
        return Ok(tui::SurfaceExit::Quit);
    }

    let view = baseline.as_ref().map(view_from);
    let (_, exit) = tui::run_surface_with_exit(ArchitectureDashboardState::new(view))?;
    Ok(exit)
}

fn snapshot_from(baseline: &ArchitectureBaseline) -> ArchitectureSnapshot {
    ArchitectureSnapshot {
        created_at: baseline.created_at.clone(),
        updated_at: baseline.updated_at.clone(),
        module_count: baseline.baseline_snapshot.module_count,
        layer_count: baseline.layers.len(),
        boundary_count: baseline.boundaries.len(),
        entry_point_count: baseline.entry_points.len(),
        violation_count: baseline.baseline_snapshot.violations.len(),
        violations: baseline
            .baseline_snapshot
            .violations
            .iter()
            .map(|violation| ViolationRecord {
                from_layer: violation.from_layer.clone(),
                to_layer: violation.to_layer.clone(),
                from_file: violation.from_file.clone(),
                to_file: violation.to_file.clone(),
                import_line: violation.import_line,
                rule: violation.rule.clone(),
            })
            .collect(),
    }
}

fn view_from(baseline: &ArchitectureBaseline) -> ArchitectureView {
    ArchitectureView {
        created_at: baseline.created_at.clone(),
        updated_at: baseline.updated_at.clone(),
        module_count: baseline.baseline_snapshot.module_count,
        layer_count: baseline.layers.len(),
        boundary_count: baseline.boundaries.len(),
        entry_point_count: baseline.entry_points.len(),
        violations: baseline
            .baseline_snapshot
            .violations
            .iter()
            .map(|violation| ArchViolationRow {
                from_layer: violation.from_layer.clone(),
                to_layer: violation.to_layer.clone(),
                from_file: violation.from_file.clone(),
                import_line: violation.import_line,
                rule: violation.rule.clone(),
            })
            .collect(),
    }
}

fn print_summary(snapshot: Option<&ArchitectureSnapshot>) {
    let Some(snapshot) = snapshot else {
        println!("No architecture baseline found at .anvil/architecture.json.");
        return;
    };
    println!("Architecture Health");
    println!("  Modules:               {}", snapshot.module_count);
    println!("  Layers:                {}", snapshot.layer_count);
    println!("  Boundaries:            {}", snapshot.boundary_count);
    println!("  Entry points:          {}", snapshot.entry_point_count);
    println!("  Baselined violations:  {}", snapshot.violation_count);
    println!(
        "  Baselined {} · updated {}",
        snapshot.created_at, snapshot.updated_at
    );
}

#[cfg(test)]
mod tests {
    use anvil_architecture::baseline::{CreateBaselineOptions, create_baseline};
    use anvil_architecture::types::BaselineViolation;

    use super::*;

    fn baseline_with(
        module_count: u32,
        violations: Vec<BaselineViolation>,
    ) -> ArchitectureBaseline {
        create_baseline(CreateBaselineOptions {
            entry_points: vec![],
            layers: None,
            boundaries: None,
            violations,
            module_count,
        })
    }

    #[test]
    fn snapshot_maps_counts_from_baseline() {
        let baseline = baseline_with(
            17,
            vec![BaselineViolation {
                id: "abc".to_string(),
                from_layer: "ui".to_string(),
                to_layer: "db".to_string(),
                from_file: "a.ts".to_string(),
                to_file: "b.ts".to_string(),
                import_line: 5,
                rule: Some("no-ui-to-db".to_string()),
            }],
        );
        let snapshot = snapshot_from(&baseline);
        assert_eq!(snapshot.module_count, 17);
        assert_eq!(snapshot.violation_count, 1);
        assert_eq!(snapshot.violations[0].rule.as_deref(), Some("no-ui-to-db"));
        // Default layers/boundaries are populated by create_baseline.
        assert!(snapshot.layer_count > 0);
    }

    #[test]
    fn view_and_snapshot_agree_on_violation_count() {
        let baseline = baseline_with(3, vec![]);
        assert_eq!(
            snapshot_from(&baseline).violation_count,
            view_from(&baseline).violations.len()
        );
    }
}
