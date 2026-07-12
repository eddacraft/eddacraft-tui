use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;

use anvil_dashboard_server::{ServerError, ensure_loopback, serve};
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

#[tokio::test]
async fn health_and_openapi_are_read_only() {
    let workspace = tempdir().expect("workspace");
    let health = request(
        workspace.path(),
        "GET /healthz HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n",
    )
    .await;
    assert!(health.starts_with("HTTP/1.1 200 OK"), "{health}");
    let (_, body) = health.split_once("\r\n\r\n").expect("health response body");
    let health_json: serde_json::Value = serde_json::from_str(body).expect("health json");
    assert_eq!(health_json["status"], "ok");
    assert_eq!(health_json["access"], "read-only");

    let openapi = request(
        workspace.path(),
        "GET /openapi.json HTTP/1.1\r\nHost: localhost:{port}\r\nConnection: close\r\n\r\n",
    )
    .await;
    assert!(openapi.starts_with("HTTP/1.1 200 OK"), "{openapi}");
    let (_, body) = openapi
        .split_once("\r\n\r\n")
        .expect("openapi response body");
    let document: serde_json::Value = serde_json::from_str(body).expect("openapi json");
    assert_eq!(document["openapi"], "3.1.0");
    assert!(document["paths"]["/api/v1/protection"]["get"].is_object());
    assert!(document["paths"]["/api/v1/plans"]["get"].is_object());

    let mutation = request(
        workspace.path(),
        "POST /api/v1/protection HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    )
    .await;
    assert!(
        mutation.starts_with("HTTP/1.1 405 Method Not Allowed"),
        "{mutation}"
    );
}

#[tokio::test]
async fn rejects_dns_rebinding_hosts() {
    let workspace = tempdir().expect("workspace");
    let response = request(
        workspace.path(),
        "GET /healthz HTTP/1.1\r\nHost: attacker.example\r\nConnection: close\r\n\r\n",
    )
    .await;

    assert!(
        response.starts_with("HTTP/1.1 421 Misdirected Request"),
        "{response}"
    );
}

#[tokio::test]
async fn rejects_hostile_browser_origins_and_fetch_sites() {
    let workspace = tempdir().expect("workspace");
    let hostile_origin = request(
        workspace.path(),
        "GET /healthz HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: https://attacker.example\r\nSec-Fetch-Site: cross-site\r\nConnection: close\r\n\r\n",
    )
    .await;
    assert!(
        hostile_origin.starts_with("HTTP/1.1 403 Forbidden"),
        "{hostile_origin}"
    );

    let forged_same_origin = request(
        workspace.path(),
        "GET /healthz HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: http://localhost:{port}\r\nSec-Fetch-Site: same-origin\r\nConnection: close\r\n\r\n",
    )
    .await;
    assert!(
        forged_same_origin.starts_with("HTTP/1.1 403 Forbidden"),
        "{forged_same_origin}"
    );

    let same_origin = request(
        workspace.path(),
        "GET /healthz HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: http://127.0.0.1:{port}\r\nSec-Fetch-Site: same-origin\r\nConnection: close\r\n\r\n",
    )
    .await;
    assert!(same_origin.starts_with("HTTP/1.1 200 OK"), "{same_origin}");
}

#[test]
fn only_loopback_addresses_are_accepted() {
    let local = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let public = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);

    assert!(ensure_loopback(local).is_ok());
    assert!(ensure_loopback(public).is_err());
}

#[tokio::test]
async fn serving_rejects_a_non_loopback_listener() {
    let workspace = tempdir().expect("workspace");
    let listener = tokio::net::TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0))
        .await
        .expect("listener");

    let error = serve(listener, workspace.path())
        .await
        .expect_err("non-loopback listeners must fail closed");

    assert!(matches!(error, ServerError::NonLoopback));
}
