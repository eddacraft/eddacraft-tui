//! CIB-424: `anvil gate --only-checks secret-detection` must fail a planted
//! credential in each newly admitted source type (`.tsx` `.jsx` `.py` `.go`
//! `.sh`) and still name the scan domain. Out-of-domain `.md` stays missed.

use std::fs;
use std::path::Path;
use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

fn planted_key() -> String {
    let mut key = String::from("sk-ant-");
    key.push_str("api03-");
    key.push_str("abcdef");
    key.push_str("ghijkl");
    key.push_str("mnopqr");
    key.push_str("stuvwx");
    key.push_str("yz0123");
    key.push_str("45");
    key
}

fn workspace() -> tempfile::TempDir {
    let dir = tempfile::Builder::new()
        .prefix("anvil-cib-424-gate-domain-")
        .tempdir()
        .expect("create temp workdir");
    let root = dir.path();
    let status = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(root)
        .status()
        .expect("run git init");
    assert!(status.success(), "git init failed");
    fs::create_dir_all(root.join("src")).expect("create src");
    dir
}

fn anvil(workdir: &Path) -> Command {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.current_dir(workdir)
        .env("ANVIL_HOME", workdir)
        .env("HOME", workdir)
        .env("USERPROFILE", workdir)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_DISABLE_UPDATE_HINT", "1")
        .env_remove("ANVIL_TOUCH_PROJECT_STATE")
        .env_remove("TRACEPARENT");
    cmd
}

fn combined(out: &std::process::Output) -> String {
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    if !out.stderr.is_empty() {
        text.push('\n');
        text.push_str(&String::from_utf8_lossy(&out.stderr));
    }
    text
}

#[test]
fn gate_secret_detection_fails_planted_keys_in_expanded_source_types() {
    let dir = workspace();
    let root = dir.path();
    let key = planted_key();
    fs::write(
        root.join("src/leak.tsx"),
        format!("export const k = '{key}';\n"),
    )
    .expect("write tsx");
    fs::write(
        root.join("src/leak.jsx"),
        format!("export const k = '{key}';\n"),
    )
    .expect("write jsx");
    fs::write(root.join("src/leak.py"), format!("k = '{key}'\n")).expect("write py");
    fs::write(
        root.join("src/leak.go"),
        format!("package main\nvar k = \"{key}\"\n"),
    )
    .expect("write go");
    fs::write(root.join("src/leak.sh"), format!("KEY={key}\n")).expect("write sh");
    fs::write(root.join("src/leak.md"), format!("key = '{key}'\n")).expect("write md");

    let out = anvil(root)
        .args(["--no-tui", "gate", "--only-checks", "secret-detection"])
        .output()
        .expect("invoke anvil gate");
    let text = combined(&out);

    assert!(
        !out.status.success(),
        "planted credentials must fail gate secret-detection; exit={:?}\n{text}",
        out.status.code()
    );
    for name in ["leak.tsx", "leak.jsx", "leak.py", "leak.go", "leak.sh"] {
        assert!(
            text.contains(name),
            "gate must name {name}; output:\n{text}"
        );
    }
    assert!(
        !text.contains("leak.md"),
        "out-of-domain .md must stay unscanned; output:\n{text}"
    );
    assert!(
        text.contains("Domain:") && text.contains(".tsx") && text.contains(".md"),
        "gate must name the expanded domain; output:\n{text}"
    );
}
