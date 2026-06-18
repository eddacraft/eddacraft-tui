//! Dart symbol and import extractor (LANGTAIL-002) — T1 (Parsed).
//!
//! | Dart construct          | `SymbolKind` | Note                            |
//! | ----------------------- | ------------ | ------------------------------- |
//! | `class C`               | `Class`      |                                 |
//! | `mixin M`               | `Class`      | a mixin is a nominal type       |
//! | `enum E`                | `Enum`       |                                 |
//! | `void f()` (top level)  | `Function`   |                                 |
//! | method in a class body  | `Method`     | qualified `Owner.method`        |
//!
//! Dart nests a function's name inside a `function_signature`, so the name is
//! read through that node rather than a direct `name` field. Visibility follows
//! Dart's library-privacy rule — a leading underscore is library-private
//! (`Internal`), everything else is `Public`. Imports come from `import`
//! directives; the edge target is the URI string (`dart:io`,
//! `package:flutter/material.dart`, or a relative path).

use anvil_kernel_types::{SymbolKind, SymbolNode, Visibility};

use super::tail_common::{finish, import_edge, node_text, push_symbol, strip_delims};
use super::{FileSymbols, LanguageExtractor};

/// Extractor for Dart (`.dart`).
pub struct DartExtractor;

impl LanguageExtractor for DartExtractor {
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
                "import_or_export" => {
                    if let Some(uri) = import_uri(node, source) {
                        imports.push(import_edge(file, uri, node));
                    }
                }
                "class_declaration" | "mixin_declaration" => {
                    if let Some(name) = field_identifier(node, source) {
                        let vis = visibility(&name);
                        push_symbol(
                            &mut symbols,
                            &mut next_id,
                            file,
                            SymbolKind::Class,
                            name.clone(),
                            vis,
                        );
                        emit_methods(node, source, file, &name, &mut symbols, &mut next_id);
                    }
                }
                "enum_declaration" => {
                    if let Some(name) = field_identifier(node, source) {
                        let vis = visibility(&name);
                        push_symbol(
                            &mut symbols,
                            &mut next_id,
                            file,
                            SymbolKind::Enum,
                            name,
                            vis,
                        );
                    }
                }
                "function_declaration" => {
                    if let Some(name) = signature_name(node, source) {
                        let vis = visibility(&name);
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
                _ => {}
            }
        }

        finish(file, symbols, imports)
    }
}

/// Emit `Owner.method` for each method in a class/mixin body. A method's name
/// lives under its `function_signature`; getters/setters and operators that do
/// not expose a plain `function_signature` name are skipped (best-effort T1).
fn emit_methods(
    type_node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    owner: &str,
    symbols: &mut Vec<SymbolNode>,
    next_id: &mut u64,
) {
    let Some(body) = type_node.child_by_field_name("body") else {
        return;
    };
    let mut found = Vec::new();
    collect_methods(body, source, &mut found);
    for name in found {
        let vis = visibility(&name);
        push_symbol(
            symbols,
            next_id,
            file,
            SymbolKind::Method,
            format!("{owner}.{name}"),
            vis,
        );
    }
}

fn collect_methods(node: tree_sitter::Node, source: &[u8], out: &mut Vec<String>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "method_declaration" {
            if let Some(name) = descendant_signature_name(child, source) {
                out.push(name);
            }
        } else {
            collect_methods(child, source, out);
        }
    }
}

/// The `name` field of a node's direct `function_signature` (top-level funcs use
/// a `signature` field wrapping it).
fn signature_name(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let sig = node
        .child_by_field_name("signature")
        .or_else(|| first_child_of_kind(node, "function_signature"))?;
    let sig = if sig.kind() == "function_signature" {
        sig
    } else {
        first_child_of_kind(sig, "function_signature")?
    };
    sig.child_by_field_name("name")
        .map(|n| node_text(n, source))
}

/// Depth-first search for the first `function_signature` name under a method
/// declaration (`method_signature` → `function_signature` → `name`).
fn descendant_signature_name(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    if node.kind() == "function_signature" {
        return node
            .child_by_field_name("name")
            .map(|n| node_text(n, source));
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(name) = descendant_signature_name(child, source) {
            return Some(name);
        }
    }
    None
}

fn first_child_of_kind<'a>(
    node: tree_sitter::Node<'a>,
    kind: &str,
) -> Option<tree_sitter::Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).find(|c| c.kind() == kind)
}

/// The `name` identifier of a class/mixin/enum declaration.
fn field_identifier(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .map(|n| node_text(n, source))
        .filter(|s| !s.is_empty())
}

/// The URI string of an `import`/`export` directive.
fn import_uri(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let uri = find_string_literal(node, source)?;
    let stripped = strip_delims(&uri);
    (!stripped.is_empty()).then_some(stripped)
}

fn find_string_literal(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    if node.kind() == "string_literal" {
        return Some(node_text(node, source));
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(s) = find_string_literal(child, source) {
            return Some(s);
        }
    }
    None
}

fn visibility(name: &str) -> Visibility {
    if name.starts_with('_') {
        Visibility::Internal
    } else {
        Visibility::Public
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::languages::Language;

    fn extract(src: &str) -> FileSymbols {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&Language::Dart.ts_language()).unwrap();
        let tree = parser.parse(src, None).unwrap();
        DartExtractor.extract(&tree, src.as_bytes(), "main.dart", 0)
    }

    #[test]
    fn extracts_classes_methods_funcs_imports() {
        let src = "import 'dart:io';\nimport 'package:flutter/material.dart';\n\nclass Greeter {\n  String hello(String name) {\n    return name;\n  }\n  void _secret() {}\n}\n\nmixin Logger {}\n\nenum Color { red, green }\n\nvoid top() {}\n";
        let fs = extract(src);
        let names: Vec<_> = fs.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Greeter"), "types: {names:?}");
        assert!(
            names.contains(&"Greeter.hello"),
            "method qualified: {names:?}"
        );
        assert!(names.contains(&"Greeter._secret"));
        assert!(names.contains(&"Logger"));
        assert!(names.contains(&"Color"));
        assert!(names.contains(&"top"), "top-level fn: {names:?}");

        let greeter = fs.symbols.iter().find(|s| s.name == "Greeter").unwrap();
        assert_eq!(greeter.kind, SymbolKind::Class);
        let color = fs.symbols.iter().find(|s| s.name == "Color").unwrap();
        assert_eq!(color.kind, SymbolKind::Enum);
        let secret = fs
            .symbols
            .iter()
            .find(|s| s.name == "Greeter._secret")
            .unwrap();
        assert_eq!(secret.visibility, Visibility::Internal);

        let targets: Vec<_> = fs.imports.iter().map(|i| i.to_source.as_str()).collect();
        assert!(targets.contains(&"dart:io"), "imports: {targets:?}");
        assert!(targets.contains(&"package:flutter/material.dart"));
        assert!(fs.calls.is_empty() && !fs.calls_partial);
    }
}
