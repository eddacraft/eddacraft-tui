use std::fs;
use std::process::Command;

use serde_json::Value;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

fn run(root: &std::path::Path, extra: &[&str]) -> std::process::Output {
    let mut command = Command::new(ANVIL_BIN);
    command
        .arg("--no-tui")
        .arg("skill")
        .arg("install")
        .args(extra)
        .arg("--workspace")
        .arg(root)
        .env("ANVIL_DEV", "1");
    command.output().expect("invoke anvil skill install")
}

#[test]
fn installs_embedded_bundle_for_codex_at_global_default() {
    let root = tempfile::tempdir().unwrap();
    let output = run(root.path(), &["--client", "codex"]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let skill = root.path().join(".agents/skills/anvil-developer-functions");
    let body = fs::read_to_string(skill.join("SKILL.md")).unwrap();
    assert!(body.contains("anvil_validate_write"));
    assert!(body.contains("safety and structural-context layer"));
    assert!(body.contains("anvil_find_dependents"));
    assert!(!body.contains("anvil_get_dependencies"));

    let reference = fs::read_to_string(skill.join("references/tool-reference.md")).unwrap();
    assert!(reference.contains("it\ndoes not write the file"));
    assert!(!reference.contains("anvil_explain"));

    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(skill.join(".anvil-managed.json")).unwrap())
            .unwrap();
    assert_eq!(
        manifest["sourceCommit"],
        "ef5b34c5f424c9de4292406405e4bedfb603a65a"
    );
    assert_eq!(manifest["bundleDigest"].as_str().unwrap().len(), 64);
    assert!(manifest["files"]["SKILL.md"].as_str().unwrap().len() == 64);
}

#[test]
fn project_scope_uses_client_project_root() {
    let root = tempfile::tempdir().unwrap();
    let output = run(
        root.path(),
        &["--client", "claude-code", "--scope", "project"],
    );
    assert!(output.status.success());
    assert!(
        root.path()
            .join(".claude/skills/anvil-developer-functions/SKILL.md")
            .exists()
    );
}

#[test]
fn repeat_install_is_idempotent_and_verify_succeeds() {
    let root = tempfile::tempdir().unwrap();
    assert!(run(root.path(), &["--client", "codex"]).status.success());
    let path = root
        .path()
        .join(".agents/skills/anvil-developer-functions/SKILL.md");
    let first = fs::read_to_string(&path).unwrap();
    assert!(run(root.path(), &["--client", "codex"]).status.success());
    assert_eq!(first, fs::read_to_string(&path).unwrap());
    assert!(
        run(root.path(), &["--client", "codex", "--verify"])
            .status
            .success()
    );
}

#[test]
fn refuses_unmanaged_or_modified_skill_files() {
    let unmanaged_root = tempfile::tempdir().unwrap();
    let unmanaged = unmanaged_root
        .path()
        .join(".agents/skills/anvil-developer-functions");
    fs::create_dir_all(&unmanaged).unwrap();
    fs::write(unmanaged.join("SKILL.md"), "user-owned").unwrap();
    let output = run(unmanaged_root.path(), &["--client", "codex"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unmanaged"));
    assert_eq!(
        fs::read_to_string(unmanaged.join("SKILL.md")).unwrap(),
        "user-owned"
    );

    let modified_root = tempfile::tempdir().unwrap();
    assert!(
        run(modified_root.path(), &["--client", "codex"])
            .status
            .success()
    );
    let modified = modified_root
        .path()
        .join(".agents/skills/anvil-developer-functions/SKILL.md");
    fs::write(&modified, "user modification").unwrap();
    let output = run(modified_root.path(), &["--client", "codex"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("modified"));
    assert_eq!(fs::read_to_string(modified).unwrap(), "user modification");
}

#[test]
fn refuses_a_managed_manifest_with_an_unknown_identity() {
    let root = tempfile::tempdir().unwrap();
    assert!(run(root.path(), &["--client", "codex"]).status.success());
    let manifest_path = root
        .path()
        .join(".agents/skills/anvil-developer-functions/.anvil-managed.json");
    let mut manifest: Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
    manifest["skill"] = Value::String("another-skill".to_string());
    fs::write(
        &manifest_path,
        format!("{}\n", serde_json::to_string_pretty(&manifest).unwrap()),
    )
    .unwrap();

    let output = run(root.path(), &["--client", "codex"]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("skill identity"));
}

#[test]
fn dry_run_resolves_without_writing() {
    let root = tempfile::tempdir().unwrap();
    let output = run(root.path(), &["--client", "codex", "--dry-run"]);
    assert!(output.status.success());
    assert!(!root.path().join(".agents/skills").exists());
}

#[cfg(unix)]
#[test]
fn refuses_an_already_installed_bundle_reached_through_a_symlink() {
    use std::os::unix::fs::symlink;

    let outside = tempfile::tempdir().unwrap();
    assert!(run(outside.path(), &["--client", "codex"]).status.success());

    let root = tempfile::tempdir().unwrap();
    symlink(outside.path().join(".agents"), root.path().join(".agents")).unwrap();

    let output = run(root.path(), &["--client", "codex"]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("symlinked path"));
}

#[cfg(unix)]
#[test]
fn refuses_a_nested_managed_directory_symlink() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    assert!(run(root.path(), &["--client", "codex"]).status.success());
    let skill = root.path().join(".agents/skills/anvil-developer-functions");
    let reference = fs::read_to_string(skill.join("references/tool-reference.md")).unwrap();

    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("tool-reference.md"), reference).unwrap();
    fs::remove_dir_all(skill.join("references")).unwrap();
    symlink(outside.path(), skill.join("references")).unwrap();

    let output = run(root.path(), &["--client", "codex"]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("symlinked path"));
}

#[cfg(unix)]
#[test]
fn refuses_a_symlinked_managed_manifest_before_reading_it() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    assert!(run(root.path(), &["--client", "codex"]).status.success());
    let skill = root.path().join(".agents/skills/anvil-developer-functions");
    let manifest_path = skill.join(".anvil-managed.json");
    let outside = tempfile::NamedTempFile::new().unwrap();
    fs::write(outside.path(), fs::read(&manifest_path).unwrap()).unwrap();
    fs::remove_file(&manifest_path).unwrap();
    symlink(outside.path(), &manifest_path).unwrap();

    let output = run(root.path(), &["--client", "codex", "--verify"]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("symlinked path"));
}
