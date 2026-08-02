use std::fmt::Write as _;
use std::fs;

use anvil_dashboard_server::{Workspace, load_plan, load_plans};
use tempfile::tempdir;

fn write_plan_fixture(root: &std::path::Path) {
    fs::create_dir_all(root.join("plans/modules")).expect("modules");
    // Explicit LF bytes — do not rely on source-file line endings when the
    // suite is compiled/run on Windows.
    fs::write(
        root.join("plans/index.aps.md"),
        b"| Module | Scope | Status | Progress | Notes |\n| --- | --- | --- | --- | --- |\n| [Dashboard Foundation](./modules/dashboard-foundation.aps.md) | DASH | Ready | 1/11 | - |\n",
    )
    .expect("index");
    fs::write(
        root.join("plans/modules/dashboard-foundation.aps.md"),
        b"# Dashboard Foundation\n\n| ID | Owner | Status | Progress |\n| --- | --- | --- | --- |\n| DASH | @eddacraft | Ready | 1/11 |\n\n## Purpose\n\nShip the local dashboard.\n\n### DASH-001: Scaffold\n\n- **Status:** Done\n- **Validation:** `pnpm test`\n\n### DASH-002: Proof view\n\n- **Status:** Ready\n- **Validation:** `pnpm typecheck`\n- **Dependencies:** DASH-001\n",
    )
    .expect("module");
}

#[test]
fn lists_indexed_plans_and_loads_selected_detail() {
    let root = tempdir().expect("workspace");
    let root_path = fs::canonicalize(root.path()).expect("canonicalize workspace");
    write_plan_fixture(&root_path);
    let workspace = Workspace::new(&root_path).expect("workspace boundary");

    let plans = load_plans(&workspace).expect("plan list");
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].id, "dashboard-foundation");
    assert_eq!(plans[0].scope, "DASH");
    assert_eq!(plans[0].status, "Ready");
    assert_eq!(plans[0].progress, "1/11");

    let detail = load_plan(&workspace, "dashboard-foundation")
        .expect("plan read")
        .expect("indexed plan");
    assert_eq!(detail.summary.title, "Dashboard Foundation");
    assert!(
        detail.purpose.contains("Ship the local dashboard"),
        "purpose was: {:?}",
        detail.purpose
    );
    assert!(!detail.actions_enabled);
    assert_eq!(detail.timeline.len(), 2);
    assert_eq!(detail.timeline[0].id, "DASH-001");
    assert_eq!(
        detail.timeline[0].validation_contract.as_deref(),
        Some("pnpm test")
    );
    assert!(detail.timeline[1].readiness);
    assert!(detail.action_message.contains("deferred"));
}

#[test]
fn duplicate_module_paths_are_read_once_and_returned_once() {
    let root = tempdir().expect("workspace");
    write_plan_fixture(root.path());
    fs::write(
        root.path().join("plans/index.aps.md"),
        "| Module | Scope | Status | Progress | Notes |\n| --- | --- | --- | --- | --- |\n| [Dashboard Foundation](./modules/dashboard-foundation.aps.md) | DASH | Ready | 1/11 | first |\n| [Dashboard Duplicate](./modules/dashboard-foundation.aps.md) | DASH2 | Ready | 1/11 | duplicate |\n",
    )
    .expect("duplicate index");
    let workspace = Workspace::new(root.path()).expect("workspace boundary");

    let plans = load_plans(&workspace).expect("bounded plan list");

    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].id, "dashboard-foundation");
}

#[test]
fn rejects_workspaces_that_exceed_aggregate_plan_limits() {
    let root = tempdir().expect("workspace");
    fs::create_dir_all(root.path().join("plans/modules")).expect("modules");
    let mut index = String::from(
        "| Module | Scope | Status | Progress | Notes |\n| --- | --- | --- | --- | --- |\n",
    );
    for number in 0..=anvil_dashboard_server::MAX_PLAN_MODULES {
        let id = format!("module-{number}");
        writeln!(
            index,
            "| [{id}](./modules/{id}.aps.md) | M{number} | Ready | 0/1 | - |"
        )
        .expect("index row");
    }
    fs::write(root.path().join("plans/index.aps.md"), index).expect("index");
    let workspace = Workspace::new(root.path()).expect("workspace boundary");

    let error = load_plans(&workspace).expect_err("module budget must fail closed");

    assert!(error.to_string().contains("module count"), "{error}");
}

#[test]
fn plan_ids_cannot_be_used_as_paths() {
    let root = tempdir().expect("workspace");
    write_plan_fixture(root.path());
    let workspace = Workspace::new(root.path()).expect("workspace boundary");

    assert!(load_plan(&workspace, "../secrets").is_err());
    assert!(load_plan(&workspace, "dashboard/foundation").is_err());
}
