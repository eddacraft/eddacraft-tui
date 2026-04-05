use std::path::Path;

use crate::filter::ScanFilter;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn default_filter() -> ScanFilter {
    ScanFilter::default_excludes()
}

// ---------------------------------------------------------------------------
// Directory-segment exclusions (DD-1)
// ---------------------------------------------------------------------------

#[test]
fn excludes_fixtures_dir() {
    let f = default_filter();
    assert!(!f.includes(Path::new("src/__fixtures__/sample.ts")));
}

#[test]
fn excludes_mocks_dir() {
    let f = default_filter();
    assert!(!f.includes(Path::new("lib/__mocks__/service.ts")));
}

#[test]
fn excludes_tests_dir() {
    let f = default_filter();
    assert!(!f.includes(Path::new("src/__tests__/app.test.ts")));
}

#[test]
fn excludes_test_data_dir() {
    let f = default_filter();
    assert!(!f.includes(Path::new("test-data/golden/report.json")));
}

#[test]
fn excludes_bare_fixtures_dir() {
    let f = default_filter();
    assert!(!f.includes(Path::new("core/fixtures/sample.json")));
}

#[test]
fn excludes_node_modules_dir() {
    let f = default_filter();
    assert!(!f.includes(Path::new("node_modules/lodash/index.js")));
}

#[test]
fn excludes_target_dir() {
    let f = default_filter();
    assert!(!f.includes(Path::new("target/debug/anvil")));
}

#[test]
fn excludes_git_dir() {
    let f = default_filter();
    assert!(!f.includes(Path::new(".git/config")));
}

// ---------------------------------------------------------------------------
// File-suffix exclusions (DD-1)
// ---------------------------------------------------------------------------

#[test]
fn excludes_test_ts_suffix() {
    let f = default_filter();
    assert!(!f.includes(Path::new("src/custom_filter.test.ts")));
}

#[test]
fn excludes_spec_ts_suffix() {
    let f = default_filter();
    assert!(!f.includes(Path::new("src/widget.spec.ts")));
}

#[test]
fn excludes_test_rs_suffix() {
    let f = default_filter();
    assert!(!f.includes(Path::new("crates/anvil/checks.test.rs")));
}

#[test]
fn excludes_underscore_test_rs_suffix() {
    let f = default_filter();
    assert!(!f.includes(Path::new("crates/anvil/checks_test.rs")));
}

// ---------------------------------------------------------------------------
// Non-matching paths are included
// ---------------------------------------------------------------------------

#[test]
fn includes_regular_source_file() {
    let f = default_filter();
    assert!(f.includes(Path::new("src/main.rs")));
}

#[test]
fn includes_regular_ts_file() {
    let f = default_filter();
    assert!(f.includes(Path::new("src/custom_filter.ts")));
}

#[test]
fn includes_regular_json_file() {
    let f = default_filter();
    assert!(f.includes(Path::new("config/settings.json")));
}

// ---------------------------------------------------------------------------
// Nested fixtures
// ---------------------------------------------------------------------------

#[test]
fn excludes_deeply_nested_fixtures() {
    let f = default_filter();
    assert!(!f.includes(Path::new("a/b/__fixtures__/c/d.ts")));
}

#[test]
fn excludes_deeply_nested_test_data() {
    let f = default_filter();
    assert!(!f.includes(Path::new("packages/core/test-data/golden/snapshot.json")));
}

// ---------------------------------------------------------------------------
// Partial matches must NOT falsely exclude
// ---------------------------------------------------------------------------

#[test]
fn does_not_exclude_partial_fixtures_match() {
    let f = default_filter();
    // "my_fixtures" is not the same segment as "fixtures"
    assert!(f.includes(Path::new("my_fixtures/file.ts")));
}

#[test]
fn does_not_exclude_partial_test_data_match() {
    let f = default_filter();
    assert!(f.includes(Path::new("my-test-data-extra/file.ts")));
}

#[test]
fn does_not_exclude_partial_git_match() {
    let f = default_filter();
    assert!(f.includes(Path::new(".github/workflows/ci.yml")));
}

#[test]
fn does_not_exclude_partial_target_match() {
    let f = default_filter();
    assert!(f.includes(Path::new("src/target_impl/main.rs")));
}

// ---------------------------------------------------------------------------
// File suffix edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_ts_excluded_but_plain_ts_included() {
    let f = default_filter();
    assert!(!f.includes(Path::new("src/widget.test.ts")));
    assert!(f.includes(Path::new("src/widget.ts")));
}

#[test]
fn spec_ts_excluded_but_inspector_ts_included() {
    let f = default_filter();
    assert!(!f.includes(Path::new("src/thing.spec.ts")));
    assert!(f.includes(Path::new("src/inspector.ts")));
}

#[test]
fn test_rs_excluded_but_plain_rs_included() {
    let f = default_filter();
    assert!(!f.includes(Path::new("src/scanner.test.rs")));
    assert!(f.includes(Path::new("src/scanner.rs")));
}

#[test]
fn underscore_test_rs_excluded_but_plain_rs_included() {
    let f = default_filter();
    assert!(!f.includes(Path::new("src/filter_test.rs")));
    assert!(f.includes(Path::new("src/filter.rs")));
}

// ---------------------------------------------------------------------------
// Path normalisation (Windows-style backslashes)
// ---------------------------------------------------------------------------

#[test]
fn normalises_backslash_paths_for_dir_segment() {
    let f = default_filter();
    assert!(!f.includes(Path::new("src\\__fixtures__\\sample.ts")));
}

#[test]
fn normalises_backslash_paths_for_suffix() {
    let f = default_filter();
    assert!(!f.includes(Path::new("src\\widget.test.ts")));
}

// ---------------------------------------------------------------------------
// Empty / root paths don't panic
// ---------------------------------------------------------------------------

#[test]
fn empty_path_does_not_panic() {
    let f = default_filter();
    assert!(f.includes(Path::new("")));
}

#[test]
fn root_path_does_not_panic() {
    let f = default_filter();
    assert!(f.includes(Path::new("/")));
}

#[test]
fn dot_path_does_not_panic() {
    let f = default_filter();
    assert!(f.includes(Path::new(".")));
}

// ---------------------------------------------------------------------------
// Absolute paths
// ---------------------------------------------------------------------------

#[test]
fn excludes_absolute_path_with_fixtures() {
    let f = default_filter();
    assert!(!f.includes(Path::new("/home/user/project/__fixtures__/data.json")));
}

#[test]
fn includes_absolute_path_without_excludes() {
    let f = default_filter();
    assert!(f.includes(Path::new("/home/user/project/src/main.rs")));
}

// ---------------------------------------------------------------------------
// Custom filter via ScanFilter::new
// ---------------------------------------------------------------------------

#[test]
fn custom_dir_pattern_excludes() {
    let f = ScanFilter::new(vec!["vendor/".to_owned()]);
    assert!(!f.includes(Path::new("deps/vendor/lib.rs")));
    assert!(f.includes(Path::new("src/main.rs")));
}

#[test]
fn custom_suffix_pattern_excludes() {
    let f = ScanFilter::new(vec!["*.snapshot".to_owned()]);
    assert!(!f.includes(Path::new("tests/output.snapshot")));
    assert!(f.includes(Path::new("tests/output.txt")));
}

#[test]
fn custom_mixed_patterns() {
    let f = ScanFilter::new(vec![
        "build/".to_owned(),
        "*.gen.ts".to_owned(),
    ]);
    assert!(!f.includes(Path::new("out/build/index.js")));
    assert!(!f.includes(Path::new("src/schema.gen.ts")));
    assert!(f.includes(Path::new("src/schema.ts")));
}
