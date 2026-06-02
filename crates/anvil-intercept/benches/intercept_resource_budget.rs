//! Intercept daemon CPU/RSS budget bench (RLB-003).
//!
//! The intercept daemon (`anvil intercept start --foreground`) is one of three
//! long-running Anvil processes; before this bench only `watch` had any
//! resource coverage and only latency benches (`ipc_roundtrip`,
//! `midedit_roundtrip`) touched the daemon. This adds a **resource budget**:
//!
//! - **idle steady-state** — the daemon running with no in-flight requests.
//!   A daemon that idles hot is the bug class this whole module exists to
//!   catch (GH #2156). Gated by [`ResourceBudget::ANVIL_INTERCEPT_IDLE_V1`].
//! - **burst** — many short-lived connections each driving one JSON-RPC
//!   request through the full accept → auth → parse → dispatch → serialise
//!   pipeline. Gated by [`ResourceBudget::ANVIL_INTERCEPT_BURST_V1`].
//!
//! It spawns the *real shipped daemon* (not an in-process listener) under a
//! private `ANVIL_HOME`, so the socket/PID land in a throwaway dir and the
//! measurement reflects the actual binary. `scan_buffer` load is out of scope
//! here — it needs a registered, peer-PID-authenticated session; the burst
//! drives the unauthenticated `session.list` verb, which still exercises the
//! whole per-request IPC pipeline. The mid-edit scan latency path is covered
//! by `midedit_roundtrip`.

#[cfg(unix)]
mod unix_bench {
    use std::error::Error;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    use anvil_bench::budget::{BudgetVerdict, ResourceBudget, evaluate};
    use anvil_bench::proc_sampler::TreeSampler;
    use anvil_bench::spawn::{ManagedChild, in_new_process_group, resolve_anvil_binary};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixStream;

    type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

    const SETTLE: Duration = Duration::from_secs(2);
    const IDLE_WINDOW: Duration = Duration::from_secs(3);
    const BURST_WINDOW: Duration = Duration::from_secs(4);
    const SAMPLE_INTERVAL: Duration = Duration::from_millis(200);
    const BURST_WORKERS: usize = 4;
    const SOCKET_WAIT: Duration = Duration::from_secs(10);

    pub fn main() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let exit = runtime.block_on(async {
            match run().await {
                Ok(verdicts) => {
                    // Emit a single JSON object {"idle": {...}, "burst": {...}}
                    // so the CI artifact is machine-parseable (not two labelled
                    // blocks concatenated).
                    let mut failed = false;
                    let mut obj = serde_json::Map::new();
                    for (name, verdict) in &verdicts {
                        let key = name.rsplit('.').next().unwrap_or(name).to_string();
                        obj.insert(
                            key,
                            serde_json::to_value(verdict).expect("verdict to value"),
                        );
                        failed |= verdict.status.is_fail();
                    }
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::Value::Object(obj))
                            .expect("verdicts serialise")
                    );
                    i32::from(failed)
                }
                Err(err) => {
                    eprintln!("intercept_resource_budget: {err}");
                    1
                }
            }
        });
        std::process::exit(exit);
    }

    async fn run() -> Result<Vec<(&'static str, BudgetVerdict)>> {
        let bin = resolve_anvil_binary()?;

        // Private ANVIL_HOME → the daemon binds <home>/intercept.sock and writes
        // its PID there, isolated from any production daemon. Must be owner-only
        // (0700) to satisfy the daemon's secure-runtime-dir check.
        let home = tempfile::tempdir()?;
        std::fs::set_permissions(home.path(), std::fs::Permissions::from_mode(0o700))?;
        let socket = home.path().join("intercept.sock");

        let mut command = Command::new(&bin);
        command
            .args(["intercept", "start", "--foreground"])
            .env("ANVIL_HOME", home.path())
            .env("ANVIL_DEV", "1")
            .env("ANVIL_DISABLE_UPDATE_HINT", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let child = in_new_process_group(&mut command).spawn()?;
        let mut daemon = ManagedChild::new(child, "anvil intercept start");
        let pid = daemon.id();

        if let Err(socket_err) = wait_for_socket(&socket, SOCKET_WAIT).await {
            // Surface a startup crash as the cause rather than a bare timeout.
            if let Err(crash) = daemon.ensure_running("while waiting for the IPC socket") {
                eprintln!("intercept_resource_budget: daemon startup failure: {crash}");
            }
            return Err(socket_err);
        }

        // Settle past the cold start, then confirm liveness before each measure.
        tokio::time::sleep(SETTLE).await;
        daemon.ensure_running("after settle")?;
        sanity_request(&socket).await?;

        let idle = measure(pid, IDLE_WINDOW, None).await?;
        daemon.ensure_running("after idle window")?;

        let burst = measure(pid, BURST_WINDOW, Some(socket.clone())).await?;
        daemon.ensure_running("after burst window")?;

        daemon.shutdown();

        Ok(vec![
            (
                "intercept.idle",
                evaluate(ResourceBudget::ANVIL_INTERCEPT_IDLE_V1, idle),
            ),
            (
                "intercept.burst",
                evaluate(ResourceBudget::ANVIL_INTERCEPT_BURST_V1, burst),
            ),
        ])
    }

    /// Measure the daemon's process tree over `window`. When `load_socket` is
    /// `Some`, hammer the socket with concurrent `session.list` requests for the
    /// whole window; when `None`, measure the idle daemon.
    async fn measure(
        pid: u32,
        window: Duration,
        load_socket: Option<PathBuf>,
    ) -> Result<anvil_bench::budget::MeasurementSample> {
        let stop = Arc::new(AtomicBool::new(false));
        let mut workers = Vec::new();
        if let Some(socket) = load_socket {
            for _ in 0..BURST_WORKERS {
                let socket = socket.clone();
                let stop = Arc::clone(&stop);
                workers.push(tokio::spawn(async move {
                    while !stop.load(Ordering::Acquire) {
                        // Ignore per-request errors (a connection refused during
                        // teardown is not a measurement failure); the loop keeps
                        // the daemon under load for the whole window.
                        let _ = session_list(&socket).await;
                    }
                }));
            }
        }

        // The sampler is blocking (sleep + /proc reads); run it off the async
        // workers so the load tasks keep the daemon busy concurrently.
        let sample = tokio::task::spawn_blocking(move || {
            let mut sampler = TreeSampler::start(pid)?;
            sampler.sample_for(window, SAMPLE_INTERVAL);
            sampler.finish()
        })
        .await
        .map_err(|e| -> Box<dyn Error + Send + Sync> {
            format!("sampler task join: {e}").into()
        })??;

        // Stop the load, then abort the tasks so a worker blocked on a wedged
        // daemon cannot hang the join past the window (abort cancels at the next
        // .await point; session_list's awaits are cancellation-safe).
        stop.store(true, Ordering::Release);
        for worker in &workers {
            worker.abort();
        }
        for worker in workers {
            let _ = worker.await;
        }
        Ok(sample)
    }

    /// One `session.list` round-trip. Each call uses a fresh connection — the
    /// same connection-churn shape as a real client and as `ipc_roundtrip`.
    async fn session_list(socket: &Path) -> Result<String> {
        let stream = UnixStream::connect(socket).await?;
        let mut client = BufReader::new(stream);
        client
            .get_mut()
            .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"session.list\",\"id\":\"bench\"}\n")
            .await?;
        let mut response = String::new();
        client.read_line(&mut response).await?;
        Ok(response)
    }

    /// Confirm the daemon answers before measuring, so a wedged daemon is a
    /// loud error rather than a misleadingly-idle measurement.
    async fn sanity_request(socket: &Path) -> Result<()> {
        let response = session_list(socket).await?;
        if !response_has_result(&response) {
            return Err(format!("daemon did not return a result: {response}").into());
        }
        Ok(())
    }

    /// A JSON-RPC reply is a success iff it parses with a top-level `result` and
    /// no `error` — robust against an error message that mentions "result".
    fn response_has_result(line: &str) -> bool {
        serde_json::from_str::<serde_json::Value>(line)
            .ok()
            .is_some_and(|v| v.get("result").is_some() && v.get("error").is_none())
    }

    async fn wait_for_socket(socket: &Path, timeout: Duration) -> Result<()> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if socket.exists() && UnixStream::connect(socket).await.is_ok() {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
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
    // Exit non-zero so a non-Unix CI run is a visible "not measured", never a
    // silent pass.
    eprintln!("intercept_resource_budget is implemented for Unix socket IPC only");
    std::process::exit(1);
}
