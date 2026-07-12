use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use axum::extract::{Path as AxumPath, Request, State};
use axum::http::StatusCode;
use axum::http::header::{CACHE_CONTROL, HOST, HeaderName, HeaderValue, X_CONTENT_TYPE_OPTIONS};
use axum::http::uri::Authority;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use tokio::net::TcpListener;

use crate::Workspace;
use crate::api::{HealthResponse, PlanDetail, PlanSummary, ProtectionOverview};
use crate::capabilities::plans::{load_plan, load_plans};
use crate::capabilities::protection::load_protection_overview;
use crate::error::{ApiError, ServerError};
use crate::openapi::openapi_document;

#[derive(Clone)]
struct AppState {
    workspace: Arc<Workspace>,
}

pub fn app(root: impl AsRef<Path>) -> Result<Router, ServerError> {
    let state = AppState {
        workspace: Arc::new(Workspace::new(root)?),
    };
    Ok(Router::new()
        .route("/healthz", get(health))
        .route("/openapi.json", get(openapi))
        .route("/api/v1/protection", get(protection))
        .route("/api/v1/plans", get(plans))
        .route("/api/v1/plans/{id}", get(plan))
        .layer(axum::middleware::from_fn(loopback_host_guard))
        .with_state(state))
}

async fn loopback_host_guard(request: Request, next: Next) -> Response {
    let is_loopback_host = request
        .headers()
        .get(HOST)
        .and_then(|host| host.to_str().ok())
        .and_then(|host| host.parse::<Authority>().ok())
        .is_some_and(|authority| {
            let host = authority.host();
            host == "127.0.0.1"
                || host == "::1"
                || host == "[::1]"
                || host.eq_ignore_ascii_case("localhost")
        });

    let mut response = if is_loopback_host {
        next.run(request).await
    } else {
        (
            StatusCode::MISDIRECTED_REQUEST,
            Json(serde_json::json!({
                "code": "loopback-host-required",
                "message": "dashboard requests must use a loopback Host header"
            })),
        )
            .into_response()
    };
    let headers = response.headers_mut();
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    headers.insert(
        HeaderName::from_static("cross-origin-resource-policy"),
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("no-referrer"),
    );
    response
}

pub fn ensure_loopback(address: SocketAddr) -> Result<(), ServerError> {
    if address.ip().is_loopback() {
        Ok(())
    } else {
        Err(ServerError::NonLoopback)
    }
}

pub async fn serve(listener: TcpListener, root: impl AsRef<Path>) -> Result<(), ServerError> {
    ensure_loopback(listener.local_addr()?)?;
    axum::serve(listener, app(root)?).await?;
    Ok(())
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse::ready())
}

async fn openapi() -> Json<serde_json::Value> {
    Json(openapi_document())
}

async fn protection(State(state): State<AppState>) -> Result<Json<ProtectionOverview>, ApiError> {
    let workspace = state.workspace;
    let overview = tokio::task::spawn_blocking(move || load_protection_overview(&workspace))
        .await
        .map_err(|_| ApiError::Worker)?;
    Ok(Json(overview))
}

async fn plans(State(state): State<AppState>) -> Result<Json<Vec<PlanSummary>>, ApiError> {
    let workspace = state.workspace;
    let plans = tokio::task::spawn_blocking(move || load_plans(&workspace))
        .await
        .map_err(|_| ApiError::Worker)??;
    Ok(Json(plans))
}

async fn plan(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<PlanDetail>, ApiError> {
    let workspace = state.workspace;
    let plan = tokio::task::spawn_blocking(move || load_plan(&workspace, &id))
        .await
        .map_err(|_| ApiError::Worker)??
        .ok_or(ApiError::PlanNotFound)?;
    Ok(Json(plan))
}
