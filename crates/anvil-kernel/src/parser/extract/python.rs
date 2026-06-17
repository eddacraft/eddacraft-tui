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
//! | `def`                 | `Function`   | module/block-level, outside functions |
//! | `class`               | `Class`      | the nominal type with members         |
//! | `def` inside a `class`| `Method`     | qualified `Owner.method` (as TS-G2)   |
//!
//! Imports come from `import` and `from … import …`. Each statement yields one
//! [`ImportEdge`] to the imported *module* (`import a.b.c` → `a.b.c`;
//! `from a.b import x, y` → `a.b`; `from . import x` → `.`; `from ..pkg import y`
//! → `..pkg`), preserving the relative-import prefix so a later resolver can map
//! it to a file. The imported *names* are not tracked as edges (mirrors the TS
//! `import { x } from "m"` → one module edge). Re-exports (the implicit
//! `__init__.py` `from .x import y` convention) are out of scope here —
//! re-export-name tracking is a separate item.
//! `importlib.import_module(...)` dynamic imports are invisible to this static
//! walk (a missed edge is a missed drift signal, never a false violation).
//!
//! ## Pass 2 — call sites (GCALL-005)
//!
//! A second walk emits symbol-level [`CallSite`]s for Python, mirroring the TS/JS
//! and Rust extractors on the GCALL-001 (ADR-086) contract. Each `call`'s caller
//! is the innermost emitted-symbol span containing it (a parallel `spans` vec, so
//! it can never mint a caller Pass 1 did not emit); the callee is resolved
//! best-effort and static. A `from m import x` binding reverse-maps a bare `x()`
//! to its export name + module specifier; a plain `import m` / `import a.b as c`
//! namespace binding resolves `m.foo()` against the module; `self.method()` /
//! `cls.method()` inside a class resolves to `Owner.method`. Cross-file callee
//! resolution is lift-time (`re_resolve_calls`, GCALL-003). Dynamic dispatch and
//! `getattr`-style calls are invisible to this static walk by design.

use std::collections::HashMap;
use std::ops::Range;

use anvil_kernel_types::{
    CallSite, CalleeRef, LocalSymbolRef, SymbolIdentity, SymbolKind, SymbolNode, TrustLevel,
    Visibility,
};

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
        // GCALL-005: the byte range of each emitted symbol's defining node, kept
        // strictly parallel to `symbols`. Pass 2 attributes a call to the
        // innermost containing span, so caller attribution uses Pass 1's *actual*
        // emitted symbols and ordinals — never a phantom caller.
        let mut spans: Vec<Range<usize>> = Vec::new();
        let mut imports = Vec::new();
        let mut next_id = id_offset;

        extract_from_node(
            tree.root_node(),
            source,
            file,
            &mut symbols,
            &mut spans,
            &mut imports,
            &mut next_id,
        );

        debug_assert_eq!(
            spans.len(),
            symbols.len(),
            "GCALL-005: every emitted symbol must record a defining-node span",
        );

        // Pass 2 (GCALL-005 / ADR-086): a separate walk emits call sites without
        // touching symbol/import emission, so the Pass 1 parity story is unchanged.
        let calls = extract_call_sites(tree.root_node(), source, &symbols, &spans);

        FileSymbols {
            file: file.to_string(),
            symbols,
            imports,
            // Python has no first-class re-export syntax; the implicit
            // `__init__.py` convention is deferred.
            reexports: Vec::<ReexportEdge>::new(),
            calls,
        }
    }
}

fn extract_from_node(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    spans: &mut Vec<Range<usize>>,
    imports: &mut Vec<ImportEdge>,
    next_id: &mut u64,
) {
    match node.kind() {
        // A module/block-level `def` outside function bodies. Functions inside
        // a class body are emitted as `Owner.method` by `emit_methods`, so this
        // arm sees only non-class-body defs.
        "function_definition" => {
            push_named(
                node,
                source,
                file,
                symbols,
                spans,
                next_id,
                SymbolKind::Function,
            );
        }
        "class_definition" => {
            let owner = field_name_text(node, source);
            push_named(
                node,
                source,
                file,
                symbols,
                spans,
                next_id,
                SymbolKind::Class,
            );
            if let Some(owner) = owner {
                emit_methods(node, source, file, symbols, spans, next_id, &owner);
            }
        }
        // `@decorator`-wrapped def/class — the real definition is in the
        // `definition` field; dispatch on it so decorated top-level items still
        // emit their symbol.
        "decorated_definition" => {
            if let Some(def) = node.child_by_field_name("definition") {
                extract_from_node(def, source, file, symbols, spans, imports, next_id);
            }
        }
        "import_statement" => extract_import(node, source, file, imports),
        "import_from_statement" => extract_import_from(node, source, file, imports),
        // Everything else (the `module` root, expression statements, `if`/`try`
        // blocks holding conditional imports) is traversed generically.
        _ => recurse_children(node, source, file, symbols, spans, imports, next_id),
    }
}

fn recurse_children(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    spans: &mut Vec<Range<usize>>,
    imports: &mut Vec<ImportEdge>,
    next_id: &mut u64,
) {
    for i in 0..u32::try_from(node.named_child_count()).unwrap_or(0) {
        if let Some(child) = node.named_child(i) {
            extract_from_node(child, source, file, symbols, spans, imports, next_id);
        }
    }
}

/// Push a symbol whose name is in the node's `name` field, recording the
/// defining node's byte range as its parallel span (GCALL-005).
fn push_named(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    spans: &mut Vec<Range<usize>>,
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
    spans.push(node.byte_range());
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
    spans: &mut Vec<Range<usize>>,
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
            // Span = the method's `function_definition` (its body holds the call
            // sites), so a call inside a method binds to `Owner.method`.
            spans.push(method.byte_range());
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

// ============================================================================
// GCALL-005 — symbol-level call-site extraction (ADR-086)
// ============================================================================

/// How a local name is bound by an `import` so a call site can name the callee's
/// **export** identity, not its local alias (ADR-086 §2).
enum Binding {
    /// A `from m import x` / `from m import x as y` binding: a bare call `x()`
    /// (or `y()`) names export `x` in module `m`. The specifier carries the
    /// relative-import prefix (`.`, `..pkg`) verbatim, as the import edge does.
    Named {
        export_name: String,
        specifier: String,
    },
    /// A module binding: `import m`, `import a.b as c`. A member call `m.foo()`
    /// resolves `foo` against the module `specifier`. Calling the binding itself
    /// (`m()`) is not a nameable symbol call.
    Module { specifier: String },
}

/// Pass 2 (ADR-086): walk the tree and emit symbol-level [`CallSite`]s. Kept
/// separate from Pass 1 so symbol/import emission is untouched. Caller
/// attribution uses Pass 1's actual emitted symbols via the parallel `spans`, so
/// the ordinal matches the lift-time identity (GCALL-003) by construction.
fn extract_call_sites(
    root: tree_sitter::Node,
    source: &[u8],
    symbols: &[SymbolNode],
    spans: &[Range<usize>],
) -> Vec<CallSite> {
    let bindings = build_import_bindings(root, source);
    let refs: Vec<&SymbolNode> = symbols.iter().collect();
    let identities = SymbolIdentity::for_file_symbols(&refs);
    let mut calls = Vec::new();
    walk_calls(root, source, &bindings, spans, &identities, &mut calls);
    calls
}

/// The caller for a call at byte `pos`: the innermost emitted-symbol span
/// containing it (smallest width wins, so a method beats its class and a nested
/// emitted symbol beats its parent), or the module-scope placeholder when none
/// contains it.
fn caller_at(pos: usize, spans: &[Range<usize>], identities: &[SymbolIdentity]) -> LocalSymbolRef {
    let mut best: Option<usize> = None;
    for (i, span) in spans.iter().enumerate() {
        if span.contains(&pos)
            && best.is_none_or(|b| span.end - span.start < spans[b].end - spans[b].start)
        {
            best = Some(i);
        }
    }
    match best.and_then(|i| identities.get(i)) {
        Some(id) => LocalSymbolRef {
            kind: id.kind,
            name: id.name.clone(),
            ordinal: id.ordinal,
            module_scope: false,
        },
        None => LocalSymbolRef {
            kind: SymbolKind::Module,
            name: String::new(),
            ordinal: 0,
            module_scope: true,
        },
    }
}

/// Build the local-name → [`Binding`] table from every `import` /
/// `from … import …` statement.
fn build_import_bindings(root: tree_sitter::Node, source: &[u8]) -> HashMap<String, Binding> {
    let mut bindings = HashMap::new();
    collect_import_bindings(root, source, &mut bindings);
    bindings
}

fn collect_import_bindings(
    node: tree_sitter::Node,
    source: &[u8],
    bindings: &mut HashMap<String, Binding>,
) {
    match node.kind() {
        "import_statement" => collect_plain_import(node, source, bindings),
        "import_from_statement" => collect_from_import(node, source, bindings),
        // Traverse generically so conditional imports in `if`/`try` blocks are
        // seen. A function-scoped `import` (inside a `def` body) is also reached
        // and registered as if file-level — a deliberate over-approximation
        // shared with the TS/Rust extractors: the worst case is a member call
        // attributed to a module that does not import it, which the lift drops
        // when the specifier matches no import edge (ADR-086 §1, never a false
        // edge). No scope tracking on the hot path.
        _ => {
            for i in 0..u32::try_from(node.named_child_count()).unwrap_or(0) {
                if let Some(child) = node.named_child(i) {
                    collect_import_bindings(child, source, bindings);
                }
            }
        }
    }
}

/// `import m` / `import a.b.c` / `import a.b as c` — bind the local name to its
/// module. A plain dotted `import a.b.c` binds the head segment `a` (the only
/// name brought into file scope) with `a` itself as the specifier: that is the
/// module `a.foo()` semantically belongs to, and forcing the cross-module path
/// keeps it from falling through to a wrong same-file `foo` lookup. Pass 1
/// records the edge to the full `a.b.c`, so `a` matches no edge and the lift
/// drops the callee (conservative miss, never a false edge — ADR-086 §1); a
/// deeper `a.b.c.foo()` chain has a nested-attribute receiver and is a bare
/// member at resolve time.
fn collect_plain_import(
    node: tree_sitter::Node,
    source: &[u8],
    bindings: &mut HashMap<String, Binding>,
) {
    for i in 0..u32::try_from(node.named_child_count()).unwrap_or(0) {
        let Some(child) = node.named_child(i) else {
            continue;
        };
        match child.kind() {
            // `import a.b.c` — local binding is the head segment, specifier too.
            "dotted_name" => {
                let module = node_text(child, source);
                let head = module.split('.').next().unwrap_or(&module).to_string();
                if !head.is_empty() {
                    bindings.insert(head.clone(), Binding::Module { specifier: head });
                }
            }
            // `import a.b as c` — `c` binds the full module path `a.b`.
            "aliased_import" => {
                let Some(name_node) = child.child_by_field_name("name") else {
                    continue;
                };
                let specifier = node_text(name_node, source);
                let local = child
                    .child_by_field_name("alias")
                    .map(|a| node_text(a, source));
                if let (Some(local), false) = (local, specifier.is_empty()) {
                    bindings.insert(local, Binding::Module { specifier });
                }
            }
            _ => {}
        }
    }
}

/// `from m import x, y as z` — each imported name binds to export `x` in module
/// `m`. `from m import *` binds no nameable symbol (skipped, like a Rust glob);
/// `from m import (a, b)` parenthesised lists are flattened by the grammar into
/// sibling name children.
fn collect_from_import(
    node: tree_sitter::Node,
    source: &[u8],
    bindings: &mut HashMap<String, Binding>,
) {
    let Some(module_node) = node.child_by_field_name("module_name") else {
        return;
    };
    let specifier = node_text(module_node, source);
    if specifier.is_empty() {
        return;
    }
    // Imported names live in the `name` field — there can be several
    // (`from m import a, b as c`), so `child_by_field_name` (first only) is not
    // enough; walk with a cursor and act on every `name`-tagged child. A
    // `wildcard_import` (`from m import *`) binds no nameable symbol.
    let mut cursor = node.walk();
    if !cursor.goto_first_child() {
        return;
    }
    loop {
        if cursor.field_name() == Some("name") {
            let child = cursor.node();
            match child.kind() {
                // `from m import x` — single-segment `dotted_name`.
                "dotted_name" => {
                    let name = node_text(child, source);
                    if !name.is_empty() {
                        bindings.insert(
                            name.clone(),
                            Binding::Named {
                                export_name: name,
                                specifier: specifier.clone(),
                            },
                        );
                    }
                }
                // `from m import x as y` — local `y` binds export `x`.
                "aliased_import" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let export_name = node_text(name_node, source);
                        let local = child
                            .child_by_field_name("alias")
                            .map_or_else(|| export_name.clone(), |a| node_text(a, source));
                        if !export_name.is_empty() {
                            bindings.insert(
                                local,
                                Binding::Named {
                                    export_name,
                                    specifier: specifier.clone(),
                                },
                            );
                        }
                    }
                }
                _ => {}
            }
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
}

/// Recursively walk emitting call sites. Each `call` is attributed to its
/// enclosing emitted symbol via [`caller_at`] over the parallel `spans` /
/// `identities` — no scope tracking, so it cannot mint a caller Pass 1 did not
/// emit.
fn walk_calls(
    node: tree_sitter::Node,
    source: &[u8],
    bindings: &HashMap<String, Binding>,
    spans: &[Range<usize>],
    identities: &[SymbolIdentity],
    calls: &mut Vec<CallSite>,
) {
    if node.kind() == "call"
        && let Some(func) = node.child_by_field_name("function")
    {
        let from = caller_at(node.start_byte(), spans, identities);
        if let Some(callee) = resolve_callee(func, source, bindings, &from) {
            calls.push(CallSite {
                from,
                callee,
                line: node_line(node),
            });
        }
    }

    for i in 0..u32::try_from(node.named_child_count()).unwrap_or(0) {
        if let Some(child) = node.named_child(i) {
            walk_calls(child, source, bindings, spans, identities, calls);
        }
    }
}

/// The class name owning a method, derived from the call's enclosing caller: a
/// `Class` caller's name, or the owner prefix of a `Method` caller's
/// `Owner.method`. Used to resolve `self.method()` / `cls.method()`.
fn enclosing_class_name(caller: &LocalSymbolRef) -> Option<String> {
    if caller.module_scope {
        return None;
    }
    match caller.kind {
        SymbolKind::Class => Some(caller.name.clone()),
        SymbolKind::Method => caller.name.split('.').next().map(ToString::to_string),
        _ => None,
    }
}

/// Resolve a Python callee expression (the `function` of a `call`) to a
/// [`CalleeRef`], or `None` when there is no statically nameable callee (a
/// subscripted/computed callee, a call-returns-callable chain). Best-effort and
/// static (ADR-086 §1); cross-file resolution is lift-time.
fn resolve_callee(
    func: tree_sitter::Node,
    source: &[u8],
    bindings: &HashMap<String, Binding>,
    caller: &LocalSymbolRef,
) -> Option<CalleeRef> {
    match func.kind() {
        // `foo()` — a `from`-import-bound name reverse-maps to its export name +
        // module; a plain `import m` binding called directly is not nameable;
        // otherwise a same-file callee.
        "identifier" => {
            let name = node_text(func, source);
            match bindings.get(&name) {
                Some(Binding::Named {
                    export_name,
                    specifier,
                }) => Some(CalleeRef {
                    name: export_name.clone(),
                    via_import: Some(specifier.clone()),
                }),
                Some(Binding::Module { .. }) => None,
                None => Some(CalleeRef {
                    name,
                    via_import: None,
                }),
            }
        }
        // `recv.method()` — `self`/`cls` resolve to `Owner.method`; a module
        // binding receiver (`m.foo()`) names the export against the module; any
        // other receiver names the bare method, leaving resolution to lift.
        "attribute" => {
            let prop_node = func.child_by_field_name("attribute")?;
            if prop_node.kind() != "identifier" {
                return None;
            }
            let prop = node_text(prop_node, source);
            let object = func.child_by_field_name("object")?;
            if object.kind() == "identifier" {
                let recv = node_text(object, source);
                if recv == "self" || recv == "cls" {
                    if let Some(class) = enclosing_class_name(caller) {
                        return Some(CalleeRef {
                            name: format!("{class}.{prop}"),
                            via_import: None,
                        });
                    }
                } else if let Some(Binding::Module { specifier }) = bindings.get(&recv) {
                    return Some(CalleeRef {
                        name: prop,
                        via_import: Some(specifier.clone()),
                    });
                }
            }
            Some(CalleeRef {
                name: prop,
                via_import: None,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use anvil_kernel_types::{
        CallSite, CalleeRef, FileSymbols, LocalSymbolRef, SymbolKind, Visibility,
    };

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
    fn nested_function_body_defs_are_not_emitted_as_top_level_symbols() {
        let fs = extract(b"def outer():\n    def inner():\n        pass\n");
        assert_eq!(names_of(&fs, SymbolKind::Function), ["outer"]);
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

    // --- Pass 2: call-site extraction (GCALL-005) ---

    /// The single call whose resolved callee name is `callee`; panics otherwise.
    fn call_to<'a>(fs: &'a FileSymbols, callee: &str) -> &'a CallSite {
        let found: Vec<&CallSite> = fs
            .calls
            .iter()
            .filter(|c| c.callee.name == callee)
            .collect();
        assert_eq!(
            found.len(),
            1,
            "expected exactly one call to {callee}, got {found:?}"
        );
        found[0]
    }

    fn fn_caller(name: &str, ordinal: u32) -> LocalSymbolRef {
        LocalSymbolRef {
            kind: SymbolKind::Function,
            name: name.to_string(),
            ordinal,
            module_scope: false,
        }
    }

    fn method_caller(name: &str) -> LocalSymbolRef {
        LocalSymbolRef {
            kind: SymbolKind::Method,
            name: name.to_string(),
            ordinal: 0,
            module_scope: false,
        }
    }

    #[test]
    fn direct_same_file_call_attributes_caller_and_callee() {
        let fs = extract(b"def helper():\n    pass\n\ndef run():\n    helper()\n");
        let call = call_to(&fs, "helper");
        assert_eq!(call.from, fn_caller("run", 0));
        assert_eq!(call.callee.via_import, None);
        assert_eq!(call.line, 5);
    }

    #[test]
    fn from_import_call_reverse_maps_to_export_name_and_module() {
        let fs = extract(b"from foo.bar import helper\ndef run():\n    helper()\n");
        let call = call_to(&fs, "helper");
        assert_eq!(
            call.callee,
            CalleeRef {
                name: "helper".to_string(),
                via_import: Some("foo.bar".to_string()),
            }
        );
        assert_eq!(call.from, fn_caller("run", 0));
    }

    #[test]
    fn from_import_alias_call_resolves_to_the_export_name() {
        let fs = extract(b"from foo import helper as h\ndef run():\n    h()\n");
        let call = call_to(&fs, "helper");
        assert_eq!(call.callee.via_import, Some("foo".to_string()));
    }

    #[test]
    fn relative_from_import_call_preserves_the_dot_prefix() {
        let fs = extract(b"from ..pkg import thing\ndef run():\n    thing()\n");
        let call = call_to(&fs, "thing");
        assert_eq!(call.callee.via_import, Some("..pkg".to_string()));
    }

    #[test]
    fn module_member_call_names_the_module_as_specifier() {
        let fs = extract(b"import os\ndef run():\n    os.getcwd()\n");
        let call = call_to(&fs, "getcwd");
        assert_eq!(call.callee.via_import, Some("os".to_string()));
    }

    #[test]
    fn aliased_module_member_call_uses_the_full_module_path() {
        let fs = extract(b"import foo.bar as fb\ndef run():\n    fb.go()\n");
        let call = call_to(&fs, "go");
        assert_eq!(call.callee.via_import, Some("foo.bar".to_string()));
    }

    #[test]
    fn plain_dotted_import_member_call_binds_the_head_conservatively() {
        // `import a.b.c` brings only `a` into scope, so `a.foo()` names module
        // `a` — not `a.b.c` (the recorded import edge). The head specifier
        // matches no edge, so the lift drops it (conservative miss, never a
        // false edge); critically it does NOT fall through to a same-file `foo`.
        let fs = extract(b"import a.b.c\ndef run():\n    a.foo()\n");
        let call = call_to(&fs, "foo");
        assert_eq!(call.callee.via_import, Some("a".to_string()));
    }

    #[test]
    fn self_method_call_resolves_to_owner_method() {
        let fs = extract(
            b"class S:\n    def a(self):\n        pass\n    def b(self):\n        self.a()\n",
        );
        let call = call_to(&fs, "S.a");
        assert_eq!(call.from, method_caller("S.b"));
        assert_eq!(call.callee.via_import, None);
    }

    #[test]
    fn cls_method_call_resolves_to_owner_method() {
        let fs = extract(
            b"class S:\n    @classmethod\n    def make(cls):\n        cls.build()\n    @classmethod\n    def build(cls):\n        pass\n",
        );
        let call = call_to(&fs, "S.build");
        assert_eq!(call.from, method_caller("S.make"));
    }

    #[test]
    fn bare_method_on_arbitrary_receiver_names_the_method_only() {
        let fs = extract(b"def run(x):\n    x.frob()\n");
        let call = call_to(&fs, "frob");
        assert_eq!(call.callee.via_import, None);
        assert_eq!(call.from, fn_caller("run", 0));
    }

    #[test]
    fn module_scope_call_attributes_to_synthetic_file_caller() {
        // A call outside any emitted symbol (a module-level statement) binds to
        // the module-scope placeholder, not a phantom function.
        let fs = extract(b"def compute():\n    return 0\n\nX = compute()\n");
        let call = call_to(&fs, "compute");
        assert!(call.from.module_scope);
        assert_eq!(call.from.kind, SymbolKind::Module);
    }

    #[test]
    fn call_in_nested_fn_attributes_to_nearest_emitted_ancestor() {
        // Python Pass 1 does not emit nested `def`s as symbols, so a call in a
        // nested function binds to the nearest emitted enclosing symbol (`outer`).
        let fs = extract(
            b"def helper():\n    pass\n\ndef outer():\n    def inner():\n        helper()\n    inner()\n",
        );
        let call = call_to(&fs, "helper");
        assert_eq!(call.from, fn_caller("outer", 0));
    }

    #[test]
    fn star_import_binding_does_not_resolve_a_specifier() {
        // `from m import *` binds no nameable symbol, so a later `foo()` is a
        // best-effort same-file callee (no via_import), never `m`.
        let fs = extract(b"from foo import *\ndef run():\n    bar()\n");
        let call = call_to(&fs, "bar");
        assert_eq!(call.callee.via_import, None);
    }

    #[test]
    fn call_extraction_is_deterministic() {
        let src =
            b"from a import c\nimport os\ndef run():\n    c()\n    os.getcwd()\n    helper()\ndef helper():\n    pass\n";
        assert_eq!(extract(src).calls, extract(src).calls);
    }
}
