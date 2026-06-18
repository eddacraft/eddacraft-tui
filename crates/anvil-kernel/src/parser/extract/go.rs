//! Go symbol and import extractor (LANGTAIL-003) — T1 (Parsed).
//!
//! | Go construct              | `SymbolKind` | Note                            |
//! | ------------------------- | ------------ | ------------------------------- |
//! | `func F()`                | `Function`   |                                 |
//! | `func (r T) M()`          | `Method`     | qualified `T.M` (receiver type) |
//! | `type T struct {…}`       | `Class`      | nominal type with fields        |
//! | `type T interface {…}`    | `Interface`  |                                 |
//! | `type T = …` / other      | `TypeAlias`  |                                 |
//!
//! Visibility follows Go's own rule — an exported identifier starts with an
//! uppercase letter. Imports come from `import` declarations (single and
//! grouped); the edge target is the import path string. Constants, variables,
//! and struct fields are out of T1 scope.

use anvil_kernel_types::{SymbolKind, SymbolNode, Visibility};

use super::tail_common::{field_text, finish, import_edge, node_text, push_symbol, strip_delims};
use super::{FileSymbols, ImportEdge, LanguageExtractor};

/// Extractor for Go (`.go`).
pub struct GoExtractor;

impl LanguageExtractor for GoExtractor {
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
            match node.kind() {
                "function_declaration" => {
                    if let Some(name) = field_text(node, "name", source) {
                        let vis = go_visibility(&name);
                        push_symbol(
                            &mut symbols,
                            &mut next_id,
                            file,
                            SymbolKind::Function,
                            name,
                            vis,
                        );
                    }
                }
                "method_declaration" => {
                    emit_method(node, source, file, &mut symbols, &mut next_id);
                }
                "type_declaration" => {
                    emit_types(node, source, file, &mut symbols, &mut next_id);
                }
                "import_declaration" => {
                    collect_imports(node, source, file, &mut imports);
                }
                _ => {}
            }
        }

        finish(file, symbols, imports)
    }
}

/// `func (r T) M()` / `func (r *T) M()` → `Method` named `T.M`. The receiver
/// type is read from the `receiver` parameter list; a pointer receiver's `*` is
/// stripped so `*T` and `T` name the same owner.
fn emit_method(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    next_id: &mut u64,
) {
    let Some(name) = field_text(node, "name", source) else {
        return;
    };
    let owner = node
        .child_by_field_name("receiver")
        .and_then(|r| receiver_type(r, source));
    let qualified = match owner {
        Some(owner) => format!("{owner}.{name}"),
        None => name.clone(),
    };
    let vis = go_visibility(&name);
    push_symbol(symbols, next_id, file, SymbolKind::Method, qualified, vis);
}

/// The receiver type identifier from a `(r T)` / `(r *T)` parameter list.
fn receiver_type(receiver: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let mut cursor = receiver.walk();
    for param in receiver.children(&mut cursor) {
        if param.kind() == "parameter_declaration"
            && let Some(ty) = param.child_by_field_name("type")
        {
            // Strip a pointer receiver's `*` and any generic type-parameter list
            // so `*Stack[T]` and `Stack` name the same owner — the method binds
            // to the type, not the instantiation (Go 1.18+ generics).
            let text = node_text(ty, source);
            let text = text.trim_start_matches('*');
            let owner = text.split('[').next().unwrap_or(text).trim();
            return Some(owner.to_string());
        }
    }
    None
}

/// A `type (…)` block or single `type T …` declaration holds one or more
/// `type_spec` / `type_alias` entries.
fn emit_types(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    next_id: &mut u64,
) {
    let mut cursor = node.walk();
    for spec in node.children(&mut cursor) {
        match spec.kind() {
            "type_spec" => {
                let Some(name) = field_text(spec, "name", source) else {
                    continue;
                };
                let kind = match spec.child_by_field_name("type").map(|t| t.kind()) {
                    Some("struct_type") => SymbolKind::Class,
                    Some("interface_type") => SymbolKind::Interface,
                    _ => SymbolKind::TypeAlias,
                };
                let vis = go_visibility(&name);
                push_symbol(symbols, next_id, file, kind, name, vis);
            }
            "type_alias" => {
                if let Some(name) = field_text(spec, "name", source) {
                    let vis = go_visibility(&name);
                    push_symbol(symbols, next_id, file, SymbolKind::TypeAlias, name, vis);
                }
            }
            _ => {}
        }
    }
}

/// Collect import paths from a single or grouped `import` declaration.
fn collect_imports(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    imports: &mut Vec<ImportEdge>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "import_spec" => push_import_spec(child, source, file, imports),
            "import_spec_list" => {
                let mut inner = child.walk();
                for spec in child.children(&mut inner) {
                    if spec.kind() == "import_spec" {
                        push_import_spec(spec, source, file, imports);
                    }
                }
            }
            _ => {}
        }
    }
}

fn push_import_spec(
    spec: tree_sitter::Node,
    source: &[u8],
    file: &str,
    imports: &mut Vec<ImportEdge>,
) {
    if let Some(path) = spec.child_by_field_name("path") {
        let module = strip_delims(&node_text(path, source));
        if !module.is_empty() {
            imports.push(import_edge(file, module, spec));
        }
    }
}

/// Go's exported-identifier rule: a leading Unicode uppercase letter exports.
fn go_visibility(name: &str) -> Visibility {
    match name.chars().next() {
        Some(c) if c.is_uppercase() => Visibility::Public,
        _ => Visibility::Internal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::languages::Language;

    fn extract(src: &str) -> FileSymbols {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&Language::Go.ts_language()).unwrap();
        let tree = parser.parse(src, None).unwrap();
        GoExtractor.extract(&tree, src.as_bytes(), "main.go", 0)
    }

    #[test]
    fn extracts_funcs_methods_types_imports() {
        let src = "package main\n\nimport (\n\t\"fmt\"\n\th \"net/http\"\n)\n\ntype Server struct {\n\tAddr string\n}\n\ntype Handler interface {\n\tServe()\n}\n\nfunc New() *Server {\n\treturn nil\n}\n\nfunc (s *Server) Start() {}\n\nfunc internalHelper() {}\n";
        let fs = extract(src);
        let names: Vec<_> = fs.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"New"), "top-level func: {names:?}");
        assert!(
            names.contains(&"Server.Start"),
            "method qualified: {names:?}"
        );
        assert!(names.contains(&"internalHelper"));
        assert!(names.contains(&"Server"));
        assert!(names.contains(&"Handler"));

        let server = fs.symbols.iter().find(|s| s.name == "Server").unwrap();
        assert_eq!(server.kind, SymbolKind::Class);
        assert_eq!(server.visibility, Visibility::Public);
        let handler = fs.symbols.iter().find(|s| s.name == "Handler").unwrap();
        assert_eq!(handler.kind, SymbolKind::Interface);
        let helper = fs
            .symbols
            .iter()
            .find(|s| s.name == "internalHelper")
            .unwrap();
        assert_eq!(helper.visibility, Visibility::Internal);

        let targets: Vec<_> = fs.imports.iter().map(|i| i.to_source.as_str()).collect();
        assert!(targets.contains(&"fmt"), "imports: {targets:?}");
        assert!(targets.contains(&"net/http"));
        // No call sites at T1.
        assert!(fs.calls.is_empty() && !fs.calls_partial);
    }

    #[test]
    fn generic_receiver_binds_to_the_base_type() {
        // Go 1.18+ generics: a method on `*Stack[T]` must bind to `Stack`, not
        // `Stack[T]`, so it matches the `type Stack[T any] struct{}` symbol.
        let src = "package c\n\ntype Stack[T any] struct {\n\titems []T\n}\n\nfunc (s *Stack[T]) Push(v T) {}\nfunc (s Stack[T]) Len() int { return 0 }\n";
        let fs = extract(src);
        let names: Vec<_> = fs.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(
            names.contains(&"Stack.Push"),
            "pointer generic receiver: {names:?}"
        );
        assert!(
            names.contains(&"Stack.Len"),
            "value generic receiver: {names:?}"
        );
        assert!(
            !names.iter().any(|n| n.contains('[')),
            "no instantiation in name: {names:?}"
        );
    }
}
