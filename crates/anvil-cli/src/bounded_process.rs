//! Bounded subprocess capture for synchronous CLI callers, including callers
//! already inside a Tokio runtime. No captured data or command arguments appear
//! in failure messages.

use std::io;
use std::process::{Command, Output, Stdio};
use std::time::Instant;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};

async fn read_capped(reader: impl AsyncRead + Unpin, cap: usize) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(cap as u64 + 1).read_to_end(&mut bytes).await?;
    if bytes.len() > cap {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "subprocess output cap exceeded",
        ));
    }
    Ok(bytes)
}

/// Drain both output pipes while feeding stdin and waiting for exit. The same
/// absolute deadline covers all four operations. Failure drops the pipe futures
/// before killing and reaping the direct child, so inherited pipes cannot make
/// cleanup wait for EOF. The input is owned by the caller and must be bounded.
pub(crate) fn output_until(
    command: Command,
    input: Vec<u8>,
    deadline: Instant,
    stdout_cap: usize,
    stderr_cap: usize,
) -> io::Result<Output> {
    std::thread::Builder::new()
        .name("anvil-bounded-process".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            runtime.block_on(async move {
                if Instant::now() >= deadline {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "subprocess deadline exceeded",
                    ));
                }
                let mut command = tokio::process::Command::from(command);
                let mut child = command
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .kill_on_drop(true)
                    .spawn()?;
                let mut stdin = child.stdin.take().expect("piped stdin");
                let stdout = child.stdout.take().expect("piped stdout");
                let stderr = child.stderr.take().expect("piped stderr");
                let result = tokio::time::timeout_at(deadline.into(), async {
                    tokio::try_join!(
                        async move {
                            stdin.write_all(&input).await?;
                            stdin.shutdown().await
                        },
                        read_capped(stdout, stdout_cap),
                        read_capped(stderr, stderr_cap),
                        child.wait(),
                    )
                })
                .await;
                match result {
                    Ok(Ok(((), stdout, stderr, status))) => Ok(Output {
                        status,
                        stdout,
                        stderr,
                    }),
                    failure => {
                        // kill() also waits/reaps. If the child already exited,
                        // wait() below is harmless and preserves cleanup on errors.
                        let _ = child.kill().await;
                        let _ = child.wait().await;
                        match failure {
                            Ok(Err(error)) => Err(error),
                            Err(_) => Err(io::Error::new(
                                io::ErrorKind::TimedOut,
                                "subprocess deadline exceeded",
                            )),
                            Ok(Ok(_)) => unreachable!(),
                        }
                    }
                }
            })
        })?
        .join()
        .map_err(|_| io::Error::other("subprocess capture worker failed"))?
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::time::Duration;

    fn shell(script: &str) -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", script]);
        command
    }

    #[test]
    fn drains_both_full_pipes_before_waiting() {
        let output = output_until(
            shell("head -c 262144 /dev/zero; head -c 262144 /dev/zero >&2"),
            vec![],
            Instant::now() + Duration::from_secs(3),
            300_000,
            300_000,
        )
        .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout.len(), 262_144);
        assert_eq!(output.stderr.len(), 262_144);
    }

    #[test]
    fn both_stream_caps_fail_without_echoing_contents() {
        for script in [
            "printf secret; exec sleep 10",
            "printf secret >&2; exec sleep 10",
        ] {
            let started = Instant::now();
            let error = output_until(
                shell(script),
                vec![],
                started + Duration::from_secs(3),
                4,
                4,
            )
            .unwrap_err();
            assert_eq!(error.to_string(), "subprocess output cap exceeded");
            assert!(started.elapsed() < Duration::from_secs(2));
        }
    }

    #[test]
    fn timeout_reaps_child_even_with_inherited_pipe() {
        let dir = tempfile::tempdir().unwrap();
        let pid_file = dir.path().join("pid");
        let mut command = shell("echo $ > \"$1\"; sleep 2 & wait");
        command.arg("fixture").arg(&pid_file);
        let started = Instant::now();
        let error = output_until(
            command,
            vec![],
            started + Duration::from_millis(300),
            1024,
            1024,
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(1));
        let pid = std::fs::read_to_string(pid_file).unwrap();
        assert!(
            !Command::new("kill")
                .args(["-0", pid.trim()])
                .stderr(Stdio::null())
                .status()
                .unwrap()
                .success()
        );
    }
}
