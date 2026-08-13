//! LAUNCH-015 + LAUNCH-016 integration: `anvil status --verify`
//! surfaces the repo language profile honestly. Supported, partial,
//! and unsupported languages each render with their own coverage
//! tier; an all-unsupported repo (e.g. Python-only) maps the
//! protection state to `unsupported` rather than claiming generic
//! coverage.
//!
//! Every test bench overrides `HOME` to a per-test tempdir so the
//! MCP probe sees an empty home. Without this, the tests would
//! pick up the developer's real `~/.cursor/mcp.json` and report
//! state changes the test isn't trying to assert.

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
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .arg("--json")
        .arg("status")
        .arg("--verify")
        .current_dir(workdir)
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env_remove("XDG_CONFIG_HOME")
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
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .arg("status")
        .arg("--verify")
        .current_dir(workdir)
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env_remove("XDG_CONFIG_HOME")
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .output()
        .expect("failed to invoke anvil");
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[cfg(not(target_os = "windows"))]
#[test]
fn ts_only_repo_shows_supported_tier() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("src/a.ts"), "export const x = 1;\n");
    write(&dir.path().join("src/b.tsx"), "export const y = 2;\n");
    write(
        &dir.path().join(".anvil.yaml"),
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

#[cfg(not(target_os = "windows"))]
#[test]
fn python_only_repo_is_supported_not_unsupported() {
    // CIB-123 reconciliation: PYLAN shipped the python-reliability
    // antipattern catalogue, default `.py` scanning, and boundary
    // analysis (T3) — the same bar that lifted Rust. A Python-only repo
    // must now report the `supported` tier and must NOT collapse to the
    // `unsupported` protection state. Without MCP wiring the state is
    // `needs_action` (the user can still get coverage), like TS/Rust.
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("app.py"), "x = 1\n");
    write(&dir.path().join("lib/util.py"), "def f(): pass\n");
    write(
        &dir.path().join(".anvil.yaml"),
        "profile: default\nchecks: []\n",
    );

    let parsed = run_verify_json(dir.path());
    let langs = parsed["repo_languages"].as_array().unwrap();
    let py = langs
        .iter()
        .find(|e| e["name"] == "Python")
        .expect("Python entry");
    assert_eq!(py["coverage_tier"], "supported");
    assert_eq!(py["files_seen"], 2);
    assert_eq!(parsed["all_languages_unsupported"], false);
    assert_eq!(parsed["state"], "needs_action");
}

#[cfg(not(target_os = "windows"))]
#[test]
fn tail_t1_language_is_unsupported_but_recognized() {
    // CIB-123: a tail T1 language (Zig) is parsed by the kernel but ships
    // no language-specific anti-pattern catalogue, so it stays the
    // `unsupported` tier — yet it must be *recognised* in the registry
    // (named "Zig"), not silently dropped into `unclassified_files_seen`.
    // A Zig-only repo therefore still maps to the `unsupported` state.
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("main.zig"), "pub fn main() void {}\n");
    write(
        &dir.path().join(".anvil.yaml"),
        "profile: default\nchecks: []\n",
    );

    let parsed = run_verify_json(dir.path());
    let langs = parsed["repo_languages"].as_array().unwrap();
    let zig = langs
        .iter()
        .find(|e| e["name"] == "Zig")
        .expect("Zig entry — must be recognised, not unclassified");
    assert_eq!(zig["coverage_tier"], "unsupported");
    assert_eq!(zig["files_seen"], 1);
    assert_eq!(parsed["all_languages_unsupported"], true);
    assert_eq!(parsed["state"], "unsupported");
}

#[cfg(not(target_os = "windows"))]
#[test]
fn rust_only_repo_is_supported_not_unsupported() {
    // RSTLAN reconciliation: Rust ships the antipattern catalogue,
    // default `.rs` scanning, and architecture analysis, so a
    // Rust-only repo must report the `supported` tier and must NOT
    // collapse to the `unsupported` protection state. Without MCP
    // wiring the state is `needs_action` (the user can still get
    // coverage), exactly like the TS-only case.
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("src/main.rs"), "fn main() {}\n");
    write(&dir.path().join("src/lib.rs"), "pub fn f() {}\n");
    write(
        &dir.path().join(".anvil.yaml"),
        "profile: default\nchecks: []\n",
    );

    let parsed = run_verify_json(dir.path());
    let langs = parsed["repo_languages"].as_array().unwrap();
    let rust = langs
        .iter()
        .find(|e| e["name"] == "Rust")
        .expect("Rust entry");
    assert_eq!(rust["coverage_tier"], "supported");
    assert_eq!(rust["files_seen"], 2);
    assert_eq!(parsed["all_languages_unsupported"], false);
    assert_eq!(parsed["state"], "needs_action");
}

#[cfg(not(target_os = "windows"))]
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
        &dir.path().join(".anvil.yaml"),
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
    write(&dir.path().join("cmd/main.go"), "package main\n");
    write(
        &dir.path().join(".anvil.yaml"),
        "profile: default\nchecks: []\n",
    );

    let stdout = run_verify_human(dir.path());
    assert!(
        stdout.contains("languages:"),
        "missing languages block: {stdout}"
    );
    // CLAWP-058: bind each language to its tier on the SAME render row.
    // Four independent `contains` checks would pass even if the tiers
    // were swapped (TypeScript shown unsupported, Python supported); the
    // contract is the per-language association. The render shape is
    // `    {name} ({n} file): {tier} — {basis}` (one language per line).
    // NB "supported" is a substring of "unsupported", so the TypeScript
    // row must be asserted to contain "supported" AND not "unsupported".
    let ts_row = stdout
        .lines()
        .find(|l| l.contains("TypeScript"))
        .unwrap_or_else(|| panic!("TypeScript not surfaced: {stdout}"));
    assert!(
        ts_row.contains("supported") && !ts_row.contains("unsupported"),
        "TypeScript row must show the `supported` tier (not unsupported): {ts_row:?}\nfull:\n{stdout}"
    );
    // CIB-123: Python is now a supported tier (PYLAN), like TypeScript.
    let py_row = stdout
        .lines()
        .find(|l| l.contains("Python"))
        .unwrap_or_else(|| panic!("Python not surfaced: {stdout}"));
    assert!(
        py_row.contains("supported") && !py_row.contains("unsupported"),
        "Python row must show the `supported` tier (not unsupported): {py_row:?}\nfull:\n{stdout}"
    );
    // Go is a tail T1 language — parsed but `unsupported` tier — the
    // unsupported contrast that catches a tier swap.
    let go_row = stdout
        .lines()
        .find(|l| l.contains("Go ("))
        .unwrap_or_else(|| panic!("Go not surfaced: {stdout}"));
    assert!(
        go_row.contains("unsupported"),
        "Go row must show the `unsupported` tier: {go_row:?}\nfull:\n{stdout}"
    );
}

#[test]
fn unclassified_files_surface_in_json_output() {
    // Round-2 council: pin the JSON `unclassified_files_seen` field
    // end-to-end so a future render rename is caught.
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("src/a.ts"), "");
    write(&dir.path().join("Makefile"), "all:\n");
    write(&dir.path().join("README"), "");
    // legacy-fallback coverage (.anvilrc deliberately) — pins the CIB-178
    // exclusion of the legacy artefact from the unclassified count.
    write(
        &dir.path().join(".anvilrc"),
        "profile: default\nchecks: []\n",
    );

    let parsed = run_verify_json(dir.path());
    let unclassified = parsed["unclassified_files_seen"]
        .as_u64()
        .expect("unclassified_files_seen must be present and numeric");
    // CLAWP-059: pin the EXACT count. A `>= 2` lower bound let the
    // classified `src/a.ts` (or any future miscount) silently inflate
    // the tally without failing. The unclassified inputs here are
    // `Makefile` and `README`; the classified `src/a.ts` must NOT
    // contribute.
    //
    // CIB-178: `.anvilrc` is an anvil-owned artefact and is deliberately
    // excluded from the unclassified count so the tool does not inflate
    // its own "unclassified" noise across activation runs — so it must
    // NOT contribute either.
    assert_eq!(
        unclassified, 2,
        "expected exactly Makefile + README as unclassified (src/a.ts is classified, .anvilrc is anvil-owned per CIB-178), got {unclassified}: {parsed}"
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
    // CLAWP-060: exercise the documented `.git` exclusion. The Go file
    // exists ONLY under `.git`, so a `Go` entry appearing in the profile
    // can only mean `.git` was walked — isolating this case from the
    // node_modules/target ones below (whose languages also live in src/).
    write(
        &dir.path().join(".git/hooks/pre-commit.go"),
        "package main\n",
    );
    write(
        &dir.path().join(".anvil.yaml"),
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
    // CLAWP-060: the only Go file lives under `.git`, so its absence
    // proves the `.git` exclusion held.
    assert!(
        langs.iter().all(|e| e["name"] != "Go"),
        "Go from .git/ leaked into profile (.git not excluded from the language walk): {langs:?}"
    );
}
