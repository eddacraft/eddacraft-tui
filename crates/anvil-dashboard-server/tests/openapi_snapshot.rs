use std::fs;
use std::path::PathBuf;

use anvil_dashboard_server::openapi_document;

#[test]
fn committed_openapi_contract_matches_the_rust_export() {
    let contract = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/dashboard/src/api/generated/openapi.json");
    let committed = fs::read_to_string(contract).expect("committed dashboard OpenAPI contract");
    let committed: serde_json::Value =
        serde_json::from_str(&committed).expect("parse committed dashboard OpenAPI contract");

    assert_eq!(committed, openapi_document());
}

#[test]
fn contract_exposes_protection_and_plan_driver_reads() {
    let document = openapi_document();
    let paths = document["paths"].as_object().expect("OpenAPI paths");

    assert_eq!(
        paths["/api/v1/protection"]["get"]["operationId"],
        "getProtectionOverview"
    );
    assert_eq!(paths["/api/v1/plans"]["get"]["operationId"], "listPlans");
    assert_eq!(paths["/api/v1/plans/{id}"]["get"]["operationId"], "getPlan");
    assert!(paths.values().all(|path| path.get("post").is_none()));
}
