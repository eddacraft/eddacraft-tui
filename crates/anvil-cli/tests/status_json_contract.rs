//! ADTRUST-005: `anvil status --json` contract test.
//!
//! Pins the v1 wire shape consumed by editor extensions and CI hooks
//! against the JSON Schema at `schemas/anvil-status.v1.json`. The
//! checks are intentionally narrow:
//!
//! - `schema_version` is the constant `"anvil.status.v1"` so consumers
//!   can refuse mismatched documents at the deserialise boundary.
//! - Every required top-level key the schema names is present.
//! - Nested required keys (`activation.state`, hook entries, profile,
//!   recent-run fields) are present with the right primitive shape.
//! - Activation state is from the documented closed set.
//!
//! The schema file itself is read and parsed by the test so a typo
//! in the file is a compile-time-ish failure rather than a silent
//! drift. Full JSON Schema validation is intentionally not run here
//! (no validator dependency in this crate's surface); that lane is
//! served by `pnpm run validate:schemas` against the same file in CI.

use std::path::Path;
use std::process::{Command, Output};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

fn run_status_json(workdir: &Path, home: &Path) -> Output {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui")
        .arg("--json")
        .arg("status")
        .current_dir(workdir)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env_remove("XDG_CONFIG_HOME")
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1");
    cmd.output().expect("failed to invoke anvil binary")
}

fn workspace_root() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR points at crates/anvil-cli; ascend two levels
    // to reach the workspace root so the schema file resolves from
    // any test runner.
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .nth(2)
        .expect("workspace root above anvil-cli")
        .to_path_buf()
}

fn load_schema() -> serde_json::Value {
    let path = workspace_root().join("schemas/anvil-status.v1.json");
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|err| panic!("failed to read schema at {}: {err}", path.display()));
    serde_json::from_slice(&bytes).expect("schema must be valid JSON")
}

#[cfg(not(target_os = "windows"))]
#[test]
fn status_json_emits_pinned_schema_version() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let out = run_status_json(dir.path(), home.path());
    assert!(
        out.status.success(),
        "anvil --json status failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let doc: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|err| panic!("status JSON did not parse: {err}\n--- stdout ---\n{stdout}"));

    assert_eq!(
        doc.get("schema_version").and_then(|v| v.as_str()),
        Some("anvil.status.v1"),
        "schema_version must pin to anvil.status.v1: {doc}"
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "single integration assertion sweep over the full v1 shape; splitting it would scatter the contract across helper functions without any reuse"
)]
fn status_json_top_level_keys_match_schema_contract() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let out = run_status_json(dir.path(), home.path());
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let doc: serde_json::Value = serde_json::from_str(&stdout).expect("status JSON must parse");

    let schema = load_schema();
    let required = schema
        .get("required")
        .and_then(|v| v.as_array())
        .expect("schema must declare top-level required");
    for key in required {
        let key = key.as_str().expect("required entries are strings");
        assert!(
            doc.get(key).is_some(),
            "status JSON missing required top-level key {key:?}; doc={doc}"
        );
    }

    // Activation block: state is the load-bearing field for surfaces.
    let activation = doc
        .get("activation")
        .and_then(|v| v.as_object())
        .expect("activation block is an object");
    let state = activation
        .get("state")
        .and_then(|v| v.as_str())
        .expect("activation.state is a string");
    let allowed = [
        "protecting",
        "ready_restart_required",
        "watching",
        "needs_action",
        "unsupported",
        "error",
    ];
    assert!(
        allowed.contains(&state),
        "activation.state {state:?} not in closed set {allowed:?}",
    );

    // Hooks shape: every entry has name/active/path.
    let hooks = doc
        .get("hooks")
        .and_then(|v| v.as_array())
        .expect("hooks is an array");
    for hook in hooks {
        let hook = hook.as_object().expect("hook entry is an object");
        assert!(
            hook.get("name")
                .and_then(serde_json::Value::as_str)
                .is_some()
        );
        assert!(
            hook.get("active")
                .and_then(serde_json::Value::as_bool)
                .is_some()
        );
        assert!(
            hook.get("path")
                .and_then(serde_json::Value::as_str)
                .is_some()
        );
    }

    // Profile shape: name + checks + path.
    let profile = doc
        .get("profile")
        .and_then(|v| v.as_object())
        .expect("profile is an object");
    assert!(profile.get("name").and_then(|v| v.as_str()).is_some());
    let checks = profile
        .get("checks")
        .and_then(|v| v.as_array())
        .expect("profile.checks is an array");
    for c in checks {
        assert!(c.as_str().is_some(), "profile.checks entries are strings");
    }
    assert!(profile.get("path").and_then(|v| v.as_str()).is_some());

    // Recent runs (may be empty in a fresh repo). When present,
    // the documented numeric fields must be integers.
    let runs = doc
        .get("recent_runs")
        .and_then(|v| v.as_array())
        .expect("recent_runs is an array");
    for run in runs {
        let run = run.as_object().expect("recent_runs entry is an object");
        assert!(
            run.get("timestamp")
                .and_then(serde_json::Value::as_str)
                .is_some()
        );
        assert!(
            run.get("passed")
                .and_then(serde_json::Value::as_bool)
                .is_some()
        );
        assert!(
            run.get("score")
                .and_then(serde_json::Value::as_f64)
                .is_some()
        );
        assert!(
            run.get("checks_run")
                .and_then(serde_json::Value::as_u64)
                .is_some()
        );
        assert!(
            run.get("checks_passed")
                .and_then(serde_json::Value::as_u64)
                .is_some()
        );
        assert!(
            run.get("duration_ms")
                .and_then(serde_json::Value::as_u64)
                .is_some()
        );
    }
}

/// Pin the schema file itself — typo in `$id`, the `const` lock, or
/// the required-fields list would be invisible to the runtime
/// emission test above. Reading the schema in-process guards that
/// the contract surface and the wire emitter agree.
#[test]
fn schema_file_is_well_formed_and_pinned() {
    let schema = load_schema();
    assert_eq!(
        schema.get("$schema").and_then(|v| v.as_str()),
        Some("https://json-schema.org/draft/2020-12/schema"),
        "schemas must declare draft 2020-12: {schema}"
    );
    let const_value = schema
        .pointer("/properties/schema_version/const")
        .and_then(|v| v.as_str())
        .expect("schema_version must lock to a const");
    assert_eq!(const_value, "anvil.status.v1");
    let required = schema
        .get("required")
        .and_then(|v| v.as_array())
        .expect("top-level required is an array");
    let names: Vec<&str> = required
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect();
    for needed in [
        "schema_version",
        "activation",
        "hooks",
        "profile",
        "recent_runs",
    ] {
        assert!(
            names.contains(&needed),
            "schema must require top-level key {needed:?}; got {names:?}"
        );
    }
}
