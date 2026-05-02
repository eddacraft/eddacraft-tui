//! Reproduce the discovery scan against a real working tree by walking
//! `git ls-files` and running the same scanners (`secret` + `antipattern`)
//! that `welcome.rs::scan_project` invokes.
//!
//! Not a regression test — gated on the `ANVIL_DISCOVERY_REPRO=1` env var
//! and a working `git` binary, so a normal `cargo test` run skips it.
//!
//! ```bash
//! ANVIL_DISCOVERY_REPRO=1 \
//!   cargo test -p eddacraft-anvil-checks --test discovery_repro -- --nocapture
//! ```
//!
//! Output is JSON-Lines on stdout: one line per finding, plus a `summary`
//! line at the end. Easy to grep / jq.

use std::collections::BTreeMap;
use std::process::Command;

use anvil_checks::antipattern::scanner::scan_file as scan_antipatterns;
use anvil_checks::antipattern::types::WarningSeverity;
use anvil_checks::secret::scanner::scan_content_with_stats;
use anvil_checks::secret::types::SecretCheckConfig;

const SOURCE_EXTS: &[&str] = &[
    ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".rs", ".go", ".py", ".html", ".css", ".scss",
    ".vue", ".svelte", ".md", ".json", ".yaml", ".yml", ".toml", ".anvil",
];

const SKIP_EXTS: &[&str] = &[
    ".lock", ".min.js", ".min.css", ".map", ".svg", ".png", ".jpg", ".jpeg", ".gif", ".ico",
    ".woff", ".woff2", ".ttf", ".eot",
];

fn ext_match(path: &str, exts: &[&str]) -> bool {
    exts.iter().any(|e| path.ends_with(e))
}

#[test]
fn discovery_repro_scan() {
    if std::env::var("ANVIL_DISCOVERY_REPRO").ok().as_deref() != Some("1") {
        eprintln!("skipping: set ANVIL_DISCOVERY_REPRO=1 to run");
        return;
    }

    let repo_root = std::env::var("ANVIL_DISCOVERY_REPRO_ROOT").unwrap_or_else(|_| {
        // Crate is at `crates/anvil-checks` — repo root is two levels up.
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.to_string_lossy().into_owned())
            .expect("repo root resolves")
    });

    let output = Command::new("git")
        .arg("-C")
        .arg(&repo_root)
        .args(["ls-files"])
        .output()
        .expect("git ls-files runs");
    assert!(
        output.status.success(),
        "git ls-files failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    let secret_config = SecretCheckConfig::default();
    let mut by_pattern: BTreeMap<String, usize> = BTreeMap::new();
    let mut total_findings = 0usize;
    let mut files_scanned = 0usize;

    let path_prefix = std::env::var("ANVIL_DISCOVERY_REPRO_PREFIX").ok();

    for rel in stdout.lines() {
        if rel.is_empty() {
            continue;
        }
        if let Some(prefix) = &path_prefix
            && !rel.starts_with(prefix)
        {
            continue;
        }
        if ext_match(rel, SKIP_EXTS) {
            continue;
        }
        if !ext_match(rel, SOURCE_EXTS) {
            continue;
        }

        let abs = std::path::PathBuf::from(&repo_root).join(rel);
        let Ok(meta) = std::fs::metadata(&abs) else {
            continue;
        };
        if meta.len() > 512 * 1024 {
            continue; // mirror welcome.rs SCAN_MAX_FILE_SIZE
        }
        let Ok(content) = std::fs::read_to_string(&abs) else {
            continue;
        };
        files_scanned += 1;
        // SCAN_MAX_FILES cap is opt-in; absence of the cap surfaces the
        // user-reported FPs that live deeper in the repo than the
        // welcome scan reaches.
        if std::env::var("ANVIL_DISCOVERY_REPRO_CAP_500")
            .ok()
            .as_deref()
            == Some("1")
            && files_scanned > 500
        {
            break;
        }

        // Secret scanner
        let (secret_findings, _) = scan_content_with_stats(&content, rel, &secret_config);
        for hit in &secret_findings {
            let key = format!("secret::{}", hit.pattern_name);
            *by_pattern.entry(key.clone()).or_insert(0) += 1;
            total_findings += 1;
            println!(
                "{{\"scanner\":\"secret\",\"pattern\":\"{}\",\"file\":\"{}\",\"line\":{},\"redacted\":{}}}",
                hit.pattern_name,
                rel,
                hit.line,
                serde_json::to_string(&hit.redacted_line).unwrap_or_else(|_| "\"\"".to_string())
            );
        }

        // Antipattern scanner
        let ap_result = scan_antipatterns(rel, &content, None);
        for warning in &ap_result.warnings {
            if warning.suppressed.is_some() {
                continue;
            }
            let sev = match warning.severity {
                WarningSeverity::Error => "error",
                WarningSeverity::Warning => "warning",
                WarningSeverity::Info => "info",
            };
            let key = format!("antipattern::{}", warning.id);
            *by_pattern.entry(key.clone()).or_insert(0) += 1;
            total_findings += 1;
            println!(
                "{{\"scanner\":\"antipattern\",\"id\":\"{}\",\"sev\":\"{}\",\"file\":\"{}\",\"line\":{},\"title\":{}}}",
                warning.id,
                sev,
                rel,
                warning.location.line,
                serde_json::to_string(&warning.title).unwrap_or_else(|_| "\"\"".to_string())
            );
        }
    }

    println!(
        "{{\"summary\":{{\"files_scanned\":{},\"total_findings\":{},\"by_pattern\":{}}}}}",
        files_scanned,
        total_findings,
        serde_json::to_string(&by_pattern).unwrap_or_else(|_| "{}".to_string())
    );
}
