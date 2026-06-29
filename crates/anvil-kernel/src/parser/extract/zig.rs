//! Zig symbol and import extractor (LTW2-003) — T1 (Parsed).
//!
//! | Zig construct                  | `SymbolKind` | Note                          |
//! | ------------------------------ | ------------ | ----------------------------- |
//! | `pub fn f()` / `fn f()`        | `Function`   | top level                     |
//! | `pub fn m()` in a container    | `Method`     | qualified `Owner.m`           |
//! | `const T = struct {…}`         | `Class`      | nominal type                  |
//! | `const T = enum {…}`           | `Enum`       |                               |
//! | `const T = union {…}`          | `Class`      |                               |
//! | `const T = opaque {…}`         | `Class`      |                               |
//!
//! Zig has no keyword visibility ladder beyond `pub`: a declaration marked
//! `pub` is [`Visibility::Public`], everything else is file-private
//! ([`Visibility::Internal`]). Imports come from `@import("…")` bound to a
//! `const`, including the chained form `@import("std").mem`; the edge target is
//! the import string. The canonical `@cImport(@cInclude("…"))` form takes a
//! nested built-in, not a string literal, so it yields no edge — a documented
//! T1 limitation. Plain value constants/variables, container fields, `test`
//! blocks, and `comptime` blocks are out of T1 scope.

use anvil_kernel_types::{SymbolKind, SymbolNode, Visibility};

use super::tail_common::{
    field_text, finish, has_modifier, import_edge, node_text, push_symbol, strip_delims,
};
use super::{FileSymbols, ImportEdge, LanguageExtractor};

/// Extractor for Zig (`.zig`/`.zon`).
pub struct ZigExtractor;

impl LanguageExtractor for ZigExtractor {
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
                        let vis = zig_visibility(node, source);
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
                "variable_declaration" => {
                    emit_var_decl(node, source, file, &mut symbols, &mut next_id, &mut imports);
                }
                _ => {}
            }
        }

        finish(file, symbols, imports)
    }
}

/// A `const`/`var` declaration: either a type (`struct`/`enum`/`union`), an
/// `@import` binding, or a plain value (out of T1 scope).
fn emit_var_decl(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    next_id: &mut u64,
    imports: &mut Vec<ImportEdge>,
) {
    let Some(value) = decl_value(node) else {
        return;
    };
    match value.kind() {
        // `const std = @import("std");` — an import edge, no symbol.
        "builtin_function" => {
            if let Some(module) = import_target(value, source) {
                imports.push(import_edge(file, module, node));
            }
        }
        // `const mem = @import("std").mem;` — the import is the object of a field
        // access. Descend the object chain to the underlying `@import` so the
        // common chained form still yields an edge.
        "field_expression" => {
            if let Some(builtin) = builtin_in_object_chain(value)
                && let Some(module) = import_target(builtin, source)
            {
                imports.push(import_edge(file, module, node));
            }
        }
        "struct_declaration" | "union_declaration" | "opaque_declaration" => {
            emit_container(
                node,
                value,
                SymbolKind::Class,
                source,
                file,
                symbols,
                next_id,
            );
        }
        "enum_declaration" => {
            emit_container(
                node,
                value,
                SymbolKind::Enum,
                source,
                file,
                symbols,
                next_id,
            );
        }
        // A plain value constant/variable (`const x = 1;`) is out of T1 scope,
        // mirroring the Go extractor's const/var omission.
        _ => {}
    }
}

/// Emit a container type symbol named by its `const` binding, then one level of
/// methods inside its body as `Owner.method` (consistent with the anchor
/// extractors' `Owner.method` naming).
fn emit_container(
    decl: tree_sitter::Node,
    container: tree_sitter::Node,
    kind: SymbolKind,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    next_id: &mut u64,
) {
    let Some(name) = decl_name(decl, source) else {
        return;
    };
    let vis = zig_visibility(decl, source);
    push_symbol(symbols, next_id, file, kind, name.clone(), vis);

    let mut cursor = container.walk();
    for child in container.children(&mut cursor) {
        if child.kind() == "function_declaration"
            && let Some(method) = field_text(child, "name", source)
        {
            let mvis = zig_visibility(child, source);
            push_symbol(
                symbols,
                next_id,
                file,
                SymbolKind::Method,
                format!("{name}.{method}"),
                mvis,
            );
        }
    }
}

/// The declared name of a `variable_declaration` — the first `identifier` child
/// (the grammar exposes no `name` field on this node; the name precedes any
/// type annotation and the value).
fn decl_name(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            let text = node_text(child, source);
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}

/// The value node of a declaration — the first named child after the `=`
/// token. Anchoring on `=` (rather than "last non-identifier child") avoids
/// mistaking a *type annotation* for the value: in `const T: struct {…} = x;`
/// the `struct` node is the annotation, and the real value `x` follows `=`.
/// A declaration with no initialiser (`var x: i32;`) yields `None`.
fn decl_value(node: tree_sitter::Node) -> Option<tree_sitter::Node> {
    let mut cursor = node.walk();
    let mut after_eq = false;
    for child in node.children(&mut cursor) {
        if after_eq && child.is_named() {
            return Some(child);
        }
        if child.kind() == "=" {
            after_eq = true;
        }
    }
    None
}

/// Descend a `field_expression` object chain (`@import("std").os.linux`) to the
/// underlying `builtin_function`, if the access bottoms out at one.
fn builtin_in_object_chain(node: tree_sitter::Node) -> Option<tree_sitter::Node> {
    let mut current = node;
    loop {
        match current.kind() {
            "builtin_function" => return Some(current),
            "field_expression" => {
                current = current
                    .child_by_field_name("object")
                    .or_else(|| current.named_child(0))?;
            }
            _ => return None,
        }
    }
}

/// The import string of an `@import("path")` built-in call; `None` for any
/// other built-in. Only `@import` is matched: its canonical form takes a string
/// literal. `@cImport(@cInclude("…"))` takes a nested built-in, not a string,
/// so it never names a module here (documented T1 limitation).
fn import_target(builtin: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let mut cursor = builtin.walk();
    let mut is_import = false;
    let mut module = None;
    for child in builtin.children(&mut cursor) {
        match child.kind() {
            "builtin_identifier" => is_import = node_text(child, source) == "@import",
            "arguments" => module = first_string(child, source),
            _ => {}
        }
    }
    if is_import {
        module.filter(|m| !m.is_empty())
    } else {
        None
    }
}

/// The first string literal anywhere directly under `node`, with its quotes
/// stripped.
fn first_string(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "string" {
            return Some(strip_delims(&node_text(child, source)));
        }
    }
    None
}

/// Zig visibility: a `pub` modifier exports; otherwise the declaration is
/// file-private.
fn zig_visibility(node: tree_sitter::Node, source: &[u8]) -> Visibility {
    if has_modifier(node, "pub", source) {
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
        parser.set_language(&Language::Zig.ts_language()).unwrap();
        let tree = parser.parse(src, None).unwrap();
        ZigExtractor.extract(&tree, src.as_bytes(), "main.zig", 0)
    }

    #[test]
    fn extracts_types_methods_funcs_imports() {
        let src = "const std = @import(\"std\");\nconst mem = @import(\"std\").mem;\n\npub const Mood = enum { happy, sad };\n\npub const Greeter = struct {\n    name: []const u8,\n\n    pub fn greet(self: Greeter) []const u8 {\n        return self.name;\n    }\n\n    fn secret(self: Greeter) void {}\n};\n\npub fn topLevel() void {}\n\nfn internalHelper() void {}\n";
        let fs = extract(src);
        let names: Vec<_> = fs.symbols.iter().map(|s| s.name.as_str()).collect();

        assert!(names.contains(&"Greeter"), "struct type: {names:?}");
        assert!(
            names.contains(&"Greeter.greet"),
            "method qualified: {names:?}"
        );
        assert!(names.contains(&"Greeter.secret"));
        assert!(names.contains(&"Mood"), "enum type: {names:?}");
        assert!(names.contains(&"topLevel"));
        assert!(names.contains(&"internalHelper"));

        let greeter = fs.symbols.iter().find(|s| s.name == "Greeter").unwrap();
        assert_eq!(greeter.kind, SymbolKind::Class);
        assert_eq!(greeter.visibility, Visibility::Public);

        let mood = fs.symbols.iter().find(|s| s.name == "Mood").unwrap();
        assert_eq!(mood.kind, SymbolKind::Enum);

        let greet = fs
            .symbols
            .iter()
            .find(|s| s.name == "Greeter.greet")
            .unwrap();
        assert_eq!(greet.kind, SymbolKind::Method);
        assert_eq!(greet.visibility, Visibility::Public);

        let secret = fs
            .symbols
            .iter()
            .find(|s| s.name == "Greeter.secret")
            .unwrap();
        assert_eq!(secret.visibility, Visibility::Internal);

        let internal = fs
            .symbols
            .iter()
            .find(|s| s.name == "internalHelper")
            .unwrap();
        assert_eq!(internal.visibility, Visibility::Internal);

        // Both the direct `@import("std")` and the chained `@import("std").mem`
        // bindings yield a "std" edge — exactly two, no more (the plain consts
        // contribute none). A count assertion guards against the chained form
        // silently regressing to zero edges.
        let targets: Vec<_> = fs.imports.iter().map(|i| i.to_source.as_str()).collect();
        assert_eq!(targets, vec!["std", "std"], "import edges: {targets:?}");

        // No call sites at T1.
        assert!(fs.calls.is_empty() && !fs.calls_partial);
    }

    #[test]
    fn type_annotation_is_not_mistaken_for_a_type_definition() {
        // `const x: <type> = value;` — the struct/enum here is a *type
        // annotation*, not a type definition, so it must not emit a spurious
        // `Class`/`Enum` symbol. The value (`undefined`) is out of T1 scope.
        let fs = extract("const x: enum { a, b } = undefined;\n");
        assert!(
            fs.symbols.is_empty(),
            "annotation must not emit a symbol: {:?}",
            fs.symbols
        );
    }

    #[test]
    fn opaque_and_union_are_class_types() {
        let fs =
            extract("pub const Handle = opaque {};\npub const Tag = union { a: u8, b: u16 };\n");
        let handle = fs.symbols.iter().find(|s| s.name == "Handle").unwrap();
        assert_eq!(handle.kind, SymbolKind::Class);
        let tag = fs.symbols.iter().find(|s| s.name == "Tag").unwrap();
        assert_eq!(tag.kind, SymbolKind::Class);
    }

    #[test]
    fn plain_const_is_not_a_symbol() {
        // A value constant carries no type/import — out of T1 scope, so it must
        // not emit a blank or spurious symbol.
        let fs = extract("const answer = 42;\n");
        assert!(fs.symbols.is_empty(), "{:?}", fs.symbols);
        assert!(fs.imports.is_empty());
    }
}
