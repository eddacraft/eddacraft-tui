//! WebAssembly-text (`.wat`/`.wast`) symbol and import extractor (LTW2-002) — T1.
//!
//! | WAT construct                  | `SymbolKind` | Note                          |
//! | ------------------------------ | ------------ | ----------------------------- |
//! | `(module $m …)`                | `Module`     | named modules only            |
//! | `(func $f …)`                  | `Function`   | named funcs only              |
//! | `(type $t …)`                  | `TypeAlias`  |                               |
//! | `(export "name" …)`            | `Export`     | the module's public surface   |
//!
//! Imports come from `(import "module" "name" …)`; the edge target is the
//! imported **module** string (the first name). Globals, tables, memories,
//! data/elem segments, and `start` are out of T1 scope. WAT identifiers carry a
//! `$` sigil, stripped from symbol names; anonymous (index-only) funcs/types
//! carry no identifier and are skipped. WAT has no visibility ladder — funcs and
//! types are [`Visibility::Internal`]; an `export` entry is the public boundary
//! and is [`Visibility::Public`]. No call sites at T1.

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
    for child in module.children(&mut cursor) {
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
    fn anonymous_module_and_func_emit_no_blank_symbols() {
        // An unnamed module with an index-only func has no identifiers — it must
        // not emit blank symbols (a missed symbol beats a false one).
        let fs = extract("(module\n  (func (param i32) (result i32) local.get 0))\n");
        assert!(fs.symbols.is_empty(), "{:?}", fs.symbols);
        assert!(fs.imports.is_empty());
    }
}
