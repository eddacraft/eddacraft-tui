//! LANGTAIL-008 external-corpus parse-robustness validation.
//!
//! The equivalent of RSTLAN-008 / PYLAN-009: run the kernel parser + symbol
//! extractor over a real-world OSS corpus and measure parse robustness —
//! files parsed, panics, error-trees, symbols extracted — per language. The
//! T1 wave has no anti-pattern catalogue, so there is no false-positive rate
//! to measure (that is the T2/T3 half of the Rust/Python bar); robustness at
//! scale is the load-bearing evidence here.
//!
//! Ignored by default (no corpus in CI). Run against a checked-out corpus:
//!
//! ```sh
//! LANGTAIL_CORPUS=/tmp/langtail-corpus \
//!   cargo test -p eddacraft-anvil-kernel --test langtail_external_validation \
//!   -- --ignored --nocapture
//! ```
//!
//! `LANGTAIL_CORPUS` is a directory; every file under it whose extension maps
//! to a [`Language`] is parsed. Per-language subdirectories are the convention
//! (`<corpus>/go`, `<corpus>/dart`, …) but not required.

// Diagnostic harness: a single linear scan-and-report body and a percentage
// display readout — both lints are noise here.
#![allow(clippy::too_many_lines, clippy::cast_precision_loss)]

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use anvil_kernel::parser::Parser;
use anvil_kernel::parser::extract::extract_symbols;
use anvil_kernel::parser::languages::Language;

#[derive(Default, Clone)]
struct Stat {
    files: usize,
    panics: usize,
    error_trees: usize,
    unreadable: usize,
    parse_errors: usize,
    symbols: usize,
    imports: usize,
}

/// Restores the previous panic hook on drop, so an early panic anywhere in the
/// test body (a failed assertion, an unexpected panic outside the inner
/// `catch_unwind`) can never leave the process-global hook suppressed for
/// subsequent tests in the same binary.
type PanicHook = Box<dyn Fn(&std::panic::PanicHookInfo<'_>) + Sync + Send + 'static>;

struct HookGuard(Option<PanicHook>);

impl Drop for HookGuard {
    fn drop(&mut self) {
        if let Some(prev) = self.0.take() {
            std::panic::set_hook(prev);
        }
    }
}

#[test]
#[ignore = "requires LANGTAIL_CORPUS pointing at a real-world OSS corpus"]
fn external_corpus_parse_robustness() {
    let Ok(root) = std::env::var("LANGTAIL_CORPUS") else {
        panic!("set LANGTAIL_CORPUS=<dir> to run this validation");
    };
    let root = Path::new(&root);
    assert!(
        root.is_dir(),
        "LANGTAIL_CORPUS must be a directory: {root:?}"
    );

    // Silence per-file panic spew; we count panics instead. The guard restores
    // the previous hook on drop even if the test panics early.
    let _hook_guard = HookGuard(Some(std::panic::take_hook()));
    std::panic::set_hook(Box::new(|_| {}));

    let mut stats: BTreeMap<String, Stat> = BTreeMap::new();
    let mut panic_files: Vec<String> = Vec::new();
    let mut error_files: Vec<String> = Vec::new();

    let mut parser = Parser::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let Some(lang) = Language::from_path(path) else {
            continue;
        };
        // Restrict to the tail-wave languages — TS/JS/Rust/Python already have
        // their own external validation.
        if !matches!(
            lang,
            Language::Dart
                | Language::Go
                | Language::Java
                | Language::Kotlin
                | Language::CSharp
                | Language::C
                | Language::Cpp
        ) {
            continue;
        }
        let key = format!("{lang:?}");
        let s = stats.entry(key.clone()).or_default();
        s.files += 1;

        // An unreadable file was counted in `files` but never parsed — record
        // it as its own failure category so the denominator stays honest rather
        // than silently inflating the clean-parse share.
        let Ok(content) = std::fs::read(path) else {
            s.unreadable += 1;
            if error_files.len() < 50 {
                error_files.push(format!("UNREADABLE {}", path.display()));
            }
            continue;
        };

        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let parsed = parser.parse_bytes(path, &content).ok()?;
            let has_error = parsed.tree.root_node().has_error();
            let fs = extract_symbols(&parsed.tree, &content, path, 0);
            Some((has_error, fs.symbols.len(), fs.imports.len()))
        }));

        match outcome {
            Err(_) => {
                s.panics += 1;
                if panic_files.len() < 50 {
                    panic_files.push(path.display().to_string());
                }
            }
            // `parse_bytes` returned `Err` (grammar load failure — extension is
            // already known here). Conservatively a non-clean parse, not a
            // free pass: count it so the robustness metric can't look better
            // than reality.
            Ok(None) => {
                s.parse_errors += 1;
                if error_files.len() < 50 {
                    error_files.push(format!("PARSE_ERR {}", path.display()));
                }
            }
            Ok(Some((has_error, syms, imps))) => {
                s.symbols += syms;
                s.imports += imps;
                if has_error {
                    s.error_trees += 1;
                    if error_files.len() < 50 {
                        error_files.push(format!("ERRTREE {}", path.display()));
                    }
                }
            }
        }
    }

    // Report — machine-greppable.
    eprintln!("\n===== LANGTAIL EXTERNAL VALIDATION =====");
    let header = format!(
        "{:<10} {:>7} {:>7} {:>10} {:>7} {:>9} {:>9} {:>9}",
        "lang", "files", "panics", "err-trees", "unread", "parse-err", "symbols", "imports"
    );
    eprintln!("{header}");
    let mut total = Stat::default();
    for (lang, s) in &stats {
        eprintln!(
            "{lang:<10} {:>7} {:>7} {:>10} {:>7} {:>9} {:>9} {:>9}",
            s.files, s.panics, s.error_trees, s.unreadable, s.parse_errors, s.symbols, s.imports
        );
        total.files += s.files;
        total.panics += s.panics;
        total.error_trees += s.error_trees;
        total.unreadable += s.unreadable;
        total.parse_errors += s.parse_errors;
        total.symbols += s.symbols;
        total.imports += s.imports;
    }
    eprintln!(
        "{:<10} {:>7} {:>7} {:>10} {:>7} {:>9} {:>9} {:>9}",
        "TOTAL",
        total.files,
        total.panics,
        total.error_trees,
        total.unreadable,
        total.parse_errors,
        total.symbols,
        total.imports
    );
    // Conservative "not cleanly parsed" rate: every file that wasn't a clean
    // error-free parse (error-tree OR unreadable OR parse_bytes failure).
    let not_clean = total.error_trees + total.unreadable + total.parse_errors;
    let not_clean_pct = if total.files > 0 {
        100.0 * not_clean as f64 / total.files as f64
    } else {
        0.0
    };
    eprintln!(
        "not-cleanly-parsed rate: {not_clean_pct:.2}% ({not_clean} of {} files) \
         [err-trees {} + unreadable {} + parse-err {}]",
        total.files, total.error_trees, total.unreadable, total.parse_errors
    );
    if !panic_files.is_empty() {
        eprintln!("\n-- panic files (up to 50) --");
        for f in &panic_files {
            eprintln!("  PANIC {f}");
        }
    }
    if !error_files.is_empty() {
        eprintln!("\n-- not-cleanly-parsed files (up to 50) --");
        for f in &error_files {
            eprintln!("  {f}");
        }
    }

    // The hard acceptance bar (LANGTAIL-008): zero panics on the load-bearing
    // parse path. Error-trees are reported, not asserted-zero — a parser
    // legitimately produces them on syntax beyond a T1 grammar's coverage
    // (e.g. bleeding-edge C++), and that rate is the data that informs the
    // C/C++ at-risk decision.
    assert_eq!(
        total.panics, 0,
        "parse path must never panic on real-world source (got {} panics)",
        total.panics
    );
    assert!(total.files > 0, "corpus produced no tail-language files");
}
