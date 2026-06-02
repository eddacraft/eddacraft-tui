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

pub mod typescript;

use std::path::Path;

use super::languages::Language;

// `FileSymbols` / `ImportEdge` are plain graph-data carriers; they were
// relocated to `anvil-kernel-types::graph` (ADR-064) so `anvil-graph-cache` can
// name them without depending on this parser crate. Re-exported here so existing
// `crate::parser::extract::{FileSymbols, ImportEdge}` paths keep resolving.
pub use anvil_kernel_types::{FileSymbols, ImportEdge};

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
    let file = file_path.to_string_lossy().to_string();
    match Language::from_path(file_path) {
        Some(Language::TypeScript | Language::Tsx | Language::JavaScript | Language::Jsx) => {
            typescript::TypeScriptExtractor.extract(tree, source, &file, id_offset)
        }
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
            }
        }
    }
}
