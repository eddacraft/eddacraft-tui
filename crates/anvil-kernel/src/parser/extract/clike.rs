//! C and C++ symbol and import extractors (LANGTAIL-007) — T1 (Parsed).
//!
//! C and C++ share most of their declaration grammar, so one walker backs both,
//! switching on a `cpp` flag for the C++-only constructs (namespaces, classes,
//! templates, in-class methods).
//!
//! | C / C++ construct          | `SymbolKind` | Note                          |
//! | -------------------------- | ------------ | ----------------------------- |
//! | `int f(...) {…}` / proto   | `Function`   | name read through declarators |
//! | `struct S` / `class C`     | `Class`      | C++ `class` is `cpp`-only     |
//! | `enum E`                   | `Enum`       |                               |
//! | `typedef … N`              | `TypeAlias`  | C / C++                       |
//! | method in a C++ class body | `Method`     | qualified `Owner.method`      |
//!
//! Includes (`#include <h>` / `"h"`) become import edges to the header path.
//! C++ declarations nest inside `namespace`/`template`/`extern "C"` blocks, all
//! of which the walk recurses through. C has no namespaces, classes, or methods.
//! There is no source-level visibility concept at file scope, so a C `static`
//! storage-class declaration reads as `Internal` and everything else `Public`;
//! C++ members default `Public` at T1 (access-specifier tracking is T2+).
//!
//! The `.h` ambiguity (C vs C++ header) is resolved deterministically at
//! detection time — `.h` is C — so a C++-only header kept as `.h` is parsed by
//! the C grammar; that is a documented T1 limitation, not a parse failure.

use anvil_kernel_types::{SymbolKind, SymbolNode, Visibility};

use super::tail_common::{finish, has_modifier, import_edge, node_text, push_symbol, strip_delims};
use super::{FileSymbols, ImportEdge, LanguageExtractor};

/// Extractor for C (`.c` / `.h`).
pub struct CExtractor;

impl LanguageExtractor for CExtractor {
    fn extract(
        &self,
        tree: &tree_sitter::Tree,
        source: &[u8],
        file: &str,
        id_offset: u64,
    ) -> FileSymbols {
        extract_clike(tree, source, file, id_offset, false)
    }
}

/// Extractor for C++ (`.cpp`/`.cc`/`.cxx`/`.hpp`/…).
pub struct CppExtractor;

impl LanguageExtractor for CppExtractor {
    fn extract(
        &self,
        tree: &tree_sitter::Tree,
        source: &[u8],
        file: &str,
        id_offset: u64,
    ) -> FileSymbols {
        extract_clike(tree, source, file, id_offset, true)
    }
}

fn extract_clike(
    tree: &tree_sitter::Tree,
    source: &[u8],
    file: &str,
    id_offset: u64,
    cpp: bool,
) -> FileSymbols {
    let mut symbols = Vec::new();
    let mut imports = Vec::new();
    let mut next_id = id_offset;

    walk(
        tree.root_node(),
        source,
        file,
        cpp,
        &mut symbols,
        &mut imports,
        &mut next_id,
    );

    finish(file, symbols, imports)
}

fn walk(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    cpp: bool,
    symbols: &mut Vec<SymbolNode>,
    imports: &mut Vec<ImportEdge>,
    next_id: &mut u64,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "preproc_include" => {
                if let Some(path) = include_path(child, source) {
                    imports.push(import_edge(file, path, child));
                }
            }
            "function_definition" => emit_function(child, source, file, symbols, next_id),
            // A declaration is a function prototype when its declarator chain
            // bottoms out in a `function_declarator` — important for headers,
            // which often carry only prototypes. Variable declarations are not
            // T1 symbols.
            "declaration" if has_function_declarator(child) => {
                emit_function(child, source, file, symbols, next_id);
            }
            "struct_specifier" | "union_specifier" => {
                emit_type(
                    child,
                    source,
                    file,
                    SymbolKind::Class,
                    cpp,
                    symbols,
                    next_id,
                );
            }
            "class_specifier" if cpp => {
                emit_type(
                    child,
                    source,
                    file,
                    SymbolKind::Class,
                    cpp,
                    symbols,
                    next_id,
                );
            }
            "enum_specifier" => {
                emit_type(
                    child,
                    source,
                    file,
                    SymbolKind::Enum,
                    false,
                    symbols,
                    next_id,
                );
            }
            "type_definition" => emit_typedef(child, source, file, symbols, next_id),
            "namespace_definition" | "linkage_specification" | "template_declaration" if cpp => {
                // Recurse through C++ wrappers so the declaration inside is found
                // with the same top-level treatment.
                let target = child.child_by_field_name("body").unwrap_or(child);
                walk(target, source, file, cpp, symbols, imports, next_id);
            }
            // Preprocessor conditionals (`#ifndef GUARD … #endif`, `#if …`)
            // wrap declarations as their children — near-universal in headers
            // via include guards. Recurse so guarded declarations are not lost.
            // Applies to both C and C++ (the preprocessor is shared).
            "preproc_ifdef" | "preproc_if" | "preproc_else" | "preproc_elif" => {
                walk(child, source, file, cpp, symbols, imports, next_id);
            }
            _ => {}
        }
    }
}

/// Emit a free function / prototype, reading the name through the declarator
/// chain. A C `static` declaration is `Internal`.
fn emit_function(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    next_id: &mut u64,
) {
    if let Some(name) = node
        .child_by_field_name("declarator")
        .and_then(|d| declarator_name(d, source))
    {
        let vis = if has_modifier(node, "static", source) {
            Visibility::Internal
        } else {
            Visibility::Public
        };
        push_symbol(symbols, next_id, file, SymbolKind::Function, name, vis);
    }
}

/// Emit a named `struct`/`union`/`class`/`enum`. Anonymous specifiers (no
/// `name` field, e.g. `typedef struct {…} N;`) are skipped here — the `typedef`
/// arm names them. For C++ class/struct bodies, member methods are emitted as
/// `Owner.method`.
fn emit_type(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    kind: SymbolKind,
    cpp: bool,
    symbols: &mut Vec<SymbolNode>,
    next_id: &mut u64,
) {
    let Some(name) = node
        .child_by_field_name("name")
        .map(|n| node_text(n, source))
        .filter(|s| !s.is_empty())
    else {
        return;
    };
    push_symbol(
        symbols,
        next_id,
        file,
        kind,
        name.clone(),
        Visibility::Public,
    );
    if cpp
        && kind == SymbolKind::Class
        && let Some(body) = node.child_by_field_name("body")
    {
        emit_methods(body, source, file, &name, symbols, next_id);
    }
}

/// Emit `Owner.method` for declared/defined methods in a C++ class body. Data
/// members (a `field_declaration` whose declarator is not a function) are not
/// T1 symbols.
fn emit_methods(
    body: tree_sitter::Node,
    source: &[u8],
    file: &str,
    owner: &str,
    symbols: &mut Vec<SymbolNode>,
    next_id: &mut u64,
) {
    let mut cursor = body.walk();
    for member in body.children(&mut cursor) {
        // `function_definition` — inline method bodies. `field_declaration` —
        // ordinary method prototypes. `declaration` — constructor/destructor
        // prototypes (`Foo();`, `~Foo();`), which the grammar models as a plain
        // declaration rather than a field. All three are methods only when the
        // declarator chain bottoms out in a function declarator.
        let is_method = match member.kind() {
            "function_definition" => true,
            "field_declaration" | "declaration" => has_function_declarator(member),
            _ => false,
        };
        if !is_method {
            continue;
        }
        if let Some(name) = member
            .child_by_field_name("declarator")
            .and_then(|d| declarator_name(d, source))
        {
            push_symbol(
                symbols,
                next_id,
                file,
                SymbolKind::Method,
                format!("{owner}.{name}"),
                Visibility::Public,
            );
        }
    }
}

/// Emit the new type name(s) introduced by a `typedef`. Only the `declarator`
/// field(s) are emitted — never the source `type` — so `typedef MyInt Other;`
/// adds `Other`, not a spurious second `MyInt` (which the original `typedef int
/// MyInt;` already named). Pointer/array/function-pointer wrappers are followed
/// to the leaf name, and a single `typedef int a, b;` emits both `a` and `b`.
fn emit_typedef(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    next_id: &mut u64,
) {
    let mut cursor = node.walk();
    if !cursor.goto_first_child() {
        return;
    }
    loop {
        if cursor.field_name() == Some("declarator")
            && let Some(name) = declarator_name(cursor.node(), source)
        {
            push_symbol(
                symbols,
                next_id,
                file,
                SymbolKind::TypeAlias,
                name,
                Visibility::Public,
            );
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
}

/// Follow the `declarator` chain to the leaf name. Handles pointer / array /
/// function wrappers and C++ `qualified_identifier` (`Foo::bar` → `bar`).
fn declarator_name(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" | "field_identifier" | "type_identifier" | "destructor_name"
        | "operator_name" => Some(node_text(node, source)),
        "qualified_identifier" => node
            .child_by_field_name("name")
            .and_then(|n| declarator_name(n, source)),
        _ => node
            .child_by_field_name("declarator")
            .and_then(|d| declarator_name(d, source)),
    }
}

/// Whether a declaration's declarator chain bottoms out in a function
/// declarator (i.e. it declares a function, not a variable).
fn has_function_declarator(node: tree_sitter::Node) -> bool {
    let mut current = node.child_by_field_name("declarator");
    while let Some(n) = current {
        if n.kind() == "function_declarator" {
            return true;
        }
        current = n.child_by_field_name("declarator");
    }
    false
}

/// The header path of an `#include`, stripped of `<>` or `""`.
fn include_path(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let path = node.child_by_field_name("path")?;
    let stripped = strip_delims(&node_text(path, source));
    (!stripped.is_empty()).then_some(stripped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::languages::Language;

    fn extract_c(src: &str) -> FileSymbols {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&Language::C.ts_language()).unwrap();
        let tree = parser.parse(src, None).unwrap();
        CExtractor.extract(&tree, src.as_bytes(), "mod.c", 0)
    }

    fn extract_cpp(src: &str) -> FileSymbols {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&Language::Cpp.ts_language()).unwrap();
        let tree = parser.parse(src, None).unwrap();
        CppExtractor.extract(&tree, src.as_bytes(), "mod.cpp", 0)
    }

    #[test]
    fn c_extracts_functions_types_includes() {
        let src = "#include <stdio.h>\n#include \"local.h\"\n\nstruct Point { int x; int y; };\n\nenum Color { RED, GREEN };\n\ntypedef int myint;\n\nstatic int helper(void) { return 0; }\n\nint main(void) {\n    return 0;\n}\n";
        let fs = extract_c(src);
        let names: Vec<_> = fs.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"main"), "funcs: {names:?}");
        assert!(names.contains(&"helper"));
        assert!(names.contains(&"Point"));
        assert!(names.contains(&"Color"));
        assert!(names.contains(&"myint"));

        let point = fs.symbols.iter().find(|s| s.name == "Point").unwrap();
        assert_eq!(point.kind, SymbolKind::Class);
        let color = fs.symbols.iter().find(|s| s.name == "Color").unwrap();
        assert_eq!(color.kind, SymbolKind::Enum);
        let myint = fs.symbols.iter().find(|s| s.name == "myint").unwrap();
        assert_eq!(myint.kind, SymbolKind::TypeAlias);
        let helper = fs.symbols.iter().find(|s| s.name == "helper").unwrap();
        assert_eq!(helper.visibility, Visibility::Internal);
        let main = fs.symbols.iter().find(|s| s.name == "main").unwrap();
        assert_eq!(main.visibility, Visibility::Public);

        let targets: Vec<_> = fs.imports.iter().map(|i| i.to_source.as_str()).collect();
        assert!(targets.contains(&"stdio.h"), "includes: {targets:?}");
        assert!(targets.contains(&"local.h"));
        assert!(fs.calls.is_empty() && !fs.calls_partial);
    }

    #[test]
    fn cpp_extracts_namespaced_classes_methods() {
        let src = "#include <string>\n\nnamespace app {\n\nclass Greeter {\npublic:\n    std::string hello();\n    void run() { }\n};\n\nint global() { return 0; }\n\n}\n";
        let fs = extract_cpp(src);
        let names: Vec<_> = fs.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Greeter"), "types (through ns): {names:?}");
        assert!(names.contains(&"Greeter.hello"), "method decl: {names:?}");
        assert!(names.contains(&"Greeter.run"), "inline method: {names:?}");
        assert!(names.contains(&"global"), "ns free fn: {names:?}");

        let greeter = fs.symbols.iter().find(|s| s.name == "Greeter").unwrap();
        assert_eq!(greeter.kind, SymbolKind::Class);

        let targets: Vec<_> = fs.imports.iter().map(|i| i.to_source.as_str()).collect();
        assert!(targets.contains(&"string"), "includes: {targets:?}");
    }

    #[test]
    fn c_include_guarded_header_still_yields_symbols() {
        // Include guards wrap all declarations in a `preproc_ifdef`; the walk
        // must recurse into it or headers (the common `.h` case) yield nothing.
        let src = "#ifndef FOO_H\n#define FOO_H\n\nstruct Foo { int x; };\nint foo_init(void);\n\n#endif\n";
        let fs = extract_c(src);
        let names: Vec<_> = fs.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Foo"), "guarded struct: {names:?}");
        assert!(names.contains(&"foo_init"), "guarded prototype: {names:?}");
    }

    #[test]
    fn typedef_emits_only_the_new_name_not_the_source_type() {
        // `typedef MyInt Other;` must add only `Other` — `MyInt` was already
        // named by its own typedef and must not be double-emitted.
        let src = "typedef int MyInt;\ntypedef MyInt Other;\n";
        let fs = extract_c(src);
        let aliases: Vec<_> = fs
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::TypeAlias)
            .map(|s| s.name.as_str())
            .collect();
        assert_eq!(
            aliases,
            vec!["MyInt", "Other"],
            "exactly one alias per typedef, no source-type double-emit: {aliases:?}"
        );
    }

    #[test]
    fn cpp_constructor_and_destructor_declarations_are_methods() {
        let src = "class Widget {\npublic:\n    Widget();\n    ~Widget();\n    void draw();\n};\n";
        let fs = extract_cpp(src);
        let names: Vec<_> = fs.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Widget.Widget"), "constructor: {names:?}");
        assert!(names.contains(&"Widget.~Widget"), "destructor: {names:?}");
        assert!(names.contains(&"Widget.draw"));
    }

    #[test]
    fn cpp_template_function_is_found() {
        let src = "template<class T>\nT identity(T x) { return x; }\n";
        let fs = extract_cpp(src);
        let names: Vec<_> = fs.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"identity"), "template fn: {names:?}");
    }
}
