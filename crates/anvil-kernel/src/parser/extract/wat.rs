//! WebAssembly-text (`.wat`/`.wast`) symbol and import extractor (LTW2-002) — T1.
//!
//! | WAT construct                  | `SymbolKind` | Note                          |
//! | ------------------------------ | ------------ | ----------------------------- |
//! | `(module $m …)`                | `Module`     | named modules only            |
//! | `(func $f …)`                  | `Function`   | named funcs only              |
//! | `(type $t …)`                  | `TypeAlias`  |                               |
//! | `(export "name" …)`            | `Export`     | the module's public surface   |
//!
//! Exports and imports are captured in **both** forms: the standalone
//! `(export "n" …)` / `(import "m" "n" …)` module fields **and** the inline
//! abbreviations `(func (export "n") …)` / `(func $f (import "m" "n") …)`
//! (common in wasm-bindgen / WASI output), which the grammar nests inside the
//! owning field. An import edge targets the imported **module** string (the
//! first name). Globals, tables, memories, data/elem segments, and `start` are
//! not emitted as symbols, but inline import/export descriptors *within* them
//! are still captured.
//!
//! WAT identifiers carry a `$` sigil, stripped from symbol names; anonymous
//! (index-only) funcs/types carry no identifier and are skipped. WAT has no
//! visibility ladder — funcs and types are [`Visibility::Internal`]; an `export`
//! entry is the public boundary and is [`Visibility::Public`]. No call sites at
//! T1.
//!
//! **T1 limitation:** import/export name strings are taken verbatim (quotes
//! stripped); WAT escape sequences (`"env\\00"`) are not decoded, so an escaped
//! name keeps its literal bytes.

use anvil_kernel_types::{SymbolKind, SymbolNode, Visibility};

use super::tail_common::{finish, import_edge, node_text, push_symbol, strip_delims};
use super::{FileSymbols, ImportEdge, LanguageExtractor};

/// Extractor for WebAssembly text (`.wat`/`.wast`).
pub struct WatExtractor;

impl LanguageExtractor for WatExtractor {
    fn extract(
        &self,
        tree: &tree_sitter::Tree,
        source: &[u8],
        file: &str,
        id_offset: u64,
    ) -> FileSymbols {
        let mut symbols = Vec::new();
        let mut imports = Vec::new();
        let mut next_id = id_offset;

        let root = tree.root_node();
        let mut cursor = root.walk();
        for node in root.children(&mut cursor) {
            if node.kind() == "module" {
                emit_module(node, source, file, &mut symbols, &mut next_id, &mut imports);
            }
        }

        finish(file, symbols, imports)
    }
}

fn emit_module(
    module: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    next_id: &mut u64,
    imports: &mut Vec<ImportEdge>,
) {
    // A named module is itself a symbol.
    if let Some(name) = wat_ident(module, source) {
        push_symbol(
            symbols,
            next_id,
            file,
            SymbolKind::Module,
            name,
            Visibility::Internal,
        );
    }

    let mut cursor = module.walk();
    for child in module.named_children(&mut cursor) {
        // Each `module_field` wraps one specific field node; tolerate a direct
        // `module_field_*` child too.
        let field = if child.kind() == "module_field" {
            child.named_child(0)
        } else {
            Some(child)
        };
        let Some(field) = field else { continue };

        match field.kind() {
            "module_field_func" => {
                if let Some(name) = wat_ident(field, source) {
                    push_symbol(
                        symbols,
                        next_id,
                        file,
                        SymbolKind::Function,
                        name,
                        Visibility::Internal,
                    );
                }
            }
            "module_field_type" => {
                if let Some(name) = wat_ident(field, source) {
                    push_symbol(
                        symbols,
                        next_id,
                        file,
                        SymbolKind::TypeAlias,
                        name,
                        Visibility::Internal,
                    );
                }
            }
            "module_field_export" => {
                if let Some(name) = first_name(field, source) {
                    push_symbol(
                        symbols,
                        next_id,
                        file,
                        SymbolKind::Export,
                        name,
                        Visibility::Public,
                    );
                }
            }
            "module_field_import" => {
                if let Some(module_name) = first_name(field, source) {
                    imports.push(import_edge(file, module_name, field));
                }
            }
            _ => {}
        }

        // Inline `(func (export "x") …)` / `(func $f (import "m" "n") …)` forms:
        // the grammar nests an `export`/`import` node inside the field (directly
        // for funcs, one wrapper deeper for memory/table/global). Scan a bounded
        // depth so the abbreviated forms — common in wasm-bindgen/WASI output —
        // still yield Export symbols and import edges, without walking into
        // instruction bodies.
        scan_inline(
            field,
            INLINE_SCAN_DEPTH,
            source,
            file,
            symbols,
            next_id,
            imports,
        );
    }
}

/// Max depth below a `module_field` that the inline import/export scan descends.
/// Inline descriptors sit at depth 1 (func) or 2 (memory/table/global via a
/// `*_fields_*` wrapper); 3 covers them while excluding deep instruction bodies.
const INLINE_SCAN_DEPTH: u32 = 3;

/// Emit Export symbols and import edges for inline `export`/`import` descriptor
/// nodes nested within a module field, bounded to [`INLINE_SCAN_DEPTH`].
fn scan_inline(
    node: tree_sitter::Node,
    depth: u32,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    next_id: &mut u64,
    imports: &mut Vec<ImportEdge>,
) {
    if depth == 0 {
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "import" => {
                if let Some(module_name) = first_name(child, source) {
                    imports.push(import_edge(file, module_name, child));
                }
            }
            "export" => {
                if let Some(name) = first_name(child, source) {
                    push_symbol(
                        symbols,
                        next_id,
                        file,
                        SymbolKind::Export,
                        name,
                        Visibility::Public,
                    );
                }
            }
            // Descend through wrappers (e.g. `memory_fields_type`) to reach a
            // nested descriptor; the depth cap keeps this off the hot path.
            _ => scan_inline(child, depth - 1, source, file, symbols, next_id, imports),
        }
    }
}

/// The `identifier`-field value of a WAT node, with its `$` sigil stripped;
/// `None` for an anonymous (index-only) construct.
fn wat_ident(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let id = node.child_by_field_name("identifier")?;
    let text = node_text(id, source);
    let name = text.strip_prefix('$').unwrap_or(&text).trim();
    (!name.is_empty()).then(|| name.to_string())
}

/// The first `(name "…")` string under a field, quotes stripped — the module
/// namespace of an import, or the exported name of an export.
fn first_name(field: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let mut cursor = field.walk();
    for child in field.children(&mut cursor) {
        if child.kind() == "name" {
            let text = strip_delims(&node_text(child, source));
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::languages::Language;

    fn extract(src: &str) -> FileSymbols {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&Language::Wat.ts_language()).unwrap();
        let tree = parser.parse(src, None).unwrap();
        WatExtractor.extract(&tree, src.as_bytes(), "mod.wat", 0)
    }

    #[test]
    fn extracts_module_funcs_types_exports_imports() {
        let src = "(module $mymod\n  (import \"env\" \"log\" (func $log (param i32)))\n  (type $binop (func (param i32 i32) (result i32)))\n  (func $add (param i32) (result i32) local.get 0)\n  (export \"add\" (func $add))\n  (global $g i32 (i32.const 0)))\n";
        let fs = extract(src);

        let module = fs.symbols.iter().find(|s| s.name == "mymod").unwrap();
        assert_eq!(module.kind, SymbolKind::Module);

        let add_fn = fs
            .symbols
            .iter()
            .find(|s| s.name == "add" && s.kind == SymbolKind::Function)
            .expect("func $add");
        assert_eq!(add_fn.visibility, Visibility::Internal);

        let binop = fs.symbols.iter().find(|s| s.name == "binop").unwrap();
        assert_eq!(binop.kind, SymbolKind::TypeAlias);

        // The `(export "add" …)` entry is a distinct, public symbol — same name
        // as the func, different kind.
        let add_export = fs
            .symbols
            .iter()
            .find(|s| s.name == "add" && s.kind == SymbolKind::Export)
            .expect("export add");
        assert_eq!(add_export.visibility, Visibility::Public);

        // Import edge targets the imported *module* namespace ("env"), not the
        // field name ("log").
        let targets: Vec<_> = fs.imports.iter().map(|i| i.to_source.as_str()).collect();
        assert_eq!(targets, vec!["env"], "import edges: {targets:?}");

        // Globals are out of T1 scope; no call sites at T1.
        assert!(!fs.symbols.iter().any(|s| s.name == "g"), "global leaked");
        assert!(fs.calls.is_empty() && !fs.calls_partial);
    }

    #[test]
    fn inline_export_and_import_forms_are_captured() {
        // The abbreviated forms nest `export`/`import` inside the field:
        //   (func (export "run") …)        — inline export, direct child
        //   (func $log (import "env" …) …) — inline import, direct child
        //   (memory (import "js" …) …)     — inline import via memory wrapper
        // All three must surface, mirroring the standalone forms.
        let src = "(module\n  (func $log (import \"env\" \"log\") (param i32))\n  (func (export \"run\") (result i32) i32.const 42)\n  (memory (import \"js\" \"mem\") 1))\n";
        let fs = extract(src);

        let exports: Vec<_> = fs
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Export)
            .map(|s| s.name.as_str())
            .collect();
        assert_eq!(exports, vec!["run"], "inline export: {exports:?}");
        assert!(
            fs.symbols
                .iter()
                .any(|s| s.name == "run" && s.visibility == Visibility::Public)
        );

        let mut targets: Vec<_> = fs.imports.iter().map(|i| i.to_source.as_str()).collect();
        targets.sort_unstable();
        assert_eq!(targets, vec!["env", "js"], "inline imports: {targets:?}");

        // `$log` is still a Function symbol; no call sites at T1.
        assert!(
            fs.symbols
                .iter()
                .any(|s| s.name == "log" && s.kind == SymbolKind::Function)
        );
        assert!(fs.calls.is_empty() && !fs.calls_partial);
    }

    #[test]
    fn anonymous_module_and_func_emit_no_blank_symbols() {
        // An unnamed module with an index-only func has no identifiers — it must
        // not emit blank symbols (a missed symbol beats a false one).
        let fs = extract("(module\n  (func (param i32) (result i32) local.get 0))\n");
        assert!(fs.symbols.is_empty(), "{:?}", fs.symbols);
        assert!(fs.imports.is_empty());
    }
}
