//! Rust symbol and import extractor (RSTLAN-002).
//!
//! Walks the `tree-sitter-rust` AST and emits [`FileSymbols`] — the same
//! `(SymbolNode, ImportEdge)` shape the TypeScript extractor produces — so the
//! symbol graph, architecture analysis, and drift baseline see Rust crates
//! exactly as they see TS modules. The grammar itself is wired in
//! [`super::super::languages`] (RSTLAN-001).
//!
//! Rust constructs map onto the existing language-agnostic [`SymbolKind`] set
//! rather than growing it (RSTLAN-002 "keep `FileSymbols` minimal"):
//!
//! | Rust item        | `SymbolKind` | Note                                   |
//! | ---------------- | ------------ | -------------------------------------- |
//! | `fn`             | `Function`   |                                        |
//! | `struct`/`union` | `Class`      | the nominal type with members          |
//! | `enum`           | `Enum`       |                                        |
//! | `trait`          | `Interface`  | a trait is Rust's interface contract   |
//! | `type X = …`     | `TypeAlias`  |                                        |
//! | `mod`            | `Module`     |                                        |
//! | `impl`/`trait` fn| `Method`     | qualified `Owner.method` (as TS-G2)    |
//!
//! Imports come from `use`, `pub use`, and `extern crate`. A `use` tree is
//! flattened to one [`ImportEdge`] per leaf path (`std::collections::HashMap`,
//! `crate::foo::bar`, each arm of `foo::{a, b}`), preserving the leading
//! anchor (`crate` / `super` / `self`) so RSTLAN-005's resolver can map the
//! path to a file. Re-export *name* tracking beyond the edge is deferred per
//! the T3 acceptance checklist; macro/proc-macro expansion is out of scope, so
//! macro-hidden symbols and edges are invisible to this static walk (a missed
//! edge is a missed drift signal, never a false violation).

use anvil_kernel_types::{SymbolKind, SymbolNode, TrustLevel, Visibility};

use super::{FileSymbols, ImportEdge, LanguageExtractor};

/// Extractor for the Rust anchor (`.rs`).
pub struct RustExtractor;

impl LanguageExtractor for RustExtractor {
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

        extract_from_node(
            tree.root_node(),
            source,
            file,
            &mut symbols,
            &mut imports,
            &mut next_id,
        );

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
        "function_item" => {
            push_named(node, source, file, symbols, next_id, SymbolKind::Function);
        }
        // A struct or union is the nominal type carrying members — the closest
        // language-agnostic kind is `Class`.
        "struct_item" | "union_item" => {
            push_named(node, source, file, symbols, next_id, SymbolKind::Class);
        }
        "enum_item" => {
            push_named(node, source, file, symbols, next_id, SymbolKind::Enum);
        }
        "type_item" => {
            push_named(node, source, file, symbols, next_id, SymbolKind::TypeAlias);
        }
        // A trait is Rust's interface contract; surface its methods as
        // `Trait.method` (mirrors TS-G2 class methods) from both default-bodied
        // `function_item`s and bodyless `function_signature_item`s.
        "trait_item" => {
            let owner = field_name_text(node, source);
            push_named(node, source, file, symbols, next_id, SymbolKind::Interface);
            if let Some(owner) = owner {
                emit_assoc_fns(node, source, file, symbols, next_id, &owner);
            }
        }
        // An `impl` block emits no symbol of its own; its associated functions
        // become `Type.method`. The Self type comes from the `type` field
        // (generics stripped: `Foo<T>` → `Foo`).
        "impl_item" => {
            if let Some(ty) = node.child_by_field_name("type") {
                let owner = base_type_name(&node_text(ty, source));
                emit_assoc_fns(node, source, file, symbols, next_id, &owner);
            }
        }
        // A module declaration is a structural symbol; recurse into an inline
        // body so nested items + `use` edges are captured. A bare `mod foo;`
        // (no body) just declares the child — file resolution is RSTLAN-005.
        "mod_item" => {
            push_named(node, source, file, symbols, next_id, SymbolKind::Module);
            if let Some(body) = node.child_by_field_name("body") {
                recurse_children(body, source, file, symbols, imports, next_id);
            }
        }
        "use_declaration" => extract_use(node, source, file, imports),
        "extern_crate_declaration" => extract_extern_crate(node, source, file, imports),
        // Everything else (the `source_file` root, attribute items, comments,
        // top-level statics/consts, expression statements) is traversed
        // generically — its children may hold items we do handle.
        _ => recurse_children(node, source, file, symbols, imports, next_id),
    }
}

/// Recurse into a node's named children with the top-level dispatcher.
fn recurse_children(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    imports: &mut Vec<ImportEdge>,
    next_id: &mut u64,
) {
    for i in 0..u32::try_from(node.named_child_count()).unwrap_or(0) {
        if let Some(child) = node.named_child(i) {
            extract_from_node(child, source, file, symbols, imports, next_id);
        }
    }
}

/// Push a symbol whose name is in the node's `name` field, with visibility
/// derived from a leading `visibility_modifier`.
fn push_named(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    next_id: &mut u64,
    kind: SymbolKind,
) {
    let Some(name) = field_name_text(node, source) else {
        return;
    };
    symbols.push(SymbolNode {
        id: *next_id,
        kind,
        name,
        visibility: item_visibility(node, source),
        file: file.to_string(),
        trust_level: TrustLevel::default(),
    });
    *next_id += 1;
}

/// Emit `Owner.method` symbols for the associated functions in an `impl` or
/// `trait` body. Both `function_item` (has a body) and `function_signature_item`
/// (trait declaration, no body) carry a `name` field. A method's visibility
/// follows its own modifier (trait items are effectively public via the trait,
/// but we report only what the source states, conservatively).
fn emit_assoc_fns(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    next_id: &mut u64,
    owner: &str,
) {
    let Some(body) = node.child_by_field_name("body") else {
        return;
    };
    for i in 0..u32::try_from(body.named_child_count()).unwrap_or(0) {
        let Some(member) = body.named_child(i) else {
            continue;
        };
        if !matches!(member.kind(), "function_item" | "function_signature_item") {
            continue;
        }
        if let Some(method) = field_name_text(member, source) {
            symbols.push(SymbolNode {
                id: *next_id,
                kind: SymbolKind::Method,
                name: format!("{owner}.{method}"),
                visibility: item_visibility(member, source),
                file: file.to_string(),
                trust_level: TrustLevel::default(),
            });
            *next_id += 1;
        }
    }
}

/// Flatten a `use_declaration` into one [`ImportEdge`] per leaf path.
fn extract_use(node: tree_sitter::Node, source: &[u8], file: &str, imports: &mut Vec<ImportEdge>) {
    let Some(arg) = node.child_by_field_name("argument") else {
        return;
    };
    let mut paths = Vec::new();
    collect_use_paths(arg, source, "", &mut paths);
    let line = node_line(node);
    for path in paths {
        if !path.is_empty() {
            imports.push(ImportEdge {
                from_file: file.to_string(),
                to_source: path,
                line,
            });
        }
    }
}

/// Recursively flatten a `use` tree, accumulating the `::`-joined prefix so a
/// grouped `foo::{a, b}` yields `foo::a` and `foo::b`.
fn collect_use_paths(node: tree_sitter::Node, source: &[u8], prefix: &str, out: &mut Vec<String>) {
    match node.kind() {
        "scoped_use_list" => {
            let path = node
                .child_by_field_name("path")
                .map(|p| node_text(p, source))
                .unwrap_or_default();
            let new_prefix = join_path(prefix, &path);
            if let Some(list) = node.child_by_field_name("list") {
                for i in 0..u32::try_from(list.named_child_count()).unwrap_or(0) {
                    if let Some(child) = list.named_child(i) {
                        collect_use_paths(child, source, &new_prefix, out);
                    }
                }
            }
        }
        "use_list" => {
            for i in 0..u32::try_from(node.named_child_count()).unwrap_or(0) {
                if let Some(child) = node.named_child(i) {
                    collect_use_paths(child, source, prefix, out);
                }
            }
        }
        // `foo::bar as baz` — the edge tracks the imported path, not the alias.
        "use_as_clause" => {
            let path = node
                .child_by_field_name("path")
                .map(|p| node_text(p, source))
                .unwrap_or_default();
            out.push(join_path(prefix, &path));
        }
        // `foo::*` — record the module the glob reaches into.
        "use_wildcard" => {
            let path = node
                .named_child(0)
                .map(|p| node_text(p, source))
                .unwrap_or_default();
            out.push(join_path(prefix, &path));
        }
        // Leaf paths: `foo`, `crate::a::B`, `super::x`, `self::y`.
        _ => out.push(join_path(prefix, &node_text(node, source))),
    }
}

fn extract_extern_crate(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    imports: &mut Vec<ImportEdge>,
) {
    if let Some(name) = field_name_text(node, source) {
        imports.push(ImportEdge {
            from_file: file.to_string(),
            to_source: name,
            line: node_line(node),
        });
    }
}

/// Join a `::`-separated prefix with a path segment, tolerating an empty side.
fn join_path(prefix: &str, path: &str) -> String {
    match (prefix.is_empty(), path.is_empty()) {
        (true, _) => path.to_string(),
        (false, true) => prefix.to_string(),
        (false, false) => format!("{prefix}::{path}"),
    }
}

/// Strip generic arguments from a type expression: `Foo<T>` → `Foo`.
fn base_type_name(ty: &str) -> String {
    ty.split('<')
        .next()
        .unwrap_or(ty)
        .trim()
        .trim_end_matches('>')
        .trim()
        .to_string()
}

/// Visibility of an item, read from a leading `visibility_modifier`. Bare `pub`
/// is `Public`; a restricted `pub(crate)` / `pub(super)` / `pub(in …)` is
/// crate-internal, so it maps to `Internal`. No modifier ⇒ `Internal`.
fn item_visibility(node: tree_sitter::Node, source: &[u8]) -> Visibility {
    for i in 0..u32::try_from(node.named_child_count()).unwrap_or(0) {
        if let Some(child) = node.named_child(i)
            && child.kind() == "visibility_modifier"
        {
            return if node_text(child, source).trim() == "pub" {
                Visibility::Public
            } else {
                Visibility::Internal
            };
        }
    }
    Visibility::Internal
}

/// Text of the node's `name` field, if present.
fn field_name_text(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .map(|n| node_text(n, source))
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

    use anvil_kernel_types::{SymbolKind, Visibility};

    use super::{LanguageExtractor, RustExtractor};
    use crate::parser::Parser;
    use crate::parser::extract::extract_symbols;

    fn extract(source: &[u8]) -> anvil_kernel_types::FileSymbols {
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("src/lib.rs"), source).unwrap();
        extract_symbols(&result.tree, source, Path::new("src/lib.rs"), 0)
    }

    fn names_of(fs: &anvil_kernel_types::FileSymbols, kind: SymbolKind) -> Vec<&str> {
        fs.symbols
            .iter()
            .filter(|s| s.kind == kind)
            .map(|s| s.name.as_str())
            .collect()
    }

    fn sources(fs: &anvil_kernel_types::FileSymbols) -> Vec<&str> {
        fs.imports.iter().map(|i| i.to_source.as_str()).collect()
    }

    #[test]
    fn extracts_functions_structs_enums_traits_type_aliases() {
        let fs = extract(
            b"
pub fn run() {}
fn helper() {}
pub struct Config { field: u32 }
enum Mode { On, Off }
pub trait Service { fn start(&self); }
type Id = u64;
",
        );

        assert_eq!(names_of(&fs, SymbolKind::Function), ["run", "helper"]);
        assert_eq!(names_of(&fs, SymbolKind::Class), ["Config"]);
        assert_eq!(names_of(&fs, SymbolKind::Enum), ["Mode"]);
        assert_eq!(names_of(&fs, SymbolKind::Interface), ["Service"]);
        assert_eq!(names_of(&fs, SymbolKind::TypeAlias), ["Id"]);
    }

    #[test]
    fn visibility_tracks_pub_modifier() {
        let fs = extract(
            b"
pub fn public_fn() {}
fn private_fn() {}
pub(crate) fn crate_fn() {}
",
        );
        let vis = |name: &str| {
            fs.symbols
                .iter()
                .find(|s| s.name == name)
                .map(|s| s.visibility)
        };
        assert_eq!(vis("public_fn"), Some(Visibility::Public));
        assert_eq!(vis("private_fn"), Some(Visibility::Internal));
        // `pub(crate)` is not part of the external API surface.
        assert_eq!(vis("crate_fn"), Some(Visibility::Internal));
    }

    #[test]
    fn impl_methods_are_qualified_by_type() {
        let fs = extract(
            b"
struct Server;
impl Server {
    pub fn new() -> Self { Server }
    fn tick(&self) {}
}
",
        );
        let methods = names_of(&fs, SymbolKind::Method);
        assert!(methods.contains(&"Server.new"), "got {methods:?}");
        assert!(methods.contains(&"Server.tick"), "got {methods:?}");
        // The impl block itself emits no symbol; the struct does.
        assert_eq!(names_of(&fs, SymbolKind::Class), ["Server"]);
    }

    #[test]
    fn generic_impl_type_is_stripped_to_base_name() {
        let fs = extract(
            b"
struct Cache<K> { _k: K }
impl<K> Cache<K> {
    fn get(&self) {}
}
",
        );
        let methods = names_of(&fs, SymbolKind::Method);
        assert!(methods.contains(&"Cache.get"), "got {methods:?}");
    }

    #[test]
    fn trait_methods_are_qualified_by_trait() {
        let fs = extract(
            b"
pub trait Greeter {
    fn greet(&self) -> String;
    fn shout(&self) -> String { String::new() }
}
",
        );
        let methods = names_of(&fs, SymbolKind::Method);
        assert!(methods.contains(&"Greeter.greet"), "got {methods:?}");
        assert!(methods.contains(&"Greeter.shout"), "got {methods:?}");
    }

    #[test]
    fn module_symbol_and_nested_items_captured() {
        let fs = extract(
            b"
mod inner {
    pub fn nested() {}
    use std::fmt;
}
mod sibling;
",
        );
        let mods = names_of(&fs, SymbolKind::Module);
        assert!(mods.contains(&"inner"));
        assert!(mods.contains(&"sibling"));
        // Items inside an inline module body are captured.
        assert!(names_of(&fs, SymbolKind::Function).contains(&"nested"));
        assert!(sources(&fs).contains(&"std::fmt"), "nested use edge");
    }

    #[test]
    fn extracts_simple_and_scoped_use_edges() {
        let fs = extract(
            b"
use std::collections::HashMap;
use crate::config::Settings;
use super::sibling::thing;
use self::local::Item;
",
        );
        let s = sources(&fs);
        assert!(s.contains(&"std::collections::HashMap"), "got {s:?}");
        assert!(s.contains(&"crate::config::Settings"), "got {s:?}");
        assert!(s.contains(&"super::sibling::thing"), "got {s:?}");
        assert!(s.contains(&"self::local::Item"), "got {s:?}");
    }

    #[test]
    fn grouped_use_expands_to_one_edge_per_leaf() {
        let fs = extract(b"use foo::bar::{Alpha, Beta, gamma::Delta};\n");
        let s = sources(&fs);
        assert!(s.contains(&"foo::bar::Alpha"), "got {s:?}");
        assert!(s.contains(&"foo::bar::Beta"), "got {s:?}");
        assert!(s.contains(&"foo::bar::gamma::Delta"), "got {s:?}");
    }

    #[test]
    fn use_alias_and_wildcard_and_extern_crate() {
        let fs = extract(
            b"
use anyhow::Result as AnyResult;
use std::io::*;
extern crate serde;
",
        );
        let s = sources(&fs);
        assert!(
            s.contains(&"anyhow::Result"),
            "alias keeps imported path: {s:?}"
        );
        assert!(s.contains(&"std::io"), "wildcard records the module: {s:?}");
        assert!(s.contains(&"serde"), "extern crate edge: {s:?}");
    }

    #[test]
    fn pub_use_is_recorded_as_an_edge() {
        let fs = extract(b"pub use crate::internal::Widget;\n");
        assert!(sources(&fs).contains(&"crate::internal::Widget"));
    }

    #[test]
    fn use_edge_carries_one_based_line_number() {
        let fs = extract(b"\n\nuse std::fmt::Debug;\n");
        let edge = fs
            .imports
            .iter()
            .find(|i| i.to_source == "std::fmt::Debug")
            .unwrap();
        assert_eq!(edge.line, 3, "use on line 3 (1-based)");
    }

    #[test]
    fn assigns_unique_ids_from_offset() {
        let mut parser = Parser::new();
        let source = b"fn a() {}\nstruct B;\nenum C { X }\n";
        let result = parser.parse_bytes(Path::new("src/lib.rs"), source).unwrap();
        let fs = extract_symbols(&result.tree, source, Path::new("src/lib.rs"), 100);

        let ids: Vec<u64> = fs.symbols.iter().map(|s| s.id).collect();
        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(ids.len(), unique.len(), "ids must be unique");
        assert!(ids.iter().all(|&id| id >= 100), "ids rebased from offset");
    }

    #[test]
    fn all_symbols_and_imports_attributed_to_file() {
        let fs = extract(b"use std::fmt;\npub fn f() {}\n");
        assert!(fs.symbols.iter().all(|s| s.file == "src/lib.rs"));
        assert!(fs.imports.iter().all(|i| i.from_file == "src/lib.rs"));
    }

    #[test]
    fn orchestrator_dispatch_matches_direct_extractor_call() {
        let mut parser = Parser::new();
        let source = b"pub fn f() {}\nuse std::fmt;\n";
        let result = parser.parse_bytes(Path::new("a.rs"), source).unwrap();

        let via_orchestrator = extract_symbols(&result.tree, source, Path::new("a.rs"), 0);
        let via_trait = RustExtractor.extract(&result.tree, source, "a.rs", 0);

        assert_eq!(via_orchestrator.symbols.len(), via_trait.symbols.len());
        assert_eq!(via_orchestrator.imports.len(), via_trait.imports.len());
        assert_eq!(via_orchestrator.symbols[0].name, "f");
    }
}
