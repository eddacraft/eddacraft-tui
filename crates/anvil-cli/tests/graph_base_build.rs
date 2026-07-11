//! End-to-end coverage for the hidden `anvil graph-base build` harness: it
//! must resolve a merge-base, build the base graph from the committed tree, and
//! print a single deterministic JSON summary line to stdout.
#![cfg(unix)]

use std::path::Path;
use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

fn git(root: &Path, args: &[&str]) -> std::process::Output {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("git available");
    assert!(out.status.success(), "git {args:?} failed: {out:?}");
    out
}

fn write_file(root: &Path, path: &str, content: &[u8]) {
    let full = root.join(path);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(full, content).unwrap();
}

/// Extract the JSON object line from mixed stdout (the harness prints exactly
/// one, but a session banner may precede it).
fn json_line(stdout: &str) -> &str {
    stdout
        .lines()
        .rev()
        .find(|l| l.trim_start().starts_with('{'))
        .unwrap_or_else(|| panic!("no JSON line in stdout:\n{stdout}"))
}

#[test]
fn graph_base_build_prints_deterministic_json_summary() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    git(root, &["init", "-q", "-b", "main"]);
    git(root, &["config", "user.email", "test@example.com"]);
    git(root, &["config", "user.name", "Test"]);
    git(root, &["config", "commit.gpgsign", "false"]);
    write_file(
        root,
        "src/a.ts",
        b"import { b } from './b';\nexport function a() { return b(); }\n",
    );
    write_file(root, "src/b.ts", b"export function b() { return 2; }\n");
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "fixture"]);
    let sha = String::from_utf8(git(root, &["rev-parse", "HEAD"]).stdout)
        .unwrap()
        .trim()
        .to_string();

    // Each run pins ANVIL_HOME to a caller-chosen dir so the write-once base
    // store is hermetic to this test (never the developer's real store) and the
    // write-once transition can be asserted deliberately per store.
    let run = |home: &Path| {
        let out = Command::new(ANVIL_BIN)
            .args(["graph-base", "build", "--merge-base", &sha])
            .arg("--repo")
            .arg(root)
            .env("ANVIL_HOME", home)
            // Local dev bypass so the licence/auth wall never intercepts the
            // hidden harness command in CI.
            .env("ANVIL_DEV", "1")
            .output()
            .expect("anvil binary runs");
        assert!(
            out.status.success(),
            "graph-base build exited non-zero: {:?}\nstderr: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr),
        );
        String::from_utf8(out.stdout).expect("utf8 stdout")
    };

    let store_a = tempfile::tempdir().unwrap();
    let store_b = tempfile::tempdir().unwrap();

    // First run against a fresh store: builds and persists, summary included.
    let first = run(store_a.path());
    let value: serde_json::Value = serde_json::from_str(json_line(&first)).expect("valid JSON");
    assert_eq!(value["merge_base"], serde_json::Value::String(sha.clone()));
    assert_eq!(value["outcome"], "written", "fresh store persists: {value}");
    assert_eq!(value["persisted"], true);
    assert_eq!(value["file_count"], 2);
    assert_eq!(value["symbol_count"], 2, "both exported functions: {value}");
    assert_eq!(
        value["edge_count"], 2,
        "the a->b import edge and the a()->b() call edge: {value}"
    );

    // Second run against the SAME store: the write-once no-op — no rebuild, so
    // no summary counts; still persisted.
    let second = run(store_a.path());
    let value2: serde_json::Value = serde_json::from_str(json_line(&second)).expect("valid JSON");
    assert_eq!(
        value2["outcome"], "already-present",
        "same store is a write-once no-op: {value2}"
    );
    assert_eq!(value2["persisted"], true);
    assert!(
        value2.get("file_count").is_none(),
        "the no-op path never rebuilt, so it carries no counts: {value2}"
    );

    // Determinism: a fresh store must reproduce run 1 byte-for-byte — the
    // summary is a pure function of the committed tree at the sha.
    let fresh = run(store_b.path());
    assert_eq!(
        json_line(&first),
        json_line(&fresh),
        "the same sha against a fresh store must print byte-identical JSON"
    );
}
