//! Language-agnostic symbol extraction.
//!
//! The orchestrator ([`extract_symbols`]) is intentionally free of any
//! language-specific knowledge: it derives the [`Language`] for a file and
//! routes `(tree, source)` through the matching [`LanguageExtractor`]
//! implementation. This is the K1 kernel-prerequisite for hosting multiple
//! anchor languages (LANGTS-005) — adding a new anchor means adding a module
//! under `parser/extract/<lang>.rs` and a dispatch arm, never editing the
//! walker with an `if lang == …` cascade.
//!
//! The cross-anchor durable ADR for this trait is deferred to RSTLAN (audit
//! §8); the shape is captured here inline and against the T3 checklist §3
//! suggested interface (`route (Language, Tree, source) through the trait, not
//! via a string-kind match`).

pub mod clike;
pub mod csharp;
pub mod dart;
pub mod go;
pub mod java;
pub mod kotlin;
pub mod python;
pub mod rust;
pub mod tail_common;
pub mod typescript;
pub mod wat;
pub mod zig;

use std::path::Path;

use super::languages::Language;

// `FileSymbols` / `ImportEdge` are plain graph-data carriers; they were
// relocated to `anvil-kernel-types::graph` (ADR-064) so `anvil-graph-cache` can
// name them without depending on this parser crate. Re-exported here so existing
// `crate::parser::extract::{FileSymbols, ImportEdge}` paths keep resolving.
pub use anvil_kernel_types::{
    CallSite, CalleeRef, FileSymbols, ImportEdge, LocalSymbolRef, ReexportEdge,
};

/// A per-language symbol extractor.
///
/// Implementations walk a tree-sitter AST for one language family and emit a
/// [`FileSymbols`]. The orchestrator owns language detection and dispatch;
/// implementations own only the walk. Symbol ids are assigned sequentially
/// starting at `id_offset` so callers can rebase ids across files (see the
/// `extract_symbols` INVARIANT comments in `watch.rs` / `embedded.rs`).
///
/// The trait is deliberately minimal — `(tree, source, file, id_offset)` in,
/// `FileSymbols` out — so a new anchor language needs no orchestrator change
/// beyond a dispatch arm. The durable cross-anchor ADR locking this shape is
/// authored by RSTLAN (audit §8); do not widen the trait speculatively here.
pub trait LanguageExtractor {
    /// Extract symbols and import edges from a parsed tree.
    fn extract(
        &self,
        tree: &tree_sitter::Tree,
        source: &[u8],
        file: &str,
        id_offset: u64,
    ) -> FileSymbols;
}

/// Extract symbols from a tree-sitter AST.
///
/// Language-agnostic: dispatches to the matching [`LanguageExtractor`] keyed on
/// the file's extension via [`Language::from_path`].
///
/// In normal flow `tree` comes from [`Parser::parse_bytes`](crate::parser::Parser::parse_bytes), which rejects
/// unsupported extensions up front — so `file_path` always maps to a known
/// language and the dispatch always reaches an extractor. The
/// unsupported-extension arm is therefore purely defensive against direct
/// misuse (a caller handing in a tree whose path has no known grammar): it
/// returns an empty [`FileSymbols`] and trips a `debug_assert!` in debug
/// builds rather than silently losing symbols.
pub fn extract_symbols(
    tree: &tree_sitter::Tree,
    source: &[u8],
    file_path: &Path,
    id_offset: u64,
) -> FileSymbols {
    let mut symbols = extract_symbols_uncapped(tree, source, file_path, id_offset);
    cap_call_sites(&mut symbols);
    symbols
}

/// Per-file `Calls` parity across the anchor languages (ADR-086 §1), recorded
/// here because the cap and the extractors share this one orchestrator:
///
/// - **TypeScript/JS** lift both `call_expression` *and* `new_expression` —
///   constructor calls (`new Foo()`) become `Calls` edges to the class.
/// - **Rust** lifts `call_expression` and `method_call_expression`; a
///   `macro_invocation` (`println!`) is a distinct node kind and is **not** a
///   call site (consistent with pass 1's symbol-only scope).
/// - **Python** lifts `call` nodes; a bare class instantiation (`Foo()`) only
///   yields an edge when `Foo` resolves to a callable symbol at lift time.
///
/// The asymmetry is intentional, not a bug — a `find_callers` consumer reads it
/// through the per-caller `heuristic` and the report's `partial` marker, never as
/// an exact cross-language guarantee.
fn extract_symbols_uncapped(
    tree: &tree_sitter::Tree,
    source: &[u8],
    file_path: &Path,
    id_offset: u64,
) -> FileSymbols {
    let file = file_path.to_string_lossy().to_string();
    match Language::from_path(file_path) {
        Some(Language::TypeScript | Language::Tsx | Language::JavaScript | Language::Jsx) => {
            typescript::TypeScriptExtractor.extract(tree, source, &file, id_offset)
        }
        Some(Language::Rust) => rust::RustExtractor.extract(tree, source, &file, id_offset),
        Some(Language::Python) => python::PythonExtractor.extract(tree, source, &file, id_offset),
        // Tail-language wave (LANGTAIL) — T1 extractors. Each owns only its
        // grammar walk; this dispatch is the sole orchestrator change per the
        // K1 contract (no `if lang == …` cascade in the walker).
        Some(Language::Dart) => dart::DartExtractor.extract(tree, source, &file, id_offset),
        Some(Language::Go) => go::GoExtractor.extract(tree, source, &file, id_offset),
        Some(Language::Java) => java::JavaExtractor.extract(tree, source, &file, id_offset),
        Some(Language::Kotlin) => kotlin::KotlinExtractor.extract(tree, source, &file, id_offset),
        Some(Language::CSharp) => csharp::CSharpExtractor.extract(tree, source, &file, id_offset),
        Some(Language::C) => clike::CExtractor.extract(tree, source, &file, id_offset),
        Some(Language::Cpp) => clike::CppExtractor.extract(tree, source, &file, id_offset),
        // Tail-language wave 2 (LTW2) — T1 extractors.
        Some(Language::Zig) => zig::ZigExtractor.extract(tree, source, &file, id_offset),
        Some(Language::Wat) => wat::WatExtractor.extract(tree, source, &file, id_offset),
        None => {
            // Reaching here means a caller passed a tree whose path has no
            // known grammar — `parse_bytes` rejects those up front, so every
            // real call site is pre-filtered. Assert the invariant in debug
            // builds so a future caller that bypasses the parser surfaces the
            // mismatch loudly instead of silently losing all symbols; release
            // builds still degrade to an empty result rather than panicking.
            debug_assert!(
                false,
                "extract_symbols called on unsupported path `{file}` — caller \
                 must pre-filter via Language::from_path / parse_bytes"
            );
            FileSymbols {
                file,
                symbols: Vec::new(),
                imports: Vec::new(),
                reexports: Vec::new(),
                calls: Vec::new(),
                calls_partial: false,
                has_unresolved_dynamic_import: false,
                content_hash: None,
            }
        }
    }
}

/// Bound a file's extracted call sites to [`MAX_CALL_SITES`] (ADR-086 §3).
///
/// Over the cap, keep the first `MAX_CALL_SITES` in the extractor's deterministic
/// walk order and set [`FileSymbols::calls_partial`] so the lift cost stays
/// bounded (ADR-031) and the daemon can mark the egress caller set `partial`
/// (GCALL-007 CALL-1) rather than silently under-report. A file at or under the
/// cap is untouched and stays honest (`calls_partial` already `false`).
fn cap_call_sites(symbols: &mut FileSymbols) {
    if symbols.calls.len() > anvil_kernel_types::MAX_CALL_SITES {
        symbols.calls.truncate(anvil_kernel_types::MAX_CALL_SITES);
        symbols.calls_partial = true;
    }
}

#[cfg(test)]
mod cap_tests {
    use super::*;
    use anvil_kernel_types::{CallSite, CalleeRef, LocalSymbolRef, MAX_CALL_SITES, SymbolKind};

    fn call(name: &str) -> CallSite {
        CallSite {
            from: LocalSymbolRef {
                kind: SymbolKind::Module,
                name: String::new(),
                ordinal: 0,
                module_scope: true,
            },
            callee: CalleeRef {
                name: name.to_string(),
                via_import: None,
            },
            line: 1,
        }
    }

    fn symbols_with(call_count: usize) -> FileSymbols {
        FileSymbols {
            file: "gen.ts".to_string(),
            calls: (0..call_count).map(|i| call(&format!("f{i}"))).collect(),
            ..FileSymbols::default()
        }
    }

    #[test]
    fn over_cap_truncates_and_marks_partial() {
        let mut symbols = symbols_with(MAX_CALL_SITES + 50);
        cap_call_sites(&mut symbols);
        assert_eq!(
            symbols.calls.len(),
            MAX_CALL_SITES,
            "call sites bounded to the cap"
        );
        assert!(symbols.calls_partial, "over-cap file is marked partial");
        // The kept prefix is the deterministic walk-order head, not a tail.
        assert_eq!(symbols.calls[0].callee.name, "f0");
    }

    #[test]
    fn at_or_under_cap_is_untouched_and_honest() {
        let mut symbols = symbols_with(MAX_CALL_SITES);
        cap_call_sites(&mut symbols);
        assert_eq!(symbols.calls.len(), MAX_CALL_SITES);
        assert!(
            !symbols.calls_partial,
            "a file exactly at the cap is not partial"
        );
    }
}
