//! GCTX-030 `graph://` resource unit tests. These cover the URI/query parsing
//! and the resource catalogue — the daemon round-trip itself is exercised by the
//! `mcp_serve_stdio` integration tests and the egress/daemon layer tests.

use super::*;

#[test]
fn list_advertises_the_graph_resources() {
    let uris: Vec<String> = list()
        .iter()
        .filter_map(|r| r.get("uri").and_then(Value::as_str).map(str::to_string))
        .collect();
    // The graph:// trio leads the aggregated `resources/list`; the RMCPF-020
    // anvil:// resources follow (covered by the `anvil` submodule tests).
    assert_eq!(
        &uris[..3],
        &[
            URI_SYMBOLS.to_string(),
            URI_EDGES.to_string(),
            URI_STATS.to_string(),
        ]
    );
    // Every advertised resource — graph:// and anvil:// alike — is read-only
    // and JSON.
    for resource in list() {
        assert_eq!(resource["mimeType"], MIME_JSON);
        assert_eq!(resource["annotations"]["readOnlyHint"], json!(true));
    }
}

#[test]
fn split_uri_separates_base_and_query_pairs() {
    let (base, query) = split_uri("graph://edges?file=src/a.ts&cursor=ab12&limit=50");
    assert_eq!(base, URI_EDGES);
    assert_eq!(query_value(&query, "file"), Some("src/a.ts"));
    assert_eq!(query_value(&query, "cursor"), Some("ab12"));
    assert_eq!(query_value(&query, "limit"), Some("50"));
    assert_eq!(query_value(&query, "missing"), None);
}

#[test]
fn split_uri_without_query_yields_empty_pairs() {
    let (base, query) = split_uri(URI_STATS);
    assert_eq!(base, URI_STATS);
    assert!(query.is_empty());
}

#[test]
fn empty_query_value_is_treated_as_absent() {
    let (_, query) = split_uri("graph://symbols?file=&cursor=x");
    // An empty `file=` must not become `Some("")` (which would be a rejected
    // empty-path filter daemon-side); it reads as absent.
    assert_eq!(query_value(&query, "file"), None);
    assert_eq!(query_value(&query, "cursor"), Some("x"));
}

#[test]
fn read_rejects_unknown_uri_as_bad_request() {
    let err = read("graph://nope").expect_err("unknown uri is rejected");
    assert!(
        matches!(err, ReadError::BadRequest(_)),
        "unknown uri is a client error"
    );
    assert!(
        err.reason().contains("unknown resource uri"),
        "{}",
        err.reason()
    );
}

#[test]
fn parse_limit_rejects_non_numeric() {
    assert!(parse_limit("12").is_ok());
    assert!(matches!(parse_limit("abc"), Err(ReadError::BadRequest(_))));
}

#[test]
fn parse_limit_enforces_advertised_page_bounds() {
    // Resources advertise MAX_PAGE_LIMIT = 200; reject zero and oversize
    // before any daemon RPC is constructed.
    assert_eq!(parse_limit("200").expect("max page is allowed"), 200);
    assert_eq!(parse_limit("1").expect("min page is allowed"), 1);
    for raw in ["0", "201", "4294967295"] {
        let err = parse_limit(raw).expect_err(&format!("limit={raw} must be BadRequest"));
        assert!(
            matches!(err, ReadError::BadRequest(_)),
            "limit={raw}: {err:?}"
        );
        assert!(
            err.reason().contains("limit"),
            "limit={raw} reason should mention limit: {}",
            err.reason()
        );
    }
}

#[test]
fn file_filter_rejects_percent_encoding() {
    // ADV-5/CR-5: a `%` reaches the value (it is not a separator) and would
    // silently mis-map a path — reject it loudly.
    let q = vec![("file".to_string(), "src/a%20b.ts".to_string())];
    assert!(matches!(
        validated_file_filter(&q),
        Err(ReadError::BadRequest(_))
    ));
    // A clean path passes through verbatim.
    let clean = vec![("file".to_string(), "src/a".to_string())];
    assert_eq!(
        validated_file_filter(&clean).unwrap().as_deref(),
        Some("src/a")
    );
    // Absent filter is fine.
    assert_eq!(validated_file_filter(&[]).unwrap(), None);
}

#[test]
fn unknown_query_key_is_rejected() {
    // The `&`-in-a-path case: `?file=src/a&b.ts` splits to file=src/a + a stray
    // `b.ts` key. The unknown-key guard rejects it rather than silently reading
    // the truncated `src/a` (copilot follow-up).
    let (_, query) = split_uri("graph://symbols?file=src/a&b.ts");
    let err = ensure_known_query_keys(&query, &["file", "cursor", "limit"])
        .expect_err("stray key from a `&` in the path is rejected");
    assert!(matches!(err, ReadError::BadRequest(_)), "{err:?}");
    assert!(err.reason().contains("b.ts"), "{}", err.reason());
    // stats accepts no params at all.
    assert!(ensure_known_query_keys(&[("file".to_string(), "x".to_string())], &[]).is_err());
    // A fully-known query passes.
    let (_, ok) = split_uri("graph://edges?file=src/a.ts&cursor=ab");
    assert!(ensure_known_query_keys(&ok, &["file", "cursor", "limit"]).is_ok());
}

#[test]
fn graph_egress_credit_refuses_once_exhausted() {
    // CIB-091d: the process-local graph:// byte credit refuses a read once the
    // cumulative payload exceeds the ceiling. The accumulator is a process-global
    // static (one process == one stdio session); the test guard serialises the
    // credit-touching tests and zeroes the counter so this starts from a known
    // fresh budget regardless of run order or parallelism.
    let _guard = lock_and_reset_graph_egress_for_test();
    let under = charge_graph_egress(1);
    assert!(
        under.is_ok(),
        "a tiny first read is within budget: {under:?}"
    );

    // A single charge larger than the whole credit must be refused with a
    // structured quota error.
    let over = charge_graph_egress(GRAPH_EGRESS_CREDIT_BYTES + 1);
    assert!(
        matches!(over, Err(ReadError::QuotaExceeded(_))),
        "an over-ceiling charge must be refused: {over:?}"
    );
    if let Err(err) = over {
        assert!(err.reason().contains("quota"), "{}", err.reason());
    }

    // Once exhausted, even a zero-byte read stays refused (the cumulative total
    // is already over the ceiling).
    assert!(matches!(
        charge_graph_egress(0),
        Err(ReadError::QuotaExceeded(_))
    ));
}
