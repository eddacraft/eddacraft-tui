//! JREL-004: one daemon identity through start and recycle, proven against the
//! built `anvil` binary.
//!
//! Every daemon here is spawned as `anvil intercept start --foreground` with a
//! per-test `HOME` / `XDG_RUNTIME_DIR` / `ANVIL_HOME` set on the child only.
//! The ensure surface under test is bare `anvil` (ADR-114), which may spawn
//! without a TTY, so a duplicate daemon is observable as a second PID file.
//!
//! One seam is not hermetic and cannot be made so through the binary. The
//! "plain shell" fixtures deliberately leave `XDG_RUNTIME_DIR` unset, which is
//! exactly the condition under which the daemon adds the real
//! `/run/user/<uid>` endpoint as an implicit sibling candidate
//! (`anvil_intercept::ipc::implicit_xdg_runtime_dir`). A developer daemon bound
//! there would be probed by those tests, and under all-candidate probing it
//! would be either reused or counted as a second live endpoint. Rather than
//! silently touch it, [`refuse_when_a_real_daemon_could_be_probed`] fails the
//! test with an actionable message. These tests therefore require a host with
//! no live daemon at the implicit runtime endpoint.
//!
//! Disjoint-runtime fixtures (two explicit `XDG_RUNTIME_DIR` values, shared
//! `HOME`, no `ANVIL_HOME`) never touch that implicit sibling: both shells have
//! a runtime dir set, so `/run/user/<uid>` is not a candidate. That is the
//! #4432 / option B acceptance case.
//!
//! The "runtime dir set, then unset" order cannot be reproduced through the
//! binary: a shell without `XDG_RUNTIME_DIR` only probes the fixed
//! `/run/user/<uid>` sibling, which a hermetic test cannot re-root. That order
//! is covered with real sockets in
//! `anvil_intercept::ensure::tests::plain_shell_reuses_runtime_dir_daemon_through_the_implicit_sibling`.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

/// One shell environment: the variables that decide where this process
/// discovers, starts, and stops the daemon.
#[derive(Clone, Debug)]
struct Shell {
    home: PathBuf,
    runtime_dir: Option<PathBuf>,
    anvil_home: Option<PathBuf>,
}

impl Shell {
    fn command(&self) -> Command {
        let mut cmd = Command::new(ANVIL_BIN);
        cmd.env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env("ANVIL_DEV", "1")
            .env("ANVIL_SKIP_WELCOME", "1")
            .env("ANVIL_DISABLE_UPDATE_HINT", "1")
            .env("ANVIL_NO_MCP", "1")
            // Save-time driver supervision is JREL-003's concern; keep the
            // daemon to its endpoint identity here.
            .env("ANVIL_NO_SAVE_TIME_DRIVER", "1")
            .env_remove("ANVIL_NO_DAEMON")
            .env_remove("ANVIL_LOG")
            .env_remove("RUST_LOG")
            .env_remove("XDG_RUNTIME_DIR")
            .env_remove("ANVIL_HOME");
        if let Some(runtime_dir) = &self.runtime_dir {
            cmd.env("XDG_RUNTIME_DIR", runtime_dir);
        }
        if let Some(anvil_home) = &self.anvil_home {
            cmd.env("ANVIL_HOME", anvil_home);
        }
        cmd
    }

    /// Where this shell binds a daemon it starts (canonical endpoint dir).
    fn canonical_dir(&self) -> PathBuf {
        if let Some(anvil_home) = &self.anvil_home {
            return anvil_home.clone();
        }
        match &self.runtime_dir {
            Some(runtime_dir) => runtime_dir.join("anvil"),
            None => self.home.join(".local/state/anvil"),
        }
    }

    fn status_json(&self) -> Output {
        self.command()
            .args(["intercept", "status", "--json"])
            .output()
            .expect("run anvil intercept status --json")
    }

    fn status_human(&self) -> Output {
        self.command()
            .args(["intercept", "status"])
            .output()
            .expect("run anvil intercept status")
    }

    fn ensure(&self, project: &Path) -> Output {
        self.command()
            .current_dir(project)
            .output()
            .expect("run bare anvil ensure")
    }

    fn stop(&self) -> Output {
        self.command()
            .args(["intercept", "stop"])
            .output()
            .expect("run anvil intercept stop")
    }
}

struct DaemonChild(Child);

impl Drop for DaemonChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Stops every daemon a shell can reach when the test ends, including any
/// duplicate a failing ensure spawned, so a red run does not leak processes.
struct StopOnDrop(Vec<Shell>);

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        for shell in &self.0 {
            let _ = shell.stop();
        }
    }
}

fn owner_only_dir(path: &Path) {
    fs::create_dir_all(path).expect("create directory");
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).expect("owner-only directory");
}

fn seed_project(root: &Path) -> PathBuf {
    let project = root.join("project");
    fs::create_dir_all(&project).expect("create project");
    let git = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&project)
        .output()
        .expect("git init");
    assert!(git.status.success(), "git init failed");
    fs::write(project.join(".anvil.json"), "{}\n").expect("write config");
    project
}

fn spawn_daemon(shell: &Shell, project: &Path) -> DaemonChild {
    let child = shell
        .command()
        .args(["intercept", "start", "--foreground"])
        .current_dir(project)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn intercept daemon");
    DaemonChild(child)
}

fn wait_for_status(shell: &Shell) -> serde_json::Value {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let output = shell.status_json();
        if output.status.success()
            && let Ok(value) = serde_json::from_slice::<serde_json::Value>(&output.stdout)
        {
            return value;
        }
        assert!(
            Instant::now() < deadline,
            "daemon did not answer status for {shell:?}: stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn pid_file_pid(dir: &Path) -> Option<u32> {
    let record = fs::read_to_string(dir.join("intercept.pid")).ok()?;
    record.lines().next()?.trim().parse().ok()
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Unset then set: a daemon started from a shell without `XDG_RUNTIME_DIR`
/// binds under the state home. A later shell that has a runtime dir must
/// converge on that verified sibling instead of starting a second daemon at
/// its own canonical endpoint.
#[test]
fn xdg_shell_ensure_reuses_state_home_daemon_instead_of_starting_a_duplicate() {
    refuse_when_a_real_daemon_could_be_probed();
    let root = tempfile::tempdir().expect("tempdir");
    let home = root.path().join("home");
    let runtime = root.path().join("runtime");
    owner_only_dir(&home);
    owner_only_dir(&runtime);
    let project = seed_project(root.path());

    let plain = Shell {
        home: home.clone(),
        runtime_dir: None,
        anvil_home: None,
    };
    let xdg = Shell {
        home: home.clone(),
        runtime_dir: Some(runtime.clone()),
        anvil_home: None,
    };
    let _cleanup = StopOnDrop(vec![xdg.clone(), plain.clone()]);

    let _daemon = spawn_daemon(&plain, &project);
    wait_for_status(&plain);
    let original_pid = pid_file_pid(&plain.canonical_dir()).expect("state-home PID file");

    let ensure = xdg.ensure(&project);
    assert!(
        ensure.status.success(),
        "ensure from the runtime-dir shell must succeed: stdout={}\nstderr={}",
        stdout_of(&ensure),
        stderr_of(&ensure)
    );
    let stdout = stdout_of(&ensure);
    assert!(
        stdout.contains("daemon: running"),
        "ensure must reuse the live state-home daemon, not start one:\n{stdout}"
    );
    assert!(
        !xdg.canonical_dir().join("intercept.pid").exists(),
        "no second daemon may be started at the runtime-dir endpoint {}",
        xdg.canonical_dir().display()
    );
    assert_eq!(
        pid_file_pid(&plain.canonical_dir()),
        Some(original_pid),
        "the state-home daemon must be the same instance after ensure"
    );

    // Discovery, status and registration from both shells name one instance.
    let status = xdg.status_human();
    let status_text = stdout_of(&status);
    assert!(
        status_text.contains(&format!(
            "socket:    {}",
            plain.canonical_dir().join("intercept.sock").display()
        )),
        "status from the runtime-dir shell must name the state-home socket:\n{status_text}"
    );
    assert!(
        status_text.contains(&format!("pid {original_pid}")),
        "status must report the PID of the daemon that answered:\n{status_text}"
    );
}

/// A stale canonical socket file (daemon crashed, inode left behind) next to
/// a live sibling must converge on the sibling; the stale inode is never a
/// reason to start a duplicate.
#[test]
fn stale_canonical_socket_with_live_sibling_converges_on_the_sibling() {
    use std::os::unix::net::UnixListener;

    refuse_when_a_real_daemon_could_be_probed();

    let root = tempfile::tempdir().expect("tempdir");
    let home = root.path().join("home");
    let runtime = root.path().join("runtime");
    owner_only_dir(&home);
    owner_only_dir(&runtime);
    let project = seed_project(root.path());

    let plain = Shell {
        home: home.clone(),
        runtime_dir: None,
        anvil_home: None,
    };
    let xdg = Shell {
        home: home.clone(),
        runtime_dir: Some(runtime.clone()),
        anvil_home: None,
    };
    let _cleanup = StopOnDrop(vec![xdg.clone(), plain.clone()]);

    // Plant the stale canonical endpoint: bind then drop leaves the socket
    // inode with no listener, exactly what a hard-killed daemon leaves.
    let canonical_dir = xdg.canonical_dir();
    owner_only_dir(&canonical_dir);
    let stale_socket = canonical_dir.join("intercept.sock");
    drop(UnixListener::bind(&stale_socket).expect("bind stale socket"));
    fs::set_permissions(&stale_socket, fs::Permissions::from_mode(0o600))
        .expect("owner-only stale socket");
    assert!(stale_socket.exists(), "stale socket inode must remain");

    let _daemon = spawn_daemon(&plain, &project);
    wait_for_status(&plain);
    let original_pid = pid_file_pid(&plain.canonical_dir()).expect("state-home PID file");

    let ensure = xdg.ensure(&project);
    assert!(
        ensure.status.success(),
        "ensure must succeed: stdout={}\nstderr={}",
        stdout_of(&ensure),
        stderr_of(&ensure)
    );
    assert!(
        stdout_of(&ensure).contains("daemon: running"),
        "ensure must reuse the live sibling:\n{}",
        stdout_of(&ensure)
    );
    assert!(
        !canonical_dir.join("intercept.pid").exists(),
        "a stale canonical socket must not trigger a duplicate daemon"
    );
    assert_eq!(pid_file_pid(&plain.canonical_dir()), Some(original_pid));
    let status_text = stdout_of(&xdg.status_human());
    assert!(
        status_text.contains(&format!("pid {original_pid}")),
        "status must report the live sibling's PID, never a stale canonical record:\n{status_text}"
    );
}

/// Intentionally isolated `ANVIL_HOME` installations are distinct execution
/// contexts (ADR-060): ensure under a second home starts its own daemon and
/// leaves the first untouched.
#[test]
fn isolated_anvil_homes_keep_distinct_daemons() {
    let root = tempfile::tempdir().expect("tempdir");
    let home = root.path().join("home");
    let home_a = root.path().join("anvil-home-a");
    let home_b = root.path().join("anvil-home-b");
    owner_only_dir(&home);
    owner_only_dir(&home_a);
    owner_only_dir(&home_b);
    let project = seed_project(root.path());

    let shell_a = Shell {
        home: home.clone(),
        runtime_dir: None,
        anvil_home: Some(home_a.clone()),
    };
    let shell_b = Shell {
        home: home.clone(),
        runtime_dir: None,
        anvil_home: Some(home_b.clone()),
    };
    let _cleanup = StopOnDrop(vec![shell_a.clone(), shell_b.clone()]);

    let _daemon_a = spawn_daemon(&shell_a, &project);
    wait_for_status(&shell_a);
    let pid_a = pid_file_pid(&home_a).expect("home A PID file");

    let ensure = shell_b.ensure(&project);
    assert!(
        ensure.status.success(),
        "ensure under home B must succeed: stdout={}\nstderr={}",
        stdout_of(&ensure),
        stderr_of(&ensure)
    );
    assert!(
        stdout_of(&ensure).contains("daemon: started"),
        "an isolated home must start its own daemon:\n{}",
        stdout_of(&ensure)
    );
    let pid_b = pid_file_pid(&home_b).expect("home B PID file");
    assert_ne!(pid_a, pid_b, "isolated homes must not share a daemon");
    assert_eq!(
        pid_file_pid(&home_a),
        Some(pid_a),
        "home A daemon untouched"
    );

    let status_b = stdout_of(&shell_b.status_human());
    assert!(
        status_b.contains(&format!(
            "socket:    {}",
            home_b.join("intercept.sock").display()
        )) && status_b.contains(&format!("pid {pid_b}")),
        "home B status must name its own socket and PID:\n{status_b}"
    );
    let stop_b = shell_b.stop();
    assert!(stop_b.status.success(), "stop under home B");
    assert!(
        stdout_of(&stop_b).contains(&pid_b.to_string()),
        "stop under home B names its own daemon:\n{}",
        stdout_of(&stop_b)
    );
    assert!(
        anvil_intercept_pid_alive(pid_a),
        "stopping home B must never signal home A's daemon"
    );
}

/// Refuse to run a fixture that leaves `XDG_RUNTIME_DIR` unset while a real
/// daemon is bound at the implicit `/run/user/<uid>` endpoint.
///
/// Those fixtures cannot mask that candidate through the binary, so probing it
/// would reach the developer's own daemon. Fail loudly instead of reusing it or
/// reporting it as a same-scope conflict.
fn refuse_when_a_real_daemon_could_be_probed() {
    let uid = nix::unistd::Uid::current().as_raw();
    let implicit = PathBuf::from(format!("/run/user/{uid}/anvil/intercept.sock"));
    assert!(
        !implicit.exists(),
        "a daemon endpoint exists at {} and this fixture leaves XDG_RUNTIME_DIR \
         unset, so the run would probe it. Stop that daemon (`anvil intercept \
         stop`) before running this suite.",
        implicit.display()
    );
}

fn anvil_intercept_pid_alive(pid: u32) -> bool {
    Path::new("/proc").join(pid.to_string()).exists()
        || Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .is_ok_and(|out| out.status.success())
}

/// Concurrent cold starts from one environment through the public ensure
/// surface: the start lock admits exactly one spawn and the rest reuse it.
/// (The cross-environment race is proven with the real rendezvous lock in
/// `anvil_intercept::ensure::tests::concurrent_cross_environment_cold_starts_converge_on_one_daemon`.)
#[test]
fn concurrent_ensures_in_one_environment_start_exactly_one_daemon() {
    let root = tempfile::tempdir().expect("tempdir");
    let home = root.path().join("home");
    let runtime = root.path().join("runtime");
    owner_only_dir(&home);
    owner_only_dir(&runtime);
    let project = seed_project(root.path());
    let xdg = Shell {
        home: home.clone(),
        runtime_dir: Some(runtime.clone()),
        anvil_home: None,
    };
    let _cleanup = StopOnDrop(vec![xdg.clone()]);

    let outputs: Vec<Output> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let shell = xdg.clone();
                let project = project.clone();
                scope.spawn(move || shell.ensure(&project))
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    let mut started = 0;
    let mut running = 0;
    for output in &outputs {
        assert!(
            output.status.success(),
            "every concurrent ensure must succeed: stdout={}\nstderr={}",
            stdout_of(output),
            stderr_of(output)
        );
        let stdout = stdout_of(output);
        if stdout.contains("daemon: started") {
            started += 1;
        } else if stdout.contains("daemon: running") {
            running += 1;
        } else {
            panic!("unexpected daemon line:\n{stdout}");
        }
    }
    assert_eq!(started, 1, "exactly one caller starts the daemon");
    assert_eq!(running, 3, "the other callers reuse it");
    let pid = pid_file_pid(&xdg.canonical_dir()).expect("one PID file");
    assert!(
        anvil_intercept_pid_alive(pid),
        "the single started daemon is alive"
    );
    assert!(
        !home.join(".local/state/anvil/intercept.pid").exists(),
        "no daemon may appear at the sibling endpoint"
    );
}

/// #4432 option B: two shells in one execution scope that disagree about the
/// canonical endpoint (`XDG_RUNTIME_DIR=/tmp/xdg-a` vs `/tmp/xdg-b`, same
/// `HOME`, no `ANVIL_HOME`) serialise on the state-home coordinator and produce
/// exactly one daemon. Both XDG values are set, so `/run/user/<uid>` is not an
/// implicit candidate.
#[test]
fn concurrent_ensures_from_disjoint_runtime_dirs_start_exactly_one_daemon() {
    let root = tempfile::tempdir().expect("tempdir");
    let home = root.path().join("home");
    let xdg_a = root.path().join("xdg-a");
    let xdg_b = root.path().join("xdg-b");
    owner_only_dir(&home);
    owner_only_dir(&xdg_a);
    owner_only_dir(&xdg_b);
    let project = seed_project(root.path());
    let shell_a = Shell {
        home: home.clone(),
        runtime_dir: Some(xdg_a.clone()),
        anvil_home: None,
    };
    let shell_b = Shell {
        home: home.clone(),
        runtime_dir: Some(xdg_b.clone()),
        anvil_home: None,
    };
    let _cleanup = StopOnDrop(vec![shell_a.clone(), shell_b.clone()]);

    let outputs: Vec<(char, Output)> = std::thread::scope(|scope| {
        let handles: Vec<_> = ['a', 'b', 'a', 'b']
            .into_iter()
            .map(|which| {
                let shell = if which == 'a' {
                    shell_a.clone()
                } else {
                    shell_b.clone()
                };
                let project = project.clone();
                scope.spawn(move || (which, shell.ensure(&project)))
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    let mut started = 0;
    let mut running = 0;
    for (which, output) in &outputs {
        assert!(
            output.status.success(),
            "ensure from shell {which} must succeed: stdout={}\nstderr={}",
            stdout_of(output),
            stderr_of(output)
        );
        let stdout = stdout_of(output);
        if stdout.contains("daemon: started") {
            started += 1;
        } else if stdout.contains("daemon: running") {
            running += 1;
        } else {
            panic!("unexpected daemon line from shell {which}:\n{stdout}");
        }
    }
    assert_eq!(started, 1, "exactly one caller starts the daemon");
    assert_eq!(running, 3, "the other callers reuse it");

    let pid_a = pid_file_pid(&shell_a.canonical_dir());
    let pid_b = pid_file_pid(&shell_b.canonical_dir());
    let pid_state = pid_file_pid(&home.join(".local/state/anvil"));
    let live_pids: Vec<u32> = [pid_a, pid_b, pid_state].into_iter().flatten().collect();
    assert_eq!(
        live_pids.len(),
        1,
        "exactly one PID file across both runtime dirs and state-home: a={pid_a:?} b={pid_b:?} state={pid_state:?}"
    );
    let pid = live_pids[0];
    assert!(
        anvil_intercept_pid_alive(pid),
        "the single started daemon is alive"
    );

    let status_a = stdout_of(&shell_a.status_human());
    let status_b = stdout_of(&shell_b.status_human());
    assert!(
        status_a.contains(&format!("pid {pid}")),
        "shell A status must name the shared daemon:\n{status_a}"
    );
    assert!(
        status_b.contains(&format!("pid {pid}")),
        "shell B status must name the shared daemon:\n{status_b}"
    );
}
