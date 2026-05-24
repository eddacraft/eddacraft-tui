use std::io::IsTerminal;

use clap::{Args, Subcommand};

use anvil_tui::surfaces::plan_dashboard::{
    PlanDashboardSnapshot, PlanDashboardState, PlanModuleRow, PlanWarningRow, PlanWorkItemRow,
};

use crate::{GlobalArgs, plan_dashboard, tui, util};

#[derive(Debug, Args)]
pub struct PlanArgs {
    #[command(subcommand)]
    command: PlanCommand,
}

#[derive(Debug, Subcommand)]
enum PlanCommand {
    /// Show active APS work in a read-only dashboard.
    Dashboard(DashboardArgs),
}

#[derive(Debug, Args)]
pub struct DashboardArgs {}

pub fn run(args: &PlanArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    match &args.command {
        PlanCommand::Dashboard(_args) => run_dashboard(global),
    }
}

fn run_dashboard(global: &GlobalArgs) -> anyhow::Result<()> {
    let root = util::workspace_root()?;
    let snapshot = plan_dashboard::build_plan_status_snapshot(&root)?;

    if global.json {
        println!("{}", serde_json::to_string_pretty(&snapshot)?);
    } else if global.no_tui || !std::io::stdout().is_terminal() || !std::io::stdin().is_terminal() {
        print_summary(&snapshot);
    } else {
        let state = PlanDashboardState::new(to_tui_snapshot(&snapshot));
        let _state = tui::run_surface(state)?;
    }

    Ok(())
}

fn print_summary(snapshot: &plan_dashboard::PlanStatusSnapshot) {
    println!("Anvil APS Work Dashboard");
    println!("Modules: {}", snapshot.modules.len());
    println!(
        "Open work items: {}",
        snapshot
            .work_items
            .iter()
            .filter(|item| !is_done_status(&item.status))
            .count()
    );
    println!("Warnings: {}", snapshot.warnings.len());
    println!();

    for module in &snapshot.modules {
        let progress = match (module.done, module.total) {
            (Some(done), Some(total)) => format!("{done}/{total}"),
            _ => "-".to_string(),
        };
        let warning_marker = if snapshot
            .warnings
            .iter()
            .any(|warning| warning.module.as_deref() == Some(module.scope.as_str()))
        {
            " !"
        } else {
            ""
        };
        println!(
            "{:<10} {:<8} {}{}",
            module.scope, progress, module.status, warning_marker
        );
    }
}

fn to_tui_snapshot(snapshot: &plan_dashboard::PlanStatusSnapshot) -> PlanDashboardSnapshot {
    PlanDashboardSnapshot {
        modules: snapshot
            .modules
            .iter()
            .map(|module| PlanModuleRow {
                scope: module.scope.clone(),
                progress: match (module.done, module.total) {
                    (Some(done), Some(total)) => format!("{done}/{total}"),
                    _ => "-".to_string(),
                },
                status: module.status.clone(),
                note: module.notes.clone().unwrap_or_else(|| module.title.clone()),
                has_warning: snapshot
                    .warnings
                    .iter()
                    .any(|warning| warning.module.as_deref() == Some(module.scope.as_str())),
            })
            .collect(),
        work_items: snapshot
            .work_items
            .iter()
            .filter(|item| !is_done_status(&item.status))
            .map(|item| PlanWorkItemRow {
                id: item.id.clone(),
                title: item.title.clone(),
                status: item.status.clone(),
                validation: item.validation.clone(),
            })
            .collect(),
        warnings: snapshot
            .warnings
            .iter()
            .map(|warning| PlanWarningRow {
                target: warning
                    .work_item
                    .clone()
                    .or_else(|| warning.module.clone())
                    .unwrap_or_else(|| "plans".to_string()),
                message: warning.message.clone(),
            })
            .collect(),
        branch: None,
        sha: None,
    }
}

fn is_done_status(status: &str) -> bool {
    matches!(
        status.to_ascii_lowercase().as_str(),
        "complete" | "completed" | "done" | "merged" | "released/shipped"
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn tui_snapshot_marks_module_warnings() {
        let snapshot = plan_dashboard::PlanStatusSnapshot {
            repo_root: PathBuf::from("/repo"),
            modules: vec![plan_dashboard::ModuleSummary {
                scope: "APSCAN".to_string(),
                title: "APS Canonical Alignment".to_string(),
                path: PathBuf::from("modules/aps.aps.md"),
                status: "In Progress".to_string(),
                done: Some(1),
                total: Some(11),
                section: None,
                notes: None,
            }],
            work_items: vec![plan_dashboard::WorkItemSummary {
                id: "APSCAN-011".to_string(),
                title: "Add APS TUI dashboard".to_string(),
                module: "APSCAN".to_string(),
                status: "Ready".to_string(),
                validation: Some("cargo test".to_string()),
                dependencies: Vec::new(),
                files: Vec::new(),
                body: String::new(),
            }],
            warnings: vec![plan_dashboard::PlanWarning {
                kind: plan_dashboard::PlanWarningKind::MissingValidation,
                module: Some("APSCAN".to_string()),
                work_item: Some("APSCAN-011".to_string()),
                message: "open work item has no validation command".to_string(),
            }],
            enrichments: Vec::new(),
        };

        let tui_snapshot = to_tui_snapshot(&snapshot);

        assert!(tui_snapshot.modules[0].has_warning);
        assert_eq!(tui_snapshot.modules[0].progress, "1/11");
        assert_eq!(tui_snapshot.work_items[0].id, "APSCAN-011");
        assert_eq!(tui_snapshot.warnings[0].target, "APSCAN-011");
    }
}
