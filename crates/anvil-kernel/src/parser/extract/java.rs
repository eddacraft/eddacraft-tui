//! Java symbol and import extractor (LANGTAIL-004) — T1 (Parsed).
//!
//! | Java construct          | `SymbolKind` | Note                            |
//! | ----------------------- | ------------ | ------------------------------- |
//! | `class C`               | `Class`      |                                 |
//! | `record R(...)`         | `Class`      | nominal type with members       |
//! | `interface I`           | `Interface`  |                                 |
//! | `enum E`                | `Enum`       |                                 |
//! | method in a type body   | `Method`     | qualified `Owner.method`        |
//!
//! Visibility is `Public` when the declaration carries the `public` modifier,
//! else `Internal` (package-private / protected / private all read as not
//! widening the public surface for T1). Imports come from `import` declarations;
//! the edge target is the fully-qualified dotted path (`java.util.List`).
//! Nested types are walked recursively and emitted by their simple name.

use anvil_kernel_types::{SymbolKind, SymbolNode, Visibility};

use super::tail_common::{field_text, finish, has_modifier, import_edge, node_text, push_symbol};
use super::{FileSymbols, ImportEdge, LanguageExtractor};

/// Extractor for Java (`.java`).
pub struct JavaExtractor;

impl LanguageExtractor for JavaExtractor {
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

        walk(
            tree.root_node(),
            source,
            file,
            None,
            &mut symbols,
            &mut imports,
            &mut next_id,
        );

        finish(file, symbols, imports)
    }
}

fn type_kind(kind: &str) -> Option<SymbolKind> {
    match kind {
        "class_declaration" | "record_declaration" => Some(SymbolKind::Class),
        "interface_declaration" | "annotation_type_declaration" => Some(SymbolKind::Interface),
        "enum_declaration" => Some(SymbolKind::Enum),
        _ => None,
    }
}

fn walk(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    owner: Option<&str>,
    symbols: &mut Vec<SymbolNode>,
    imports: &mut Vec<ImportEdge>,
    next_id: &mut u64,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "import_declaration" => {
                if let Some(path) = import_path(child, source) {
                    imports.push(import_edge(file, path, child));
                }
            }
            k if type_kind(k).is_some() => {
                let Some(name) = field_text(child, "name", source) else {
                    continue;
                };
                let kind = type_kind(k).unwrap();
                let vis = visibility(child, source);
                push_symbol(symbols, next_id, file, kind, name.clone(), vis);
                // Recurse into the body with this type as the method owner so
                // members are named `Owner.method` and nested types are captured.
                if let Some(body) = child.child_by_field_name("body") {
                    walk(body, source, file, Some(&name), symbols, imports, next_id);
                }
            }
            "method_declaration" => {
                if let Some(name) = field_text(child, "name", source) {
                    let qualified = match owner {
                        Some(o) => format!("{o}.{name}"),
                        None => name,
                    };
                    let vis = visibility(child, source);
                    push_symbol(symbols, next_id, file, SymbolKind::Method, qualified, vis);
                }
            }
            _ => {
                // Bodies and other containers: keep descending so members under
                // any wrapper node are reached.
                walk(child, source, file, owner, symbols, imports, next_id);
            }
        }
    }
}

/// The dotted module path of an `import` declaration, ignoring `static` and a
/// trailing `.*` wildcard (the edge points at the package/type, not a member).
fn import_path(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if matches!(child.kind(), "scoped_identifier" | "identifier") {
            let text = node_text(child, source);
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}

fn visibility(node: tree_sitter::Node, source: &[u8]) -> Visibility {
    if has_modifier(node, "public", source) {
        Visibility::Public
    } else {
        Visibility::Internal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::languages::Language;

    fn extract(src: &str) -> FileSymbols {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&Language::Java.ts_language()).unwrap();
        let tree = parser.parse(src, None).unwrap();
        JavaExtractor.extract(&tree, src.as_bytes(), "App.java", 0)
    }

    #[test]
    fn extracts_types_methods_imports() {
        let src = "package app;\n\nimport java.util.List;\nimport static java.lang.Math.PI;\n\npublic class Service {\n    public void run() {}\n    private int helper() { return 0; }\n}\n\ninterface Runnable2 {\n    void go();\n}\n\nenum Color { RED, GREEN }\n\nrecord Point(int x, int y) {}\n";
        let fs = extract(src);
        let names: Vec<_> = fs.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Service"));
        assert!(
            names.contains(&"Service.run"),
            "method qualified: {names:?}"
        );
        assert!(names.contains(&"Service.helper"));
        assert!(names.contains(&"Runnable2"));
        assert!(names.contains(&"Color"));
        assert!(names.contains(&"Point"));

        let service = fs.symbols.iter().find(|s| s.name == "Service").unwrap();
        assert_eq!(service.kind, SymbolKind::Class);
        assert_eq!(service.visibility, Visibility::Public);
        let iface = fs.symbols.iter().find(|s| s.name == "Runnable2").unwrap();
        assert_eq!(iface.kind, SymbolKind::Interface);
        let color = fs.symbols.iter().find(|s| s.name == "Color").unwrap();
        assert_eq!(color.kind, SymbolKind::Enum);
        let helper = fs
            .symbols
            .iter()
            .find(|s| s.name == "Service.helper")
            .unwrap();
        assert_eq!(helper.visibility, Visibility::Internal);

        let targets: Vec<_> = fs.imports.iter().map(|i| i.to_source.as_str()).collect();
        assert!(targets.contains(&"java.util.List"), "imports: {targets:?}");
        assert!(targets.contains(&"java.lang.Math.PI"));
        assert!(fs.calls.is_empty() && !fs.calls_partial);
    }
}
