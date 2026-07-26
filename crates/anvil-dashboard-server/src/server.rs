use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use axum::extract::{Path as AxumPath, Request, State};
use axum::http::StatusCode;
use axum::http::header::{
    CACHE_CONTROL, CONTENT_TYPE, HOST, HeaderName, HeaderValue, ORIGIN, X_CONTENT_TYPE_OPTIONS,
};
use axum::http::uri::{Authority, Uri};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use tokio::net::TcpListener;

use crate::Workspace;
use crate::api::{HealthResponse, PatternCatalogue, PlanDetail, PlanSummary, ProtectionOverview};
use crate::assets::{self, Asset};
use crate::capabilities::patterns::load_pattern_catalogue;
use crate::capabilities::plans::{load_plan, load_plans};
use crate::capabilities::protection::load_protection_overview;
use crate::error::{ApiError, ServerError};
use crate::openapi::openapi_document;

#[derive(Clone)]
struct AppState {
    workspace: Arc<Workspace>,
}

fn app(root: impl AsRef<Path>) -> Result<Router, ServerError> {
    let state = AppState {
        workspace: Arc::new(Workspace::new(root)?),
    };
    Ok(Router::new()
        .route("/healthz", get(health))
        .route("/openapi.json", get(openapi))
        .route("/api/v1/protection", get(protection))
        .route("/api/v1/patterns", get(patterns))
        .route("/api/v1/plans", get(plans))
        .route("/api/v1/plans/{id}", get(plan))
        // The UI is served from the same origin as the API so the loopback
        // host/origin guard covers both, and the browser needs no CORS grant.
        .fallback(ui)
        .layer(axum::middleware::from_fn(loopback_host_guard))
        .with_state(state))
}

/// Serve the embedded dashboard UI.
///
/// Runs only for paths no API route claimed.
async fn ui(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    // Unmatched API surface stays JSON. Answering a programmatic request with
    // the HTML shell would turn "this endpoint does not exist" into a parse
    // error somewhere else entirely. The discriminator is the first segment, so
    // a near-miss like `/healthz/extra` is still answered as API.
    let root_segment = path.split('/').next().unwrap_or_default();
    if matches!(root_segment, "api" | "healthz" | "openapi.json") {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "code": "not-found",
                "message": "no such dashboard endpoint"
            })),
        )
            .into_response();
    }

    if let Some(asset) = assets::get(path) {
        return asset_response(asset);
    }
    // Client-side routes resolve to the shell so the router can take over on
    // a deep link or a refresh.
    if assets::is_client_route(path)
        && let Some(shell) = assets::get(assets::INDEX)
    {
        return asset_response(shell);
    }
    if assets::is_bundled() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "code": "asset-not-found",
                "message": "no such dashboard asset"
            })),
        )
            .into_response();
    }
    ui_not_bundled()
}

fn asset_response(asset: &Asset) -> Response {
    ([(CONTENT_TYPE, asset.content_type)], asset.bytes).into_response()
}

/// The honest answer when this binary carries no UI bundle.
///
/// A development build made without `pnpm --filter @eddacraft/anvil-dashboard
/// build` still serves the full API; only the UI is absent. Saying so beats a
/// bare 404, which reads as a broken install.
fn ui_not_bundled() -> Response {
    const BODY: &str = concat!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">",
        "<title>anvil dashboard</title></head><body>",
        "<h1>Dashboard UI not bundled</h1>",
        "<p>This build of <code>anvil</code> was compiled without the dashboard ",
        "UI assets. The read-only API is still available at ",
        "<code>/api/v1/protection</code>.</p>",
        "<p>To bundle the UI from a repository checkout, build the app and then ",
        "rebuild the binary:</p>",
        "<pre>pnpm --filter @eddacraft/anvil-dashboard build\ncargo build -p eddacraft-anvil</pre>",
        "</body></html>",
    );
    (
        StatusCode::SERVICE_UNAVAILABLE,
        [(CONTENT_TYPE, "text/html; charset=utf-8")],
        BODY,
    )
        .into_response()
}

async fn loopback_host_guard(request: Request, next: Next) -> Response {
    let host = request
        .headers()
        .get(HOST)
        .and_then(|host| host.to_str().ok())
        .and_then(|host| host.parse::<Authority>().ok());
    let is_loopback_host = host.as_ref().is_some_and(|authority| {
        let host = authority.host();
        host == "127.0.0.1"
            || host == "::1"
            || host == "[::1]"
            || host.eq_ignore_ascii_case("localhost")
    });
    let origin_allowed = host
        .as_ref()
        .is_some_and(|authority| browser_origin_is_allowed(&request, authority));
    let fetch_site_allowed = request
        .headers()
        .get(HeaderName::from_static("sec-fetch-site"))
        .and_then(|value| value.to_str().ok())
        .is_none_or(|site| matches!(site, "same-origin" | "none"));

    let mut response = if !is_loopback_host {
        (
            StatusCode::MISDIRECTED_REQUEST,
            Json(serde_json::json!({
                "code": "loopback-host-required",
                "message": "dashboard requests must use a loopback Host header"
            })),
        )
            .into_response()
    } else if !origin_allowed || !fetch_site_allowed {
        (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "code": "cross-origin-request-rejected",
                "message": "dashboard requests must originate from the exact loopback authority"
            })),
        )
            .into_response()
    } else {
        next.run(request).await
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

fn browser_origin_is_allowed(request: &Request, host: &Authority) -> bool {
    request
        .headers()
        .get(ORIGIN)
        .and_then(|origin| origin.to_str().ok())
        .is_none_or(|origin| {
            origin.parse::<Uri>().ok().is_some_and(|origin| {
                origin.scheme_str() == Some("http") && origin.authority() == Some(host)
            })
        })
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

async fn patterns(State(state): State<AppState>) -> Result<Json<PatternCatalogue>, ApiError> {
    let workspace = state.workspace;
    let catalogue = tokio::task::spawn_blocking(move || load_pattern_catalogue(&workspace))
        .await
        .map_err(|_| ApiError::Worker)?;
    Ok(Json(catalogue))
}
