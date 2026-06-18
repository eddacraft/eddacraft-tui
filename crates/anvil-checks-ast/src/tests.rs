//! Good/bad fixture pairs for each AST rule, plus the registry-completeness and
//! query-drift guards ADR-071 §3/§4 require.

use std::path::PathBuf;

use anvil_checks::antipattern::registry_loader::{
    Detection, LoadRegistryOptions, load_compiled_registry, reset_registry_cache,
};

use super::*;

fn workspace_registry_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root")
        .join("patterns/compiled/registry.json")
}

fn test_opts() -> AstScanOptions {
    AstScanOptions {
        registry_path: Some(workspace_registry_path()),
        include_opt_in: false,
    }
}

/// Scan one file and return only the actionable (non-suppressed) findings for
/// `id`.
fn fires(path: &str, content: &str, id: &str) -> bool {
    let out = scan_bytes(&[(path, content.as_bytes())], None, &test_opts());
    assert!(
        out.init_errors.is_empty(),
        "init errors: {:?}",
        out.init_errors
    );
    out.warnings
        .iter()
        .any(|w| w.id == id && w.suppressed.is_none())
}

fn scan(path: &str, content: &str) -> AstScanOutput {
    scan_bytes(&[(path, content.as_bytes())], None, &test_opts())
}

/// Like [`fires`] but with `include_opt_in` set — for opt-in rules (RS-004).
fn fires_opt_in(path: &str, content: &str, id: &str) -> bool {
    let opts = AstScanOptions {
        registry_path: Some(workspace_registry_path()),
        include_opt_in: true,
    };
    let out = scan_bytes(&[(path, content.as_bytes())], None, &opts);
    assert!(
        out.init_errors.is_empty(),
        "init errors: {:?}",
        out.init_errors
    );
    out.warnings
        .iter()
        .any(|w| w.id == id && w.suppressed.is_none())
}

// --- RS-001 unwrap / expect -------------------------------------------------

#[test]
fn rs001_fires_on_unwrap_in_prod_code() {
    let src = "fn run() { let n = parse().unwrap(); }\n";
    assert!(fires("src/lib.rs", src, "RS-001"));
}

#[test]
fn rs001_fires_on_expect() {
    let src = "fn run() { let n = parse().expect(\"bad\"); }\n";
    assert!(fires("src/lib.rs", src, "RS-001"));
}

#[test]
fn rs001_silent_on_non_unwrap_method() {
    let src = "fn run() { let n = thing.clone(); let m = thing.map(|x| x); }\n";
    assert!(!fires("src/lib.rs", src, "RS-001"));
}

#[test]
fn rs001_excluded_in_cfg_test_module() {
    let src = "#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() { foo().unwrap(); }\n}\n";
    assert!(!fires("src/lib.rs", src, "RS-001"));
}

#[test]
fn rs001_excluded_in_cfg_all_test() {
    let src = "#[cfg(all(test, feature = \"x\"))]\nmod tests { fn t() { foo().unwrap(); } }\n";
    assert!(!fires("src/lib.rs", src, "RS-001"));
}

#[test]
fn rs001_excluded_by_inner_cfg_test_attribute() {
    // Council adversarial MAJOR: `#![cfg(test)]` inner attribute (a leading
    // child of the mod body, not a preceding sibling) must also exclude.
    let src = "mod it {\n    #![cfg(test)]\n    fn t() { foo().unwrap(); }\n}\n";
    assert!(!fires("src/lib.rs", src, "RS-001"));
}

#[test]
fn rs001_reports_on_unwrap_line_in_multiline_chain() {
    // Council adversarial MAJOR: a multi-line chain must report on the
    // `.unwrap()` line (line 4), not the receiver's start line (line 2), so the
    // @anvil-ignore directive sits directly above the unwrap.
    let src = "fn run() {\n    let v = thing()\n        .transform()\n        .unwrap();\n}\n";
    let out = scan("src/lib.rs", src);
    let w = out
        .warnings
        .iter()
        .find(|w| w.id == "RS-001")
        .expect("RS-001 warning");
    assert_eq!(w.location.line, 4, "should report on the .unwrap() line");
}

#[test]
fn rs001_not_excluded_by_cfg_not_test() {
    // cfg(not(test)) is production code — the unwrap must still fire.
    let src = "#[cfg(not(test))]\nmod prod { fn t() { foo().unwrap(); } }\n";
    assert!(fires("src/lib.rs", src, "RS-001"));
}

#[test]
fn rs001_excluded_in_tests_directory() {
    let src = "fn t() { foo().unwrap(); }\n";
    assert!(!fires("crates/x/tests/it.rs", src, "RS-001"));
}

#[test]
fn rs001_excluded_in_test_module_file() {
    // RSTLAN-008 dogfood: a `tests.rs` file is included via
    // `#[cfg(test)] mod tests;` from lib.rs, so the file itself has no cfg
    // marker — exclude it by basename.
    let src = "fn t() { foo().unwrap(); }\n";
    assert!(!fires("crates/x/src/tests.rs", src, "RS-001"));
}

#[test]
fn rs001_excluded_in_build_script() {
    // RSTLAN-008 dogfood: build scripts panic/unwrap idiomatically and are not
    // shipped runtime code.
    let src = "fn main() { something().unwrap(); }\n";
    assert!(!fires("crates/x/build.rs", src, "RS-001"));
}

#[test]
fn rs002_excluded_in_build_script() {
    let src = "fn main() { panic!(\"build failed\"); }\n";
    assert!(!fires("crates/x/build.rs", src, "RS-002"));
}

#[test]
fn rs001_suppressed_by_directive() {
    let src = "fn run() {\n    // @anvil-ignore RS-001 -- infallible here\n    let n = parse().unwrap();\n}\n";
    // Not actionable (suppressed), but the warning is still emitted with the
    // suppression recorded — same contract as the regex scanner.
    assert!(!fires("src/lib.rs", src, "RS-001"));
    let out = scan("src/lib.rs", src);
    assert!(
        out.warnings
            .iter()
            .any(|w| w.id == "RS-001" && w.suppressed.is_some()),
        "expected a suppressed RS-001 warning, got {:?}",
        out.warnings
    );
}

// --- RS-002 panic! ----------------------------------------------------------

#[test]
fn rs002_fires_on_panic_in_prod() {
    let src = "fn run() { panic!(\"nope\"); }\n";
    assert!(fires("src/lib.rs", src, "RS-002"));
}

#[test]
fn rs002_excluded_in_cfg_test() {
    let src = "#[cfg(test)]\nmod t { fn x() { panic!(\"in test\"); } }\n";
    assert!(!fires("src/lib.rs", src, "RS-002"));
}

#[test]
fn rs002_silent_on_other_macros() {
    let src = "fn run() { println!(\"hi\"); vec![1, 2, 3]; }\n";
    assert!(!fires("src/lib.rs", src, "RS-002"));
}

// --- RS-003 unsafe without SAFETY ------------------------------------------

#[test]
fn rs003_fires_on_unguarded_unsafe() {
    let src = "fn run() {\n    unsafe { deref(p); }\n}\n";
    assert!(fires("src/lib.rs", src, "RS-003"));
}

#[test]
fn rs003_clean_with_safety_comment() {
    let src = "fn run() {\n    // SAFETY: p is a valid, aligned, non-null borrow\n    unsafe { deref(p); }\n}\n";
    assert!(!fires("src/lib.rs", src, "RS-003"));
}

#[test]
fn rs003_clean_with_safety_comment_and_blank_line() {
    // A blank line between comment and block must not defeat the rule
    // (AST-sibling semantics, not byte proximity).
    let src = "fn run() {\n    // SAFETY: justified\n\n    unsafe { deref(p); }\n}\n";
    assert!(!fires("src/lib.rs", src, "RS-003"));
}

#[test]
fn rs003_fires_when_unrelated_statement_intervenes() {
    let src = "fn run() {\n    // SAFETY: not for the block below\n    let x = 1;\n    unsafe { deref(p); }\n}\n";
    assert!(fires("src/lib.rs", src, "RS-003"));
}

#[test]
fn rs003_clean_with_safety_comment_inside_match_arm() {
    // Council kernel-maintainer MAJOR: a `// SAFETY:` comment written inside a
    // match arm precedes the match_arm, not the whole match statement.
    let src = "fn run() {\n    match x {\n        // SAFETY: valid for this arm\n        Foo => unsafe { deref(p); }\n        _ => {}\n    }\n}\n";
    assert!(!fires("src/lib.rs", src, "RS-003"));
}

#[test]
fn rs003_does_not_panic_on_multibyte_comment() {
    // Council kernel-maintainer CRITICAL: byte-slicing `// SAFETÉ` at index 6
    // used to split a multi-byte char and panic.
    let src = "fn run() {\n    // SAFETÉ accentué\n    unsafe { deref(p); }\n}\n";
    // The comment is not a SAFETY word boundary, so the rule still fires — but
    // crucially this must not panic.
    assert!(fires("src/lib.rs", src, "RS-003"));
}

#[test]
fn rs003_safety_requires_word_boundary() {
    // `SAFETYNET` is not a SAFETY comment (no word boundary).
    let src = "fn run() {\n    // SAFETYNET is unrelated\n    unsafe { deref(p); }\n}\n";
    assert!(fires("src/lib.rs", src, "RS-003"));
}

#[test]
fn rs003_clean_with_multiline_safety_comment() {
    // External-FP dogfood (tokio): `// SAFETY:` is the first line of a
    // multi-line rationale block, so the comment immediately above the block is
    // a continuation line. The SAFETY keyword anywhere in the contiguous
    // comment run must be credited.
    let src = "fn run() {\n    // SAFETY: p is valid because the caller upholds the\n    // invariant that it points to an initialised, aligned T.\n    unsafe { deref(p); }\n}\n";
    assert!(!fires("src/lib.rs", src, "RS-003"));
}

#[test]
fn rs003_clean_with_safety_comment_inside_block() {
    // External-FP dogfood (tokio): `// SAFETY:` written as the first statement
    // inside the unsafe block rather than above it.
    let src = "fn run() {\n    unsafe {\n        // SAFETY: justified at the point of use\n        deref(p);\n    }\n}\n";
    assert!(!fires("src/lib.rs", src, "RS-003"));
}

#[test]
fn rs003_fires_when_comment_run_has_no_safety() {
    // A multi-line comment run with no SAFETY keyword must still fire — the
    // run-walk must not over-credit ordinary comments.
    let src = "fn run() {\n    // just a note\n    // another note about the call\n    unsafe { deref(p); }\n}\n";
    assert!(fires("src/lib.rs", src, "RS-003"));
}

#[test]
fn rs003_excluded_in_test_target_path() {
    // External-FP dogfood: `unsafe` in a tests/ target is test scaffolding, not
    // shipped runtime code — mirror the RS-001/RS-002 test exclusion.
    let src = "fn t() {\n    unsafe { deref(p); }\n}\n";
    assert!(!fires("tests/integration.rs", src, "RS-003"));
}

#[test]
fn rs003_excluded_in_cfg_test_module() {
    let src = "#[cfg(test)]\nmod tests {\n    fn t() {\n        unsafe { deref(p); }\n    }\n}\n";
    assert!(!fires("src/lib.rs", src, "RS-003"));
}

// --- RS-004 serde deny_unknown_fields --------------------------------------

#[test]
fn rs004_fires_on_deserialize_without_deny() {
    // RS-004 is opt-in (RSTLAN-008): flags every Deserialize struct, so it is
    // off by default and exercised here with include_opt_in.
    let src = "#[derive(Debug, Deserialize)]\nstruct Config { port: u16 }\n";
    assert!(fires_opt_in("src/lib.rs", src, "RS-004"));
}

#[test]
fn rs004_off_by_default() {
    let src = "#[derive(Debug, Deserialize)]\nstruct Config { port: u16 }\n";
    assert!(!fires("src/lib.rs", src, "RS-004"));
}

#[test]
fn rs004_clean_with_deny_unknown_fields() {
    let src =
        "#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\nstruct Config { port: u16 }\n";
    assert!(!fires_opt_in("src/lib.rs", src, "RS-004"));
}

#[test]
fn rs004_silent_without_deserialize_derive() {
    let src = "#[derive(Debug, Clone)]\nstruct Plain { port: u16 }\n";
    assert!(!fires_opt_in("src/lib.rs", src, "RS-004"));
}

#[test]
fn rs004_handles_path_qualified_derive() {
    let src = "#[derive(serde::Deserialize)]\nstruct Config { port: u16 }\n";
    assert!(fires_opt_in("src/lib.rs", src, "RS-004"));
}

#[test]
fn rs004_skips_tuple_and_unit_structs() {
    // Council adversarial MINOR: deny_unknown_fields is a no-op on tuple/unit
    // structs, so flagging them gives misleading advice.
    let tuple = "#[derive(Deserialize)]\nstruct Point(f64, f64);\n";
    assert!(!fires_opt_in("src/lib.rs", tuple, "RS-004"));
    let unit = "#[derive(Deserialize)]\nstruct Marker;\n";
    assert!(!fires_opt_in("src/lib.rs", unit, "RS-004"));
}

// --- Cross-cutting ----------------------------------------------------------

#[test]
fn metadata_mirrors_registry_provenance() {
    let src = "fn run() { let n = parse().unwrap(); }\n";
    let out = scan("src/lib.rs", src);
    let w = out
        .warnings
        .iter()
        .find(|w| w.id == "RS-001")
        .expect("RS-001 warning");
    assert_eq!(w.family.as_deref(), Some("rust-reliability"));
    assert_eq!(
        w.definition_ref.as_deref(),
        Some("patterns/rust-reliability/definition.anvil")
    );
    assert_eq!(w.spectrum_position, Some(3));
    assert!(w.fingerprint.is_some());
    assert_eq!(
        w.severity,
        anvil_checks::antipattern::types::WarningSeverity::Info
    );
}

#[test]
fn output_is_deterministically_sorted() {
    let src = "fn run() {\n    let a = x.unwrap();\n    unsafe { y(); }\n    let b = z.expect(\"m\");\n}\n";
    let out = scan("src/lib.rs", src);
    let keys: Vec<(usize, Option<usize>, &str)> = out
        .warnings
        .iter()
        .map(|w| (w.location.line, w.location.column, w.id.as_str()))
        .collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted, "warnings must be deterministically ordered");
}

#[test]
fn non_rust_files_are_skipped() {
    let out = scan_bytes(
        &[("src/app.ts", b"const x: any = y.unwrap();")],
        None,
        &test_opts(),
    );
    assert_eq!(out.files_scanned, 0);
    assert!(out.warnings.is_empty());
}

#[test]
fn parse_error_emits_skip_diagnostic_not_findings() {
    let src = "fn run( { let n = parse().unwrap(); \n"; // unbalanced
    let out = scan("src/lib.rs", src);
    assert!(
        out.warnings.iter().any(|w| w.id == AST_PARSE_SKIP_ID),
        "expected a parse-skip diagnostic"
    );
    assert!(
        !out.warnings.iter().any(|w| w.id == "RS-001"),
        "no rule findings from an un-parseable file"
    );
}

#[test]
fn registry_ast_rules_all_have_predicates() {
    // ADR-071 §3 completeness guard: every `Detection::Ast` rule in the compiled
    // registry must have a predicate-table entry, or the scanner would silently
    // produce nothing for it.
    let registry = load_compiled_registry(&LoadRegistryOptions {
        registry_path: Some(workspace_registry_path()),
    })
    .registry
    .expect("workspace registry loads");
    for cp in &registry.patterns {
        if matches!(cp.detection, Detection::Ast { .. }) {
            assert!(
                kind_for(&cp.id).is_some(),
                "registry AST rule {} has no predicate in anvil-checks-ast",
                cp.id
            );
        }
    }
}

#[test]
fn predicate_queries_match_registry_snapshot() {
    // ADR-071 §4 drift guard: the predicate table's expected `ast_query` must
    // equal the compiled registry's, so a query edit can't desync from the
    // predicate that assumes its captures.
    let registry = load_compiled_registry(&LoadRegistryOptions {
        registry_path: Some(workspace_registry_path()),
    })
    .registry
    .expect("workspace registry loads");
    for cp in &registry.patterns {
        if let Detection::Ast { ast_query } = &cp.detection {
            let (_, expected) =
                kind_for(&cp.id).unwrap_or_else(|| panic!("missing predicate for {}", cp.id));
            assert_eq!(
                ast_query, expected,
                "ast_query drift for {}: registry vs predicate table",
                cp.id
            );
        }
    }
}

#[test]
fn no_init_errors_against_workspace_registry() {
    let out = scan("src/lib.rs", "fn f() {}\n");
    assert!(
        out.init_errors.is_empty(),
        "workspace registry must load cleanly: {:?}",
        out.init_errors
    );
}

// --- CIB-050: registry load failures must surface, not silently disable ----

#[test]
fn malformed_registry_surfaces_loader_warnings_as_init_errors() {
    reset_registry_cache();
    // ADR-071 §3 "fail loudly, never silently produce nothing": a registry
    // that exists but cannot be parsed must not yield a default clean scan.
    // The loader's warnings ("missing file, parse error, schema mismatch")
    // have to reach `AstScanOutput` the same way per-rule init failures do.
    let dir = tempfile::tempdir().unwrap();
    let bad = dir.path().join("registry.json");
    std::fs::write(&bad, "{ this is not valid json").unwrap();

    let opts = AstScanOptions {
        registry_path: Some(bad),
        include_opt_in: false,
    };
    let out = scan_bytes(&[("src/lib.rs", b"fn f() { x.unwrap(); }\n")], None, &opts);

    assert!(
        !out.init_errors.is_empty(),
        "an unparseable registry must surface the loader warnings instead of \
         reporting a silent clean scan: {out:?}",
    );
    assert!(
        out.init_errors
            .iter()
            .any(|e| e.contains("schema validation")),
        "init_errors should carry the loader's parse/schema warning: {:?}",
        out.init_errors,
    );
    assert!(
        out.patterns_checked.is_empty() && out.warnings.is_empty(),
        "no rules ran, so the output must not claim otherwise: {out:?}",
    );
}

#[test]
fn unreadable_registry_surfaces_loader_warnings_as_init_errors() {
    reset_registry_cache();
    // Same guarantee for the read-failure shape ("Failed to read registry
    // at ..."): an explicit registry path that does not exist on disk.
    //
    // NOTE: the loader only returns `registry: None` for an explicit
    // *file-shaped* path that fails `read_to_string` (a directory here —
    // stable across platforms); a missing override path falls back to the
    // embedded registry with a warning instead, which is the working-scan
    // case, not this one.
    let dir = tempfile::tempdir().unwrap();
    let unreadable = dir.path().join("registry-as-dir");
    std::fs::create_dir(&unreadable).unwrap();

    let opts = AstScanOptions {
        registry_path: Some(unreadable),
        include_opt_in: false,
    };
    let out = scan_bytes(&[("src/lib.rs", b"fn f() {}\n")], None, &opts);

    assert!(
        out.init_errors
            .iter()
            .any(|e| e.contains("Failed to read registry")),
        "a read failure on an explicit registry path must surface in \
         init_errors: {:?}",
        out.init_errors,
    );
}

#[test]
fn missing_override_path_falls_back_to_embedded_and_surfaces_warning() {
    // A configured registry path that does not exist falls back to the
    // embedded catalogue (the scan still runs), but the misconfiguration
    // warning must surface through `init_errors`, not vanish.
    reset_registry_cache();
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("no-such-registry.json");

    let opts = AstScanOptions {
        registry_path: Some(missing),
        include_opt_in: false,
    };
    let out = scan_bytes(&[("src/lib.rs", b"fn f() { x.unwrap(); }\n")], None, &opts);

    assert!(
        out.init_errors
            .iter()
            .any(|e| e.contains("falling back to embedded registry")),
        "the embedded-fallback warning must surface in init_errors: {:?}",
        out.init_errors,
    );
    assert!(
        !out.patterns_checked.is_empty(),
        "the embedded registry still runs the scan: {out:?}",
    );
}
