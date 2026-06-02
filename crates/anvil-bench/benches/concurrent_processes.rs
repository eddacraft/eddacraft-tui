//! Concurrent multi-process resource bench (RLB-005).
//!
//! Each Anvil process caps its own rayon pool at N/2 cores, so running several
//! at once can oversubscribe the box — a cost no single-process bench can see.
//! This bench runs all three long-running processes **together under load** —
//! `anvil watch` (with file churn driving per-save checks), the intercept
//! daemon (IPC load), and the MCP server (tools/call load) — and measures their
//! combined process tree against [`ResourceBudget::ANVIL_CONCURRENT_ALL_V1`].
//!
//! Like the watch bench, it needs a quiet box with inotify headroom: `watch`
//! refuses to start once the host's `fs.inotify` watch limit is exhausted, and
//! the bench reports that as a loud error rather than a misleadingly-low
//! aggregate.

#[cfg(unix)]
mod unix_bench {
    use std::error::Error;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixStream;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    use anvil_bench::budget::{BudgetVerdict, ResourceBudget, evaluate};
    use anvil_bench::fixture::{RepoSpec, generate_repo};
    use anvil_bench::proc_sampler::MultiTreeSampler;
    use anvil_bench::spawn::{ManagedChild, resolve_anvil_binary};

    type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

    const SETTLE: Duration = Duration::from_secs(2);
    const MEASURE_WINDOW: Duration = Duration::from_secs(5);
    const SAMPLE_INTERVAL: Duration = Duration::from_millis(200);
    const CHURN_INTERVAL: Duration = Duration::from_millis(250);
    const SOCKET_WAIT: Duration = Duration::from_secs(10);

    pub fn main() {
        let exit = match run() {
            Ok(verdict) => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&verdict).expect("verdict serialises")
                );
                i32::from(verdict.status.is_fail())
            }
            Err(err) => {
                eprintln!("concurrent_processes: {err}");
                1
            }
        };
        std::process::exit(exit);
    }

    fn run() -> Result<BudgetVerdict> {
        if !cfg!(target_os = "linux") {
            return Err("concurrent resource bench requires Linux /proc".into());
        }
        let bin = resolve_anvil_binary()?;

        let workdir = tempfile::tempdir()?;
        let repo = generate_repo(&RepoSpec::small(), workdir.path())?;
        let churn_files = collect_churnable_files(repo.root());
        if churn_files.is_empty() {
            return Err("synthetic repo produced no churnable source files".into());
        }

        // --- intercept daemon (private ANVIL_HOME) ---
        let home = tempfile::tempdir()?;
        std::fs::set_permissions(home.path(), std::fs::Permissions::from_mode(0o700))?;
        let socket = home.path().join("intercept.sock");
        let mut daemon = ManagedChild::new(
            Command::new(&bin)
                .args(["intercept", "start", "--foreground"])
                .env("ANVIL_HOME", home.path())
                .env("ANVIL_DEV", "1")
                .env("ANVIL_DISABLE_UPDATE_HINT", "1")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?,
            "anvil intercept start",
        );
        wait_for_socket(&socket, SOCKET_WAIT)
            .inspect_err(|_| drop(daemon.ensure_running("while waiting for the IPC socket")))?;

        // --- MCP server (cwd = repo) ---
        let mut mcp_child = Command::new(&bin)
            .args(["mcp", "serve", "--stdio"])
            .current_dir(repo.root())
            .env("ANVIL_DEV", "1")
            .env("ANVIL_DISABLE_UPDATE_HINT", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let mut mcp_stdin = mcp_child.stdin.take().ok_or("mcp stdin not piped")?;
        let mut mcp_reader = BufReader::new(mcp_child.stdout.take().ok_or("mcp stdout not piped")?);
        let mut mcp = ManagedChild::new(mcp_child, "anvil mcp serve");
        mcp_handshake(&mut mcp_stdin, &mut mcp_reader)?;

        // --- watch (cwd = repo, default check action) ---
        let mut watch = ManagedChild::new(
            Command::new(&bin)
                .args(["--json", "--no-tui", "watch", "--all", "--debounce=100"])
                .current_dir(repo.root())
                .env("ANVIL_DEV", "1")
                .env("ANVIL_DISABLE_UPDATE_HINT", "1")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?,
            "anvil watch",
        );

        let roots = vec![daemon.id(), mcp.id(), watch.id()];

        std::thread::sleep(SETTLE);
        daemon.ensure_running("after settle")?;
        mcp.ensure_running("after settle")?;
        watch.ensure_running("after settle (inotify watch limit exhausted?)")?;

        // --- drive all three concurrently ---
        let stop = Arc::new(AtomicBool::new(false));
        let intercept_driver = spawn_intercept_load(socket.clone(), Arc::clone(&stop));
        let mcp_driver = spawn_mcp_load(mcp_stdin, mcp_reader, Arc::clone(&stop));
        let churn_driver = spawn_churn(churn_files, Arc::clone(&stop));

        let mut sampler = MultiTreeSampler::start(roots)?;
        sampler.sample_for(MEASURE_WINDOW, SAMPLE_INTERVAL);
        stop.store(true, Ordering::Relaxed);
        let _ = intercept_driver.join();
        let _ = mcp_driver.join();
        let _ = churn_driver.join();
        let sample = sampler.finish()?;

        daemon.ensure_running("after measurement window")?;
        mcp.ensure_running("after measurement window")?;
        watch.ensure_running("after measurement window")?;

        daemon.shutdown();
        mcp.shutdown();
        watch.shutdown();

        Ok(evaluate(ResourceBudget::ANVIL_CONCURRENT_ALL_V1, sample))
    }

    fn spawn_intercept_load(socket: PathBuf, stop: Arc<AtomicBool>) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                // Connection-churn load over the IPC pipeline; per-request errors
                // during teardown are not measurement failures.
                let _ = session_list(&socket);
            }
        })
    }

    fn spawn_mcp_load(
        mut stdin: std::process::ChildStdin,
        mut reader: BufReader<std::process::ChildStdout>,
        stop: Arc<AtomicBool>,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let mut seq: u64 = 1;
            while !stop.load(Ordering::Relaxed) {
                if mcp_tool_call(&mut stdin, &mut reader, seq).is_err() {
                    break;
                }
                seq += 1;
            }
        })
    }

    fn spawn_churn(files: Vec<PathBuf>, stop: Arc<AtomicBool>) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let mut cursor = 0usize;
            let mut tick: u64 = 0;
            while !stop.load(Ordering::Relaxed) {
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .append(true)
                    .open(&files[cursor % files.len()])
                {
                    let _ = writeln!(f, "// churn {tick}");
                }
                cursor = cursor.wrapping_add(1);
                tick = tick.wrapping_add(1);
                std::thread::sleep(CHURN_INTERVAL);
            }
        })
    }

    fn session_list(socket: &Path) -> Result<()> {
        let mut stream = UnixStream::connect(socket)?;
        stream
            .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"session.list\",\"id\":\"bench\"}\n")?;
        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        reader.read_line(&mut response)?;
        Ok(())
    }

    fn mcp_handshake(stdin: &mut impl Write, reader: &mut impl BufRead) -> Result<()> {
        send(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0", "id": "init", "method": "initialize",
                "params": { "protocolVersion": "2024-11-05", "capabilities": {} }
            }),
        )?;
        let init = read_line(reader)?;
        if !init.contains("\"result\"") {
            return Err(format!("mcp initialize did not return a result: {init}").into());
        }
        send(
            stdin,
            &serde_json::json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
        )
    }

    fn mcp_tool_call(stdin: &mut impl Write, reader: &mut impl BufRead, seq: u64) -> Result<()> {
        send(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0", "id": seq, "method": "tools/call",
                "params": {
                    "name": "anvil_validate_write",
                    "arguments": {
                        "path": format!("src/bench_{seq}.ts"),
                        "operation": "create",
                        "proposedContent": "export const value = 1;\n"
                    }
                }
            }),
        )?;
        let response = read_line(reader)?;
        if response.contains("\"result\"") {
            Ok(())
        } else {
            Err(format!("tools/call {seq} returned no result").into())
        }
    }

    fn send(stdin: &mut impl Write, message: &serde_json::Value) -> Result<()> {
        let mut line = serde_json::to_vec(message)?;
        line.push(b'\n');
        stdin.write_all(&line)?;
        stdin.flush()?;
        Ok(())
    }

    fn read_line(reader: &mut impl BufRead) -> Result<String> {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Err("mcp server closed stdout".into());
        }
        Ok(line)
    }

    fn collect_churnable_files(root: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                match entry.file_type() {
                    Ok(ft) if ft.is_dir() => stack.push(path),
                    Ok(ft) if ft.is_file() => {
                        if matches!(
                            path.extension().and_then(|e| e.to_str()),
                            Some("ts" | "js" | "rs")
                        ) {
                            out.push(path);
                        }
                    }
                    _ => {}
                }
            }
        }
        out.sort();
        out
    }

    fn wait_for_socket(socket: &Path, timeout: Duration) -> Result<()> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if socket.exists() && UnixStream::connect(socket).is_ok() {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Err(format!(
            "intercept socket {} did not become connectable within {timeout:?}",
            socket.display()
        )
        .into())
    }
}

#[cfg(unix)]
fn main() {
    unix_bench::main();
}

#[cfg(not(unix))]
fn main() {
    println!("concurrent_processes is implemented for Unix only");
}
