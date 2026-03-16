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
        "function_declaration" => {
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
        "class_declaration" => {
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
        "export_statement" => {
            if let Some(decl) = node.child_by_field_name("declaration") {
                let before = symbols.len();
                extract_from_node(decl, source, file, symbols, imports, next_id);
                // Mark ALL symbols added by this export as public
                for sym in &mut symbols[before..] {
                    sym.visibility = Visibility::Public;
                }
            } else {
                // Handle named export clauses: `export { foo, bar }`
                let mut handled_clause = false;
                for i in 0..u32::try_from(node.named_child_count()).unwrap_or(0) {
                    if let Some(child) = node.named_child(i) {
                        if child.kind() == "export_clause" {
                            handled_clause = true;
                            for j in 0..u32::try_from(child.named_child_count()).unwrap_or(0) {
                                if let Some(spec) = child.named_child(j) {
                                    if spec.kind() == "export_specifier" {
                                        let name = if let Some(alias) =
                                            spec.child_by_field_name("alias")
                                        {
                                            node_text(alias, source)
                                        } else if let Some(name_node) =
                                            spec.child_by_field_name("name")
                                        {
                                            node_text(name_node, source)
                                        } else {
                                            continue;
                                        };
                                        // Mark existing symbol as Public if found
                                        if let Some(sym) =
                                            symbols.iter_mut().find(|s| s.name == name)
                                        {
                                            sym.visibility = Visibility::Public;
                                        }
                                    }
                                }
                            }
                        }
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
        }
        "import_statement" => {
            if let Some(source_node) = node.child_by_field_name("source") {
                let raw = node_text(source_node, source);
                let module_path = raw.trim_matches(|c| c == '\'' || c == '"');
                imports.push(ImportEdge {
                    from_file: file.to_string(),
                    to_source: module_path.to_string(),
                });
            }
        }
        "call_expression" => {
            // Capture CommonJS require() calls as imports
            if let Some(func) = node.child_by_field_name("function") {
                if node_text(func, source) == "require" {
                    if let Some(args) = node.child_by_field_name("arguments") {
                        if let Some(arg) = args.named_child(0) {
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
                }
            }
        }
        "assignment_expression" => {
            // Capture CommonJS module.exports / exports.foo as Public symbols
            if let Some(left) = node.child_by_field_name("left") {
                let left_text = node_text(left, source);
                if left_text.starts_with("module.exports") || left_text.starts_with("exports.") {
                    // Extract the assigned name if it's `exports.foo = ...`
                    if left_text.starts_with("exports.") {
                        if let Some(name) = left_text.strip_prefix("exports.") {
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
                    } else {
                        // module.exports = ... — mark all preceding symbols as Public
                        for sym in symbols.iter_mut().filter(|s| s.file == file) {
                            sym.visibility = Visibility::Public;
                        }
                    }
                }
            }
        }
        "lexical_declaration" => {
            for i in 0..u32::try_from(node.named_child_count()).unwrap_or(0) {
                if let Some(child) = node.named_child(i) {
                    if child.kind() == "variable_declarator" {
                        if let Some(name_node) = child.child_by_field_name("name") {
                            if let Some(value) = child.child_by_field_name("value") {
                                if value.kind() == "arrow_function" {
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
                    }
                }
            }
        }
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
}
