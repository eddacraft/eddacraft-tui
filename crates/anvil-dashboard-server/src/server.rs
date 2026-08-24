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
use axum::routing::{MethodRouter, get};
use axum::{Json, Router};
use tokio::net::TcpListener;

use crate::Workspace;
use crate::api::{
    HealthResponse, PatternCatalogue, PlanDetail, PlanSummary, ProtectionHistory,
    ProtectionOverview,
};
use crate::assets::{self, Asset};
use crate::capabilities::history::load_protection_history;
use crate::capabilities::patterns::load_pattern_catalogue;
use crate::capabilities::plans::{load_plan, load_plans};
use crate::capabilities::protection::load_protection_overview;
use crate::error::{ApiError, ServerError};
use crate::openapi::openapi_document;

#[derive(Clone)]
struct AppState {
    workspace: Arc<Workspace>,
}

struct DashboardRoute {
    path: &'static str,
    method_router: fn() -> MethodRouter<AppState>,
}

/// The single declarative authority for dashboard HTTP route registration.
const DASHBOARD_ROUTES: &[DashboardRoute] = &[
    DashboardRoute {
        path: "/healthz",
        method_router: || get(health),
    },
    DashboardRoute {
        path: "/openapi.json",
        method_router: || get(openapi),
    },
    DashboardRoute {
        path: "/api/v1/protection",
        method_router: || get(protection),
    },
    DashboardRoute {
        path: "/api/v1/protection/history",
        method_router: || get(protection_history),
    },
    DashboardRoute {
        path: "/api/v1/patterns",
        method_router: || get(patterns),
    },
    DashboardRoute {
        path: "/api/v1/plans",
        method_router: || get(plans),
    },
    DashboardRoute {
        path: "/api/v1/plans/{id}",
        method_router: || get(plan),
    },
];

/// Every concrete HTTP path registered by the dashboard server.
pub fn dashboard_route_paths() -> impl ExactSizeIterator<Item = &'static str> {
    DASHBOARD_ROUTES.iter().map(|route| route.path)
}

fn app(root: impl AsRef<Path>) -> Result<Router, ServerError> {
    let state = AppState {
        workspace: Arc::new(Workspace::new(root)?),
    };
    let router = DASHBOARD_ROUTES
        .iter()
        .fold(Router::new(), |router, route| {
            router.route(route.path, (route.method_router)())
        });
    Ok(router
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
    // Vite fingerprints everything under `assets/` with a content hash, so a
    // given URL's bytes can never change — cache it hard. Everything else,
    // the app shell above all, stays `no-store`: `index.html` keeps a stable
    // URL and is what points at the new hashes after an upgrade, so caching it
    // would pin the browser to the previous build's assets.
    if asset.path.starts_with("assets/") {
        return (
            [
                (CONTENT_TYPE, asset.content_type),
                (CACHE_CONTROL, "public, max-age=31536000, immutable"),
            ],
            asset.bytes,
        )
            .into_response();
    }
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
    // `no-store` is the default for everything — API payloads are workspace
    // state and must never persist. A handler may opt out by setting its own
    // `Cache-Control` first (only the content-hashed UI assets do), so this
    // fills in rather than overwrites.
    if !headers.contains_key(CACHE_CONTROL) {
        headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
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

async fn protection_history(
    State(state): State<AppState>,
) -> Result<Json<ProtectionHistory>, ApiError> {
    let workspace = state.workspace;
    let history = tokio::task::spawn_blocking(move || load_protection_history(&workspace))
        .await
        .map_err(|_| ApiError::Worker)?;
    Ok(Json(history))
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    use super::*;
    use syn::visit::{self, Visit};

    const ROUTER_COMPOSITION_METHODS: &[&str] =
        &["route", "nest", "route_service", "nest_service", "merge"];

    #[derive(Default)]
    struct RouterCompositionVisitor {
        method_calls: Vec<String>,
        function_paths: Vec<String>,
        macro_tokens: Vec<String>,
        includes: usize,
    }

    fn collect_macro_composition(tokens: proc_macro2::TokenStream, methods: &mut Vec<String>) {
        for token in tokens {
            match token {
                proc_macro2::TokenTree::Ident(identifier)
                    if ROUTER_COMPOSITION_METHODS.contains(&identifier.to_string().as_str()) =>
                {
                    methods.push(identifier.to_string());
                }
                proc_macro2::TokenTree::Group(group) => {
                    collect_macro_composition(group.stream(), methods);
                }
                _ => {}
            }
        }
    }

    impl<'ast> Visit<'ast> for RouterCompositionVisitor {
        fn visit_expr_method_call(&mut self, expression: &'ast syn::ExprMethodCall) {
            let method = expression.method.to_string();
            if ROUTER_COMPOSITION_METHODS.contains(&method.as_str()) {
                self.method_calls.push(method);
            }
            visit::visit_expr_method_call(self, expression);
        }

        fn visit_expr_path(&mut self, expression: &'ast syn::ExprPath) {
            if expression.qself.is_some() || expression.path.segments.len() > 1 {
                let method = expression
                    .path
                    .segments
                    .last()
                    .expect("qualified expression path has a segment")
                    .ident
                    .to_string();
                if ROUTER_COMPOSITION_METHODS.contains(&method.as_str()) {
                    self.function_paths.push(method);
                }
            }
            visit::visit_expr_path(self, expression);
        }

        fn visit_macro(&mut self, expression: &'ast syn::Macro) {
            if expression.path.is_ident("include") {
                self.includes += 1;
            }
            collect_macro_composition(expression.tokens.clone(), &mut self.macro_tokens);
            visit::visit_macro(self, expression);
        }
    }

    fn router_composition(source: &str) -> RouterCompositionVisitor {
        let syntax = syn::parse_file(source).expect("dashboard Rust source must parse");
        let mut visitor = RouterCompositionVisitor::default();
        visitor.visit_file(&syntax);
        visitor
    }

    fn rust_sources(root: &Path) -> Vec<(PathBuf, String)> {
        fn collect(root: &Path, sources: &mut Vec<(PathBuf, String)>) {
            for entry in
                std::fs::read_dir(root).expect("dashboard source directory must be readable")
            {
                let path = entry
                    .expect("dashboard source entry must be readable")
                    .path();
                if path.is_dir() {
                    collect(&path, sources);
                } else if path.extension().is_some_and(|extension| extension == "rs") {
                    let source = std::fs::read_to_string(&path)
                        .expect("dashboard Rust source must be UTF-8");
                    sources.push((path, source));
                }
            }
        }

        let mut sources = Vec::new();
        collect(root, &mut sources);
        sources.sort_by(|left, right| left.0.cmp(&right.0));
        sources
    }

    #[test]
    fn runtime_routes_match_openapi_paths_exactly() {
        let runtime_paths: Vec<_> = dashboard_route_paths().collect();
        let runtime: BTreeSet<_> = runtime_paths.iter().copied().collect();
        assert_eq!(
            runtime.len(),
            runtime_paths.len(),
            "dashboard runtime route registry contains duplicates"
        );
        let document = openapi_document();
        let openapi: BTreeSet<_> = document["paths"]
            .as_object()
            .expect("OpenAPI paths must be an object")
            .keys()
            .map(String::as_str)
            .collect();

        assert_eq!(runtime, openapi);
    }

    #[test]
    fn runtime_has_no_route_registration_outside_the_declarative_authority() {
        let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut route_registration_sites = Vec::new();

        for (path, source) in rust_sources(&source_root) {
            let composition = router_composition(&source);
            assert_eq!(
                composition.function_paths,
                Vec::<String>::new(),
                "dashboard routes must not use function-path registration in {}",
                path.display()
            );
            assert_eq!(
                composition.macro_tokens,
                Vec::<String>::new(),
                "dashboard routes must not be registered inside macro tokens in {}",
                path.display()
            );
            if composition.includes > 0 {
                assert_eq!(path, source_root.join("assets.rs"));
                assert_eq!(composition.includes, 1);
            }
            assert_eq!(
                composition
                    .method_calls
                    .iter()
                    .filter(|method| method.as_str() != "route")
                    .cloned()
                    .collect::<Vec<_>>(),
                Vec::<String>::new(),
                "dashboard router composition must be governed in {}",
                path.display()
            );
            route_registration_sites.extend(
                composition
                    .method_calls
                    .into_iter()
                    .filter(|method| method == "route")
                    .map(|_| path.clone()),
            );
        }
        assert_eq!(
            route_registration_sites.len(),
            1,
            "dashboard routes must be registered only by DASHBOARD_ROUTES"
        );
        assert_eq!(
            route_registration_sites[0],
            source_root.join("server.rs"),
            "the sole route registration must consume DASHBOARD_ROUTES"
        );
    }

    #[test]
    fn syntax_guard_detects_whitespace_and_comment_composition_bypasses() {
        for method in ROUTER_COMPOSITION_METHODS {
            let dot_source =
                format!("fn f(router: Router) {{ router.{method} /* gap */ (future); }}");
            assert_eq!(
                router_composition(&dot_source).method_calls,
                [method.to_string()]
            );
            let function_source =
                format!("fn f(router: Router) {{ Router::{method}\n(router, future); }}");
            assert_eq!(
                router_composition(&function_source).function_paths,
                [method.to_string()]
            );
        }
    }

    #[test]
    fn syntax_guard_detects_router_function_pointer_aliases() {
        let source = "fn f() { let add = Router::route; add(router, path, handler); }";
        assert_eq!(
            router_composition(source).function_paths,
            ["route".to_owned()]
        );
    }

    #[test]
    fn syntax_guard_detects_route_composition_inside_macros() {
        let source = "macro_rules! add { ($r:expr) => { $r.route(\"/future\", get(future)) }; }";
        assert_eq!(
            router_composition(source).macro_tokens,
            ["route".to_owned()]
        );
    }
}
