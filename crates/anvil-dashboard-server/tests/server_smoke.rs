use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use anvil_dashboard_server::{ServerError, app, ensure_loopback, serve};
use axum::body::{Body, to_bytes};
use axum::http::header::HOST;
use axum::http::{Method, Request, StatusCode};
use tempfile::tempdir;
use tower::ServiceExt;

#[tokio::test]
async fn health_and_openapi_are_read_only() {
    let workspace = tempdir().expect("workspace");
    let router = app(workspace.path()).expect("router");

    let health = router
        .clone()
        .oneshot(
            Request::get("/healthz")
                .header(HOST, "127.0.0.1:4217")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("health response");
    assert_eq!(health.status(), StatusCode::OK);
    let body = to_bytes(health.into_body(), 64 * 1024)
        .await
        .expect("health body");
    let health_json: serde_json::Value = serde_json::from_slice(&body).expect("health json");
    assert_eq!(health_json["status"], "ok");
    assert_eq!(health_json["access"], "read-only");

    let openapi = router
        .clone()
        .oneshot(
            Request::get("/openapi.json")
                .header(HOST, "localhost:4217")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("openapi response");
    assert_eq!(openapi.status(), StatusCode::OK);
    let body = to_bytes(openapi.into_body(), 512 * 1024)
        .await
        .expect("openapi body");
    let document: serde_json::Value = serde_json::from_slice(&body).expect("openapi json");
    assert_eq!(document["openapi"], "3.1.0");
    assert!(document["paths"]["/api/v1/protection"]["get"].is_object());
    assert!(document["paths"]["/api/v1/plans"]["get"].is_object());

    let mutation = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/protection")
                .header(HOST, "[::1]:4217")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("mutation response");
    assert_eq!(mutation.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn rejects_dns_rebinding_hosts() {
    let workspace = tempdir().expect("workspace");
    let router = app(workspace.path()).expect("router");

    let response = router
        .oneshot(
            Request::get("/healthz")
                .header(HOST, "attacker.example")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::MISDIRECTED_REQUEST);
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
