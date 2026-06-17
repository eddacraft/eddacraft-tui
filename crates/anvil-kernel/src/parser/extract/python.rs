//! Python symbol and import extractor (PYLAN-002).
//!
//! Walks the `tree-sitter-python` AST and emits [`FileSymbols`] — the same
//! `(SymbolNode, ImportEdge)` shape the TypeScript and Rust extractors produce —
//! so the symbol graph, architecture analysis, and drift baseline see Python
//! modules exactly as they see TS modules and Rust crates. The grammar is wired
//! in [`super::super::languages`] (PYLAN-001).
//!
//! Python constructs map onto the existing language-agnostic [`SymbolKind`] set
//! rather than growing it:
//!
//! | Python construct      | `SymbolKind` | Note                                  |
//! | --------------------- | ------------ | ------------------------------------- |
//! | `def`                 | `Function`   | module-level / nested defs            |
//! | `class`               | `Class`      | the nominal type with members         |
//! | `def` inside a `class`| `Method`     | qualified `Owner.method` (as TS-G2)   |
//!
//! Imports come from `import` and `from … import …`. Each statement yields one
//! [`ImportEdge`] to the imported *module* (`import a.b.c` → `a.b.c`;
//! `from a.b import x, y` → `a.b`; `from . import x` → `.`; `from ..pkg import y`
//! → `..pkg`), preserving the relative-import prefix so a later resolver can map
//! it to a file. The imported *names* are not tracked as edges (mirrors the TS
//! `import { x } from "m"` → one module edge). Re-exports (the implicit
//! `__init__.py` `from .x import y` convention) and call sites are out of scope
//! here — re-export-name tracking and `calls` (GCALL-005) are separate items.
//! `importlib.import_module(...)` dynamic imports are invisible to this static
//! walk (a missed edge is a missed drift signal, never a false violation).

use anvil_kernel_types::{SymbolKind, SymbolNode, TrustLevel, Visibility};

use super::{FileSymbols, ImportEdge, LanguageExtractor, ReexportEdge};

/// Extractor for the Python anchor (`.py`, `.pyi`).
pub struct PythonExtractor;

impl LanguageExtractor for PythonExtractor {
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
            // Python has no first-class re-export syntax; the implicit
            // `__init__.py` convention is deferred. Call sites are GCALL-005.
            reexports: Vec::<ReexportEdge>::new(),
            calls: Vec::new(),
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
        // A module-level (or nested) `def`. Functions inside a class body are
        // emitted as `Owner.method` by `emit_methods`, so this arm sees only
        // non-class-body defs.
        "function_definition" => {
            push_named(node, source, file, symbols, next_id, SymbolKind::Function);
        }
        "class_definition" => {
            let owner = field_name_text(node, source);
            push_named(node, source, file, symbols, next_id, SymbolKind::Class);
            if let Some(owner) = owner {
                emit_methods(node, source, file, symbols, next_id, &owner);
            }
        }
        // `@decorator`-wrapped def/class — the real definition is in the
        // `definition` field; dispatch on it so decorated top-level items still
        // emit their symbol.
        "decorated_definition" => {
            if let Some(def) = node.child_by_field_name("definition") {
                extract_from_node(def, source, file, symbols, imports, next_id);
            }
        }
        "import_statement" => extract_import(node, source, file, imports),
        "import_from_statement" => extract_import_from(node, source, file, imports),
        // Everything else (the `module` root, expression statements, `if`/`try`
        // blocks holding conditional imports) is traversed generically.
        _ => recurse_children(node, source, file, symbols, imports, next_id),
    }
}

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

/// Push a symbol whose name is in the node's `name` field.
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
    let visibility = name_visibility(&name);
    symbols.push(SymbolNode {
        id: *next_id,
        kind,
        name,
        visibility,
        file: file.to_string(),
        trust_level: TrustLevel::default(),
    });
    *next_id += 1;
}

/// Emit `Owner.method` symbols for the `def`s directly in a class body. A method
/// may be wrapped in a `decorated_definition` (`@property`, `@staticmethod`); the
/// inner `function_definition` carries the `name`.
fn emit_methods(
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
        let method = match member.kind() {
            "function_definition" => member,
            "decorated_definition" => match member.child_by_field_name("definition") {
                Some(def) if def.kind() == "function_definition" => def,
                _ => continue,
            },
            _ => continue,
        };
        if let Some(name) = field_name_text(method, source) {
            symbols.push(SymbolNode {
                id: *next_id,
                kind: SymbolKind::Method,
                name: format!("{owner}.{name}"),
                visibility: name_visibility(&name),
                file: file.to_string(),
                trust_level: TrustLevel::default(),
            });
            *next_id += 1;
        }
    }
}

/// `import a`, `import a.b.c`, `import a.b as c` — one edge per imported module
/// (the `dotted_name`, or an `aliased_import`'s `name`).
fn extract_import(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    imports: &mut Vec<ImportEdge>,
) {
    let line = node_line(node);
    for i in 0..u32::try_from(node.named_child_count()).unwrap_or(0) {
        let Some(child) = node.named_child(i) else {
            continue;
        };
        let module = match child.kind() {
            "dotted_name" => node_text(child, source),
            "aliased_import" => child
                .child_by_field_name("name")
                .map(|n| node_text(n, source))
                .unwrap_or_default(),
            _ => continue,
        };
        if !module.is_empty() {
            imports.push(ImportEdge {
                from_file: file.to_string(),
                to_source: module,
                line,
            });
        }
    }
}

/// `from a.b import x, y`, `from . import x`, `from ..pkg import y` — one edge to
/// the `module_name` (a `dotted_name` or a `relative_import` whose text carries
/// the leading dots). The imported names are not tracked as separate edges.
fn extract_import_from(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    imports: &mut Vec<ImportEdge>,
) {
    let Some(module_node) = node.child_by_field_name("module_name") else {
        return;
    };
    let module = node_text(module_node, source);
    if !module.is_empty() {
        imports.push(ImportEdge {
            from_file: file.to_string(),
            to_source: module,
            line: node_line(node),
        });
    }
}

/// Python visibility convention: a leading underscore marks a non-public name
/// (`_internal`, `__private`); everything else is public. Dunder names
/// (`__init__`) start with `_` and so read as internal, which is conservative
/// and harmless for surface diffing.
fn name_visibility(name: &str) -> Visibility {
    if name.starts_with('_') {
        Visibility::Internal
    } else {
        Visibility::Public
    }
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

    use crate::parser::Parser;
    use crate::parser::extract::extract_symbols;

    fn extract(source: &[u8]) -> anvil_kernel_types::FileSymbols {
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("pkg/mod.py"), source).unwrap();
        extract_symbols(&result.tree, source, Path::new("pkg/mod.py"), 0)
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
    fn extracts_functions_and_classes() {
        let fs = extract(b"def run():\n    pass\n\nclass Config:\n    pass\n");
        assert_eq!(names_of(&fs, SymbolKind::Function), ["run"]);
        assert_eq!(names_of(&fs, SymbolKind::Class), ["Config"]);
    }

    #[test]
    fn class_methods_are_qualified_by_the_class() {
        let fs = extract(
            b"class Service:\n    def start(self):\n        pass\n    def _stop(self):\n        pass\n",
        );
        assert_eq!(
            names_of(&fs, SymbolKind::Method),
            ["Service.start", "Service._stop"]
        );
        // The class itself is still a single Class symbol.
        assert_eq!(names_of(&fs, SymbolKind::Class), ["Service"]);
    }

    #[test]
    fn decorated_def_and_method_are_emitted() {
        let fs = extract(
            b"@app.route('/')\ndef handler():\n    pass\n\nclass S:\n    @property\n    def value(self):\n        return 1\n",
        );
        assert_eq!(names_of(&fs, SymbolKind::Function), ["handler"]);
        assert_eq!(names_of(&fs, SymbolKind::Method), ["S.value"]);
    }

    #[test]
    fn visibility_follows_leading_underscore() {
        let fs = extract(b"def public():\n    pass\n\ndef _internal():\n    pass\n");
        let vis = |name: &str| {
            fs.symbols
                .iter()
                .find(|s| s.name == name)
                .map(|s| s.visibility)
        };
        assert_eq!(vis("public"), Some(Visibility::Public));
        assert_eq!(vis("_internal"), Some(Visibility::Internal));
    }

    #[test]
    fn plain_and_dotted_and_aliased_imports() {
        let fs = extract(b"import os\nimport foo.bar as fb\n");
        assert_eq!(sources(&fs), ["os", "foo.bar"]);
    }

    #[test]
    fn from_import_records_the_module() {
        let fs = extract(b"from foo.bar import baz, qux\n");
        assert_eq!(sources(&fs), ["foo.bar"]);
    }

    #[test]
    fn relative_imports_preserve_the_dot_prefix() {
        let fs = extract(b"from . import x\nfrom ..pkg import y\n");
        assert_eq!(sources(&fs), [".", "..pkg"]);
    }

    #[test]
    fn star_import_records_the_module() {
        let fs = extract(b"from foo import *\n");
        assert_eq!(sources(&fs), ["foo"]);
    }

    #[test]
    fn import_edges_carry_one_based_line_numbers() {
        let fs = extract(b"x = 1\nimport os\n");
        assert_eq!(fs.imports.len(), 1);
        assert_eq!(fs.imports[0].line, 2);
    }

    #[test]
    fn pyi_stub_files_are_supported() {
        let mut parser = Parser::new();
        let src = b"def typed() -> int: ...\n";
        let result = parser.parse_bytes(Path::new("pkg/mod.pyi"), src).unwrap();
        let fs = extract_symbols(&result.tree, src, Path::new("pkg/mod.pyi"), 0);
        assert_eq!(names_of(&fs, SymbolKind::Function), ["typed"]);
    }
}
