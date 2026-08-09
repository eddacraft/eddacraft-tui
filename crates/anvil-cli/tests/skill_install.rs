use std::fs;
use std::process::Command;

use serde_json::Value;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

fn run(root: &std::path::Path, extra: &[&str]) -> std::process::Output {
    // macOS tempdirs live under `/var` → `/private/var`. The skill installer
    // refuses destination paths that walk through a symlink component, so the
    // workspace must be the resolved path — not the raw tempdir string.
    let root = fs::canonicalize(root).expect("canonicalize workspace for skill install");
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
fn help_explains_scripted_multi_client_enumeration() {
    let output = Command::new(ANVIL_BIN)
        .args(["--no-tui", "skill", "install", "--help"])
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .output()
        .expect("invoke anvil skill install help");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stdout = stdout.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        stdout
            .contains("Required for non-interactive installation; repeat to select more than one."),
        "stdout: {stdout}"
    );
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
    // Phrase must stay on one physical line (include_str install contract).
    assert!(body.contains("pre-write enforcement gate"));
    assert!(body.contains("anvil_apply_patch"));
    assert!(body.contains("anvil_find_dependents"));
    assert!(!body.contains("anvil_get_dependencies"));

    let reference = fs::read_to_string(skill.join("references/tool-reference.md")).unwrap();
    assert!(reference.contains("Call it before applying a write."));
    assert!(!reference.contains("anvil_explain"));

    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(skill.join(".anvil-managed.json")).unwrap())
            .unwrap();
    assert_eq!(
        manifest["sourceCommit"],
        "ef5b34c5f424c9de4292406405e4bedfb603a65a"
    );
    assert_eq!(manifest["bundleDigest"].as_str().unwrap().len(), 64);
    assert_eq!(manifest["files"]["SKILL.md"].as_str().unwrap().len(), 64);
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
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unmanaged"));
    assert!(
        stderr.contains("move it outside the skills directory tree"),
        "stderr: {stderr}"
    );
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
fn refuses_extra_user_owned_files_in_a_managed_directory() {
    let root = tempfile::tempdir().unwrap();
    assert!(run(root.path(), &["--client", "codex"]).status.success());
    let skill = root.path().join(".agents/skills/anvil-developer-functions");
    let extra = skill.join("notes/private.md");
    fs::create_dir_all(extra.parent().unwrap()).unwrap();
    fs::write(&extra, "user-owned notes").unwrap();

    let output = run(root.path(), &["--client", "codex"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unmanaged entry"));
    assert!(
        stderr.contains("move it outside the skills directory tree"),
        "stderr: {stderr}"
    );
    assert_eq!(fs::read_to_string(extra).unwrap(), "user-owned notes");
    assert!(skill.join("SKILL.md").exists());
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

/// CIB-279 (guarding CIB-237). `skill install` printed `report.path.display()`,
/// so a workspace that arrives NT-extended — which is exactly what
/// `fs::canonicalize` hands back on Windows — echoed `\\?\C:\...` at the user.
///
/// The strip is pure string logic, so feeding a Windows-shaped `--workspace`
/// exercises it on every host instead of only on the dispatch-gated Windows
/// matrix. `--dry-run` keeps the run read-only, so no such directory is
/// created on a Windows host either.
#[test]
fn install_output_never_echoes_a_windows_verbatim_prefix() {
    let output = Command::new(ANVIL_BIN)
        .args([
            "--no-tui",
            "skill",
            "install",
            "--dry-run",
            "--client",
            "claude-code",
            "--scope",
            "project",
            "--workspace",
            r"\\?\C:\project",
        ])
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .output()
        .expect("invoke anvil skill install");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains(r"\\?\"),
        "verbatim prefix echoed back at the user: {stdout}"
    );
    assert!(
        stdout.contains(r"C:\project"),
        "destination should still name the requested workspace: {stdout}"
    );
}

/// CIB-282. The `--json` branch sits immediately above the human one in
/// `run_install` and used to serialise the same value as a raw `PathBuf`, so
/// serde emitted the underlying string and the `\\?\` prefix survived — one
/// surface, two path styles, which is what CIB-237 was filed to end.
///
/// The assertions are on the JSON **string**, never on a deserialised
/// `PathBuf`: `Path` equality is component-wise and Windows accepts `/`, so a
/// `PathBuf` comparison would pass on every platform whether or not the
/// prefix was stripped.
#[test]
fn install_json_never_emits_a_windows_verbatim_prefix() {
    let output = Command::new(ANVIL_BIN)
        .args([
            "--no-tui",
            "--json",
            "skill",
            "install",
            "--dry-run",
            "--client",
            "claude-code",
            "--scope",
            "project",
            "--workspace",
            r"\\?\C:\project",
        ])
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .output()
        .expect("invoke anvil skill install");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    // The raw payload: in JSON each backslash is escaped, so the leaked
    // prefix appears as `\\\\?\\`. Checking the wire form catches it even if
    // the envelope shape changes.
    assert!(
        !stdout.contains(r"\\\\?\\"),
        "verbatim prefix survived into the JSON payload: {stdout}"
    );

    let value: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|err| panic!("skill install --json did not emit JSON ({err}): {stdout}"));
    let path = value["targets"][0]["path"]
        .as_str()
        .unwrap_or_else(|| panic!("targets[0].path is not a string: {stdout}"));

    assert!(
        !path.contains(r"\\?\"),
        "targets[0].path carries the verbatim prefix: {path}"
    );
    assert!(
        path.contains(r"C:\project"),
        "targets[0].path should still name the requested workspace: {path}"
    );
}

/// CIB-282. The verbatim UNC form takes a different branch of
/// `strip_verbatim_prefix` — it has to regain the leading `\\` that the
/// prefix subsumes, or `\\?\UNC\server\share` renders as `server\share`,
/// which reads as a relative path. The drive-letter case above cannot catch
/// that, so JSON gets its own guard for it.
#[test]
fn install_json_renders_a_verbatim_unc_path_as_a_usable_unc_path() {
    let output = Command::new(ANVIL_BIN)
        .args([
            "--no-tui",
            "--json",
            "skill",
            "install",
            "--dry-run",
            "--client",
            "claude-code",
            "--scope",
            "project",
            "--workspace",
            r"\\?\UNC\server\share",
        ])
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .output()
        .expect("invoke anvil skill install");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|err| panic!("skill install --json did not emit JSON ({err}): {stdout}"));
    let path = value["targets"][0]["path"]
        .as_str()
        .unwrap_or_else(|| panic!("targets[0].path is not a string: {stdout}"));

    assert!(
        !path.contains(r"\\?\"),
        "targets[0].path carries the verbatim prefix: {path}"
    );
    assert!(
        path.starts_with(r"\\server\share"),
        "a UNC path must keep its leading double backslash or it reads as relative: {path}"
    );
}

/// CIB-285. The success paths strip the NT-extended prefix; the error paths
/// interpolated `Path::display()` and did not, so a failed install disagreed
/// with the run that preceded it — on the one surface a reader consults once
/// something has already gone wrong.
///
/// Forces the unmanaged-directory refusal, the cheapest error to provoke
/// deterministically, and asserts on the stderr **string**. A `PathBuf`
/// assertion could not catch this: `Path` equality is component-wise and
/// Windows accepts `/`, so it passes whether or not the prefix was stripped.
///
/// Unix-only. On Windows `\\?\C:\project` is a real verbatim path rather than
/// the literal relative directory name this fixture builds, so the test would
/// reach outside its tempdir. The rendering is pure string logic, so Linux
/// carries the signal — the same reasoning CIB-279 records.
#[cfg(unix)]
#[test]
fn install_errors_never_carry_a_windows_verbatim_prefix() {
    let root = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(root.path()).expect("canonicalize workspace");

    // On Unix a leading backslash is an ordinary filename character, so this
    // is a relative directory name — which is what makes the fixture
    // buildable on this host at all.
    let workspace = r"\\?\C:\project";
    // `join` with a backslash-leading string is only ambiguous on Windows,
    // where `\` is a separator and would discard `root`. This test is
    // `cfg(unix)` precisely because the string has to behave as a filename,
    // which is what makes the fixture constructible at all.
    #[allow(clippy::join_absolute_paths)]
    let unmanaged = root
        .join(workspace)
        .join(".claude/skills/anvil-developer-functions");
    fs::create_dir_all(&unmanaged).unwrap();
    fs::write(unmanaged.join("SKILL.md"), "user-owned").unwrap();

    let output = Command::new(ANVIL_BIN)
        .args([
            "--no-tui",
            "skill",
            "install",
            "--client",
            "claude-code",
            "--scope",
            "project",
            "--workspace",
            workspace,
        ])
        .current_dir(&root)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .output()
        .expect("invoke anvil skill install");

    assert!(
        !output.status.success(),
        "expected the unmanaged directory to be refused; stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unmanaged"),
        "expected the unmanaged refusal, got: {stderr}"
    );
    assert!(
        !stderr.contains(r"\\?\"),
        "verbatim prefix leaked into error text: {stderr}"
    );
    assert!(
        stderr.contains(r"C:\project"),
        "the error should still name the destination: {stderr}"
    );

    // The user's file survives — the refusal is the point, not a casualty of
    // the rendering change.
    assert_eq!(
        fs::read_to_string(unmanaged.join("SKILL.md")).unwrap(),
        "user-owned"
    );
}

/// CIB-285 asked for one shared helper precisely so a later site could not
/// reintroduce the split. Only one of the 24 error sites has behavioural
/// coverage (the unmanaged refusal above), so the other 23 rest on this.
///
/// Reads the source at compile time, so it cannot drift from the file it
/// guards nor depend on the working directory at run time.
///
/// **What this does and does not cover.** It is a substring check over the
/// two spellings that actually reach `Display` for a path here: `.display()`
/// and `to_string_lossy()`. It does not catch every conceivable route —
/// `{:?}`, `.to_str().unwrap()`, or a `Cow` round-trip would each walk past
/// it. Verification demonstrated exactly that: swapping one site to
/// `to_string_lossy()` left the whole suite green before this second
/// assertion existed. Treat it as a tripwire for the likely mistakes, not a
/// proof of the invariant.
#[test]
fn skill_command_renders_every_path_through_the_shared_helper() {
    let source = include_str!("../src/commands/skill.rs");

    assert!(
        !source.contains(".display()"),
        "`skill.rs` calls `Path::display()`, which emits the Windows verbatim \
         prefix into user-facing text (CIB-285). Use \
         `crate::display_path::shown(path)` instead — it renders the way the \
         success paths do."
    );

    // `TargetReport::new` legitimately owns exactly one `to_string_lossy`,
    // and renders it through `strip_verbatim_prefix` on the next line
    // (CIB-282). A second occurrence is a path being stringified somewhere
    // that has not been thought about, which is how this defect class keeps
    // coming back.
    let lossy = source.matches("to_string_lossy()").count();
    assert_eq!(
        lossy, 1,
        "expected exactly one `to_string_lossy()` in skill.rs (the rendered \
         `TargetReport::new`), found {lossy}. A raw path reaching text is the \
         CIB-285 defect wearing a different spelling — route it through \
         `crate::display_path::shown(path)`."
    );

    assert!(
        source.contains("display_path::shown("),
        "expected skill.rs to render paths through the shared helper"
    );
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
