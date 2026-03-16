use std::path::Path;

use anvil_kernel_types::{SymbolKind, SymbolNode, TrustLevel, Visibility};

/// Extracted symbols from a single file.
#[derive(Debug, Clone)]
pub struct FileSymbols {
    pub file: String,
    pub symbols: Vec<SymbolNode>,
    pub imports: Vec<ImportEdge>,
}

/// An import edge from one file to another.
#[derive(Debug, Clone)]
pub struct ImportEdge {
    pub from_file: String,
    pub to_source: String,
}

/// Extract symbols from a tree-sitter AST.
pub fn extract_symbols(
    tree: &tree_sitter::Tree,
    source: &[u8],
    file_path: &Path,
    id_offset: u64,
) -> FileSymbols {
    let file = file_path.to_string_lossy().to_string();
    let root = tree.root_node();
    let mut symbols = Vec::new();
    let mut imports = Vec::new();
    let mut next_id = id_offset;

    extract_from_node(
        root,
        source,
        &file,
        &mut symbols,
        &mut imports,
        &mut next_id,
    );

    FileSymbols {
        file,
        symbols,
        imports,
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
        "export_statement" => extract_export(node, source, file, symbols, imports, next_id),
        "import_statement" => extract_import(node, source, file, imports),
        "call_expression" => extract_require(node, source, file, imports),
        "assignment_expression" => extract_cjs_export(node, source, file, symbols, next_id),
        "lexical_declaration" => extract_lexical(node, source, file, symbols, next_id),
        _ => {}
    }

    // Recurse into children (except for nodes we've already handled)
    if !matches!(
        node.kind(),
        "function_declaration" | "class_declaration" | "lexical_declaration" | "export_statement"
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
    if let Some(name_node) = node.child_by_field_name("name") {
        let name = node_text(name_node, source);
        symbols.push(SymbolNode {
            id: *next_id,
            kind: SymbolKind::Class,
            name,
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
        let rhs_kind = rhs.map(|n| n.kind());
        if rhs_kind == Some("object") {
            let rhs_node = rhs.unwrap();
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

fn node_text(node: tree_sitter::Node, source: &[u8]) -> String {
    node.utf8_text(source).unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

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
}
