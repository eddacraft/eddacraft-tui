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

use std::path::Path;

use anvil_intercept::save_time::SymbolParser;
use anvil_kernel::parser::Parser;
use anvil_kernel::parser::extract::extract_symbols;
use anvil_kernel_types::FileSymbols;

/// Per-file id space: the low [`SYMBOL_ID_SHIFT`] bits of a symbol id are the
/// parser's 0-based within-file index; the high bits are a path-derived file
/// tag. 2^20 ≈ 1M symbols per file is far beyond any real source file.
const SYMBOL_ID_SHIFT: u32 = 20;

/// Mask that truncates the path hash to the bits above [`SYMBOL_ID_SHIFT`]
/// (the file-tag space). Applied to the hash *before* the left shift.
const PATH_HASH_MASK: u64 = (1u64 << (64 - SYMBOL_ID_SHIFT)) - 1;

/// A stable, collision-resistant symbol-id base for `path`.
///
/// `extract_symbols` assigns 0-based sequential ids per file; feeding every file
/// `id_offset = 0` would collide ids across files in the daemon's warm graph.
/// This derives a per-file base from the path so (a) re-parsing the same path
/// yields the same base (stable identity for the cache to match against) and
/// (b) distinct paths get distinct id ranges. A residual hash collision is
/// **safe**, not a false attestation: two files sharing a base produce a
/// `DuplicateSymbol` on apply, which `certify` reports as `UnreliableGraph` ⇒
/// a conservative `Partial`. A stable per-daemon range allocator would remove
/// even that precision loss (a future option if collisions ever bite).
///
/// Uses FNV-1a over the path bytes — a fixed, auditable algorithm — rather than
/// `DefaultHasher`, whose hash is an undocumented stdlib internal that could
/// change between releases.
fn stable_symbol_id_base(path: &Path) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = FNV_OFFSET;
    for byte in path.as_os_str().as_encoded_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    (hash & PATH_HASH_MASK) << SYMBOL_ID_SHIFT
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
        // Reject non-UTF-8 content: the extractor reads identifier text via
        // `utf8_text`, which silently yields "" on invalid UTF-8. Two distinct
        // bad-UTF-8 identifiers would both become the empty name ⇒ the surface
        // could read "unchanged" across a real rename (a B2 false-attestation).
        // A non-UTF-8 file is simply not certifiable here ⇒ safe `Partial`.
        if std::str::from_utf8(bytes).is_err() {
            tracing::debug!(
                target: "anvil_intercept::save_time",
                path = %path.display(),
                "symbol parse skipped: file is not valid UTF-8 (⇒ Partial)",
            );
            return None;
        }
        // An unsupported extension / language-init failure is a clean `None`
        // (⇒ a safe `Partial` verdict), never a panic.
        let mut parser = Parser::new();
        let result = match parser.parse_bytes(path, bytes) {
            Ok(result) => result,
            Err(error) => {
                // A supported-extension file that fails to parse degrades to
                // `Partial`; log it so an operator can tell that apart from a
                // cold-cache `Partial` or an unsupported language.
                tracing::debug!(
                    target: "anvil_intercept::save_time",
                    path = %path.display(),
                    %error,
                    "symbol parse failed (⇒ Partial)",
                );
                return None;
            }
        };
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
    fn non_utf8_bytes_are_none_not_a_false_surface() {
        // Invalid UTF-8 in identifier bytes would render as the empty name via
        // `utf8_text`, which could read as an unchanged surface across a real
        // rename (a B2 hazard). Reject it ⇒ safe `Partial`.
        let parser = KernelSymbolParser::new();
        let bytes = b"export function \xff\xfe() {}";
        assert!(
            parser.parse(Path::new("src/a.ts"), bytes).is_none(),
            "non-UTF-8 content is not certifiable here",
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

    /// Capstone integration: the REAL `KernelSymbolParser` (with its
    /// path-stable id base) drives a `Certified` verdict end to end through the
    /// daemon's `SaveTimeConn`. A first save warms the cache (cold ⇒ `Partial`);
    /// a second save of the same clean body is self-contained ⇒ `Certified`.
    /// This exercises the real id base across two parses (call 2 only certifies
    /// because the base is stable), closing the gap left by the daemon-side
    /// fake-parser test.
    #[test]
    fn real_parser_certifies_repeat_save_through_daemon() {
        use anvil_checks::antipattern::types::AntipatternCheckConfig;
        use anvil_intercept::confinement::Confinement;
        use anvil_intercept::ipc::SaveTimeDispatch;
        use anvil_intercept::save_time::{SaveTimeConn, SaveTimeState};
        use anvil_intercept::workspace_pool::WorkScheduler;
        use anvil_intercept_proto::protocol::{
            ChangeDescriptor, ChangeKindWire, Coverage, ValidatePathsRequest,
        };

        let tmp = tempfile::tempdir().expect("tempdir");
        let src = tmp.path().join("src");
        std::fs::create_dir(&src).expect("mkdir");
        std::fs::write(src.join("a.ts"), b"export function foo() { return 1; }").expect("write");

        let state = SaveTimeState::new(
            WorkScheduler::new().expect("scheduler"),
            AntipatternCheckConfig::default(),
            Confinement::open_default(),
        )
        .with_parser(std::sync::Arc::new(KernelSymbolParser::new()));
        let mut conn = SaveTimeConn::new(&state);

        let request = ValidatePathsRequest {
            workspace_root: tmp.path().to_string_lossy().into_owned(),
            paths: vec![ChangeDescriptor {
                path: "src/a.ts".to_string(),
                change: ChangeKindWire::Modified,
                content_hash: None,
                mtime: None,
            }],
        };

        // First save: cold cache ⇒ Partial, but it warms the graph with foo.
        let first = conn.validate_paths(&request).expect("admitted");
        assert_eq!(
            first.coverage,
            Coverage::Partial,
            "cold first save is Partial"
        );
        // Second save of the same clean body: self-contained ⇒ Certified.
        let second = conn.validate_paths(&request).expect("admitted");
        assert_eq!(
            second.coverage,
            Coverage::Certified,
            "a self-contained re-save certifies through the real parser",
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
