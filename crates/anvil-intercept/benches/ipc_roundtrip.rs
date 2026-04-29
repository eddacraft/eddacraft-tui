#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::sync::Arc;
#[cfg(unix)]
use std::time::{Duration, Instant};

#[cfg(unix)]
use anvil_intercept::Shutdown;
#[cfg(unix)]
use anvil_intercept::ipc::{IpcListener, NoopDispatcher, handle_jsonrpc_value_for_benchmark};
#[cfg(unix)]
use serde_json::json;
#[cfg(unix)]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(unix)]
use tokio::net::UnixStream;

#[cfg(unix)]
const SAMPLES: usize = 200;

#[cfg(unix)]
fn main() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let dispatcher = Arc::new(NoopDispatcher);
        let mut service_samples = Vec::with_capacity(SAMPLES);
        for index in 0..SAMPLES {
            let request = json!({
                "jsonrpc": "2.0",
                "method": "session.list",
                "id": format!("service-{index}"),
            });
            let started = Instant::now();
            let response =
                handle_jsonrpc_value_for_benchmark(request, &dispatcher).expect("service response");
            service_samples.push(started.elapsed());
            assert!(
                response.get("result").is_some(),
                "unexpected response: {response}"
            );
        }
        report_dimensions("validation.service");
        report("validation.service", &mut service_samples);

        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o700))
            .expect("secure tempdir permissions");
        let socket = tmp.path().join("intercept.sock");
        let listener = IpcListener::bind(&socket, NoopDispatcher).expect("bind listener");
        let (shutdown, token) = Shutdown::new();
        let handle = tokio::spawn(async move { listener.serve(token).await });

        let mut roundtrip_samples = Vec::with_capacity(SAMPLES);
        for index in 0..SAMPLES {
            let client = UnixStream::connect(&socket).await.expect("connect client");
            let mut client = BufReader::new(client);
            let request = format!(
                "{{\"jsonrpc\":\"2.0\",\"method\":\"session.list\",\"id\":\"bench-{index}\"}}\n"
            );
            let started = Instant::now();
            client
                .get_mut()
                .write_all(request.as_bytes())
                .await
                .expect("write request");
            let mut response = String::new();
            client
                .read_line(&mut response)
                .await
                .expect("read response");
            assert!(
                response.contains("\"result\""),
                "unexpected benchmark response: {response}"
            );
            roundtrip_samples.push(started.elapsed());
        }

        report_dimensions("validation.roundtrip");
        report("validation.roundtrip", &mut roundtrip_samples);

        shutdown.trigger();
        tokio::time::timeout(Duration::from_secs(1), handle)
            .await
            .expect("listener timeout")
            .expect("listener join")
            .expect("listener ok");
    });
}

#[cfg(unix)]
fn report_dimensions(boundary: &str) {
    println!(
        "dimensions: mode=watch boundary={boundary} surface=cli-harness contentSource=disk ruleSet=none fixtureCorpus=synthetic-spike contentSize=0 platform={} daemonState=warm driverProtocol=json-rpc-2.0 debounceMs=0",
        std::env::consts::OS,
    );
}

#[cfg(unix)]
fn report(name: &str, samples: &mut [Duration]) {
    samples.sort_unstable();
    println!(
        "{name}: samples={} p50={} p95={} p99={}",
        samples.len(),
        fmt(samples[percentile_index(samples.len(), 50)]),
        fmt(samples[percentile_index(samples.len(), 95)]),
        fmt(samples[percentile_index(samples.len(), 99)]),
    );
}

#[cfg(unix)]
fn percentile_index(len: usize, percentile: usize) -> usize {
    ((len.saturating_sub(1)) * percentile) / 100
}

#[cfg(unix)]
fn fmt(duration: Duration) -> String {
    format!("{:.3}ms", duration.as_secs_f64() * 1_000.0)
}

#[cfg(not(unix))]
fn main() {
    println!("ipc_roundtrip benchmark is currently implemented for Unix socket IPC only");
}
