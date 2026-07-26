//! The UI-serving half of the dashboard server.
//!
//! The embedded bundle is a build-time input, so these tests assert the
//! invariants that hold either way and branch only where the two states
//! genuinely differ. CI builds the SPA before the Rust tests, so the bundled
//! branch is the one exercised on the merge path; the unbundled branch is what
//! a Rust-only contributor sees locally.

use std::net::Ipv4Addr;
use std::path::Path;

use anvil_dashboard_server::{is_bundled, serve};
use tempfile::tempdir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn request(root: &Path, request: &str) -> String {
    let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .expect("listener");
    let address = listener.local_addr().expect("listener address");
    let root = root.to_path_buf();
    let server = tokio::spawn(async move { serve(listener, root).await });
    let mut stream = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect");
    stream
        .write_all(
            request
                .replace("{port}", &address.port().to_string())
                .as_bytes(),
        )
        .await
        .expect("write request");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .await
        .expect("read response");
    server.abort();
    response
}

async fn get(root: &Path, path: &str) -> String {
    request(
        root,
        &format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{{port}}\r\nConnection: close\r\n\r\n"),
    )
    .await
}

/// An unmatched API path must never answer with the HTML shell — that turns a
/// missing endpoint into a parse error in the caller.
#[tokio::test]
async fn unmatched_api_paths_stay_json() {
    let workspace = tempdir().expect("workspace");

    for path in ["/api/v1/nope", "/api/", "/healthz/extra"] {
        let response = get(workspace.path(), path).await;
        assert!(
            response.starts_with("HTTP/1.1 404 Not Found"),
            "{path}: {response}"
        );
        let (head, body) = response.split_once("\r\n\r\n").expect("response body");
        assert!(
            head.contains("application/json"),
            "{path} must stay JSON: {head}"
        );
        let json: serde_json::Value = serde_json::from_str(body).expect("json body");
        assert!(json["code"].is_string(), "{path}: {json}");
    }
}

/// The UI shares the API's origin, so it inherits the loopback guard rather
/// than needing a second one.
#[tokio::test]
async fn ui_requests_obey_the_loopback_guard() {
    let workspace = tempdir().expect("workspace");

    let rebinding = request(
        workspace.path(),
        "GET / HTTP/1.1\r\nHost: attacker.example\r\nConnection: close\r\n\r\n",
    )
    .await;
    assert!(
        rebinding.starts_with("HTTP/1.1 421 Misdirected Request"),
        "{rebinding}"
    );

    let cross_site = request(
        workspace.path(),
        "GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: https://attacker.example\r\nSec-Fetch-Site: cross-site\r\nConnection: close\r\n\r\n",
    )
    .await;
    assert!(
        cross_site.starts_with("HTTP/1.1 403 Forbidden"),
        "{cross_site}"
    );
}

#[tokio::test]
async fn serves_the_app_shell_when_bundled() {
    if !is_bundled() {
        return;
    }
    let workspace = tempdir().expect("workspace");

    // The root and every client-side route resolve to the shell so deep links
    // and refreshes work.
    for path in ["/", "/gates", "/gates/abc123", "/warnings/breakdown"] {
        let response = get(workspace.path(), path).await;
        assert!(
            response.starts_with("HTTP/1.1 200 OK"),
            "{path}: {response}"
        );
        assert!(
            response.contains("text/html"),
            "{path} must serve the shell: {response}"
        );
    }

    // A missing *file* is a 404, not the shell.
    let missing = get(workspace.path(), "/assets/does-not-exist.js").await;
    assert!(missing.starts_with("HTTP/1.1 404 Not Found"), "{missing}");
}

#[tokio::test]
async fn reports_an_absent_bundle_honestly() {
    if is_bundled() {
        return;
    }
    let workspace = tempdir().expect("workspace");
    let response = get(workspace.path(), "/").await;

    assert!(
        response.starts_with("HTTP/1.1 503 Service Unavailable"),
        "{response}"
    );
    assert!(
        response.contains("not bundled"),
        "the page must say why: {response}"
    );

    // The API stays fully available without the UI.
    let health = get(workspace.path(), "/healthz").await;
    assert!(health.starts_with("HTTP/1.1 200 OK"), "{health}");
}
