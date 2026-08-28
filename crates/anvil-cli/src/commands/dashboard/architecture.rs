//! `anvil dashboard architecture` — native architecture-health dashboard
//! (TDASH-002). Loads the live architecture definition (project config
//! section or `.anvil/architecture.yaml`) and renders it: `--json` emits a
//! structured snapshot, non-TTY prints a plain summary, TTY runs the
//! Ratatui surface. A missing definition is a legitimate empty state, not
//! an error. The TypeScript-era `.anvil/architecture.json` snapshot is
//! not consulted.

use std::io::IsTerminal;

use anvil_architecture::ArchitectureDefinition;
use anvil_tui::surfaces::dashboard::architecture::{ArchitectureDashboardState, ArchitectureView};

use crate::architecture_source::{ArchitectureOrigin, resolve_architecture};
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
    let resolved = resolve_architecture(&root)?;
    let snapshot = resolved
        .as_ref()
        .map(|(definition, origin)| snapshot_from_definition(definition, origin));

    if global.json {
        let payload = ArchitectureJson {
            baseline_present: snapshot.is_some(),
            snapshot,
        };
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(tui::SurfaceExit::Quit);
    }

    if global.no_tui || !std::io::stdout().is_terminal() || !std::io::stdin().is_terminal() {
        print_summary(snapshot.as_ref());
        return Ok(tui::SurfaceExit::Quit);
    }

    let view = snapshot.as_ref().map(view_from_snapshot);
    let (_, exit) = tui::run_surface_with_exit(ArchitectureDashboardState::new(view))?;
    Ok(exit)
}

fn origin_label(origin: &ArchitectureOrigin) -> String {
    match origin {
        ArchitectureOrigin::Section(_) => "project config".to_string(),
        ArchitectureOrigin::LegacyFile(_) => ".anvil/architecture.yaml".to_string(),
    }
}

fn snapshot_from_definition(
    definition: &ArchitectureDefinition,
    origin: &ArchitectureOrigin,
) -> ArchitectureSnapshot {
    ArchitectureSnapshot {
        created_at: origin_label(origin),
        updated_at: "live".to_string(),
        module_count: 0,
        layer_count: definition.layers.len(),
        boundary_count: definition.rules.len(),
        entry_point_count: 0,
        violation_count: 0,
        violations: vec![],
    }
}

fn view_from_snapshot(snapshot: &ArchitectureSnapshot) -> ArchitectureView {
    ArchitectureView {
        created_at: snapshot.created_at.clone(),
        updated_at: snapshot.updated_at.clone(),
        module_count: snapshot.module_count,
        layer_count: snapshot.layer_count,
        boundary_count: snapshot.boundary_count,
        entry_point_count: snapshot.entry_point_count,
        violations: vec![],
    }
}

fn print_summary(snapshot: Option<&ArchitectureSnapshot>) {
    let Some(snapshot) = snapshot else {
        println!(
            "No architecture definition found. Add an `architecture` section to the project config, or create `.anvil/architecture.yaml`."
        );
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
    use std::collections::BTreeMap;

    use anvil_architecture::{ArchitectureDefinition, ArchitectureTemplate, Layer};

    use super::*;

    fn sample_definition() -> ArchitectureDefinition {
        let mut layers = BTreeMap::new();
        layers.insert(
            "ui".into(),
            Layer {
                patterns: vec!["src/ui/**".into()],
                depends_on: vec!["domain".into()],
                description: None,
            },
        );
        layers.insert(
            "domain".into(),
            Layer {
                patterns: vec!["src/domain/**".into()],
                depends_on: vec![],
                description: None,
            },
        );
        ArchitectureDefinition {
            schema_version: "0.1.0".into(),
            template: ArchitectureTemplate::Custom,
            layers,
            bounded_contexts: None,
            rules: vec![],
            options: None,
        }
    }

    #[test]
    fn snapshot_maps_counts_from_definition() {
        let definition = sample_definition();
        let origin =
            ArchitectureOrigin::LegacyFile(std::path::PathBuf::from(".anvil/architecture.yaml"));
        let snapshot = snapshot_from_definition(&definition, &origin);
        assert_eq!(snapshot.layer_count, 2);
        assert_eq!(snapshot.violation_count, 0);
        assert_eq!(snapshot.created_at, ".anvil/architecture.yaml");
        assert_eq!(snapshot.updated_at, "live");
    }

    #[test]
    fn view_and_snapshot_agree_on_layer_count() {
        let definition = sample_definition();
        let origin =
            ArchitectureOrigin::LegacyFile(std::path::PathBuf::from(".anvil/architecture.yaml"));
        let snapshot = snapshot_from_definition(&definition, &origin);
        assert_eq!(
            snapshot.layer_count,
            view_from_snapshot(&snapshot).layer_count
        );
    }
}
