//! Kotlin symbol and import extractor (LANGTAIL-005) — T1 (Parsed).
//!
//! | Kotlin construct        | `SymbolKind` | Note                            |
//! | ----------------------- | ------------ | ------------------------------- |
//! | `class C` / `data class`| `Class`      |                                 |
//! | `object O`              | `Class`      | a singleton is a nominal type   |
//! | `interface I`           | `Interface`  |                                 |
//! | `enum class E`          | `Enum`       |                                 |
//! | `fun f()` (top level)   | `Function`   |                                 |
//! | `fun f()` in a body     | `Method`     | qualified `Owner.method`        |
//!
//! `tree-sitter-kotlin-ng` models `class`, `interface`, and `enum class` all as
//! `class_declaration`, distinguished by a keyword/modifier child, and attaches
//! the body as an *unnamed* `class_body` child (no `body` field) — both handled
//! below. Kotlin is newline-sensitive: single-line bodies can trip the grammar,
//! so realistic multi-line source is what the fixtures exercise. Visibility is
//! `Public` by default; a `private` / `internal` / `protected` modifier reads as
//! `Internal`. Imports come from `import` directives.

use anvil_kernel_types::{SymbolKind, SymbolNode, Visibility};

use super::tail_common::{field_text, finish, has_modifier, import_edge, node_text, push_symbol};
use super::{FileSymbols, ImportEdge, LanguageExtractor};

/// Extractor for Kotlin (`.kt` / `.kts`).
pub struct KotlinExtractor;

impl LanguageExtractor for KotlinExtractor {
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
            "import" | "import_header" => {
                if let Some(path) = import_path(child, source) {
                    imports.push(import_edge(file, path, child));
                }
            }
            "class_declaration" | "object_declaration" => {
                let Some(name) = field_text(child, "name", source) else {
                    continue;
                };
                let kind = class_kind(child, source);
                let vis = visibility(child, source);
                push_symbol(symbols, next_id, file, kind, name.clone(), vis);
                if let Some(body) = type_body(child) {
                    walk(body, source, file, Some(&name), symbols, imports, next_id);
                }
            }
            "function_declaration" => {
                if let Some(name) = field_text(child, "name", source) {
                    let (kind, qualified) = match owner {
                        Some(o) => (SymbolKind::Method, format!("{o}.{name}")),
                        None => (SymbolKind::Function, name),
                    };
                    let vis = visibility(child, source);
                    push_symbol(symbols, next_id, file, kind, qualified, vis);
                }
            }
            _ => {}
        }
    }
}

/// `class` vs `interface` vs `enum class` — kotlin-ng keeps all three as
/// `class_declaration`, so the discriminator is a keyword/modifier child.
fn class_kind(node: tree_sitter::Node, source: &[u8]) -> SymbolKind {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "interface" {
            return SymbolKind::Interface;
        }
    }
    if has_modifier(node, "enum", source) {
        return SymbolKind::Enum;
    }
    SymbolKind::Class
}

/// The body of a type declaration — an *unnamed* `class_body` / `enum_class_body`
/// child in kotlin-ng (there is no `body` field).
fn type_body(node: tree_sitter::Node) -> Option<tree_sitter::Node> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|c| matches!(c.kind(), "class_body" | "enum_class_body"))
}

fn import_path(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if matches!(child.kind(), "qualified_identifier" | "identifier") {
            let text = node_text(child, source);
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}

fn visibility(node: tree_sitter::Node, source: &[u8]) -> Visibility {
    if has_modifier(node, "private", source)
        || has_modifier(node, "internal", source)
        || has_modifier(node, "protected", source)
    {
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
        parser
            .set_language(&Language::Kotlin.ts_language())
            .unwrap();
        let tree = parser.parse(src, None).unwrap();
        KotlinExtractor.extract(&tree, src.as_bytes(), "Main.kt", 0)
    }

    #[test]
    fn extracts_classes_interfaces_funcs_imports() {
        let src = "package app\n\nimport kotlin.io.println\n\nclass Greeter(val name: String) {\n    fun hello(): String {\n        return name\n    }\n    private fun secret() {}\n}\n\ninterface Service {\n    fun run()\n}\n\nenum class Color {\n    RED, GREEN\n}\n\nobject Registry\n\nfun main() {\n    println(\"hi\")\n}\n";
        let fs = extract(src);
        let names: Vec<_> = fs.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Greeter"), "types: {names:?}");
        assert!(
            names.contains(&"Greeter.hello"),
            "method qualified: {names:?}"
        );
        assert!(names.contains(&"Greeter.secret"));
        assert!(names.contains(&"Service"));
        assert!(names.contains(&"Color"));
        assert!(names.contains(&"Registry"));
        assert!(names.contains(&"main"), "top-level fn: {names:?}");

        let greeter = fs.symbols.iter().find(|s| s.name == "Greeter").unwrap();
        assert_eq!(greeter.kind, SymbolKind::Class);
        let svc = fs.symbols.iter().find(|s| s.name == "Service").unwrap();
        assert_eq!(svc.kind, SymbolKind::Interface);
        let color = fs.symbols.iter().find(|s| s.name == "Color").unwrap();
        assert_eq!(color.kind, SymbolKind::Enum);
        let main = fs.symbols.iter().find(|s| s.name == "main").unwrap();
        assert_eq!(main.kind, SymbolKind::Function);
        let secret = fs
            .symbols
            .iter()
            .find(|s| s.name == "Greeter.secret")
            .unwrap();
        assert_eq!(secret.visibility, Visibility::Internal);

        let targets: Vec<_> = fs.imports.iter().map(|i| i.to_source.as_str()).collect();
        assert!(
            targets.contains(&"kotlin.io.println"),
            "imports: {targets:?}"
        );
        assert!(fs.calls.is_empty() && !fs.calls_partial);
    }
}
