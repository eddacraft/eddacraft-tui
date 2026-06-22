use std::io::IsTerminal;
use std::path::Path;
use std::process::Command;

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
        PlanCommand::Dashboard(_args) => {
            // CIB-046: the APS dashboard is an internal-developer surface,
            // gated behind `tui-dashboard.aps-dashboard`. Refuse before any
            // workspace I/O when the gate is closed.
            if !crate::feature_flags::aps_dashboard_access_allowed() {
                return Err(refuse_dashboard(global));
            }
            run_dashboard(global)
        }
    }
}

/// CIB-046: refuse the internal-developer APS dashboard when the gate is
/// closed. Mirrors the admin auth-required envelope so structured consumers
/// get a JSON error and humans get a clear next step, and returns
/// [`crate::output::AuthRequired`] so `main` maps it to `EXIT_AUTH_REQUIRED`.
fn refuse_dashboard(global: &GlobalArgs) -> anyhow::Error {
    let detail = "`anvil plan dashboard` is an internal-developer surface. Set \
         ANVIL_DEV=1 for local development, or set ANVIL_ADMIN_KEY.";
    if global.json {
        eprintln!(
            "{}",
            serde_json::json!({
                "error": "authentication_required",
                "detail": detail,
            })
        );
    } else {
        eprintln!("Authentication required: {detail}");
    }
    crate::output::AuthRequired.into()
}

fn run_dashboard(global: &GlobalArgs) -> anyhow::Result<()> {
    let root = util::workspace_root()?;

    if global.json {
        let snapshot = plan_dashboard::build_plan_status_snapshot(&root)?;
        println!("{}", serde_json::to_string_pretty(&snapshot)?);
    } else if global.no_tui || !std::io::stdout().is_terminal() || !std::io::stdin().is_terminal() {
        let snapshot = plan_dashboard::build_plan_status_snapshot(&root)?;
        print_summary(&snapshot);
    } else {
        loop {
            let snapshot = plan_dashboard::build_plan_status_snapshot(&root)?;
            let state = PlanDashboardState::new(to_tui_snapshot(&snapshot));
            let state = tui::run_surface(state)?;
            if !state.rescan_requested {
                break;
            }
        }
    }

    Ok(())
}

fn print_summary(snapshot: &plan_dashboard::PlanStatusSnapshot) {
    println!("anvil APS Work Dashboard");
    println!("Modules: {}", snapshot.modules.len());
    println!(
        "Open work items: {}",
        snapshot
            .work_items
            .iter()
            .filter(|item| !plan_dashboard::is_done_status(&item.status))
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
            .filter(|item| !plan_dashboard::is_done_status(&item.status))
            .map(|item| PlanWorkItemRow {
                id: item.id.clone(),
                title: item.title.clone(),
                module: item.module.clone(),
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
                module: warning.module.clone(),
                message: warning.message.clone(),
            })
            .collect(),
        branch: git_output(&snapshot.repo_root, &["rev-parse", "--abbrev-ref", "HEAD"])
            .filter(|branch| branch != "HEAD"),
        sha: git_output(&snapshot.repo_root, &["rev-parse", "--short", "HEAD"]),
    }
}

fn git_output(repo_root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!value.is_empty()).then_some(value)
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
            work_items: vec![
                plan_dashboard::WorkItemSummary {
                    id: "APSCAN-011".to_string(),
                    title: "Add APS TUI dashboard".to_string(),
                    module: "APSCAN".to_string(),
                    status: "Ready".to_string(),
                    validation: Some("cargo test".to_string()),
                    dependencies: Vec::new(),
                    files: Vec::new(),
                    body: String::new(),
                },
                plan_dashboard::WorkItemSummary {
                    id: "APSCAN-010".to_string(),
                    title: "Archived item".to_string(),
                    module: "APSCAN".to_string(),
                    status: "Archived".to_string(),
                    validation: None,
                    dependencies: Vec::new(),
                    files: Vec::new(),
                    body: String::new(),
                },
            ],
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
        assert_eq!(tui_snapshot.work_items.len(), 1);
        assert_eq!(tui_snapshot.warnings[0].target, "APSCAN-011");
    }

    fn test_global() -> GlobalArgs {
        GlobalArgs {
            json: false,
            no_tui: false,
            verbose: false,
            anvil_home: None,
            touch_project_state: false,
        }
    }

    #[test]
    fn refuse_dashboard_returns_auth_required() {
        // CIB-046: a closed gate yields an AuthRequired error so `main`
        // maps it to EXIT_AUTH_REQUIRED, the same exit code as `admin`.
        let err = refuse_dashboard(&test_global());
        assert!(err.is::<crate::output::AuthRequired>());
    }
}
