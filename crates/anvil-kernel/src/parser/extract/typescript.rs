//! TypeScript / JavaScript symbol extractor.
//!
//! This is the canonical TS-family [`LanguageExtractor`] impl. It is a verbatim
//! port of the original `extract.rs` walker (LANGTS-005 K1) — the parity test
//! in this module asserts the emitted [`FileSymbols`] are identical to the
//! pre-refactor behaviour.

use anvil_kernel_types::{SymbolKind, SymbolNode, TrustLevel, Visibility};

use super::{FileSymbols, ImportEdge, LanguageExtractor};

/// Extractor for the TypeScript / TSX / JavaScript / JSX family.
///
/// The tree-sitter TS and JS grammars share the node kinds this walker
/// inspects (`function_declaration`, `class_declaration`, `export_statement`,
/// `import_statement`, CJS `require` / `module.exports`), so a single impl
/// covers the whole family. Rust / Python anchors get their own modules.
pub struct TypeScriptExtractor;

impl LanguageExtractor for TypeScriptExtractor {
    fn extract(
        &self,
        tree: &tree_sitter::Tree,
        source: &[u8],
        file: &str,
        id_offset: u64,
    ) -> FileSymbols {
        let root = tree.root_node();
        let mut symbols = Vec::new();
        let mut imports = Vec::new();
        let mut next_id = id_offset;

        extract_from_node(root, source, file, &mut symbols, &mut imports, &mut next_id);

        FileSymbols {
            file: file.to_string(),
            symbols,
            imports,
        }
    }
}

fn extract_from_node(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    imports: &mut Vec<ImportEdge>,
    next_id: &mut u64,
) {
    match node.kind() {
        "function_declaration" => extract_function(node, source, file, symbols, next_id),
        "class_declaration" => extract_class(node, source, file, symbols, next_id),
        // TS-G1: type-shape declarations become first-class symbols.
        "interface_declaration" => {
            extract_named_decl(node, source, file, symbols, next_id, SymbolKind::Interface);
        }
        "type_alias_declaration" => {
            extract_named_decl(node, source, file, symbols, next_id, SymbolKind::TypeAlias);
        }
        "enum_declaration" => {
            extract_named_decl(node, source, file, symbols, next_id, SymbolKind::Enum);
        }
        "export_statement" => extract_export(node, source, file, symbols, imports, next_id),
        "import_statement" => extract_import(node, source, file, imports),
        "call_expression" => extract_require(node, source, file, imports),
        "assignment_expression" => extract_cjs_export(node, source, file, symbols, next_id),
        "lexical_declaration" => extract_lexical(node, source, file, symbols, next_id),
        _ => {}
    }

    // Recurse into children (except for nodes we've already handled).
    // Note: lexical_declaration is NOT excluded — we must recurse into it so
    // that nested call_expression nodes (e.g. `const fs = require('node:fs')`)
    // are visited by extract_require.
    if !matches!(
        node.kind(),
        "function_declaration"
            | "class_declaration"
            | "export_statement"
            // TS-G1 declarations are fully handled by extract_named_decl; their
            // bodies hold only type-level members (method/property signatures,
            // enum members) we don't extract, and skipping recursion prevents a
            // nested initialiser expression (e.g. `require(...)` inside an enum
            // member) from emitting a spurious edge.
            | "interface_declaration"
            | "type_alias_declaration"
            | "enum_declaration"
    ) {
        for i in 0..u32::try_from(node.named_child_count()).unwrap_or(0) {
            if let Some(child) = node.named_child(i) {
                extract_from_node(child, source, file, symbols, imports, next_id);
            }
        }
    }
}

fn extract_function(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    next_id: &mut u64,
) {
    if let Some(name_node) = node.child_by_field_name("name") {
        let name = node_text(name_node, source);
        symbols.push(SymbolNode {
            id: *next_id,
            kind: SymbolKind::Function,
            name,
            visibility: Visibility::Internal,
            file: file.to_string(),
            trust_level: TrustLevel::default(),
        });
        *next_id += 1;
    }
}

fn extract_class(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    next_id: &mut u64,
) {
    let Some(name_node) = node.child_by_field_name("name") else {
        return;
    };
    let class_name = node_text(name_node, source);
    symbols.push(SymbolNode {
        id: *next_id,
        kind: SymbolKind::Class,
        name: class_name.clone(),
        visibility: Visibility::Internal,
        file: file.to_string(),
        trust_level: TrustLevel::default(),
    });
    *next_id += 1;

    // TS-G2: surface each class method as its own symbol, qualifying the name
    // with the owning class (`Owner.method`) so the parent is recoverable
    // without a structural parent edge (deferred — see `SymbolKind::Method`).
    // `class_declaration` is excluded from the generic child recursion in
    // `extract_from_node`, so methods are emitted only here (no double-count).
    if let Some(body) = node.child_by_field_name("body") {
        for i in 0..u32::try_from(body.named_child_count()).unwrap_or(0) {
            let Some(member) = body.named_child(i) else {
                continue;
            };
            if member.kind() != "method_definition" {
                continue;
            }
            if let Some(method_name) = member.child_by_field_name("name") {
                symbols.push(SymbolNode {
                    id: *next_id,
                    kind: SymbolKind::Method,
                    name: format!("{class_name}.{}", node_text(method_name, source)),
                    visibility: Visibility::Internal,
                    file: file.to_string(),
                    trust_level: TrustLevel::default(),
                });
                *next_id += 1;
            }
        }
    }
}

/// Emit a single named declaration — interface / type alias / enum (TS-G1) —
/// as a symbol of `kind`. Visibility defaults to `Internal`; an enclosing
/// `export_statement` upgrades it to `Public` via [`extract_export`].
fn extract_named_decl(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    next_id: &mut u64,
    kind: SymbolKind,
) {
    if let Some(name_node) = node.child_by_field_name("name") {
        symbols.push(SymbolNode {
            id: *next_id,
            kind,
            name: node_text(name_node, source),
            visibility: Visibility::Internal,
            file: file.to_string(),
            trust_level: TrustLevel::default(),
        });
        *next_id += 1;
    }
}

fn extract_export(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    imports: &mut Vec<ImportEdge>,
    next_id: &mut u64,
) {
    if let Some(decl) = node.child_by_field_name("declaration") {
        let before = symbols.len();
        extract_from_node(decl, source, file, symbols, imports, next_id);
        for sym in &mut symbols[before..] {
            sym.visibility = Visibility::Public;
        }
        return;
    }

    // `export default <expression>` uses the "value" field. Try to extract a
    // named symbol from the expression; if it is anonymous (function expression
    // or class expression with no name), emit a synthetic "default" symbol so
    // the module's public API surface is not silently omitted.
    if let Some(value) = node.child_by_field_name("value") {
        let name = value
            .child_by_field_name("name")
            .map(|n| node_text(n, source));
        let sym_name = name.unwrap_or_else(|| String::from("default"));
        let kind = match value.kind() {
            "function_expression" | "arrow_function" => SymbolKind::Function,
            "class" => SymbolKind::Class,
            _ => SymbolKind::Export,
        };
        symbols.push(SymbolNode {
            id: *next_id,
            kind,
            name: sym_name,
            visibility: Visibility::Public,
            file: file.to_string(),
            trust_level: TrustLevel::default(),
        });
        *next_id += 1;
        return;
    }

    let mut handled_clause = false;
    for i in 0..u32::try_from(node.named_child_count()).unwrap_or(0) {
        if let Some(child) = node.named_child(i)
            && child.kind() == "export_clause"
        {
            handled_clause = true;
            extract_export_clause(child, source, file, symbols, next_id);
        }
    }

    // Re-export from another module: `export { x } from './mod'`
    if let Some(source_node) = node.child_by_field_name("source") {
        let raw = node_text(source_node, source);
        let module_path = raw.trim_matches(|c| c == '\'' || c == '"');
        imports.push(ImportEdge {
            from_file: file.to_string(),
            to_source: module_path.to_string(),
            line: node_line(node),
        });
    }

    if !handled_clause {
        symbols.push(SymbolNode {
            id: *next_id,
            kind: SymbolKind::Export,
            name: String::from("*"),
            visibility: Visibility::Public,
            file: file.to_string(),
            trust_level: TrustLevel::default(),
        });
        *next_id += 1;
    }
}

fn extract_export_clause(
    clause: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    next_id: &mut u64,
) {
    for j in 0..u32::try_from(clause.named_child_count()).unwrap_or(0) {
        if let Some(spec) = clause.named_child(j) {
            if spec.kind() != "export_specifier" {
                continue;
            }
            // Use the local name (not the alias) when looking up existing symbols.
            // `export { foo as bar }` should mark the symbol named `foo` as Public.
            let local_name = if let Some(name_node) = spec.child_by_field_name("name") {
                node_text(name_node, source)
            } else {
                continue;
            };
            if let Some(sym) = symbols.iter_mut().find(|s| s.name == local_name) {
                sym.visibility = Visibility::Public;
            } else {
                // No local symbol found — create a public export node so the
                // public API surface is correctly tracked. This handles cases
                // like `export { foo }` where foo is declared after the export,
                // or re-exports from other modules.
                symbols.push(SymbolNode {
                    id: *next_id,
                    kind: SymbolKind::Export,
                    name: local_name,
                    visibility: Visibility::Public,
                    file: file.to_string(),
                    trust_level: TrustLevel::default(),
                });
                *next_id += 1;
            }
        }
    }
}

fn extract_import(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    imports: &mut Vec<ImportEdge>,
) {
    if let Some(source_node) = node.child_by_field_name("source") {
        let raw = node_text(source_node, source);
        let module_path = raw.trim_matches(|c| c == '\'' || c == '"');
        imports.push(ImportEdge {
            from_file: file.to_string(),
            to_source: module_path.to_string(),
            line: node_line(node),
        });
    }
}

fn extract_require(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    imports: &mut Vec<ImportEdge>,
) {
    if let Some(func) = node.child_by_field_name("function")
        && node_text(func, source) == "require"
        && let Some(args) = node.child_by_field_name("arguments")
        && let Some(arg) = args.named_child(0)
    {
        let raw = node_text(arg, source);
        let module_path = raw.trim_matches(|c| c == '\'' || c == '"');
        if !module_path.is_empty() {
            imports.push(ImportEdge {
                from_file: file.to_string(),
                to_source: module_path.to_string(),
                line: node_line(node),
            });
        }
    }
}

fn extract_cjs_export(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    next_id: &mut u64,
) {
    let Some(left) = node.child_by_field_name("left") else {
        return;
    };
    let left_text = node_text(left, source);

    if let Some(prop) = left_text.strip_prefix("module.exports.") {
        // `module.exports.foo = ...` — mark only the specific property as Public
        mark_or_add_public_symbol(prop, file, symbols, next_id);
    } else if left_text == "module.exports" {
        // `module.exports = { foo, bar }` — only mark referenced symbols as Public.
        // If the RHS is not an object literal or is too complex, fall back to
        // marking all symbols as Public (conservative).
        let rhs = node.child_by_field_name("right");
        if let Some(rhs_node) = rhs.filter(|n| n.kind() == "object") {
            let mut property_names = Vec::new();
            for i in 0..u32::try_from(rhs_node.named_child_count()).unwrap_or(0) {
                if let Some(child) = rhs_node.named_child(i) {
                    match child.kind() {
                        "shorthand_property_identifier" => {
                            property_names.push(node_text(child, source));
                        }
                        "pair" => {
                            // Use the property key (exported name), not the value
                            if let Some(key_node) = child.child_by_field_name("key") {
                                property_names.push(node_text(key_node, source));
                            }
                        }
                        _ => {}
                    }
                }
            }
            for name in &property_names {
                mark_or_add_public_symbol(name, file, symbols, next_id);
            }
        } else {
            // RHS is a single identifier or complex expression — mark only that
            // identifier, or fall back to all symbols if unresolvable.
            if let Some(rhs_node) = rhs {
                if rhs_node.kind() == "identifier" {
                    let name = node_text(rhs_node, source);
                    mark_or_add_public_symbol(&name, file, symbols, next_id);
                } else {
                    // Complex RHS (call expression, ternary, etc.) — conservative fallback
                    for sym in symbols.iter_mut().filter(|s| s.file == file) {
                        sym.visibility = Visibility::Public;
                    }
                }
            }
        }
    } else if let Some(prop) = left_text.strip_prefix("exports.") {
        // `exports.foo = ...` — mark only the specific property as Public
        mark_or_add_public_symbol(prop, file, symbols, next_id);
    }
}

fn mark_or_add_public_symbol(
    name: &str,
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    next_id: &mut u64,
) {
    if let Some(sym) = symbols.iter_mut().find(|s| s.name == name) {
        sym.visibility = Visibility::Public;
    } else {
        symbols.push(SymbolNode {
            id: *next_id,
            kind: SymbolKind::Export,
            name: name.to_string(),
            visibility: Visibility::Public,
            file: file.to_string(),
            trust_level: TrustLevel::default(),
        });
        *next_id += 1;
    }
}

fn extract_lexical(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    next_id: &mut u64,
) {
    for i in 0..u32::try_from(node.named_child_count()).unwrap_or(0) {
        if let Some(child) = node.named_child(i)
            && child.kind() == "variable_declarator"
            && let Some(name_node) = child.child_by_field_name("name")
            && let Some(value) = child.child_by_field_name("value")
            && value.kind() == "arrow_function"
        {
            let name = node_text(name_node, source);
            symbols.push(SymbolNode {
                id: *next_id,
                kind: SymbolKind::Function,
                name,
                visibility: Visibility::Internal,
                file: file.to_string(),
                trust_level: TrustLevel::default(),
            });
            *next_id += 1;
        }
    }
}

/// 1-based line number from a tree-sitter node, or 0 if the row overflows u32.
fn node_line(node: tree_sitter::Node) -> u32 {
    u32::try_from(node.start_position().row)
        .ok()
        .map_or(0, |r| r.saturating_add(1))
}

fn node_text(node: tree_sitter::Node, source: &[u8]) -> String {
    node.utf8_text(source).unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use anvil_kernel_types::{SymbolKind, SymbolNode, Visibility};

    use super::{LanguageExtractor, TypeScriptExtractor};
    use crate::parser::Parser;
    use crate::parser::extract::extract_symbols;

    #[test]
    fn extracts_functions_from_typescript() {
        let source = b"
function greet(name: string): string {
    return name;
}

const add = (a: number, b: number) => a + b;
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);
        let fns: Vec<&str> = symbols
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Function)
            .map(|s| s.name.as_str())
            .collect();

        assert!(fns.contains(&"greet"));
        assert!(fns.contains(&"add"));
    }

    #[test]
    fn extracts_classes() {
        let source = b"
class Greeter {
    greet() { return 'hello'; }
}
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);
        let classes: Vec<&str> = symbols
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Class)
            .map(|s| s.name.as_str())
            .collect();

        assert!(classes.contains(&"Greeter"));
    }

    // --- TS-G1: interface / type-alias / enum declarations ---

    #[test]
    fn extracts_interface_type_alias_and_enum_symbols() {
        let source = b"
interface Shape { area(): number; }
type Id = string | number;
enum Color { Red, Green, Blue }
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();
        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);

        let by_kind = |k: SymbolKind| -> Vec<&str> {
            symbols
                .symbols
                .iter()
                .filter(|s| s.kind == k)
                .map(|s| s.name.as_str())
                .collect()
        };
        assert_eq!(by_kind(SymbolKind::Interface), ["Shape"], "TS-G1 interface");
        assert_eq!(by_kind(SymbolKind::TypeAlias), ["Id"], "TS-G1 type alias");
        assert_eq!(by_kind(SymbolKind::Enum), ["Color"], "TS-G1 enum");
    }

    #[test]
    fn exported_type_shapes_are_public() {
        let source = b"
export interface Public {}
interface Internal {}
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();
        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);

        let vis = |name: &str| {
            symbols
                .symbols
                .iter()
                .find(|s| s.name == name)
                .map(|s| s.visibility)
        };
        assert_eq!(vis("Public"), Some(Visibility::Public));
        assert_eq!(vis("Internal"), Some(Visibility::Internal));
    }

    // --- TS-G2: class methods as separate symbols, parent encoded in the name ---

    #[test]
    fn extracts_class_methods_with_qualified_names() {
        let source = b"
class Service {
    constructor() {}
    start(): void {}
    private stop(): void {}
}
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();
        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);

        let methods: Vec<&str> = symbols
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Method)
            .map(|s| s.name.as_str())
            .collect();

        // Each method is surfaced as `Owner.method` so the owning class is
        // recoverable from the name (TS-G2 parent link).
        assert!(methods.contains(&"Service.start"), "got {methods:?}");
        assert!(methods.contains(&"Service.stop"), "got {methods:?}");
        assert!(methods.contains(&"Service.constructor"), "got {methods:?}");
        // The class itself is still emitted as a Class symbol.
        assert!(
            symbols
                .symbols
                .iter()
                .any(|s| s.kind == SymbolKind::Class && s.name == "Service")
        );
    }

    #[test]
    fn methods_of_exported_class_are_public() {
        let source = b"
export class Api {
    handle(): void {}
}
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();
        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);

        let method = symbols
            .symbols
            .iter()
            .find(|s| s.kind == SymbolKind::Method && s.name == "Api.handle")
            .expect("Api.handle method symbol");
        assert_eq!(method.visibility, Visibility::Public);
    }

    #[test]
    fn marks_exports_as_public() {
        let source = b"
export function greet() {}
function internal() {}
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);
        let greet = symbols.symbols.iter().find(|s| s.name == "greet").unwrap();
        let internal = symbols
            .symbols
            .iter()
            .find(|s| s.name == "internal")
            .unwrap();

        assert_eq!(greet.visibility, Visibility::Public);
        assert_eq!(internal.visibility, Visibility::Internal);
    }

    #[test]
    fn extracts_imports() {
        let source = b"
import { something } from './module';
import * as fs from 'node:fs';
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);
        let import_sources: Vec<&str> = symbols
            .imports
            .iter()
            .map(|i| i.to_source.as_str())
            .collect();

        assert!(import_sources.contains(&"./module"));
        assert!(import_sources.contains(&"node:fs"));

        let module_import = symbols
            .imports
            .iter()
            .find(|i| i.to_source == "./module")
            .unwrap();
        assert_eq!(module_import.line, 2, "import on line 2 (1-based)");

        let fs_import = symbols
            .imports
            .iter()
            .find(|i| i.to_source == "node:fs")
            .unwrap();
        assert_eq!(fs_import.line, 3, "import on line 3 (1-based)");
    }

    #[test]
    fn export_does_not_flip_unrelated_symbol() {
        // Regression: an export with no declaration (e.g. `export {}`)
        // should not flip a prior symbol to Public.
        let source = b"
function internal() {}
export {};
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);
        let internal = symbols
            .symbols
            .iter()
            .find(|s| s.name == "internal")
            .unwrap();
        assert_eq!(internal.visibility, Visibility::Internal);
    }

    #[test]
    fn assigns_unique_ids() {
        let source = b"
function a() {}
function b() {}
class C {}
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 100);
        let ids: Vec<u64> = symbols.symbols.iter().map(|s| s.id).collect();

        // All unique
        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(ids.len(), unique.len());

        // Starting from offset
        assert!(ids.iter().all(|&id| id >= 100));
    }

    #[test]
    fn export_alias_uses_local_name() {
        // `export { foo as bar }` should mark the symbol named `foo` as Public,
        // not look for a symbol named `bar`.
        let source = b"
function foo() {}
function bar() {}
export { foo as bar };
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);
        let foo = symbols.symbols.iter().find(|s| s.name == "foo").unwrap();
        let bar = symbols.symbols.iter().find(|s| s.name == "bar").unwrap();

        assert_eq!(
            foo.visibility,
            Visibility::Public,
            "foo should be Public (it is the exported symbol)"
        );
        assert_eq!(
            bar.visibility,
            Visibility::Internal,
            "bar should stay Internal (it is not exported)"
        );
    }

    #[test]
    fn anonymous_default_export_emits_default_symbol() {
        let source = b"export default function() { return 42; }";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);
        let default_sym = symbols.symbols.iter().find(|s| s.name == "default");

        assert!(
            default_sym.is_some(),
            "anonymous default export should produce a 'default' symbol"
        );
        let sym = default_sym.unwrap();
        assert_eq!(sym.visibility, Visibility::Public);
        assert_eq!(sym.kind, SymbolKind::Function);
    }

    #[test]
    fn anonymous_default_class_export_emits_default_symbol() {
        let source = b"export default class {}";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);
        let default_sym = symbols.symbols.iter().find(|s| s.name == "default");

        assert!(
            default_sym.is_some(),
            "anonymous default class export should produce a 'default' symbol"
        );
        let sym = default_sym.unwrap();
        assert_eq!(sym.visibility, Visibility::Public);
    }

    #[test]
    fn named_default_export_does_not_duplicate() {
        let source = b"export default function greet() { return 'hi'; }";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);
        let public_syms: Vec<&SymbolNode> = symbols
            .symbols
            .iter()
            .filter(|s| s.visibility == Visibility::Public)
            .collect();

        assert_eq!(
            public_syms.len(),
            1,
            "named default export should produce exactly one public symbol"
        );
        assert_eq!(public_syms[0].name, "greet");
    }

    #[test]
    fn module_exports_property_marks_only_that_symbol() {
        // `module.exports.foo = ...` should only mark `foo` as Public,
        // not all symbols in the file.
        let source = b"
function foo() {}
function bar() {}
module.exports.foo = foo;
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);
        let foo = symbols.symbols.iter().find(|s| s.name == "foo").unwrap();
        let bar = symbols.symbols.iter().find(|s| s.name == "bar").unwrap();

        assert_eq!(
            foo.visibility,
            Visibility::Public,
            "foo should be Public (it is the exported property)"
        );
        assert_eq!(
            bar.visibility,
            Visibility::Internal,
            "bar should stay Internal"
        );
    }

    #[test]
    fn module_exports_object_marks_only_referenced_symbols() {
        let source = b"
function foo() {}
function bar() {}
module.exports = { foo };
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.js"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.js"), 0);
        let foo = symbols.symbols.iter().find(|s| s.name == "foo").unwrap();
        let bar = symbols.symbols.iter().find(|s| s.name == "bar").unwrap();

        assert_eq!(
            foo.visibility,
            Visibility::Public,
            "foo should be Public (it is in the module.exports object)"
        );
        assert_eq!(
            bar.visibility,
            Visibility::Internal,
            "bar should stay Internal (it is not in the module.exports object)"
        );
    }

    #[test]
    fn extracts_require_inside_lexical_declaration() {
        let source = b"const fs = require('node:fs');\nconst path = require('path');\n";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.js"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.js"), 0);
        let import_sources: Vec<&str> = symbols
            .imports
            .iter()
            .map(|i| i.to_source.as_str())
            .collect();

        assert!(
            import_sources.contains(&"node:fs"),
            "require('node:fs') inside const should be captured"
        );
        assert!(
            import_sources.contains(&"path"),
            "require('path') inside const should be captured"
        );

        let fs_req = symbols
            .imports
            .iter()
            .find(|i| i.to_source == "node:fs")
            .unwrap();
        assert_eq!(fs_req.line, 1, "first require on line 1");

        let path_req = symbols
            .imports
            .iter()
            .find(|i| i.to_source == "path")
            .unwrap();
        assert_eq!(path_req.line, 2, "second require on line 2");
    }

    #[test]
    fn module_exports_single_identifier_marks_only_that_symbol() {
        let source = b"
function foo() {}
function bar() {}
module.exports = foo;
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.js"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.js"), 0);
        let foo = symbols.symbols.iter().find(|s| s.name == "foo").unwrap();
        let bar = symbols.symbols.iter().find(|s| s.name == "bar").unwrap();

        assert_eq!(
            foo.visibility,
            Visibility::Public,
            "foo should be Public (it is the module.exports value)"
        );
        assert_eq!(
            bar.visibility,
            Visibility::Internal,
            "bar should stay Internal"
        );
    }

    #[test]
    fn reexport_captures_line_number() {
        let source = b"export { foo } from './utils';\nexport { bar } from './helpers';\n";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);

        let utils_import = symbols
            .imports
            .iter()
            .find(|i| i.to_source == "./utils")
            .unwrap();
        assert_eq!(utils_import.line, 1, "re-export from ./utils on line 1");

        let helpers_import = symbols
            .imports
            .iter()
            .find(|i| i.to_source == "./helpers")
            .unwrap();
        assert_eq!(helpers_import.line, 2, "re-export from ./helpers on line 2");
    }

    /// K1 parity: the TS extractor, after the `LanguageExtractor`-trait
    /// refactor, must produce byte-for-byte the same `FileSymbols` (symbol ids,
    /// kinds, names, visibilities, and import edges with line numbers) as the
    /// pre-refactor walker. The expected values below were captured from the
    /// original `extract.rs` walker against this fixture; any drift here is a
    /// behavioural regression in the port, not a test that needs updating.
    #[test]
    fn ts_extractor_parity_snapshot() {
        // A fixture exercising the breadth of the walker: ESM function/class
        // exports, an internal symbol, an arrow-fn const, a named import, and
        // a re-export.
        let source = b"import { dep } from './dep';\n\
export function greet(name: string): string { return name; }\n\
function internal() {}\n\
export class Greeter {}\n\
const add = (a: number, b: number) => a + b;\n\
export { foo } from './other';\n";

        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("src/mod.ts"), source).unwrap();
        let fs = extract_symbols(&result.tree, source, Path::new("src/mod.ts"), 0);

        // --- Symbols: id / kind / name / visibility, in emission order ---
        let got: Vec<(u64, SymbolKind, &str, Visibility)> = fs
            .symbols
            .iter()
            .map(|s| (s.id, s.kind, s.name.as_str(), s.visibility))
            .collect();
        let expected: Vec<(u64, SymbolKind, &str, Visibility)> = vec![
            (0, SymbolKind::Function, "greet", Visibility::Public),
            (1, SymbolKind::Function, "internal", Visibility::Internal),
            (2, SymbolKind::Class, "Greeter", Visibility::Public),
            (3, SymbolKind::Function, "add", Visibility::Internal),
            (4, SymbolKind::Export, "foo", Visibility::Public),
        ];
        assert_eq!(
            got, expected,
            "symbol parity drifted from pre-refactor walker"
        );

        // Every symbol is attributed to the file.
        assert!(fs.symbols.iter().all(|s| s.file == "src/mod.ts"));

        // --- Imports: source + 1-based line, in emission order ---
        let imports: Vec<(&str, u32)> = fs
            .imports
            .iter()
            .map(|i| (i.to_source.as_str(), i.line))
            .collect();
        assert_eq!(
            imports,
            vec![("./dep", 1), ("./other", 6)],
            "import-edge parity drifted from pre-refactor walker"
        );
        assert!(fs.imports.iter().all(|i| i.from_file == "src/mod.ts"));
    }

    /// K1: the orchestrator routes through `TypeScriptExtractor` for every
    /// member of the TS family (.ts/.tsx/.js/.jsx) and yields identical results
    /// to calling the extractor directly.
    #[test]
    fn orchestrator_dispatch_matches_direct_extractor_call() {
        let source = b"export function f() {}\n";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("a.ts"), source).unwrap();

        let via_orchestrator = extract_symbols(&result.tree, source, Path::new("a.ts"), 0);
        let via_trait = TypeScriptExtractor.extract(&result.tree, source, "a.ts", 0);

        assert_eq!(via_orchestrator.symbols.len(), via_trait.symbols.len());
        assert_eq!(via_orchestrator.symbols[0].name, "f");
        assert_eq!(via_trait.symbols[0].name, "f");
    }
}
