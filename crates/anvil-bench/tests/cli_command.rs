use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anvil_bench::cli_command::{CommandBenchmarkConfig, FixtureSpec, run};

#[test]
fn cli_command_report_excludes_raw_args_and_counts_only_measured_iterations() {
    let fake = fake_anvil_script("echo ok");
    let out = tempfile::tempdir().unwrap().path().join("report.json");
    let config = CommandBenchmarkConfig {
        name: "fake-status".to_owned(),
        anvil_bin: fake,
        anvil_args: vec![
            "status".to_owned(),
            "--token".to_owned(),
            "super-secret".to_owned(),
        ],
        repeat: 2,
        warmup: 1,
        fixture: FixtureSpec::Empty,
        timeout: Duration::from_secs(5),
        sample_interval: Duration::from_millis(10),
        output: Some(out.clone()),
        include_raw_argv: false,
    };

    let report = run(&config).expect("benchmark runs");

    assert_eq!(report.schema_version, 1);
    assert_eq!(report.name, "fake-status");
    assert_eq!(report.command_family.as_deref(), Some("status"));
    assert_eq!(
        report.iterations.len(),
        2,
        "warmups are not reported as samples"
    );
    assert_eq!(report.aggregate.samples, 2);
    assert_eq!(report.aggregate.failures, 0);
    assert_eq!(report.aggregate.timeouts, 0);
    assert!(report.raw_argv.is_none(), "raw argv must be opt-in only");
    let json = serde_json::to_string(&report).unwrap();
    assert!(
        !json.contains("super-secret"),
        "report must not leak raw argument values"
    );
    assert!(json.contains("ANVIL_USAGE_DISABLE"));
    assert!(out.is_file(), "configured output report is written");
}

#[test]
fn cli_command_runner_times_out_and_records_failure_without_panicking() {
    let fake = fake_anvil_script("sleep 2");
    let config = CommandBenchmarkConfig {
        name: "fake-timeout".to_owned(),
        anvil_bin: fake,
        anvil_args: vec!["status".to_owned()],
        repeat: 1,
        warmup: 0,
        fixture: FixtureSpec::Empty,
        timeout: Duration::from_millis(50),
        sample_interval: Duration::from_millis(10),
        output: None,
        include_raw_argv: false,
    };

    let report = run(&config).expect("timeout is reported, not raised");

    assert_eq!(report.iterations.len(), 1);
    assert!(report.iterations[0].timed_out);
    assert_eq!(report.iterations[0].exit_code, None);
    assert_eq!(report.aggregate.failures, 1);
    assert_eq!(report.aggregate.timeouts, 1);
}

#[test]
fn cli_command_runner_does_not_invoke_a_shell_for_anvil_args() {
    let capture = tempfile::NamedTempFile::new().unwrap();
    let capture_path = capture.path().to_path_buf();
    let fake = fake_anvil_capture_script(&capture_path);
    let config = CommandBenchmarkConfig {
        name: "fake-literal".to_owned(),
        anvil_bin: fake,
        anvil_args: vec!["status".to_owned(), "$(touch should-not-exist)".to_owned()],
        repeat: 1,
        warmup: 0,
        fixture: FixtureSpec::Empty,
        timeout: Duration::from_secs(5),
        sample_interval: Duration::from_millis(10),
        output: None,
        include_raw_argv: false,
    };

    let report = run(&config).expect("benchmark runs");

    assert_eq!(report.aggregate.failures, 0);
    assert!(
        fs::read_to_string(&capture_path)
            .unwrap()
            .contains("$(touch should-not-exist)")
    );
    assert!(
        !Path::new("should-not-exist").exists(),
        "argument must not be shell-expanded"
    );
}

fn fake_anvil_script(body: &str) -> PathBuf {
    let dir = tempfile::tempdir().unwrap();
    let path = fake_path(dir.path());
    std::mem::forget(dir);
    write_script(&path, body);
    path
}

fn fake_anvil_capture_script(capture: &Path) -> PathBuf {
    let dir = tempfile::tempdir().unwrap();
    let path = fake_path(dir.path());
    std::mem::forget(dir);
    let body = if cfg!(windows) {
        format!("@echo off\r\necho %* > {}\r\n", capture.display())
    } else {
        format!("printf '%s\\n' \"$@\" > '{}'\n", capture.display())
    };
    write_script(&path, &body);
    path
}

fn fake_path(dir: &Path) -> PathBuf {
    dir.join(if cfg!(windows) {
        "fake-anvil.cmd"
    } else {
        "fake-anvil"
    })
}

fn write_script(path: &Path, body: &str) {
    let script = if cfg!(windows) {
        if body.starts_with("@echo off") {
            body.to_owned()
        } else {
            format!("@echo off\r\n{body}\r\n")
        }
    } else {
        format!("#!/usr/bin/env bash\nset -euo pipefail\n{body}\n")
    };
    fs::write(path, script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }
}
