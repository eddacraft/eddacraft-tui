//! CIB-279: pin that `check --all` and `check <file>` both route their
//! locations through the shared renderer (CIB-237).
//!
//! The unit tests in `commands::check` prove the renderer itself handles
//! Windows-shaped input. They cannot prove either invocation still *calls* it —
//! and an uncovered surface next to a green suite is precisely the gap CIB-279
//! was filed for. These run on the host's own separators, so they are a wiring
//! pin rather than a CIB-237 regression test: they fail if a surface starts
//! emitting absolute paths, not if the Windows handling regresses.

use std::fs;
use std::process::Command;

use serde_json::Value;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

/// A git repository with one analysable file, so `check` resolves a workspace
/// root the way it does in the field (`git rev-parse --show-toplevel`).
///
/// No commit is made, and `--show-toplevel` answers from `.git` alone, so the
/// repository needs no configured identity.
fn workspace() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create workspace");
    let root = dir.path();
    let status = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(root)
        .status()
        .expect("run git init");
    assert!(status.success(), "git init failed");
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(root.join("src/app.ts"), "export const value = 1;\n").expect("write source");
    dir
}

fn check(root: &std::path::Path, extra: &[&str]) -> Value {
    let mut command = Command::new(ANVIL_BIN);
    command
        .args(["--no-tui", "--json", "check"])
        .args(extra)
        .current_dir(root)
        // Keep the run out of the developer's real `~/.anvil`: `check` writes
        // usage-observation state, and a test has no business touching it.
        .env("ANVIL_HOME", root)
        .env("HOME", root)
        .env("USERPROFILE", root)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1");
    command.env_remove("ANVIL_TOUCH_PROJECT_STATE");
    command.env_remove("TRACEPARENT");
    let output = command.output().expect("invoke anvil check");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // A blocking finding exits non-zero while still emitting a well-formed
    // `files` array, so parsing alone would let a failed run look like a pass.
    // The fixture is clean, so anything but success means the setup drifted.
    assert!(
        output.status.success(),
        "check {extra:?} exited {}\nstdout: {stdout}\nstderr: {stderr}",
        output.status
    );
    serde_json::from_str(&stdout).unwrap_or_else(|err| {
        panic!("check {extra:?} did not emit JSON ({err})\nstdout: {stdout}\nstderr: {stderr}")
    })
}

fn reported_files(value: &Value) -> Vec<String> {
    value["files"]
        .as_array()
        .expect("files array")
        .iter()
        .map(|f| f.as_str().expect("file string").to_string())
        .collect()
}

/// Both selection modes must report repo-relative locations. Asserted on the
/// strings, because a `Path` comparison is component-wise and would accept a
/// separator style the user never sees as equal.
#[test]
fn all_and_explicit_file_selection_report_repo_relative_paths() {
    let workspace = workspace();
    let root = workspace.path();

    let all = reported_files(&check(root, &["--all"]));
    assert!(
        all.contains(&"src/app.ts".to_string()),
        "check --all should report the file repo-relative: {all:?}"
    );

    // The explicit form is given an absolute path — the shape a tool or an
    // editor integration passes — and must still render it repo-relative.
    let absolute = root.join("src/app.ts");
    let explicit = reported_files(&check(
        root,
        &[absolute.to_str().expect("utf-8 workspace path")],
    ));
    assert_eq!(
        explicit,
        vec!["src/app.ts".to_string()],
        "check <file> should report the file repo-relative"
    );
}
