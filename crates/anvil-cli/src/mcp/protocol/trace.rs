//! MCP26-009: W3C Trace Context extraction from request `_meta`.
//!
//! Valid `traceparent` values are bound onto the current tracing span via
//! [`anvil_observability`]. Malformed values are ignored (optionally logged
//! at debug). They never become authority inputs and are never forwarded to
//! subprocess environments from this module.

use anvil_observability::{TraceContext, bind_traceparent_to_current_span};
use serde_json::Value;
use tracing::{debug, field, info_span};

use super::render::ProtocolEra;

const META_TRACEPARENT: &str = "traceparent";
const META_TRACESTATE: &str = "tracestate";
const META_BAGGAGE: &str = "baggage";

/// Extract and apply W3C trace context from `params._meta`, then enter a
/// request span annotated with protocol era, version, and method.
///
/// Returns a span guard that must be held for the duration of request handling.
pub fn enter_request_span(
    method: Option<&str>,
    era: ProtocolEra,
    protocol_version: Option<&str>,
    params: Option<&Value>,
) -> tracing::span::EnteredSpan {
    let span = info_span!(
        "mcp.request",
        mcp.method = method.unwrap_or(""),
        mcp.protocol_era = era_label(era),
        mcp.protocol_version = protocol_version.unwrap_or(""),
        trace_id = field::Empty,
        parent_id = field::Empty,
        trace_flags = field::Empty,
        mcp.tracestate_present = false,
        mcp.baggage_present = false,
    );
    let entered = span.entered();

    if let Some(meta) = params.and_then(|p| p.get("_meta")).and_then(Value::as_object) {
        if let Some(raw) = meta.get(META_TRACEPARENT).and_then(Value::as_str) {
            match TraceContext::parse(raw.trim()) {
                Ok(ctx) => {
                    bind_traceparent_to_current_span(&ctx);
                }
                Err(err) => {
                    // Malformed → ignore; never panic or affect auth.
                    debug!(error = %err, "mcp: ignoring malformed traceparent in request _meta");
                }
            }
        }
        // Record presence only — do not trust or forward baggage/tracestate as
        // authority. Values are not copied into env or policy.
        if meta.get(META_TRACESTATE).and_then(Value::as_str).is_some_and(|s| !s.is_empty()) {
            tracing::Span::current().record("mcp.tracestate_present", true);
        }
        if meta.contains_key(META_BAGGAGE) {
            tracing::Span::current().record("mcp.baggage_present", true);
        }
    }

    entered
}

fn era_label(era: ProtocolEra) -> &'static str {
    match era {
        ProtocolEra::Modern => "modern",
        ProtocolEra::Legacy => "legacy",
    }
}

/// Pure helper for tests: try to parse a traceparent string from `_meta`.
#[cfg(test)]
pub fn extract_traceparent(params: Option<&Value>) -> Option<TraceContext> {
    let raw = params?
        .get("_meta")?
        .get(META_TRACEPARENT)?
        .as_str()?;
    TraceContext::parse(raw.trim()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn valid_traceparent_extracts() {
        let params = json!({
            "_meta": {
                "traceparent": "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"
            }
        });
        let ctx = extract_traceparent(Some(&params)).expect("valid");
        assert_eq!(ctx.trace_id(), "0af7651916cd43dd8448eb211c80319c");
    }

    #[test]
    fn malformed_traceparent_is_ignored() {
        let params = json!({
            "_meta": {
                "traceparent": "00-not-valid"
            }
        });
        assert!(extract_traceparent(Some(&params)).is_none());
    }

    #[test]
    fn missing_meta_is_none() {
        assert!(extract_traceparent(Some(&json!({}))).is_none());
        assert!(extract_traceparent(None).is_none());
    }

    #[test]
    fn enter_request_span_does_not_panic_on_garbage() {
        let params = json!({
            "_meta": {
                "traceparent": "garbage",
                "tracestate": "vendor=1",
                "baggage": { "k": "v" }
            }
        });
        let _guard = enter_request_span(
            Some("tools/list"),
            ProtocolEra::Modern,
            Some("2026-07-28"),
            Some(&params),
        );
        // Guard drop ends the span; no panic is the acceptance signal for
        // "invalid metadata cannot panic the server".
    }
}
