//! RSTLAN-008 dogfood: run the Rust AST anti-pattern catalogue over Anvil's own
//! crates as the T3 acceptance evidence.
//!
//! Ignored by default (it walks the whole workspace `crates/` tree). Run with:
//!
//! ```text
//! cargo test -p eddacraft-anvil-checks-ast --test dogfood -- --ignored --nocapture
//! ```
//!
//! It doubles as a regression guard: a future `tree-sitter-rust` bump or query
//! edit that makes the scanner panic — or mass-skip Anvil's own source — fails
//! here rather than silently degrading the catalogue.

use std::path::{Path, PathBuf};

use anvil_checks_ast::{AST_PARSE_SKIP_ID, AstScanOptions, scan_bytes};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Skip build output and VCS metadata.
            if matches!(
                path.file_name().and_then(|n| n.to_str()),
                Some("target" | ".git")
            ) {
                continue;
            }
            collect_rs(&path, out);
        } else if path.extension().and_then(|x| x.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
#[ignore = "RSTLAN-008 dogfood: scans the whole workspace; run with --ignored --nocapture"]
fn dogfood_ast_catalogue_on_anvil_crates() {
    let root = workspace_root();
    let registry = root.join("patterns/compiled/registry.json");
    let mut paths = Vec::new();
    collect_rs(&root.join("crates"), &mut paths);
    assert!(!paths.is_empty(), "expected .rs files under crates/");

    // Read once; scan in a single pass so the registry loads and the queries
    // compile exactly once.
    let owned: Vec<(String, Vec<u8>)> = paths
        .iter()
        .filter_map(|p| {
            std::fs::read(p)
                .ok()
                .map(|b| (p.to_string_lossy().into_owned(), b))
        })
        .collect();
    let refs: Vec<(&str, &[u8])> = owned
        .iter()
        .map(|(p, b)| (p.as_str(), b.as_slice()))
        .collect();

    let opts = AstScanOptions {
        registry_path: Some(registry),
        include_opt_in: false,
    };
    let root_str = root.to_string_lossy().into_owned();
    let out = scan_bytes(&refs, Some(&root_str), &opts);

    assert!(
        out.init_errors.is_empty(),
        "init errors: {:?}",
        out.init_errors
    );

    let mut by_id: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut samples: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    let mut parse_skips = 0usize;
    let mut suppressed = 0usize;
    for w in &out.warnings {
        if w.id == AST_PARSE_SKIP_ID {
            parse_skips += 1;
        } else if w.suppressed.is_some() {
            suppressed += 1;
        } else {
            *by_id.entry(w.id.clone()).or_default() += 1;
            let s = samples.entry(w.id.clone()).or_default();
            if s.len() < 6 {
                s.push(format!("{}:{}", w.location.file, w.location.line));
            }
        }
    }
    let actionable: usize = by_id.values().sum();

    // Bucket by likely FP source so the dogfood evidence is quantified.
    let mut in_build_rs = 0usize;
    let mut in_test_basename = 0usize;
    for w in &out.warnings {
        if w.id == AST_PARSE_SKIP_ID || w.suppressed.is_some() {
            continue;
        }
        let f = &w.location.file;
        if f.ends_with("/build.rs") || f == "build.rs" {
            in_build_rs += 1;
        } else if f.ends_with("/tests.rs") || f.ends_with("/test.rs") {
            in_test_basename += 1;
        }
    }

    println!("\n=== RSTLAN-008 AST dogfood — Anvil crates ===");
    println!(
        "  (FP buckets) build.rs findings: {in_build_rs}, tests.rs/test.rs findings: {in_test_basename}"
    );
    println!("files scanned:        {}", out.files_scanned);
    println!("parse-skips:          {parse_skips}");
    println!("suppressed:           {suppressed}");
    println!("actionable findings:  {actionable}");
    for (id, n) in &by_id {
        println!("  {id}: {n}");
        for loc in samples.get(id).into_iter().flatten() {
            println!("      e.g. {loc}");
        }
    }

    // Zero-panic bar (§16.5 #9): reaching this line means no panic during
    // parse/extract across the whole substrate.
    //
    // Clean-parse bar: tree-sitter-rust must parse Anvil's own source — a mass
    // parse-skip would mean the catalogue is blind to most of the tree.
    // Clean-parse bar: parse-skips must stay under 2% (integer math: skips*50 <
    // files). 0 today — tree-sitter-rust parses all of Anvil cleanly.
    assert!(
        parse_skips * 50 < out.files_scanned,
        "parse-skip rate too high ({parse_skips}/{} files) — \
         tree-sitter-rust grammar regression?",
        out.files_scanned
    );

    // FP bar (§16.5 #9): the two FP classes the dogfood surfaced — build scripts
    // and `tests.rs`/`test.rs` test-module files — must stay excluded.
    assert_eq!(
        in_build_rs, 0,
        "RS-001/RS-002 fired in a build script — the build.rs exclusion regressed"
    );
    assert_eq!(
        in_test_basename, 0,
        "RS-001/RS-002 fired in a test-module file — the tests.rs exclusion regressed"
    );
}
