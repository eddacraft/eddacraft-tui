//! Shared helpers for the LANGTAIL tail-wave T1 extractors.
//!
//! T1 (Parsed) extraction is deliberately shallow — top-level functions,
//! classes/types, and imports, plus one level of methods inside type bodies so
//! the symbol graph names methods as `Owner.method` consistently with the
//! anchor extractors (TS-G2 / Rust). It emits **no** call sites: every tail
//! language carries `calls: []` / `calls_partial: false`. Per-language
//! anti-pattern catalogues, suppression syntax, and policy hooks are T2/T3
//! anchor work and are explicitly out of scope here (see the module spec).
//!
//! These helpers keep each per-language walker focused on its grammar's node
//! kinds; the cross-cutting mechanics (symbol emission, id counting, import
//! edges, visibility shapes) live here once.

use anvil_kernel_types::{SymbolNode, TrustLevel, Visibility};

use super::{FileSymbols, ImportEdge};

/// UTF-8 text of a node's source span (lossily empty on the impossible
/// non-UTF-8 case — the parser feed is always UTF-8).
pub(super) fn node_text(node: tree_sitter::Node, source: &[u8]) -> String {
    node.utf8_text(source).unwrap_or_default().to_string()
}

/// Text of a child accessed by field name, if present.
pub(super) fn field_text(node: tree_sitter::Node, field: &str, source: &[u8]) -> Option<String> {
    node.child_by_field_name(field)
        .map(|n| node_text(n, source))
        .filter(|s| !s.is_empty())
}

/// Emit a named symbol, advancing the id counter. A no-op for an empty name so
/// a name-less or malformed declaration is skipped rather than emitting a blank
/// symbol (a missed symbol is a missed signal, never a false one).
pub(super) fn push_symbol(
    symbols: &mut Vec<SymbolNode>,
    next_id: &mut u64,
    file: &str,
    kind: anvil_kernel_types::SymbolKind,
    name: String,
    visibility: Visibility,
) {
    if name.is_empty() {
        return;
    }
    symbols.push(SymbolNode {
        id: *next_id,
        kind,
        name,
        visibility,
        file: file.to_string(),
        trust_level: TrustLevel::default(),
    });
    *next_id += 1;
}

/// Build a file→module import edge anchored at `node`'s 1-based start line.
pub(super) fn import_edge(file: &str, to_source: String, node: tree_sitter::Node) -> ImportEdge {
    ImportEdge {
        from_file: file.to_string(),
        to_source,
        // tree-sitter rows are 0-based; ImportEdge.line is 1-based (0 = unknown).
        line: u32::try_from(node.start_position().row + 1).unwrap_or(0),
    }
}

/// An empty result carrier — used by the defensive arms and as the shape every
/// tail extractor returns (no re-exports, no call sites at T1).
pub(super) fn finish(
    file: &str,
    symbols: Vec<SymbolNode>,
    imports: Vec<ImportEdge>,
) -> FileSymbols {
    FileSymbols {
        file: file.to_string(),
        symbols,
        imports,
        reexports: Vec::new(),
        calls: Vec::new(),
        calls_partial: false,
    }
}

/// Strip one matching pair of surrounding quotes (`"…"`, `'…'`) or include
/// brackets (`<…>`) from an import/include literal, returning the inner module
/// path. Leaves an unquoted string untouched.
pub(super) fn strip_delims(s: &str) -> String {
    let s = s.trim();
    let bytes = s.as_bytes();
    if bytes.len() >= 2 {
        let (first, last) = (bytes[0], bytes[bytes.len() - 1]);
        let matched = matches!((first, last), (b'"', b'"') | (b'\'', b'\'') | (b'<', b'>'));
        if matched {
            return s[1..s.len() - 1].to_string();
        }
    }
    s.to_string()
}

/// Whether a node has a direct child whose text is exactly `keyword` (used for
/// modifier scans like `public` / `private` / `internal` / `static`).
pub(super) fn has_modifier(node: tree_sitter::Node, keyword: &str, source: &[u8]) -> bool {
    let mut cursor = node.walk();
    // Modifiers may sit under a wrapping `modifiers` node (Java/C#) or be loose
    // sibling children (Kotlin/C). Scan one level, then one level under any
    // `modifiers`/`modifier` wrapper.
    for child in node.children(&mut cursor) {
        if node_text(child, source) == keyword {
            return true;
        }
        if matches!(child.kind(), "modifiers" | "modifier") {
            let mut inner = child.walk();
            for m in child.children(&mut inner) {
                if node_text(m, source) == keyword {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_delims_handles_each_form() {
        assert_eq!(strip_delims("\"local.h\""), "local.h");
        assert_eq!(strip_delims("<stdio.h>"), "stdio.h");
        assert_eq!(strip_delims("'dart:io'"), "dart:io");
        assert_eq!(strip_delims("java.util.List"), "java.util.List");
        assert_eq!(strip_delims("\"\""), "");
    }
}
