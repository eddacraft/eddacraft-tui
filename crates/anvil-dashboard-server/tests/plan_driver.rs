use std::fs;

use anvil_dashboard_server::{Workspace, load_plan, load_plans};
use tempfile::tempdir;

fn write_plan_fixture(root: &std::path::Path) {
    fs::create_dir_all(root.join("plans/modules")).expect("modules");
    fs::write(
        root.join("plans/index.aps.md"),
        "| Module | Scope | Status | Progress | Notes |\n| --- | --- | --- | --- | --- |\n| [Dashboard Foundation](./modules/dashboard-foundation.aps.md) | DASH | Ready | 1/11 | - |\n",
    )
    .expect("index");
    fs::write(
        root.join("plans/modules/dashboard-foundation.aps.md"),
        "# Dashboard Foundation\n\n| ID | Owner | Status | Progress |\n| --- | --- | --- | --- |\n| DASH | @eddacraft | Ready | 1/11 |\n\n## Purpose\n\nShip the local dashboard.\n\n### DASH-001: Scaffold\n\n- **Status:** Done\n- **Validation:** `pnpm test`\n\n### DASH-002: Proof view\n\n- **Status:** Ready\n- **Validation:** `pnpm typecheck`\n- **Dependencies:** DASH-001\n",
    )
    .expect("module");
}

#[test]
fn lists_indexed_plans_and_loads_selected_detail() {
    let root = tempdir().expect("workspace");
    write_plan_fixture(root.path());
    let workspace = Workspace::new(root.path()).expect("workspace boundary");

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
    assert!(detail.purpose.contains("Ship the local dashboard"));
    assert!(!detail.actions_enabled);
    assert_eq!(detail.timeline.len(), 2);
    assert_eq!(detail.timeline[0].id, "DASH-001");
    assert_eq!(detail.timeline[0].evidence.as_deref(), Some("pnpm test"));
    assert!(detail.timeline[1].readiness);
    assert!(detail.action_message.contains("deferred"));
}

#[test]
fn plan_ids_cannot_be_used_as_paths() {
    let root = tempdir().expect("workspace");
    write_plan_fixture(root.path());
    let workspace = Workspace::new(root.path()).expect("workspace boundary");

    assert!(load_plan(&workspace, "../secrets").is_err());
    assert!(load_plan(&workspace, "dashboard/foundation").is_err());
}
