//! MLP2-047: end-to-end subprocess smoke tests for `anvil hook pre-push`.
//!
//! Helper coverage already exists across `anvil-hook`, `anvil-l4`, and
//! `anvil-cli::commands::hook` (40+ tests). This file proves the
//! production binary spawns, reads `anvil/project-id` + (optional)
//! `anvil/policy.*`, parses git's pre-push stdin, walks the ADR-038
//! stages, emits the documented stderr lines, and exits with the
//! documented status — without any in-process shortcut into
//! `run_pre_push_with_engine`.
//!
//! Linux-only by design: the pre-push surface compiles on every
//! `cfg(unix)` and on Windows, but `anvil/project-id` permission
//! semantics differ on macOS / Windows. Smoke variants for those
//! targets are tracked by MLP2-027 / MLP2-028.

#![cfg(target_os = "linux")]

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use tempfile::TempDir;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");
const FIXTURE_PROJECT_UUID: &str = "01997e4a-1b2c-7345-8901-abcdef123456";

/// Build a tempdir-rooted fixture with the minimum on-disk state the
/// pre-push hook needs to clear its `read_project_id` gate. Callers
/// drop additional state (e.g. `anvil/policy.yml`) before invoking the
/// binary.
fn fixture_repo() -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().to_path_buf();
    std::fs::create_dir_all(root.join("anvil")).expect("create anvil dir");
    std::fs::write(
        root.join("anvil").join("project-id"),
        format!("project_uuid: {FIXTURE_PROJECT_UUID}\n"),
    )
    .expect("write anvil/project-id");
    (tmp, root)
}

/// Spawn `anvil hook pre-push` with the fixture as its cwd (the
/// production binary derives `repo_root` from `current_dir`, see
/// `commands::hook::run`), feed the requested stdin, and collect
/// stdout/stderr + exit status. The git pre-push wall-clock budget
/// (`PRE_PUSH_BUDGET = 2s`) keeps wait time bounded even in the
/// failure cases.
fn run_pre_push(repo_root: &Path, stdin: &str) -> std::process::Output {
    let mut child = Command::new(ANVIL_BIN)
        .arg("hook")
        .arg("pre-push")
        .current_dir(repo_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn `anvil hook pre-push`");
    {
        let mut child_stdin = child.stdin.take().expect("child stdin piped");
        child_stdin
            .write_all(stdin.as_bytes())
            .expect("write pre-push stdin");
    }
    child
        .wait_with_output()
        .expect("`anvil hook pre-push` subprocess waits")
}

/// `anvil hook pre-push` against a repo that has a project-id but no
/// policy file must short-circuit at `load_policy`'s `Ok(None)` arm,
/// exit 0, emit nothing, and leave the witness chain alone.
///
/// Pins the end-to-end binary contract for the "Anvil-opted-in but
/// nothing to enforce" surface — the most common state on a fresh
/// `anvil init` repo before policy lands.
#[test]
fn pre_push_no_policy_exits_zero_with_no_output() {
    let (_tmp, root) = fixture_repo();
    // Valid pre-push update line. Short hex SHAs (`aaa111`, 6 hex
    // chars) are accepted by `parse_pre_push_input`, which validates
    // 4..=64 hex without requiring full-length SHA-1 / SHA-256.
    let stdin = "refs/heads/main aaa111 refs/heads/main bbb222\n";

    let out = run_pre_push(&root, stdin);

    assert!(
        out.status.success(),
        "no-policy push must exit 0; got {:?}, stderr = {}",
        out.status,
        String::from_utf8_lossy(&out.stderr),
    );
    assert!(
        out.stderr.is_empty(),
        "no-policy push must emit no stderr; got {:?}",
        String::from_utf8_lossy(&out.stderr),
    );
    assert!(
        !root.join("anvil").join("witness").exists(),
        "no-policy push must not create `anvil/witness/`",
    );
}

/// Bumping `required_anvil_version` above the running binary's version
/// must route through `VersionFloorOutcome::BelowFloor`: exit 0 (Serena
/// rule — an internal precondition does not block the user), with one
/// `ErrorClass::VersionFloor` line on stderr naming both the policy
/// field and the operator's remediation ("upgrade anvil").
///
/// Pins the production binary's MLP2-020 contract end-to-end so a
/// future refactor cannot silently downgrade the floor check to a hard
/// block, swap the stderr wording away from `required_anvil_version`,
/// or drop the "upgrade anvil" remediation hint.
#[test]
fn pre_push_below_required_anvil_version_admits_with_upgrade_line() {
    let (_tmp, root) = fixture_repo();
    // Floor pinned far above any plausible running binary so the hook
    // unconditionally routes through `BelowFloor`.
    // `RequiredAnvilVersion::parse` uses `semver::Version::parse`
    // (exact version, NOT `VersionReq`), so the value MUST be a bare
    // semver triple like `99.0.0` — prefixing with `>=` parses as
    // `InvalidFloor` and the hook surfaces `EmbeddedFailed` instead.
    // `Policy::validate` also requires at least one branch rule even
    // when the floor check fires first, hence the trailing no-op
    // `main` rule.
    std::fs::write(
        root.join("anvil").join("policy.yml"),
        concat!(
            "required_anvil_version: '99.0.0'\n",
            "branches:\n",
            "  - pattern: main\n",
            "    require: l4_or_l3\n",
            "    on_no_witness: validate_at_l4\n",
        ),
    )
    .expect("write anvil/policy.yml");
    let stdin = "refs/heads/main aaa111 refs/heads/main bbb222\n";

    let out = run_pre_push(&root, stdin);

    assert!(
        out.status.success(),
        "version-floor unmet must still admit (Serena rule); got {:?}, stderr = {}",
        out.status,
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("required_anvil_version"),
        "stderr must name the policy field for actionable remediation; got {stderr:?}",
    );
    assert!(
        stderr.contains("upgrade anvil"),
        "stderr must point the operator at the right fix; got {stderr:?}",
    );
    // MLP2-020 ordering: the floor check fires BEFORE chain
    // verification (`hook.rs` §"Ordering note"), so the binary must
    // not open the witness chain on this path.
    assert!(
        !root.join("anvil").join("witness").exists(),
        "version-floor unmet must not touch `anvil/witness/`",
    );
}
