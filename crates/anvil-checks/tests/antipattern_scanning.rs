//! Integration tests for the antipattern scanning module.
//!
//! These tests exercise the public API (`anvil_checks::antipattern::*`) with
//! realistic code snippets that represent genuine development scenarios.

use anvil_checks::antipattern::{
    AntipatternCheckConfig, WarningSeverity, count_by_severity, create_warning_result,
    get_default_patterns, get_enabled_patterns, get_pattern_ids, is_valid_pattern_id,
    run_antipattern_check, scan_file, scan_files, validate_warning_result_consistency,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn temp_dir(label: &str) -> std::path::PathBuf {
    let unique = format!(
        "anvil-ap-integ-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    );
    let path = std::env::temp_dir().join(unique);
    let _ = std::fs::create_dir_all(&path);
    path
}

// ---------------------------------------------------------------------------
// Pattern catalogue
// ---------------------------------------------------------------------------

#[test]
fn pattern_catalogue_is_non_empty() {
    let ids = get_pattern_ids();
    assert!(
        !ids.is_empty(),
        "registry-backed catalogue should expose at least one pattern"
    );
    for id in &ids {
        assert!(is_valid_pattern_id(id), "{id} should be valid");
    }
    assert!(!is_valid_pattern_id("AP-999"));
}

#[test]
fn default_patterns_exclude_opt_in() {
    let defaults = get_default_patterns();
    let enabled = get_enabled_patterns();

    assert!(
        defaults.len() < enabled.len(),
        "default set should be smaller than full enabled set"
    );
    for pattern in &defaults {
        assert!(!pattern.opt_in, "{} should not be opt-in", pattern.id);
    }
}

// ---------------------------------------------------------------------------
// scan_file — TypeScript antipatterns
// ---------------------------------------------------------------------------

#[test]
fn detects_explicit_any_in_realistic_service() {
    let content = r"
import { Injectable } from '@nestjs/common';

@Injectable()
export class UserService {
  private cache: any = {};

  async findUser(id: string): Promise<any> {
    const result = await this.db.query(`SELECT * FROM users WHERE id = $1`, [id]);
    return result.rows[0] as any;
  }
}
";

    let result = scan_file("src/services/user.service.ts", content, None);
    let any_warnings: Vec<_> = result
        .warnings
        .iter()
        .filter(|w| w.id == "AP-003")
        .collect();

    // CLAWP-041: assert the EXACT count. The fixture has exactly three
    // `any` uses (the `cache` field, the `Promise<any>` return type, and
    // the `as any` cast); a `>= 3` lower bound would pass even if the
    // scanner emitted duplicate or spurious AP-003 warnings for the same
    // source.
    assert_eq!(
        any_warnings.len(),
        3,
        "expected exactly 3 AP-003 warnings (cache field, return type, cast), got {any_warnings:#?}"
    );
}

#[test]
fn detects_empty_catch_block_in_error_handler() {
    let content = r"
export async function fetchData(url: string) {
  try {
    const response = await fetch(url);
    return response.json();
  } catch (err) {}
}
";

    let result = scan_file("src/api/client.ts", content, None);
    assert!(
        result.warnings.iter().any(|w| w.id == "AP-006"),
        "should detect the empty catch block"
    );
}

#[test]
fn non_empty_catch_block_does_not_trigger_ap006() {
    // CLAWP-042: paired negative for `detects_empty_catch_block_in_error_handler`.
    // Without it, AP-006 could fire on every `catch` regardless of body
    // and the positive test would still pass. The discriminator is body
    // EMPTINESS, so the catch braces must be on a single line (matching
    // the positive test's `} catch (err) {}` layout) and differ only in
    // having a real body — otherwise the test would be passing on layout,
    // not emptiness.
    let content = "\
export async function fetchData(url: string) {
  try {
    const response = await fetch(url);
    return response.json();
  } catch (err) { console.error('fetch failed', err); throw err; }
}
";

    let result = scan_file("src/api/client.ts", content, None);
    assert!(
        !result.warnings.iter().any(|w| w.id == "AP-006"),
        "a non-empty single-line catch block must not trigger AP-006, got: {:?}",
        result.warnings
    );
}

#[test]
fn detects_ts_ignore_directive() {
    let content = "// @ts-ignore\nconst value = unsafeFunction();\n";

    let result = scan_file("src/legacy/adapter.ts", content, None);
    assert!(
        result.warnings.iter().any(|w| w.id == "AP-004"),
        "should detect @ts-ignore"
    );
}

#[test]
fn detects_broad_eslint_disable() {
    let content = "/* eslint-disable */\nimport { something } from './module';\n";

    let result = scan_file("src/generated/types.ts", content, None);
    assert!(
        result.warnings.iter().any(|w| w.id == "AP-001"),
        "should detect broad eslint-disable"
    );
}

// ---------------------------------------------------------------------------
// Extension filtering
// ---------------------------------------------------------------------------

#[test]
fn typescript_patterns_do_not_fire_on_html_files() {
    let content = "const value: any = input;\ntry { x(); } catch (e) {}";

    let result = scan_file("templates/page.html", content, None);
    let ts_warnings: Vec<_> = result
        .warnings
        .iter()
        .filter(|w| w.id == "AP-003" || w.id == "AP-006")
        .collect();

    assert!(
        ts_warnings.is_empty(),
        "TypeScript patterns should not fire on .html files"
    );
}

// ---------------------------------------------------------------------------
// Allowlist / suppression
// ---------------------------------------------------------------------------

#[test]
fn test_files_are_allowlisted_for_any_type() {
    let content = "const mock: any = { id: 'test-123' };";

    let result = scan_file("src/__tests__/user.test.ts", content, None);
    assert!(
        result.warnings.is_empty(),
        "__tests__ files should be allowlisted for AP-003"
    );
}

#[test]
fn declaration_files_are_allowlisted_for_any_type() {
    let content = "export declare const config: any;";

    let result = scan_file("types/global.d.ts", content, None);
    assert!(
        result.warnings.is_empty(),
        ".d.ts files should be allowlisted for AP-003"
    );
}

#[test]
fn suppression_comment_marks_warning_as_suppressed() {
    let content = "\
// @anvil-ignore AP-003 -- legacy bridge type\n\
const bridge: any = legacyModule.connect();\n";

    let result = scan_file("src/bridge.ts", content, None);
    assert_eq!(result.warnings.len(), 1);
    let w = &result.warnings[0];
    assert!(
        w.suppressed.is_some(),
        "warning should be marked suppressed"
    );
    assert_eq!(w.suppressed.as_ref().unwrap().reason, "legacy bridge type");
}

#[test]
fn suppression_for_wrong_pattern_does_not_apply() {
    let content = "\
// @anvil-ignore AP-001\n\
const value: any = input;\n";

    let result = scan_file("src/app.ts", content, None);
    assert_eq!(result.warnings.len(), 1);
    assert!(
        result.warnings[0].suppressed.is_none(),
        "AP-001 suppression should not apply to AP-003 finding"
    );
}

// ---------------------------------------------------------------------------
// scan_files — batch scanning
// ---------------------------------------------------------------------------

#[test]
fn scan_files_aggregates_results_across_multiple_files() {
    let files: Vec<(&str, &str)> = vec![
        ("src/a.ts", "const x: any = 1;"),
        ("src/b.ts", "try { work(); } catch (e) {}"),
        ("src/c.ts", "const clean = 42;"),
    ];

    let results = scan_files(&files, None);
    assert_eq!(results.len(), 3);

    let total_warnings: usize = results.iter().map(|r| r.warnings.len()).sum();
    assert_eq!(total_warnings, 2, "should find 1 any + 1 empty catch");
}

// ---------------------------------------------------------------------------
// run_antipattern_check — file-based orchestration
// ---------------------------------------------------------------------------

#[test]
fn check_passes_on_clean_codebase() {
    let dir = temp_dir("clean");
    let f = dir.join("service.ts");
    std::fs::write(
        &f,
        "export function greet(name: string): string { return `Hello, ${name}!`; }",
    )
    .unwrap();

    let fs = f.to_string_lossy().to_string();
    let files = [fs.as_str()];
    let result = run_antipattern_check(&files, &AntipatternCheckConfig::default(), None);

    assert!(result.passed);
    assert_eq!(result.score, 100);
    assert!(result.message.contains("no issues found"));

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn check_fails_when_threshold_is_warning_and_warning_present() {
    let dir = temp_dir("threshold-warn");
    let f = dir.join("unsafe.ts");
    std::fs::write(&f, "const value: any = source;").unwrap();

    let config = AntipatternCheckConfig {
        severity_threshold: WarningSeverity::Warning,
        ..AntipatternCheckConfig::default()
    };
    let fs = f.to_string_lossy().to_string();
    let files = [fs.as_str()];
    let result = run_antipattern_check(&files, &config, None);

    assert!(!result.passed, "should fail at warning threshold");
    assert_eq!(result.score, 95, "one warning = 5 point penalty");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn check_passes_with_default_error_threshold_despite_warnings() {
    let dir = temp_dir("default-threshold");
    let f = dir.join("warn.ts");
    std::fs::write(&f, "const value: any = source;").unwrap();

    let fs = f.to_string_lossy().to_string();
    let files = [fs.as_str()];
    let result = run_antipattern_check(&files, &AntipatternCheckConfig::default(), None);

    assert!(
        result.passed,
        "default error threshold should pass with only warnings"
    );

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn suppressed_warnings_do_not_count_towards_failure() {
    let dir = temp_dir("suppressed-check");
    let f = dir.join("legacy.ts");
    std::fs::write(
        &f,
        "// @anvil-ignore AP-003 -- legacy bridge\nconst value: any = source;",
    )
    .unwrap();

    let config = AntipatternCheckConfig {
        severity_threshold: WarningSeverity::Warning,
        ..AntipatternCheckConfig::default()
    };
    let fs = f.to_string_lossy().to_string();
    let files = [fs.as_str()];
    let result = run_antipattern_check(&files, &config, None);

    assert!(
        result.passed,
        "suppressed warnings should not cause failure"
    );
    assert_eq!(result.score, 100);
    assert_eq!(result.warnings.summary.suppressed, 1);

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn unscannable_extensions_are_skipped() {
    let dir = temp_dir("skip-ext");
    let ts = dir.join("app.ts");
    let txt = dir.join("notes.txt");
    let md = dir.join("README.md");

    std::fs::write(&ts, "const x: any = 1;").unwrap();
    std::fs::write(&txt, "const x: any = 1;").unwrap();
    std::fs::write(&md, "const x: any = 1;").unwrap();

    let ts_s = ts.to_string_lossy().to_string();
    let txt_s = txt.to_string_lossy().to_string();
    let md_s = md.to_string_lossy().to_string();
    let files = [ts_s.as_str(), txt_s.as_str(), md_s.as_str()];
    let result = run_antipattern_check(&files, &AntipatternCheckConfig::default(), None);

    assert_eq!(result.files_scanned, 1, "only .ts should be scanned");

    let _ = std::fs::remove_dir_all(dir);
}

// ---------------------------------------------------------------------------
// Warning result consistency
// ---------------------------------------------------------------------------

#[test]
fn warning_result_consistency_holds_after_aggregation() {
    let content = "\
const a: any = 1;\n\
// @ts-ignore\n\
const b = legacy();\n\
try { x(); } catch (e) {}\n";

    let result = scan_file("src/mixed.ts", content, None);
    let warning_result =
        create_warning_result(result.warnings.clone(), result.patterns_checked.clone());

    assert!(validate_warning_result_consistency(&warning_result));

    let summary = count_by_severity(&result.warnings);
    assert_eq!(summary.total, result.warnings.len());
}

// ---------------------------------------------------------------------------
// Realistic multi-file project scan
// ---------------------------------------------------------------------------

#[test]
fn realistic_project_scan_finds_expected_issues() {
    let dir = temp_dir("project");
    let _ = std::fs::create_dir_all(dir.join("src"));

    // TypeScript with issues
    std::fs::write(
        dir.join("src/handler.ts"),
        "export function handle(input: any) {\n\
         try { process(input); } catch (e) {}\n\
         }",
    )
    .unwrap();

    // Clean TypeScript
    std::fs::write(
        dir.join("src/utils.ts"),
        "export function add(a: number, b: number): number { return a + b; }",
    )
    .unwrap();

    let config = AntipatternCheckConfig {
        include_opt_in: true,
        ..AntipatternCheckConfig::default()
    };

    let handler = dir.join("src/handler.ts").to_string_lossy().to_string();
    let utils = dir.join("src/utils.ts").to_string_lossy().to_string();
    let files = [handler.as_str(), utils.as_str()];

    let result = run_antipattern_check(&files, &config, None);

    assert_eq!(result.files_scanned, 2);
    // handler.ts: any (AP-003) + empty catch (AP-006)
    let ids: Vec<&str> = result
        .warnings
        .warnings
        .iter()
        .map(|w| w.id.as_str())
        .collect();
    assert!(
        ids.contains(&"AP-003"),
        "expected AP-003 warning, got {ids:?}"
    );
    assert!(
        ids.contains(&"AP-006"),
        "expected AP-006 warning, got {ids:?}"
    );

    let _ = std::fs::remove_dir_all(dir);
}
