//! C# symbol and import extractor (LANGTAIL-006) — T1 (Parsed).
//!
//! | C# construct            | `SymbolKind` | Note                            |
//! | ----------------------- | ------------ | ------------------------------- |
//! | `class C` / `struct S`  | `Class`      | nominal type with members       |
//! | `record R`              | `Class`      |                                 |
//! | `interface I`           | `Interface`  |                                 |
//! | `enum E`                | `Enum`       |                                 |
//! | method in a type body   | `Method`     | qualified `Owner.method`        |
//!
//! Declarations nest inside `namespace` blocks (block-scoped or file-scoped),
//! so the walk recurses through namespace bodies. Visibility is `Public` when
//! the declaration carries the `public` modifier, else `Internal`. Imports come
//! from `using` directives; the edge target is the imported namespace path.

use anvil_kernel_types::{SymbolKind, SymbolNode, Visibility};

use super::tail_common::{field_text, finish, has_modifier, import_edge, node_text, push_symbol};
use super::{FileSymbols, ImportEdge, LanguageExtractor};

/// Extractor for C# (`.cs`).
pub struct CSharpExtractor;

impl LanguageExtractor for CSharpExtractor {
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
        "class_declaration"
        | "struct_declaration"
        | "record_declaration"
        | "record_struct_declaration" => Some(SymbolKind::Class),
        "interface_declaration" => Some(SymbolKind::Interface),
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
            "using_directive" => {
                if let Some(path) = using_path(child, source) {
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
                // namespace_declaration / file_scoped_namespace_declaration /
                // declaration_list and other containers: keep descending.
                walk(child, source, file, owner, symbols, imports, next_id);
            }
        }
    }
}

/// The namespace path of a `using` directive. Takes the **last** name-like
/// child so an alias directive (`using Alias = Target;`) resolves to `Target`,
/// not the leading `Alias` identifier; `using static System.Math;` and plain
/// `using System.Text;` both have the path as their last name child too.
fn using_path(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let mut cursor = node.walk();
    let mut last = None;
    for child in node.children(&mut cursor) {
        if matches!(
            child.kind(),
            "qualified_name" | "identifier" | "member_access_expression" | "generic_name"
        ) {
            let text = node_text(child, source);
            if !text.is_empty() {
                last = Some(text);
            }
        }
    }
    last
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
        parser
            .set_language(&Language::CSharp.ts_language())
            .unwrap();
        let tree = parser.parse(src, None).unwrap();
        CSharpExtractor.extract(&tree, src.as_bytes(), "Program.cs", 0)
    }

    #[test]
    fn extracts_types_methods_imports_through_namespace() {
        let src = "using System;\nusing System.Text;\n\nnamespace App\n{\n    public class Service\n    {\n        public void Run() {}\n        int Helper() { return 0; }\n    }\n\n    interface IService { void Go(); }\n\n    enum Color { Red, Green }\n}\n";
        let fs = extract(src);
        let names: Vec<_> = fs.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Service"), "types: {names:?}");
        assert!(
            names.contains(&"Service.Run"),
            "method qualified: {names:?}"
        );
        assert!(names.contains(&"Service.Helper"));
        assert!(names.contains(&"IService"));
        assert!(names.contains(&"Color"));

        let service = fs.symbols.iter().find(|s| s.name == "Service").unwrap();
        assert_eq!(service.kind, SymbolKind::Class);
        assert_eq!(service.visibility, Visibility::Public);
        let iface = fs.symbols.iter().find(|s| s.name == "IService").unwrap();
        assert_eq!(iface.kind, SymbolKind::Interface);
        let color = fs.symbols.iter().find(|s| s.name == "Color").unwrap();
        assert_eq!(color.kind, SymbolKind::Enum);
        let helper = fs
            .symbols
            .iter()
            .find(|s| s.name == "Service.Helper")
            .unwrap();
        assert_eq!(helper.visibility, Visibility::Internal);

        let targets: Vec<_> = fs.imports.iter().map(|i| i.to_source.as_str()).collect();
        assert!(targets.contains(&"System"), "imports: {targets:?}");
        assert!(targets.contains(&"System.Text"));
        assert!(fs.calls.is_empty() && !fs.calls_partial);
    }

    #[test]
    fn using_alias_resolves_to_target_not_alias() {
        let src = "using Text = System.Text;\nusing static System.Math;\n\nnamespace App { class C {} }\n";
        let fs = extract(src);
        let targets: Vec<_> = fs.imports.iter().map(|i| i.to_source.as_str()).collect();
        assert!(
            targets.contains(&"System.Text"),
            "alias must resolve to target, not `Text`: {targets:?}"
        );
        assert!(
            !targets.contains(&"Text"),
            "alias name must not be emitted as the import target: {targets:?}"
        );
        assert!(
            targets.contains(&"System.Math"),
            "static using: {targets:?}"
        );
    }

    #[test]
    fn handles_file_scoped_namespace() {
        let src = "namespace App;\n\npublic class Widget { public void Draw() {} }\n";
        let fs = extract(src);
        let names: Vec<_> = fs.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Widget"), "file-scoped ns: {names:?}");
        assert!(names.contains(&"Widget.Draw"));
    }
}
