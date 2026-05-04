//! LAUNCH-015 + LAUNCH-016 integration: `anvil status --verify`
//! surfaces the repo language profile honestly. Supported, partial,
//! and unsupported languages each render with their own coverage
//! tier; an all-unsupported repo (e.g. Python-only) maps the
//! protection state to `unsupported` rather than claiming generic
//! coverage.

use std::fs;
use std::path::Path;
use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

fn run_verify_json(workdir: &Path) -> serde_json::Value {
    let out = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .arg("--json")
        .arg("status")
        .arg("--verify")
        .current_dir(workdir)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .output()
        .expect("failed to invoke anvil");
    assert!(
        out.status.success(),
        "anvil status --verify failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "JSON parse failed: {e}\nstdout: {}",
            String::from_utf8_lossy(&out.stdout)
        )
    })
}

fn run_verify_human(workdir: &Path) -> String {
    let out = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .arg("status")
        .arg("--verify")
        .current_dir(workdir)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .output()
        .expect("failed to invoke anvil");
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn ts_only_repo_shows_supported_tier() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("src/a.ts"), "export const x = 1;\n");
    write(&dir.path().join("src/b.tsx"), "export const y = 2;\n");
    write(
        &dir.path().join(".anvilrc"),
        "profile: default\nchecks: []\n",
    );

    let parsed = run_verify_json(dir.path());
    let langs = parsed["repo_languages"].as_array().unwrap();
    assert!(!langs.is_empty(), "TS files should appear in profile");
    let ts = langs
        .iter()
        .find(|e| e["name"] == "TypeScript")
        .expect("TypeScript entry");
    assert_eq!(ts["coverage_tier"], "supported");
    assert_eq!(ts["files_seen"], 2);
    assert_eq!(parsed["all_languages_unsupported"], false);
    // State is `needs_action` because no MCP wiring; not `unsupported`.
    assert_eq!(parsed["state"], "needs_action");
}

#[test]
fn python_only_repo_state_is_unsupported() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("app.py"), "x = 1\n");
    write(&dir.path().join("lib/util.py"), "def f(): pass\n");
    write(
        &dir.path().join(".anvilrc"),
        "profile: default\nchecks: []\n",
    );

    let parsed = run_verify_json(dir.path());
    let langs = parsed["repo_languages"].as_array().unwrap();
    let py = langs
        .iter()
        .find(|e| e["name"] == "Python")
        .expect("Python entry");
    assert_eq!(py["coverage_tier"], "unsupported");
    assert_eq!(py["files_seen"], 2);
    assert_eq!(parsed["all_languages_unsupported"], true);
    // LAUNCH-008 + LAUNCH-015 + LAUNCH-016: a Python-only repo
    // without MCP gets the literal `unsupported` state, not
    // `needs_action`. Telling them to run `anvil start` would not
    // produce coverage.
    assert_eq!(parsed["state"], "unsupported");
}

#[test]
fn mixed_repo_does_not_collapse_to_unsupported() {
    // Even with two unsupported files, the presence of supported
    // languages keeps the state at `needs_action` (the user can
    // still get coverage on the TS subset).
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("src/a.ts"), "export const x = 1;\n");
    write(&dir.path().join("scripts/util.py"), "x = 1\n");
    write(&dir.path().join("main.rs"), "fn main() {}\n");
    write(
        &dir.path().join(".anvilrc"),
        "profile: default\nchecks: []\n",
    );

    let parsed = run_verify_json(dir.path());
    assert_eq!(parsed["all_languages_unsupported"], false);
    assert_eq!(parsed["state"], "needs_action");
    let names: Vec<&str> = parsed["repo_languages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"TypeScript"));
    assert!(names.contains(&"Python"));
    assert!(names.contains(&"Rust"));
}

#[test]
fn human_render_shows_per_language_breakdown() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("src/a.ts"), "export const x = 1;\n");
    write(&dir.path().join("scripts/util.py"), "x = 1\n");
    write(
        &dir.path().join(".anvilrc"),
        "profile: default\nchecks: []\n",
    );

    let stdout = run_verify_human(dir.path());
    assert!(stdout.contains("languages:"), "missing languages block: {stdout}");
    assert!(
        stdout.contains("TypeScript"),
        "TypeScript not surfaced: {stdout}"
    );
    assert!(
        stdout.contains("supported"),
        "supported tier not labelled: {stdout}"
    );
    assert!(
        stdout.contains("Python"),
        "Python not surfaced: {stdout}"
    );
    assert!(
        stdout.contains("unsupported"),
        "unsupported tier not labelled: {stdout}"
    );
}

#[test]
fn vendored_dirs_are_excluded_from_language_count() {
    // Files in node_modules / target / .git must not bias the
    // profile — the user did not write them. PR 5 mirrors the
    // `ScanFilter` denylist for this walk.
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("src/a.ts"), "");
    write(&dir.path().join("node_modules/dep/index.ts"), "");
    write(&dir.path().join("node_modules/dep/setup.py"), "");
    write(&dir.path().join("target/debug/build/foo.rs"), "");
    write(
        &dir.path().join(".anvilrc"),
        "profile: default\nchecks: []\n",
    );

    let parsed = run_verify_json(dir.path());
    let langs = parsed["repo_languages"].as_array().unwrap();
    let ts = langs
        .iter()
        .find(|e| e["name"] == "TypeScript")
        .expect("TypeScript should appear (single src file)");
    assert_eq!(ts["files_seen"], 1, "node_modules TS should not count");
    // Python and Rust files only existed in vendored dirs — must
    // not appear at all.
    assert!(
        langs.iter().all(|e| e["name"] != "Python"),
        "Python from node_modules leaked into profile: {langs:?}"
    );
    assert!(
        langs.iter().all(|e| e["name"] != "Rust"),
        "Rust from target/ leaked into profile: {langs:?}"
    );
}
