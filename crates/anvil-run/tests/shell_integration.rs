//! INTL-006: shell integration smoke tests.
//!
//! These tests do not depend on a live daemon. They source the
//! shipped `shell/anvil-run.sh` in a `bash --noprofile` subprocess
//! and assert the dispatcher behaves the way the planning contract
//! specifies:
//!
//! 1. With `ANVIL_RUN_DISABLE=1` the wrapped command runs verbatim
//!    without invoking the launcher.
//! 2. With `ANVIL_RUN_BIN` pointing at a stub, the wrapper calls
//!    that stub with `--tool <name> -- <cmd> [args...]`.
//! 3. When the binary is not on `$PATH` and `ANVIL_RUN_BIN` is
//!    unset, the wrapper falls through so the user is not blocked.
//!
//! Unix-only: the helpers rely on `std::os::unix::fs::PermissionsExt`,
//! `chmod +x` semantics, and a POSIX `bash`. The shell wrapper itself
//! only targets bash/zsh; a Windows port would need a separate test.

#![cfg(unix)]

use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

fn script_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("shell")
        .join("anvil-run.sh")
}

#[cfg(unix)]
fn write_stub(dir: &Path, name: &str, body: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(body.as_bytes()).unwrap();
    let mut perms = f.metadata().unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).unwrap();
    path
}

#[cfg(unix)]
fn bash_path() -> std::path::PathBuf {
    // Locate bash via the parent's PATH so the test can clear PATH
    // for the child without breaking exec resolution.
    for dir in std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .unwrap_or_default()
    {
        let candidate = dir.join("bash");
        if candidate.is_file() {
            return candidate;
        }
    }
    std::path::PathBuf::from("/usr/bin/bash")
}

#[cfg(unix)]
fn run_bash(script: &str, envs: &[(&str, &str)]) -> std::process::Output {
    let mut cmd = Command::new(bash_path());
    cmd.arg("--noprofile").arg("--norc").arg("-c").arg(script);
    cmd.env_clear();
    cmd.env("PATH", std::env::var("PATH").unwrap_or_default());
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.output().expect("bash runs")
}

#[cfg(unix)]
#[test]
fn dispatcher_invokes_stub_with_tool_and_trailing_args() {
    let tmp = tempfile::tempdir().unwrap();
    let stub_log = tmp.path().join("stub.log");
    // Stub records its argv to `stub_log`. We then assert the file
    // contents to pin the exact wire shape.
    let stub = write_stub(
        tmp.path(),
        "fake-anvil-run",
        &format!(
            "#!/usr/bin/env bash\nprintf '%s\\n' \"$@\" > \"{}\"\n",
            stub_log.display(),
        ),
    );
    let script = format!(
        ". {script}\nclaude hello world",
        script = script_path().display(),
    );
    let out = run_bash(&script, &[("ANVIL_RUN_BIN", stub.to_str().unwrap())]);
    assert!(out.status.success(), "wrapper exited non-zero: {out:?}");
    let captured = std::fs::read_to_string(&stub_log).expect("stub log");
    let lines: Vec<&str> = captured.lines().collect();
    assert_eq!(
        lines,
        vec!["--tool", "claude-code", "--", "claude", "hello", "world"],
        "stub argv must match the documented wire shape",
    );
}

#[cfg(unix)]
#[test]
fn anvil_run_disable_bypasses_the_launcher() {
    let tmp = tempfile::tempdir().unwrap();
    let stub_log = tmp.path().join("stub.log");
    // The stub MUST NOT be invoked when ANVIL_RUN_DISABLE is set.
    let stub = write_stub(
        tmp.path(),
        "fake-anvil-run",
        &format!(
            "#!/usr/bin/env bash\nprintf 'launcher invoked\\n' > \"{}\"\n",
            stub_log.display(),
        ),
    );
    let script = format!(
        ". {script}\nclaude --version >/dev/null 2>&1 || true",
        script = script_path().display(),
    );
    let out = run_bash(
        &script,
        &[
            ("ANVIL_RUN_BIN", stub.to_str().unwrap()),
            ("ANVIL_RUN_DISABLE", "1"),
        ],
    );
    assert!(out.status.success() || out.status.code().is_some());
    assert!(
        !stub_log.exists(),
        "ANVIL_RUN_DISABLE must bypass the launcher; stub log: {:?}",
        std::fs::read_to_string(&stub_log).unwrap_or_default(),
    );
}

#[cfg(unix)]
#[test]
fn missing_binary_falls_through_to_the_underlying_command() {
    // No ANVIL_RUN_BIN, and `anvil-run` is not on $PATH (we use
    // env_clear + only restore PATH from the harness's PATH;
    // bash's PATH won't include our binary's target dir under
    // cargo test, which is what we want — the wrapper should
    // gracefully run the underlying command anyway).
    //
    // We can't easily guarantee `anvil-run` is not on PATH on the
    // dev host, so to keep this test deterministic we set PATH to
    // a single tmp dir with only an `aider` stub. The wrapper
    // sources, observes anvil-run is absent, and falls through to
    // `command aider ...`, which calls our stub.
    let tmp = tempfile::tempdir().unwrap();
    let aider_log = tmp.path().join("aider.log");
    let bash = bash_path();
    write_stub(
        tmp.path(),
        "aider",
        &format!(
            "#!{bash}\nprintf 'aider called\\n' > \"{}\"\n",
            aider_log.display(),
            bash = bash.display(),
        ),
    );
    let script = format!(
        ". {script}\naider --do-something",
        script = script_path().display(),
    );
    let mut cmd = Command::new(&bash);
    cmd.arg("--noprofile").arg("--norc").arg("-c").arg(&script);
    cmd.env_clear();
    cmd.env("PATH", tmp.path().to_string_lossy().to_string());
    let out = cmd.output().expect("bash runs");
    assert!(out.status.success(), "wrapper exited non-zero: {out:?}");
    assert!(aider_log.exists(), "fallback must execute the real tool");
}
