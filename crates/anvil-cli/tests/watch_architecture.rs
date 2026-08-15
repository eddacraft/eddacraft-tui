//! #3918: watch must report the same architecture verdicts as gate.
//!
//! `anvil watch --action none` used the kernel's cross-layer classifier and
//! did not reload or fail-closed on architecture policy edits. Gate's
//! architecture check already produces named boundaries such as
//! `no-core-to-app` and rejects `depends_on: [missing]`.
//!
//! These process tests spawn the real binary against inline, delegated, and
//! legacy standalone topologies and compare watch vs gate for the same file.

#![cfg(unix)]

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

static WATCH_LOCK: Mutex<()> = Mutex::new(());

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");
const WAIT_BUDGET: Duration = Duration::from_secs(12);

const CLEAN_ENTITY: &str = "pub struct Entity;\n";
const FORBIDDEN_ENTITY: &str = "use crate::app::service::Service;\npub struct Entity;\n";
const SERVICE: &str = "pub struct Service;\n";

const ARCH_YAML: &str = r#"schema_version: "0.1.0"
layers:
  core:
    patterns:
      - "src/core/**"
    depends_on: []
  app:
    patterns:
      - "src/app/**"
    depends_on: [core]
"#;

const ARCH_YAML_INVALID: &str = r#"schema_version: "0.1.0"
layers:
  core:
    patterns:
      - "src/core/**"
    depends_on: []
  app:
    patterns:
      - "src/app/**"
    depends_on: [missing]
"#;

#[derive(Clone, Copy)]
enum Topology {
    Inline,
    Delegated,
    LegacyStandalone,
}

impl Topology {
    fn name(self) -> &'static str {
        match self {
            Self::Inline => "inline",
            Self::Delegated => "delegated",
            Self::LegacyStandalone => "legacy-standalone",
        }
    }
}

struct WatchProcess {
    child: Child,
    stdout_rx: mpsc::Receiver<String>,
    stderr_rx: mpsc::Receiver<String>,
    reader: Option<thread::JoinHandle<()>>,
    err_reader: Option<thread::JoinHandle<()>>,
}

impl WatchProcess {
    fn spawn(workdir: &Path, home: &Path) -> Self {
        let mut cmd = Command::new(ANVIL_BIN);
        cmd.args([
            "--no-tui",
            "--json",
            "watch",
            "--action",
            "none",
            "--patterns",
            "src/**/*.rs",
            "--no-daemon",
            "--debounce",
            "50",
        ]);
        cmd.current_dir(workdir)
            .env("HOME", home)
            .env("USERPROFILE", home)
            .env_remove("XDG_CONFIG_HOME")
            .env("ANVIL_DEV", "1")
            .env("ANVIL_SKIP_WELCOME", "1")
            .env("ANVIL_DISABLE_UPDATE_HINT", "1");
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().expect("spawn anvil watch");
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");
        let (stdout_tx, stdout_rx) = mpsc::channel();
        let (stderr_tx, stderr_rx) = mpsc::channel();

        let reader = thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                match line {
                    Ok(line) => {
                        if stdout_tx.send(line).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        let err_reader = thread::spawn(move || {
            for line in BufReader::new(stderr).lines() {
                match line {
                    Ok(line) => {
                        if stderr_tx.send(line).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            child,
            stdout_rx,
            stderr_rx,
            reader: Some(reader),
            err_reader: Some(err_reader),
        }
    }

    fn drain_available(&self, stdout: &mut Vec<String>, stderr: &mut Vec<String>) {
        while let Ok(line) = self.stdout_rx.try_recv() {
            stdout.push(line);
        }
        while let Ok(line) = self.stderr_rx.try_recv() {
            stderr.push(line);
        }
    }

    fn collect_until<F>(&mut self, budget: Duration, mut predicate: F) -> (Vec<String>, Vec<String>)
    where
        F: FnMut(&str, &[String], &[String]) -> bool,
    {
        let deadline = Instant::now() + budget;
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let poll_step = Duration::from_millis(50);
        while Instant::now() < deadline {
            if let Ok(Some(status)) = self.child.try_wait() {
                thread::sleep(Duration::from_millis(50));
                self.drain_available(&mut stdout, &mut stderr);
                if stdout.is_empty() && stderr.is_empty() {
                    stderr.push(format!("watch exited early: {status}"));
                }
                return (stdout, stderr);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            let wait = remaining.min(poll_step);
            match self.stdout_rx.recv_timeout(wait) {
                Ok(line) => {
                    stdout.push(line);
                    if predicate(stdout.last().expect("just pushed"), &stdout, &stderr) {
                        return (stdout, stderr);
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
            while let Ok(line) = self.stderr_rx.try_recv() {
                stderr.push(line);
                if predicate(stderr.last().expect("just pushed"), &stdout, &stderr) {
                    return (stdout, stderr);
                }
            }
        }
        (stdout, stderr)
    }

    fn shutdown(mut self) -> (Vec<String>, String) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        if let Some(reader) = self.err_reader.take() {
            let _ = reader.join();
        }
        let mut leftover = Vec::new();
        while let Ok(line) = self.stdout_rx.try_recv() {
            leftover.push(line);
        }
        let mut stderr = String::new();
        while let Ok(line) = self.stderr_rx.try_recv() {
            stderr.push_str(&line);
            stderr.push('\n');
        }
        (leftover, stderr)
    }
}

impl Drop for WatchProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn seed_rust_workspace(root: &Path, topology: Topology) {
    std::fs::create_dir_all(root.join("src/core")).unwrap();
    std::fs::create_dir_all(root.join("src/app")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(root.join("src/core/entity.rs"), CLEAN_ENTITY).unwrap();
    std::fs::write(root.join("src/app/service.rs"), SERVICE).unwrap();
    match topology {
        Topology::Inline => {
            std::fs::write(
                root.join(".anvil.yaml"),
                format!("architecture:\n{}", indent_yaml(ARCH_YAML)),
            )
            .unwrap();
        }
        Topology::Delegated => {
            std::fs::create_dir_all(root.join(".anvil")).unwrap();
            std::fs::write(root.join(".anvil/architecture.yaml"), ARCH_YAML).unwrap();
            std::fs::write(
                root.join(".anvil.yaml"),
                "architecture:\n  source: \".anvil/architecture.yaml\"\n",
            )
            .unwrap();
        }
        Topology::LegacyStandalone => {
            std::fs::create_dir_all(root.join(".anvil")).unwrap();
            std::fs::write(root.join(".anvil/architecture.yaml"), ARCH_YAML).unwrap();
        }
    }
}

fn indent_yaml(yaml: &str) -> String {
    yaml.lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("  {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn architecture_source_path(root: &Path, topology: Topology) -> PathBuf {
    match topology {
        Topology::Inline => root.join(".anvil.yaml"),
        Topology::Delegated | Topology::LegacyStandalone => root.join(".anvil/architecture.yaml"),
    }
}

fn write_invalid_policy(root: &Path, topology: Topology) {
    match topology {
        Topology::Inline => {
            std::fs::write(
                root.join(".anvil.yaml"),
                format!("architecture:\n{}", indent_yaml(ARCH_YAML_INVALID)),
            )
            .unwrap();
        }
        Topology::Delegated | Topology::LegacyStandalone => {
            std::fs::write(root.join(".anvil/architecture.yaml"), ARCH_YAML_INVALID).unwrap();
        }
    }
}

fn configure_cmd(command: &mut Command, workdir: &Path, home: &Path) {
    command
        .current_dir(workdir)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env_remove("XDG_CONFIG_HOME")
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_DISABLE_UPDATE_HINT", "1");
}

fn run_gate_architecture(workdir: &Path, home: &Path) -> String {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.args(["--no-tui", "gate", "--only-checks", "architecture"]);
    configure_cmd(&mut cmd, workdir, home);
    let output = cmd.output().expect("run anvil gate");
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.stderr.is_empty() {
        text.push('\n');
        text.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    text
}

fn run_architecture_validate(workdir: &Path, home: &Path) -> std::process::Output {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.args(["--no-tui", "architecture", "validate"]);
    configure_cmd(&mut cmd, workdir, home);
    cmd.output().expect("run anvil architecture validate")
}

fn combined(stdout: &[String], stderr: &[String]) -> String {
    let mut out = stdout.join("\n");
    if !stderr.is_empty() {
        out.push('\n');
        out.push_str(&stderr.join("\n"));
    }
    out
}

fn wait_ready(proc: &mut WatchProcess) -> (Vec<String>, Vec<String>) {
    let (stdout, stderr) = proc.collect_until(WAIT_BUDGET, |line, _, _| {
        line.contains("\"event_type\":\"snapshot\"") || line.contains("[watching] ready")
    });
    assert!(
        stdout.iter().any(|line| {
            line.contains("\"event_type\":\"snapshot\"") || line.contains("[watching] ready")
        }) || stderr.iter().any(|line| line.contains("[watching] ready")),
        "watch never became ready within {WAIT_BUDGET:?}; stdout={stdout:?} stderr={stderr:?}"
    );
    (stdout, stderr)
}

fn assert_same_boundary_verdict(topology: Topology) {
    let _guard = WATCH_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let workdir = tempfile::tempdir().expect("workdir");
    let home = tempfile::tempdir().expect("home");
    seed_rust_workspace(workdir.path(), topology);

    let mut proc = WatchProcess::spawn(workdir.path(), home.path());
    let (ready_out, ready_err) = wait_ready(&mut proc);

    std::fs::write(workdir.path().join("src/core/entity.rs"), FORBIDDEN_ENTITY)
        .expect("write forbidden import");

    let (after_out, after_err) = proc.collect_until(WAIT_BUDGET, |line, stdout, stderr| {
        combined(stdout, stderr).contains("no-core-to-app") || line.contains("no-core-to-app")
    });
    let (leftover, shutdown_err) = proc.shutdown();
    let mut watch_text = combined(&ready_out, &ready_err);
    watch_text.push('\n');
    watch_text.push_str(&combined(&after_out, &after_err));
    watch_text.push('\n');
    watch_text.push_str(&leftover.join("\n"));
    watch_text.push('\n');
    watch_text.push_str(&shutdown_err);

    let gate_text = run_gate_architecture(workdir.path(), home.path());

    assert!(
        gate_text.contains("no-core-to-app"),
        "{}: gate must report no-core-to-app; output={gate_text}",
        topology.name()
    );
    assert!(
        watch_text.contains("no-core-to-app"),
        "{}: watch must report the same no-core-to-app boundary as gate; watch={watch_text} gate={gate_text}",
        topology.name()
    );
}

#[test]
fn watch_and_gate_agree_on_forbidden_rust_import_inline() {
    assert_same_boundary_verdict(Topology::Inline);
}

#[test]
fn watch_and_gate_agree_on_forbidden_rust_import_delegated() {
    assert_same_boundary_verdict(Topology::Delegated);
}

#[test]
fn watch_and_gate_agree_on_forbidden_rust_import_legacy_standalone() {
    assert_same_boundary_verdict(Topology::LegacyStandalone);
}

fn assert_invalid_policy_is_loud(topology: Topology) {
    let _guard = WATCH_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let workdir = tempfile::tempdir().expect("workdir");
    let home = tempfile::tempdir().expect("home");
    seed_rust_workspace(workdir.path(), topology);

    let mut proc = WatchProcess::spawn(workdir.path(), home.path());
    let _ = wait_ready(&mut proc);

    write_invalid_policy(workdir.path(), topology);
    let _ = architecture_source_path(workdir.path(), topology);

    let (after_out, after_err) = proc.collect_until(WAIT_BUDGET, |line, stdout, stderr| {
        let text = combined(stdout, stderr);
        let hay = format!("{text}\n{line}");
        hay.contains("unknown layer") || hay.contains("preflight") || hay.contains("missing")
    });

    // A later save must not look clean under the stale valid policy.
    std::fs::write(
        workdir.path().join("src/app/service.rs"),
        "pub struct Service;\n// touch\n",
    )
    .unwrap();
    let (touch_out, touch_err) = proc.collect_until(WAIT_BUDGET, |line, stdout, stderr| {
        let text = combined(stdout, stderr);
        let hay = format!("{text}\n{line}");
        hay.contains("unknown layer") || hay.contains("preflight") || hay.contains("missing")
    });

    let (leftover, shutdown_err) = proc.shutdown();
    let mut watch_text = combined(&after_out, &after_err);
    watch_text.push('\n');
    watch_text.push_str(&combined(&touch_out, &touch_err));
    watch_text.push('\n');
    watch_text.push_str(&leftover.join("\n"));
    watch_text.push('\n');
    watch_text.push_str(&shutdown_err);

    let validate = run_architecture_validate(workdir.path(), home.path());
    assert!(
        !validate.status.success(),
        "{}: architecture validate must exit 1 on depends_on: [missing]",
        topology.name()
    );

    assert!(
        watch_text.contains("unknown layer")
            || watch_text.contains("preflight")
            || watch_text.contains("missing"),
        "{}: invalid policy edit must be loud in watch; output={watch_text}",
        topology.name()
    );
    assert!(
        !watch_text.contains("Architecture config is valid"),
        "{}: watch must not claim valid architecture after an invalid edit; output={watch_text}",
        topology.name()
    );
}

#[test]
fn watch_fails_closed_on_invalid_inline_policy_edit() {
    assert_invalid_policy_is_loud(Topology::Inline);
}

#[test]
fn watch_fails_closed_on_invalid_delegated_policy_edit() {
    assert_invalid_policy_is_loud(Topology::Delegated);
}

#[test]
fn watch_fails_closed_on_invalid_legacy_policy_edit() {
    assert_invalid_policy_is_loud(Topology::LegacyStandalone);
}
