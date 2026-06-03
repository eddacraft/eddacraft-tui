//! DSV-005: the kernel-backed [`SymbolParser`] the intercept daemon enriches
//! its save-time verdict with.
//!
//! ADR-064 keeps the resident daemon (`anvil-intercept`) free of tree-sitter:
//! the daemon defines the [`SymbolParser`] trait (a Messaging Gateway) and never
//! links a parser. `anvil-cli` deps both the kernel (the tree-sitter parser) and
//! the daemon, so the parser links into the **binary**, not the daemon crate —
//! the `daemon_dep_boundary` guard stays green. This module is the
//! tree-sitter-backed impl injected via `ForegroundOpts::with_symbol_parser`.
//!
//! The daemon hands [`SymbolParser::parse`] the exact openat2-guarded bytes it
//! read and hashed, so the parsed symbols provably describe the attested bytes
//! (the Content Enricher "enrich the message you hold" property — no second read
//! that could race the edit, the B2 hazard).
#![cfg(unix)]

use std::hash::{Hash, Hasher};
use std::path::Path;

use anvil_intercept::save_time::SymbolParser;
use anvil_kernel::parser::Parser;
use anvil_kernel::parser::extract::extract_symbols;
use anvil_kernel_types::FileSymbols;

/// Per-file id space: the low [`SYMBOL_ID_SHIFT`] bits of a symbol id are the
/// parser's 0-based within-file index; the high bits are a path-derived file
/// tag. 2^20 ≈ 1M symbols per file is far beyond any real source file.
const SYMBOL_ID_SHIFT: u32 = 20;

/// Mask for the path-derived file tag (the bits above [`SYMBOL_ID_SHIFT`]).
const FILE_TAG_MASK: u64 = (1u64 << (64 - SYMBOL_ID_SHIFT)) - 1;

/// A stable, collision-resistant symbol-id base for `path`.
///
/// `extract_symbols` assigns 0-based sequential ids per file; feeding every file
/// `id_offset = 0` would collide ids across files in the daemon's warm graph.
/// This derives a per-file base from the path so (a) re-parsing the same path
/// yields the same base (stable identity for the cache to match against) and
/// (b) distinct paths get distinct id ranges (no cross-file collision). The
/// hash is deterministic within a process; the daemon's cache is in-memory and
/// rebuilt on restart, so cross-restart stability is not required.
fn stable_symbol_id_base(path: &Path) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    (hasher.finish() & FILE_TAG_MASK) << SYMBOL_ID_SHIFT
}

/// The tree-sitter-backed parser. Stateless — a fresh [`Parser`] is built per
/// call (tree-sitter's `Parser` is not `Sync`), which is acceptable on the
/// single-file interactive verdict path.
#[derive(Debug, Default)]
pub struct KernelSymbolParser;

impl KernelSymbolParser {
    /// Construct the parser. Cheap — no per-instance state.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl SymbolParser for KernelSymbolParser {
    fn parse(&self, path: &Path, bytes: &[u8]) -> Option<FileSymbols> {
        // An unsupported extension / language-init failure is a clean `None`
        // (⇒ a safe `Partial` verdict), never a panic.
        let mut parser = Parser::new();
        let result = parser.parse_bytes(path, bytes).ok()?;
        Some(extract_symbols(
            &result.tree,
            bytes,
            path,
            stable_symbol_id_base(path),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_typescript_public_surface() {
        let parser = KernelSymbolParser::new();
        let symbols = parser
            .parse(
                Path::new("src/a.ts"),
                b"export function foo() { return 1; }",
            )
            .expect("a .ts file parses");
        assert_eq!(symbols.file, "src/a.ts");
        assert!(
            symbols.symbols.iter().any(|s| s.name == "foo"),
            "the public `foo` is extracted: {symbols:?}",
        );
    }

    #[test]
    fn unsupported_extension_is_none_not_panic() {
        let parser = KernelSymbolParser::new();
        assert!(
            parser.parse(Path::new("README.md"), b"# title").is_none(),
            "an unsupported language is a safe None (⇒ Partial)",
        );
    }

    #[test]
    fn id_base_is_stable_per_path_and_distinct_across_paths() {
        // Same path → same base (the cache can match a re-parse).
        assert_eq!(
            stable_symbol_id_base(Path::new("src/a.ts")),
            stable_symbol_id_base(Path::new("src/a.ts")),
        );
        // Distinct paths → distinct bases (no cross-file id collision).
        assert_ne!(
            stable_symbol_id_base(Path::new("src/a.ts")),
            stable_symbol_id_base(Path::new("src/b.ts")),
        );
        // The base leaves the low bits free for within-file ids.
        assert_eq!(
            stable_symbol_id_base(Path::new("src/a.ts")) & ((1 << SYMBOL_ID_SHIFT) - 1),
            0
        );
    }

    /// The parsed surface matches what the daemon's certify compares (by name),
    /// so a real parse drives the same Certified/Partial decision the fake
    /// parser proves in `anvil-intercept`.
    #[test]
    fn re_parsing_same_bytes_is_deterministic() {
        let parser = KernelSymbolParser::new();
        let bytes = b"export function foo() {}\nexport const bar = 1;";
        let first = parser.parse(Path::new("src/a.ts"), bytes).expect("parse");
        let second = parser.parse(Path::new("src/a.ts"), bytes).expect("parse");
        let names = |fs: &FileSymbols| {
            let mut n: Vec<String> = fs.symbols.iter().map(|s| s.name.clone()).collect();
            n.sort();
            n
        };
        assert_eq!(
            names(&first),
            names(&second),
            "re-parsing the same bytes yields the same surface",
        );
    }
}
