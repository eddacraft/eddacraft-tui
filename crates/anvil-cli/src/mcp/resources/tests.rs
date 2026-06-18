//! GCTX-030 `graph://` resource unit tests. These cover the URI/query parsing
//! and the resource catalogue — the daemon round-trip itself is exercised by the
//! `mcp_serve_stdio` integration tests and the egress/daemon layer tests.

use super::*;

#[test]
fn list_advertises_the_three_graph_resources() {
    let uris: Vec<String> = list()
        .iter()
        .filter_map(|r| r.get("uri").and_then(Value::as_str).map(str::to_string))
        .collect();
    assert_eq!(uris, vec![URI_SYMBOLS, URI_EDGES, URI_STATS]);
    // Every resource is advertised read-only and JSON.
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
