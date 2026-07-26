//! The embedded dashboard UI bundle.
//!
//! `build.rs` compiles `apps/dashboard/dist` into [`ASSETS`]. When the SPA was
//! not built before the crate, the table is empty and the server says so
//! plainly instead of pretending the UI exists — the same honest-data-state
//! posture the API takes for absent workspace artefacts.

/// One file from the built SPA.
pub(crate) struct Asset {
    /// Slash-separated path relative to the bundle root, as requested.
    pub(crate) path: &'static str,
    pub(crate) content_type: &'static str,
    pub(crate) bytes: &'static [u8],
}

include!(concat!(env!("OUT_DIR"), "/dashboard_assets.rs"));

/// The SPA shell, served for every client-side route.
pub(crate) const INDEX: &str = "index.html";

/// Whether this binary carries a UI bundle.
pub fn is_bundled() -> bool {
    !ASSETS.is_empty()
}

/// Look up an asset by its request path.
pub(crate) fn get(path: &str) -> Option<&'static Asset> {
    ASSETS.iter().find(|asset| asset.path == path)
}

/// Whether a request path should resolve to the SPA shell when it matches no
/// asset.
///
/// A client-side route (`/gates/123`) must serve the shell so the router can
/// take over. A missing *file* (`/assets/index-abc.js`) must not: answering a
/// script request with HTML produces a console parse error that reads like an
/// application bug rather than a missing asset. The last path segment carrying
/// an extension is the signal.
pub(crate) fn is_client_route(path: &str) -> bool {
    !path
        .rsplit('/')
        .next()
        .is_some_and(|segment| segment.contains('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_routes_are_extensionless() {
        assert!(is_client_route(""));
        assert!(is_client_route("gates"));
        assert!(is_client_route("gates/123"));
        assert!(is_client_route("warnings/breakdown"));
    }

    #[test]
    fn asset_requests_are_not_client_routes() {
        assert!(!is_client_route("index.html"));
        assert!(!is_client_route("assets/index-abc123.js"));
        assert!(!is_client_route("favicon.ico"));
    }

    #[test]
    fn embedded_bundle_is_self_consistent() {
        // Holds whether or not a bundle was built: an empty table trivially
        // satisfies it, and a populated one must carry a shell to fall back to.
        if is_bundled() {
            assert!(get(INDEX).is_some(), "a bundle must contain {INDEX}");
            for asset in ASSETS {
                assert!(!asset.path.starts_with('/'), "paths are relative");
                assert!(!asset.content_type.is_empty(), "every asset is typed");
            }
        }
    }
}
