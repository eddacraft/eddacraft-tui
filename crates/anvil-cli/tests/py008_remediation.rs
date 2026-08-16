//! #3923: PY-008 human/TUI output must surface the rule-specific
//! eval/exec/compile nudge already present in JSON, not the generic
//! python-reliability family suggestion.

use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

const SOURCE: &str = "\
eval(user_input)
exec(payload)
compile(src, \"<s>\", \"exec\")
";

const FAMILY_BOILERPLATE: &[&str] = &[
    "# type: ignore",
    "import *",
    "except ValueError",
    "logging",
    "instead of `Any`",
];

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
    fs::write(root.join("src/app.py"), SOURCE).expect("write fixture");
    dir
}

fn run_check(root: &Path, extra: &[&str]) -> std::process::Output {
    Command::new(ANVIL_BIN)
        .args(["--no-tui"])
        .args(extra)
        .arg("check")
        .arg("src/app.py")
        .current_dir(root)
        .env("ANVIL_HOME", root)
        .env("HOME", root)
        .env("XDG_CONFIG_HOME", root.join(".config"))
        .env("XDG_CACHE_HOME", root.join(".cache"))
        .env("XDG_DATA_HOME", root.join(".local/share"))
        .env("USERPROFILE", root)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .env_remove("ANVIL_TOUCH_PROJECT_STATE")
        .env_remove("TRACEPARENT")
        .output()
        .expect("invoke anvil check")
}

fn assert_rule_specific(text: &str, surface: &str) {
    assert!(
        text.contains("eval") && text.contains("exec") && text.contains("compile"),
        "{surface} must name eval/exec/compile remediation\n{text}"
    );
    assert!(
        text.contains("structured dispatch")
            || text.contains("ast.literal_eval")
            || text.contains("RestrictedPython"),
        "{surface} must show the rule-specific nudge, not family boilerplate\n{text}"
    );
    for boilerplate in FAMILY_BOILERPLATE {
        assert!(
            !text.contains(boilerplate),
            "{surface} must not lead with family boilerplate {boilerplate:?}\n{text}"
        );
    }
}

#[test]
fn human_output_surfaces_py008_rule_nudge_for_each_shape() {
    let dir = workspace();
    let output = run_check(dir.path(), &[]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "PY-008 is an error; stdout={stdout}\nstderr={stderr}"
    );
    assert_rule_specific(&stdout, "human/TUI check");
    for line in ["src/app.py:1", "src/app.py:2", "src/app.py:3"] {
        assert!(
            stdout.contains(line),
            "human output must show each finding ({line})\n{stdout}"
        );
    }
}

#[test]
fn json_output_keeps_py008_nudge_and_all_findings() {
    let dir = workspace();
    let output = run_check(dir.path(), &["--json"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let json: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|err| panic!("JSON stdout ({err})\nstdout={stdout}\nstderr={stderr}"));
    let warnings = json["warnings"]
        .as_array()
        .expect("warnings array")
        .iter()
        .filter(|w| w["id"] == "PY-008")
        .collect::<Vec<_>>();
    assert!(
        warnings.len() >= 3,
        "expected eval/exec/compile findings, got {warnings:?}"
    );
    for warning in warnings {
        let nudge = warning["nudge"].as_str().unwrap_or_default();
        assert_rule_specific(nudge, "JSON nudge");
        assert!(
            warning.get("suggestion").is_some(),
            "JSON schema must keep suggestion: {warning}"
        );
    }
}
