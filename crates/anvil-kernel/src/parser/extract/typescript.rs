//! TypeScript / JavaScript symbol extractor.
//!
//! This is the canonical TS-family [`LanguageExtractor`] impl. It is a verbatim
//! port of the original `extract.rs` walker (LANGTS-005 K1) — the parity test
//! in this module asserts the emitted [`FileSymbols`] are identical to the
//! pre-refactor behaviour.

use std::collections::HashMap;

use anvil_kernel_types::{
    CallSite, CalleeRef, LocalSymbolRef, SymbolIdentity, SymbolKind, SymbolNode, TrustLevel,
    Visibility,
};

use super::{FileSymbols, ImportEdge, LanguageExtractor, ReexportEdge};

/// Extractor for the TypeScript / TSX / JavaScript / JSX family.
///
/// The tree-sitter TS and JS grammars share the node kinds this walker
/// inspects (`function_declaration`, `class_declaration`, `export_statement`,
/// `import_statement`, CJS `require` / `module.exports`), so a single impl
/// covers the whole family. Rust / Python anchors get their own modules.
pub struct TypeScriptExtractor;

impl LanguageExtractor for TypeScriptExtractor {
    fn extract(
        &self,
        tree: &tree_sitter::Tree,
        source: &[u8],
        file: &str,
        id_offset: u64,
    ) -> FileSymbols {
        let root = tree.root_node();
        let mut symbols = Vec::new();
        let mut imports = Vec::new();
        let mut reexports = Vec::new();
        // CIB-093 N1: set when a dynamic `require(...)`/`import(...)` call has a
        // non-string-literal argument, so its target cannot be resolved to a
        // static import edge. Threaded through the walker like the other
        // accumulators; the daemon folds it onto the GraphDelta so certify fails
        // closed on an unknowable (possibly privileged) dynamic import.
        let mut has_unresolved_dynamic_import = false;
        let mut next_id = id_offset;
        // GCALL-002: the byte range of each emitted symbol's defining node, kept
        // strictly parallel to `symbols`. Pass 2 attributes a call to the
        // innermost containing span, so caller attribution uses pass 1's *actual*
        // emitted symbol set (no re-recognition) — correct across nesting,
        // export wrappers, and arrow consts by construction.
        let mut spans: Vec<std::ops::Range<usize>> = Vec::new();

        extract_from_node(
            root,
            source,
            file,
            &mut symbols,
            &mut spans,
            &mut imports,
            &mut reexports,
            &mut has_unresolved_dynamic_import,
            &mut next_id,
        );
        debug_assert_eq!(
            spans.len(),
            symbols.len(),
            "GCALL-002: every emitted symbol must record a defining-node span",
        );

        // Pass 2 (GCALL-002 / ADR-086): symbol-level call sites. A separate walk
        // so pass 1's symbol/import/reexport emission stays byte-identical (the
        // parity test guards it).
        let calls = extract_call_sites(root, source, &symbols, &spans);

        FileSymbols {
            file: file.to_string(),
            symbols,
            imports,
            reexports,
            calls,
            calls_partial: false,
            has_unresolved_dynamic_import,
        }
    }
}

// The accumulators (symbols + their parallel spans + imports + reexports + the
// id counter) are all genuinely threaded through this recursive walker; bundling
// them into a struct would not reduce the coupling, only hide it.
#[allow(clippy::too_many_arguments)]
fn extract_from_node(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    spans: &mut Vec<std::ops::Range<usize>>,
    imports: &mut Vec<ImportEdge>,
    reexports: &mut Vec<ReexportEdge>,
    has_unresolved_dynamic_import: &mut bool,
    next_id: &mut u64,
) {
    match node.kind() {
        "function_declaration" => extract_function(node, source, file, symbols, spans, next_id),
        "class_declaration" => extract_class(node, source, file, symbols, spans, next_id),
        // TS-G1: type-shape declarations become first-class symbols.
        "interface_declaration" => {
            extract_named_decl(
                node,
                source,
                file,
                symbols,
                spans,
                next_id,
                SymbolKind::Interface,
            );
        }
        "type_alias_declaration" => {
            extract_named_decl(
                node,
                source,
                file,
                symbols,
                spans,
                next_id,
                SymbolKind::TypeAlias,
            );
        }
        "enum_declaration" => {
            extract_named_decl(
                node,
                source,
                file,
                symbols,
                spans,
                next_id,
                SymbolKind::Enum,
            );
        }
        "export_statement" => {
            extract_export(
                node,
                source,
                file,
                symbols,
                spans,
                imports,
                reexports,
                has_unresolved_dynamic_import,
                next_id,
            );
        }
        "import_statement" => extract_import(node, source, file, imports),
        "call_expression" => {
            extract_dynamic_import(node, source, file, imports, has_unresolved_dynamic_import);
        }
        "assignment_expression" => extract_cjs_export(node, source, file, symbols, spans, next_id),
        "lexical_declaration" => extract_lexical(node, source, file, symbols, spans, next_id),
        _ => {}
    }

    // Recurse into children (except for nodes we've already handled).
    // Note: lexical_declaration is NOT excluded — we must recurse into it so
    // that nested call_expression nodes (e.g. `const fs = require('node:fs')`)
    // are visited by extract_require.
    if !matches!(
        node.kind(),
        "function_declaration"
            | "class_declaration"
            | "export_statement"
            // TS-G1 declarations are fully handled by extract_named_decl; their
            // bodies hold only type-level members (method/property signatures,
            // enum members) we don't extract, and skipping recursion prevents a
            // nested initialiser expression (e.g. `require(...)` inside an enum
            // member) from emitting a spurious edge.
            | "interface_declaration"
            | "type_alias_declaration"
            | "enum_declaration"
    ) {
        for i in 0..u32::try_from(node.named_child_count()).unwrap_or(0) {
            if let Some(child) = node.named_child(i) {
                extract_from_node(
                    child,
                    source,
                    file,
                    symbols,
                    spans,
                    imports,
                    reexports,
                    has_unresolved_dynamic_import,
                    next_id,
                );
            }
        }
    }
}

fn extract_function(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    spans: &mut Vec<std::ops::Range<usize>>,
    next_id: &mut u64,
) {
    if let Some(name_node) = node.child_by_field_name("name") {
        let name = node_text(name_node, source);
        symbols.push(SymbolNode {
            id: *next_id,
            kind: SymbolKind::Function,
            name,
            visibility: Visibility::Internal,
            file: file.to_string(),
            trust_level: TrustLevel::default(),
        });
        spans.push(node.byte_range());
        *next_id += 1;
    }
}

fn extract_class(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    spans: &mut Vec<std::ops::Range<usize>>,
    next_id: &mut u64,
) {
    let Some(name_node) = node.child_by_field_name("name") else {
        return;
    };
    let class_name = node_text(name_node, source);
    symbols.push(SymbolNode {
        id: *next_id,
        kind: SymbolKind::Class,
        name: class_name.clone(),
        visibility: Visibility::Internal,
        file: file.to_string(),
        trust_level: TrustLevel::default(),
    });
    spans.push(node.byte_range());
    *next_id += 1;

    // TS-G2: surface each class method as its own symbol, qualifying the name
    // with the owning class (`Owner.method`) so the parent is recoverable
    // without a structural parent edge (deferred — see `SymbolKind::Method`).
    // `class_declaration` is excluded from the generic child recursion in
    // `extract_from_node`, so methods are emitted only here (no double-count).
    if let Some(body) = node.child_by_field_name("body") {
        for i in 0..u32::try_from(body.named_child_count()).unwrap_or(0) {
            let Some(member) = body.named_child(i) else {
                continue;
            };
            if member.kind() != "method_definition" {
                continue;
            }
            if let Some(method_name) = member.child_by_field_name("name") {
                symbols.push(SymbolNode {
                    id: *next_id,
                    kind: SymbolKind::Method,
                    name: format!("{class_name}.{}", node_text(method_name, source)),
                    visibility: Visibility::Internal,
                    file: file.to_string(),
                    trust_level: TrustLevel::default(),
                });
                spans.push(member.byte_range());
                *next_id += 1;
            }
        }
    }
}

/// Emit a single named declaration — interface / type alias / enum (TS-G1) —
/// as a symbol of `kind`. Visibility defaults to `Internal`; an enclosing
/// `export_statement` upgrades it to `Public` via [`extract_export`].
fn extract_named_decl(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    spans: &mut Vec<std::ops::Range<usize>>,
    next_id: &mut u64,
    kind: SymbolKind,
) {
    if let Some(name_node) = node.child_by_field_name("name") {
        symbols.push(SymbolNode {
            id: *next_id,
            kind,
            name: node_text(name_node, source),
            visibility: Visibility::Internal,
            file: file.to_string(),
            trust_level: TrustLevel::default(),
        });
        spans.push(node.byte_range());
        *next_id += 1;
    }
}

#[allow(clippy::too_many_arguments)] // same threaded accumulators as extract_from_node
fn extract_export(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    spans: &mut Vec<std::ops::Range<usize>>,
    imports: &mut Vec<ImportEdge>,
    reexports: &mut Vec<ReexportEdge>,
    has_unresolved_dynamic_import: &mut bool,
    next_id: &mut u64,
) {
    if let Some(decl) = node.child_by_field_name("declaration") {
        let before = symbols.len();
        extract_from_node(
            decl,
            source,
            file,
            symbols,
            spans,
            imports,
            reexports,
            has_unresolved_dynamic_import,
            next_id,
        );
        for sym in &mut symbols[before..] {
            sym.visibility = Visibility::Public;
        }
        return;
    }

    // `export default <expression>` uses the "value" field. Try to extract a
    // named symbol from the expression; if it is anonymous (function expression
    // or class expression with no name), emit a synthetic "default" symbol so
    // the module's public API surface is not silently omitted.
    if let Some(value) = node.child_by_field_name("value") {
        let name = value
            .child_by_field_name("name")
            .map(|n| node_text(n, source));
        let sym_name = name.unwrap_or_else(|| String::from("default"));
        let kind = match value.kind() {
            "function_expression" | "arrow_function" => SymbolKind::Function,
            "class" => SymbolKind::Class,
            _ => SymbolKind::Export,
        };
        symbols.push(SymbolNode {
            id: *next_id,
            kind,
            name: sym_name,
            visibility: Visibility::Public,
            file: file.to_string(),
            trust_level: TrustLevel::default(),
        });
        // Span = the exported expression (a `function`/`class` body holds the
        // default export's call sites).
        spans.push(value.byte_range());
        *next_id += 1;
        return;
    }

    let mut handled_clause = false;
    for i in 0..u32::try_from(node.named_child_count()).unwrap_or(0) {
        let Some(child) = node.named_child(i) else {
            continue;
        };
        match child.kind() {
            "export_clause" => {
                handled_clause = true;
                extract_export_clause(child, source, file, symbols, spans, next_id);
            }
            // `export * as ns from "m"` — a namespace re-export binds the whole
            // module surface to `ns`. Emit a public `Export` symbol named `ns`
            // (not the bare `*` fallback) and mark the clause handled.
            "namespace_export" => {
                handled_clause = true;
                let name =
                    namespace_export_name(child, source).unwrap_or_else(|| String::from("*"));
                symbols.push(SymbolNode {
                    id: *next_id,
                    kind: SymbolKind::Export,
                    name,
                    visibility: Visibility::Public,
                    file: file.to_string(),
                    trust_level: TrustLevel::default(),
                });
                spans.push(child.byte_range());
                *next_id += 1;
            }
            _ => {}
        }
    }

    // Re-export from another module: `export { x } from './mod'`,
    // `export * from './mod'`. Emit the file-level dependency edge (the module
    // really is a dependency) AND a first-class Reexport edge per re-exported
    // name so impact analysis (GV2-011) sees the widened public surface — a
    // re-export, unlike a plain import, re-publishes the named symbol.
    if let Some(source_node) = node.child_by_field_name("source") {
        let raw = node_text(source_node, source);
        let module_path = raw.trim_matches(|c| c == '\'' || c == '"');
        let line = node_line(node);
        imports.push(ImportEdge {
            from_file: file.to_string(),
            to_source: module_path.to_string(),
            line,
        });
        for name in reexport_names(node, source) {
            reexports.push(ReexportEdge {
                from_file: file.to_string(),
                exported_name: name,
                to_source: module_path.to_string(),
                line,
            });
        }
    }

    if !handled_clause {
        symbols.push(SymbolNode {
            id: *next_id,
            kind: SymbolKind::Export,
            name: String::from("*"),
            visibility: Visibility::Public,
            file: file.to_string(),
            trust_level: TrustLevel::default(),
        });
        spans.push(node.byte_range());
        *next_id += 1;
    }
}

fn extract_export_clause(
    clause: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    spans: &mut Vec<std::ops::Range<usize>>,
    next_id: &mut u64,
) {
    for j in 0..u32::try_from(clause.named_child_count()).unwrap_or(0) {
        if let Some(spec) = clause.named_child(j) {
            if spec.kind() != "export_specifier" {
                continue;
            }
            // Use the local name (not the alias) when looking up existing symbols.
            // `export { foo as bar }` should mark the symbol named `foo` as Public.
            let local_name = if let Some(name_node) = spec.child_by_field_name("name") {
                node_text(name_node, source)
            } else {
                continue;
            };
            let matched_kind = symbols
                .iter()
                .find(|s| s.name == local_name)
                .map(|s| s.kind);
            if let Some(kind) = matched_kind {
                // Mark the named symbol Public. If it is a class, its methods
                // (emitted as `Owner.method`, TS-G2) are part of the same
                // exported surface, so flip them too — otherwise method
                // visibility would wrongly depend on whether the class was
                // exported inline (`export class`) or via a clause
                // (`export { Foo }`).
                let method_prefix = format!("{local_name}.");
                for sym in symbols.iter_mut() {
                    let is_owned_method = kind == SymbolKind::Class
                        && sym.kind == SymbolKind::Method
                        && sym.name.starts_with(&method_prefix);
                    if sym.name == local_name || is_owned_method {
                        sym.visibility = Visibility::Public;
                    }
                }
            } else {
                // No local symbol found — create a public export node so the
                // public API surface is correctly tracked. This handles cases
                // like `export { foo }` where foo is declared after the export,
                // or re-exports from other modules.
                symbols.push(SymbolNode {
                    id: *next_id,
                    kind: SymbolKind::Export,
                    name: local_name,
                    visibility: Visibility::Public,
                    file: file.to_string(),
                    trust_level: TrustLevel::default(),
                });
                spans.push(spec.byte_range());
                *next_id += 1;
            }
        }
    }
}

fn extract_import(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    imports: &mut Vec<ImportEdge>,
) {
    if let Some(source_node) = node.child_by_field_name("source") {
        let raw = node_text(source_node, source);
        let module_path = raw.trim_matches(|c| c == '\'' || c == '"');
        imports.push(ImportEdge {
            from_file: file.to_string(),
            to_source: module_path.to_string(),
            line: node_line(node),
        });
    }
}

/// Handle a CJS `require(...)` call or an ESM dynamic `import(...)` expression
/// (CIB-093 N1).
///
/// Both are `call_expression` nodes; the callee is the `identifier` `require`
/// for CJS, and the bare `import` keyword node (kind `"import"`) for a dynamic
/// import. A non-dynamic-import call (`compute(x)`, `foo.bar()`) is ignored.
///
/// Two sub-cases, mirroring the static-vs-unknown split the trust pass needs:
///
/// - **Literal specifier** (`require('fs')`, `import('fs')`): a determinable
///   static import. Emit an [`ImportEdge`] so it flows through
///   `is_privileged_import` exactly like a top-level `import … from 'fs'`.
/// - **Computed specifier** (`require(x)`, `import(`./${x}`)`,
///   `require(a + b)`): the target is unknowable statically, so it MUST NOT be
///   emitted as an edge (that would invent a garbage `to_source`) and MUST set
///   `has_unresolved_dynamic_import` so the certify path fails closed — a
///   computed dynamic import can reach a privileged built-in with no static edge
///   for the trust pass to see.
fn extract_dynamic_import(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    imports: &mut Vec<ImportEdge>,
    has_unresolved_dynamic_import: &mut bool,
) {
    let Some(func) = node.child_by_field_name("function") else {
        return;
    };
    // The callee is either the `require` identifier or the `import` keyword node.
    let is_dynamic_import = func.kind() == "import" || node_text(func, source) == "require";
    if !is_dynamic_import {
        return;
    }
    let Some(args) = node.child_by_field_name("arguments") else {
        return;
    };
    let Some(arg) = args.named_child(0) else {
        // `require()` / `import()` with no argument — nothing determinable, but
        // nothing reachable either; leave it alone.
        return;
    };

    // A string-literal argument is the one statically determinable shape. In the
    // tree-sitter TS/JS grammar a literal specifier is a `string` node; anything
    // else (identifier, template_string, member_expression, binary_expression…)
    // is computed and unknowable.
    if arg.kind() == "string" {
        let raw = node_text(arg, source);
        let module_path = raw.trim_matches(|c| c == '\'' || c == '"');
        if !module_path.is_empty() {
            imports.push(ImportEdge {
                from_file: file.to_string(),
                to_source: module_path.to_string(),
                line: node_line(node),
            });
        }
    } else {
        // Computed/unresolvable dynamic import — fail closed downstream.
        *has_unresolved_dynamic_import = true;
    }
}

fn extract_cjs_export(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    spans: &mut Vec<std::ops::Range<usize>>,
    next_id: &mut u64,
) {
    let Some(left) = node.child_by_field_name("left") else {
        return;
    };
    let left_text = node_text(left, source);
    let span = node.byte_range();

    if let Some(prop) = left_text.strip_prefix("module.exports.") {
        // `module.exports.foo = ...` — mark only the specific property as Public
        mark_or_add_public_symbol(prop, file, symbols, spans, &span, next_id);
    } else if left_text == "module.exports" {
        // `module.exports = { foo, bar }` — only mark referenced symbols as Public.
        // If the RHS is not an object literal or is too complex, fall back to
        // marking all symbols as Public (conservative).
        let rhs = node.child_by_field_name("right");
        if let Some(rhs_node) = rhs.filter(|n| n.kind() == "object") {
            let mut property_names = Vec::new();
            for i in 0..u32::try_from(rhs_node.named_child_count()).unwrap_or(0) {
                if let Some(child) = rhs_node.named_child(i) {
                    match child.kind() {
                        "shorthand_property_identifier" => {
                            property_names.push(node_text(child, source));
                        }
                        "pair" => {
                            // Use the property key (exported name), not the value
                            if let Some(key_node) = child.child_by_field_name("key") {
                                property_names.push(node_text(key_node, source));
                            }
                        }
                        _ => {}
                    }
                }
            }
            for name in &property_names {
                mark_or_add_public_symbol(name, file, symbols, spans, &span, next_id);
            }
        } else {
            // RHS is a single identifier or complex expression — mark only that
            // identifier, or fall back to all symbols if unresolvable.
            if let Some(rhs_node) = rhs {
                if rhs_node.kind() == "identifier" {
                    let name = node_text(rhs_node, source);
                    mark_or_add_public_symbol(&name, file, symbols, spans, &span, next_id);
                } else {
                    // Complex RHS (call expression, ternary, etc.) — conservative fallback
                    for sym in symbols.iter_mut().filter(|s| s.file == file) {
                        sym.visibility = Visibility::Public;
                    }
                }
            }
        }
    } else if let Some(prop) = left_text.strip_prefix("exports.") {
        // `exports.foo = ...` — mark only the specific property as Public
        mark_or_add_public_symbol(prop, file, symbols, spans, &span, next_id);
    }
}

fn mark_or_add_public_symbol(
    name: &str,
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    spans: &mut Vec<std::ops::Range<usize>>,
    span: &std::ops::Range<usize>,
    next_id: &mut u64,
) {
    if let Some(sym) = symbols.iter_mut().find(|s| s.name == name) {
        sym.visibility = Visibility::Public;
    } else {
        symbols.push(SymbolNode {
            id: *next_id,
            kind: SymbolKind::Export,
            name: name.to_string(),
            visibility: Visibility::Public,
            file: file.to_string(),
            trust_level: TrustLevel::default(),
        });
        spans.push(span.clone());
        *next_id += 1;
    }
}

fn extract_lexical(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    spans: &mut Vec<std::ops::Range<usize>>,
    next_id: &mut u64,
) {
    for i in 0..u32::try_from(node.named_child_count()).unwrap_or(0) {
        if let Some(child) = node.named_child(i)
            && child.kind() == "variable_declarator"
            && let Some(name_node) = child.child_by_field_name("name")
            && let Some(value) = child.child_by_field_name("value")
            && value.kind() == "arrow_function"
        {
            let name = node_text(name_node, source);
            symbols.push(SymbolNode {
                id: *next_id,
                kind: SymbolKind::Function,
                name,
                visibility: Visibility::Internal,
                file: file.to_string(),
                trust_level: TrustLevel::default(),
            });
            // Span = the declarator (`x = () => {…}`), which contains the arrow
            // body's call sites.
            spans.push(child.byte_range());
            *next_id += 1;
        }
    }
}

/// The re-exported names for an `export … from "mod"` statement.
///
/// - `export { a, b as c } from "m"` → `["a", "b"]` (the source-side `name`
///   field, matching how [`extract_export_clause`] resolves the re-exported
///   symbol; alias tracking is deferred).
/// - `export * as ns from "m"` → `["ns"]` (namespace re-export).
/// - `export * from "m"` (bare wildcard) → `["*"]`.
fn reexport_names(node: tree_sitter::Node, source: &[u8]) -> Vec<String> {
    for i in 0..u32::try_from(node.named_child_count()).unwrap_or(0) {
        let Some(child) = node.named_child(i) else {
            continue;
        };
        match child.kind() {
            "export_clause" => {
                let mut names = Vec::new();
                for j in 0..u32::try_from(child.named_child_count()).unwrap_or(0) {
                    if let Some(spec) = child.named_child(j)
                        && spec.kind() == "export_specifier"
                        && let Some(name_node) = spec.child_by_field_name("name")
                    {
                        names.push(node_text(name_node, source));
                    }
                }
                return names;
            }
            "namespace_export" => {
                return vec![
                    namespace_export_name(child, source).unwrap_or_else(|| String::from("*")),
                ];
            }
            _ => {}
        }
    }
    // No export clause — `export * from "m"`: a wildcard re-export.
    vec![String::from("*")]
}

/// The bound identifier of a `namespace_export` node (`* as ns` → `ns`).
fn namespace_export_name(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    // The grammar exposes the alias as the namespace_export's named child
    // (an `identifier`); fall back across named children defensively.
    for i in 0..u32::try_from(node.named_child_count()).unwrap_or(0) {
        if let Some(child) = node.named_child(i)
            && child.kind() == "identifier"
        {
            return Some(node_text(child, source));
        }
    }
    node.named_child(0).map(|n| node_text(n, source))
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
// GCALL-002 — symbol-level call-site extraction (ADR-086)
// ============================================================================

/// How a local name is bound by an `import` so a call site can name the callee's
/// **export** identity, not its local alias (ADR-086 §2).
enum Binding {
    /// A named or default import: `import { a } / { b as c } / d from "m"`.
    /// `export_name` is the name in the target module (`"default"` for a default
    /// import — resolved to `Unresolved` at lift time per ADR-086).
    Named {
        export_name: String,
        specifier: String,
    },
    /// A namespace import: `import * as ns from "m"`. Member calls `ns.foo()`
    /// resolve `foo` against `specifier`.
    Namespace { specifier: String },
}

/// Pass 2 (ADR-086): walk the tree and emit symbol-level [`CallSite`]s. Kept
/// separate from pass 1 so symbol/import emission is untouched.
///
/// Caller attribution uses pass 1's **actual** emitted symbols: each call is
/// attributed to the innermost emitted-symbol span ([`spans`] parallel to
/// [`symbols`]) that contains it, and the caller's identity comes straight from
/// [`SymbolIdentity::for_file_symbols`]. So a nested function/class that pass 1
/// did not emit has no span and its calls fall to the nearest enclosing emitted
/// symbol — never a phantom caller — and the ordinal matches the lift-time
/// identity (GCALL-003) by construction.
fn extract_call_sites(
    root: tree_sitter::Node,
    source: &[u8],
    symbols: &[SymbolNode],
    spans: &[std::ops::Range<usize>],
) -> Vec<CallSite> {
    let bindings = build_import_bindings(root, source);
    let refs: Vec<&SymbolNode> = symbols.iter().collect();
    let identities = SymbolIdentity::for_file_symbols(&refs);
    let mut calls = Vec::new();
    walk_calls(root, source, &bindings, spans, &identities, &mut calls);
    calls
}

/// The caller for a call/new at byte `pos`: the innermost emitted-symbol span
/// containing it (smallest width wins, so a method beats its class, a nested
/// emitted function beats its parent), or the module-scope placeholder when none
/// contains it.
fn caller_at(
    pos: usize,
    spans: &[std::ops::Range<usize>],
    identities: &[SymbolIdentity],
) -> LocalSymbolRef {
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

/// Build the local-name → [`Binding`] table from every `import_statement`.
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
    if node.kind() == "import_statement" {
        let specifier = node.child_by_field_name("source").map(|s| {
            node_text(s, source)
                .trim_matches(|c| c == '\'' || c == '"')
                .to_string()
        });
        // `import_clause` is a child node kind, not a field.
        let clause = (0..u32::try_from(node.named_child_count()).unwrap_or(0))
            .filter_map(|i| node.named_child(i))
            .find(|c| c.kind() == "import_clause");
        if let (Some(specifier), Some(clause)) = (specifier, clause) {
            collect_clause_bindings(clause, source, &specifier, bindings);
        }
        return;
    }
    for i in 0..u32::try_from(node.named_child_count()).unwrap_or(0) {
        if let Some(child) = node.named_child(i) {
            collect_import_bindings(child, source, bindings);
        }
    }
}

fn collect_clause_bindings(
    clause: tree_sitter::Node,
    source: &[u8],
    specifier: &str,
    bindings: &mut HashMap<String, Binding>,
) {
    for i in 0..u32::try_from(clause.named_child_count()).unwrap_or(0) {
        let Some(child) = clause.named_child(i) else {
            continue;
        };
        match child.kind() {
            // `import d from "m"` — the default binding is an identifier child.
            "identifier" => {
                bindings.insert(
                    node_text(child, source),
                    Binding::Named {
                        export_name: "default".to_string(),
                        specifier: specifier.to_string(),
                    },
                );
            }
            // `import * as ns from "m"`.
            "namespace_import" => {
                if let Some(alias) = child.named_child(0) {
                    bindings.insert(
                        node_text(alias, source),
                        Binding::Namespace {
                            specifier: specifier.to_string(),
                        },
                    );
                }
            }
            // `import { a, b as c } from "m"`.
            "named_imports" => {
                for j in 0..u32::try_from(child.named_child_count()).unwrap_or(0) {
                    let Some(spec) = child.named_child(j) else {
                        continue;
                    };
                    if spec.kind() != "import_specifier" {
                        continue;
                    }
                    let Some(name_node) = spec.child_by_field_name("name") else {
                        continue;
                    };
                    let export_name = node_text(name_node, source);
                    // `alias` field present ⇒ the local binding is the alias.
                    let local = spec
                        .child_by_field_name("alias")
                        .map_or_else(|| export_name.clone(), |a| node_text(a, source));
                    bindings.insert(
                        local,
                        Binding::Named {
                            export_name,
                            specifier: specifier.to_string(),
                        },
                    );
                }
            }
            _ => {}
        }
    }
}

/// Recursively walk emitting call sites. Each call/new is attributed to its
/// enclosing emitted symbol via [`caller_at`] over the parallel `spans` /
/// `identities` — no scope tracking, so it cannot mint a caller pass 1 did not
/// emit.
fn walk_calls(
    node: tree_sitter::Node,
    source: &[u8],
    bindings: &HashMap<String, Binding>,
    spans: &[std::ops::Range<usize>],
    identities: &[SymbolIdentity],
    calls: &mut Vec<CallSite>,
) {
    let callee_node = match node.kind() {
        "call_expression" => node.child_by_field_name("function"),
        "new_expression" => node.child_by_field_name("constructor"),
        _ => None,
    };
    if let Some(func) = callee_node {
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
/// `Owner.method`. Used to resolve `this.method()` to `Owner.method`.
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

/// Resolve a callee expression (the `function` of a call or `constructor` of a
/// `new`) to a [`CalleeRef`], or `None` when there is no statically nameable
/// callee (a computed member, an IIFE, a `require`/dynamic-`import`). Resolution
/// is best-effort and static (ADR-086 §1); cross-file resolution is lift-time.
fn resolve_callee(
    func: tree_sitter::Node,
    source: &[u8],
    bindings: &HashMap<String, Binding>,
    caller: &LocalSymbolRef,
) -> Option<CalleeRef> {
    match func.kind() {
        "identifier" => {
            let name = node_text(func, source);
            // `require(...)` is a CJS import (pass 1), never a symbol call.
            if name == "require" {
                return None;
            }
            match bindings.get(&name) {
                Some(Binding::Named {
                    export_name,
                    specifier,
                }) => Some(CalleeRef {
                    name: export_name.clone(),
                    via_import: Some(specifier.clone()),
                }),
                // Calling a namespace binding directly (`ns()`) is not nameable.
                Some(Binding::Namespace { .. }) => None,
                None => Some(CalleeRef {
                    name,
                    via_import: None,
                }),
            }
        }
        "member_expression" => {
            let property = func.child_by_field_name("property")?;
            // A computed member (`obj[x]()`) has no static property name.
            if property.kind() != "property_identifier" && property.kind() != "identifier" {
                return None;
            }
            let prop = node_text(property, source);
            let object = func.child_by_field_name("object")?;
            match object.kind() {
                // `ns.foo()` where `ns` is a namespace import.
                "identifier" => {
                    if let Some(Binding::Namespace { specifier }) =
                        bindings.get(&node_text(object, source))
                    {
                        return Some(CalleeRef {
                            name: prop,
                            via_import: Some(specifier.clone()),
                        });
                    }
                    Some(CalleeRef {
                        name: prop,
                        via_import: None,
                    })
                }
                // `this.method()` inside a class resolves to `Owner.method`.
                "this" => {
                    let class = enclosing_class_name(caller)?;
                    Some(CalleeRef {
                        name: format!("{class}.{prop}"),
                        via_import: None,
                    })
                }
                // Any other receiver: name the member, leave resolution to lift.
                _ => Some(CalleeRef {
                    name: prop,
                    via_import: None,
                }),
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use anvil_kernel_types::{SymbolKind, SymbolNode, Visibility};

    use super::{LanguageExtractor, TypeScriptExtractor};
    use crate::parser::Parser;
    use crate::parser::extract::extract_symbols;

    #[test]
    fn extracts_functions_from_typescript() {
        let source = b"
function greet(name: string): string {
    return name;
}

const add = (a: number, b: number) => a + b;
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);
        let fns: Vec<&str> = symbols
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Function)
            .map(|s| s.name.as_str())
            .collect();

        assert!(fns.contains(&"greet"));
        assert!(fns.contains(&"add"));
    }

    #[test]
    fn extracts_classes() {
        let source = b"
class Greeter {
    greet() { return 'hello'; }
}
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);
        let classes: Vec<&str> = symbols
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Class)
            .map(|s| s.name.as_str())
            .collect();

        assert!(classes.contains(&"Greeter"));
    }

    // --- TS-G1: interface / type-alias / enum declarations ---

    #[test]
    fn extracts_interface_type_alias_and_enum_symbols() {
        let source = b"
interface Shape { area(): number; }
type Id = string | number;
enum Color { Red, Green, Blue }
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();
        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);

        let by_kind = |k: SymbolKind| -> Vec<&str> {
            symbols
                .symbols
                .iter()
                .filter(|s| s.kind == k)
                .map(|s| s.name.as_str())
                .collect()
        };
        assert_eq!(by_kind(SymbolKind::Interface), ["Shape"], "TS-G1 interface");
        assert_eq!(by_kind(SymbolKind::TypeAlias), ["Id"], "TS-G1 type alias");
        assert_eq!(by_kind(SymbolKind::Enum), ["Color"], "TS-G1 enum");
    }

    #[test]
    fn exported_type_shapes_are_public() {
        let source = b"
export interface Public {}
interface Internal {}
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();
        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);

        let vis = |name: &str| {
            symbols
                .symbols
                .iter()
                .find(|s| s.name == name)
                .map(|s| s.visibility)
        };
        assert_eq!(vis("Public"), Some(Visibility::Public));
        assert_eq!(vis("Internal"), Some(Visibility::Internal));
    }

    // --- TS-G2: class methods as separate symbols, parent encoded in the name ---

    #[test]
    fn extracts_class_methods_with_qualified_names() {
        let source = b"
class Service {
    constructor() {}
    start(): void {}
    private stop(): void {}
}
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();
        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);

        let methods: Vec<&str> = symbols
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Method)
            .map(|s| s.name.as_str())
            .collect();

        // Each method is surfaced as `Owner.method` so the owning class is
        // recoverable from the name (TS-G2 parent link).
        assert!(methods.contains(&"Service.start"), "got {methods:?}");
        assert!(methods.contains(&"Service.stop"), "got {methods:?}");
        assert!(methods.contains(&"Service.constructor"), "got {methods:?}");
        // The class itself is still emitted as a Class symbol.
        assert!(
            symbols
                .symbols
                .iter()
                .any(|s| s.kind == SymbolKind::Class && s.name == "Service")
        );
    }

    #[test]
    fn methods_of_exported_class_are_public() {
        let source = b"
export class Api {
    handle(): void {}
}
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();
        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);

        let method = symbols
            .symbols
            .iter()
            .find(|s| s.kind == SymbolKind::Method && s.name == "Api.handle")
            .expect("Api.handle method symbol");
        assert_eq!(method.visibility, Visibility::Public);
    }

    #[test]
    fn methods_are_public_when_class_exported_via_clause() {
        // `export { Service }` must flip the class AND its methods Public —
        // method visibility should not depend on inline-vs-clause export syntax.
        let source = b"
class Service {
    run(): void {}
}
export { Service };
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();
        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);

        let vis = |name: &str| {
            symbols
                .symbols
                .iter()
                .find(|s| s.name == name)
                .map(|s| s.visibility)
        };
        assert_eq!(vis("Service"), Some(Visibility::Public));
        assert_eq!(
            vis("Service.run"),
            Some(Visibility::Public),
            "clause-exported class's methods must be Public too"
        );
    }

    #[test]
    fn marks_exports_as_public() {
        let source = b"
export function greet() {}
function internal() {}
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);
        let greet = symbols.symbols.iter().find(|s| s.name == "greet").unwrap();
        let internal = symbols
            .symbols
            .iter()
            .find(|s| s.name == "internal")
            .unwrap();

        assert_eq!(greet.visibility, Visibility::Public);
        assert_eq!(internal.visibility, Visibility::Internal);
    }

    #[test]
    fn extracts_imports() {
        let source = b"
import { something } from './module';
import * as fs from 'node:fs';
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);
        let import_sources: Vec<&str> = symbols
            .imports
            .iter()
            .map(|i| i.to_source.as_str())
            .collect();

        assert!(import_sources.contains(&"./module"));
        assert!(import_sources.contains(&"node:fs"));

        let module_import = symbols
            .imports
            .iter()
            .find(|i| i.to_source == "./module")
            .unwrap();
        assert_eq!(module_import.line, 2, "import on line 2 (1-based)");

        let fs_import = symbols
            .imports
            .iter()
            .find(|i| i.to_source == "node:fs")
            .unwrap();
        assert_eq!(fs_import.line, 3, "import on line 3 (1-based)");
    }

    #[test]
    fn export_does_not_flip_unrelated_symbol() {
        // Regression: an export with no declaration (e.g. `export {}`)
        // should not flip a prior symbol to Public.
        let source = b"
function internal() {}
export {};
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);
        let internal = symbols
            .symbols
            .iter()
            .find(|s| s.name == "internal")
            .unwrap();
        assert_eq!(internal.visibility, Visibility::Internal);
    }

    #[test]
    fn assigns_unique_ids() {
        let source = b"
function a() {}
function b() {}
class C {}
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 100);
        let ids: Vec<u64> = symbols.symbols.iter().map(|s| s.id).collect();

        // All unique
        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(ids.len(), unique.len());

        // Starting from offset
        assert!(ids.iter().all(|&id| id >= 100));
    }

    #[test]
    fn export_alias_uses_local_name() {
        // `export { foo as bar }` should mark the symbol named `foo` as Public,
        // not look for a symbol named `bar`.
        let source = b"
function foo() {}
function bar() {}
export { foo as bar };
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);
        let foo = symbols.symbols.iter().find(|s| s.name == "foo").unwrap();
        let bar = symbols.symbols.iter().find(|s| s.name == "bar").unwrap();

        assert_eq!(
            foo.visibility,
            Visibility::Public,
            "foo should be Public (it is the exported symbol)"
        );
        assert_eq!(
            bar.visibility,
            Visibility::Internal,
            "bar should stay Internal (it is not exported)"
        );
    }

    #[test]
    fn anonymous_default_export_emits_default_symbol() {
        let source = b"export default function() { return 42; }";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);
        let default_sym = symbols.symbols.iter().find(|s| s.name == "default");

        assert!(
            default_sym.is_some(),
            "anonymous default export should produce a 'default' symbol"
        );
        let sym = default_sym.unwrap();
        assert_eq!(sym.visibility, Visibility::Public);
        assert_eq!(sym.kind, SymbolKind::Function);
    }

    #[test]
    fn anonymous_default_class_export_emits_default_symbol() {
        let source = b"export default class {}";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);
        let default_sym = symbols.symbols.iter().find(|s| s.name == "default");

        assert!(
            default_sym.is_some(),
            "anonymous default class export should produce a 'default' symbol"
        );
        let sym = default_sym.unwrap();
        assert_eq!(sym.visibility, Visibility::Public);
    }

    #[test]
    fn named_default_export_does_not_duplicate() {
        let source = b"export default function greet() { return 'hi'; }";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);
        let public_syms: Vec<&SymbolNode> = symbols
            .symbols
            .iter()
            .filter(|s| s.visibility == Visibility::Public)
            .collect();

        assert_eq!(
            public_syms.len(),
            1,
            "named default export should produce exactly one public symbol"
        );
        assert_eq!(public_syms[0].name, "greet");
    }

    #[test]
    fn module_exports_property_marks_only_that_symbol() {
        // `module.exports.foo = ...` should only mark `foo` as Public,
        // not all symbols in the file.
        let source = b"
function foo() {}
function bar() {}
module.exports.foo = foo;
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);
        let foo = symbols.symbols.iter().find(|s| s.name == "foo").unwrap();
        let bar = symbols.symbols.iter().find(|s| s.name == "bar").unwrap();

        assert_eq!(
            foo.visibility,
            Visibility::Public,
            "foo should be Public (it is the exported property)"
        );
        assert_eq!(
            bar.visibility,
            Visibility::Internal,
            "bar should stay Internal"
        );
    }

    #[test]
    fn module_exports_object_marks_only_referenced_symbols() {
        let source = b"
function foo() {}
function bar() {}
module.exports = { foo };
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.js"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.js"), 0);
        let foo = symbols.symbols.iter().find(|s| s.name == "foo").unwrap();
        let bar = symbols.symbols.iter().find(|s| s.name == "bar").unwrap();

        assert_eq!(
            foo.visibility,
            Visibility::Public,
            "foo should be Public (it is in the module.exports object)"
        );
        assert_eq!(
            bar.visibility,
            Visibility::Internal,
            "bar should stay Internal (it is not in the module.exports object)"
        );
    }

    #[test]
    fn extracts_require_inside_lexical_declaration() {
        let source = b"const fs = require('node:fs');\nconst path = require('path');\n";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.js"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.js"), 0);
        let import_sources: Vec<&str> = symbols
            .imports
            .iter()
            .map(|i| i.to_source.as_str())
            .collect();

        assert!(
            import_sources.contains(&"node:fs"),
            "require('node:fs') inside const should be captured"
        );
        assert!(
            import_sources.contains(&"path"),
            "require('path') inside const should be captured"
        );

        let fs_req = symbols
            .imports
            .iter()
            .find(|i| i.to_source == "node:fs")
            .unwrap();
        assert_eq!(fs_req.line, 1, "first require on line 1");

        let path_req = symbols
            .imports
            .iter()
            .find(|i| i.to_source == "path")
            .unwrap();
        assert_eq!(path_req.line, 2, "second require on line 2");
    }

    // --- CIB-093 N1: dynamic require()/import() ---

    #[test]
    fn literal_require_does_not_flag_unresolved_dynamic_import() {
        // A string-literal require is a determinable static import — captured as
        // an ImportEdge, not flagged as unresolved.
        let source = b"const cp = require('child_process');\n";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("a.js"), source).unwrap();
        let symbols = extract_symbols(&result.tree, source, Path::new("a.js"), 0);
        assert!(
            symbols
                .imports
                .iter()
                .any(|i| i.to_source == "child_process"),
            "literal require must produce an ImportEdge"
        );
        assert!(
            !symbols.has_unresolved_dynamic_import,
            "a literal require is fully resolved"
        );
    }

    #[test]
    fn literal_dynamic_import_is_captured_as_import_edge() {
        // (a): a string-literal `import('fs')` IS a determinable privileged
        // import — model it the same as a static import so it flows through
        // is_privileged_import, not as an unknown.
        let source = b"const fs = await import('fs');\n";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("a.js"), source).unwrap();
        let symbols = extract_symbols(&result.tree, source, Path::new("a.js"), 0);
        assert!(
            symbols.imports.iter().any(|i| i.to_source == "fs"),
            "literal import('fs') must produce an ImportEdge, got {:?}",
            symbols.imports
        );
        assert!(
            !symbols.has_unresolved_dynamic_import,
            "a literal dynamic import is fully resolved"
        );
    }

    #[test]
    fn computed_require_flags_unresolved_dynamic_import() {
        // (b): `require(someVar)` — the target is unknowable statically. It must
        // NOT emit a garbage `someVar` import edge, and it MUST set the
        // unresolved-dynamic-import signal so the certify path fails closed.
        let source = b"const mod = require(pickModule());\n";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("a.js"), source).unwrap();
        let symbols = extract_symbols(&result.tree, source, Path::new("a.js"), 0);
        assert!(
            symbols.has_unresolved_dynamic_import,
            "a computed require must flag an unresolved dynamic import"
        );
        assert!(
            symbols.imports.is_empty(),
            "a computed require must not emit a garbage import edge, got {:?}",
            symbols.imports
        );
    }

    #[test]
    fn computed_dynamic_import_flags_unresolved_dynamic_import() {
        // (b): a template-string `import(`./${x}`)` is unresolvable.
        let source = b"const m = import(`./${name}`);\n";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("a.js"), source).unwrap();
        let symbols = extract_symbols(&result.tree, source, Path::new("a.js"), 0);
        assert!(
            symbols.has_unresolved_dynamic_import,
            "a computed dynamic import must flag unresolved"
        );
    }

    #[test]
    fn identifier_named_require_call_is_not_a_dynamic_import() {
        // Defensive: only a `require`/`import` callee is a dynamic import; an
        // ordinary call like `compute(x)` must not flag anything.
        let source = b"function f() { return compute(x); }\n";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("a.js"), source).unwrap();
        let symbols = extract_symbols(&result.tree, source, Path::new("a.js"), 0);
        assert!(
            !symbols.has_unresolved_dynamic_import,
            "an ordinary call must not flag a dynamic import"
        );
    }

    #[test]
    fn module_exports_single_identifier_marks_only_that_symbol() {
        let source = b"
function foo() {}
function bar() {}
module.exports = foo;
";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.js"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.js"), 0);
        let foo = symbols.symbols.iter().find(|s| s.name == "foo").unwrap();
        let bar = symbols.symbols.iter().find(|s| s.name == "bar").unwrap();

        assert_eq!(
            foo.visibility,
            Visibility::Public,
            "foo should be Public (it is the module.exports value)"
        );
        assert_eq!(
            bar.visibility,
            Visibility::Internal,
            "bar should stay Internal"
        );
    }

    #[test]
    fn reexport_captures_line_number() {
        let source = b"export { foo } from './utils';\nexport { bar } from './helpers';\n";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();

        let symbols = extract_symbols(&result.tree, source, Path::new("test.ts"), 0);

        let utils_import = symbols
            .imports
            .iter()
            .find(|i| i.to_source == "./utils")
            .unwrap();
        assert_eq!(utils_import.line, 1, "re-export from ./utils on line 1");

        let helpers_import = symbols
            .imports
            .iter()
            .find(|i| i.to_source == "./helpers")
            .unwrap();
        assert_eq!(helpers_import.line, 2, "re-export from ./helpers on line 2");
    }

    #[test]
    fn reexport_emits_first_class_reexport_edges() {
        // `export { a, b as c } from "m"` re-exports `a` and `b`; `export * from`
        // is a wildcard. Each yields a ReexportEdge (in addition to the
        // dependency ImportEdge) so impact analysis sees the widened surface.
        let source = b"export { foo, bar as baz } from './utils';\nexport * from './all';\n";
        let mut parser = Parser::new();
        let result = parser
            .parse_bytes(Path::new("src/index.ts"), source)
            .unwrap();
        let fs = extract_symbols(&result.tree, source, Path::new("src/index.ts"), 0);

        let got: Vec<(&str, &str, u32)> = fs
            .reexports
            .iter()
            .map(|r| (r.exported_name.as_str(), r.to_source.as_str(), r.line))
            .collect();
        assert_eq!(
            got,
            vec![
                ("foo", "./utils", 1),
                ("bar", "./utils", 1),
                ("*", "./all", 2),
            ],
            "re-export edges (source-side names; `*` for wildcard)"
        );
        assert!(fs.reexports.iter().all(|r| r.from_file == "src/index.ts"));

        // The dependency import edges are still emitted alongside.
        let import_srcs: Vec<&str> = fs.imports.iter().map(|i| i.to_source.as_str()).collect();
        assert!(import_srcs.contains(&"./utils"));
        assert!(import_srcs.contains(&"./all"));
    }

    #[test]
    fn plain_import_emits_no_reexport_edge() {
        let source = b"import { x } from './dep';\n";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("a.ts"), source).unwrap();
        let fs = extract_symbols(&result.tree, source, Path::new("a.ts"), 0);
        assert!(
            fs.reexports.is_empty(),
            "a plain import must not produce a re-export edge"
        );
    }

    #[test]
    fn namespace_reexport_binds_the_alias_not_wildcard() {
        // `export * as ns from "m"` re-publishes the whole module under `ns`;
        // the edge name is `ns`, and a public `Export` symbol named `ns` is
        // emitted (not the bare `*` fallback used for `export * from "m"`).
        let source = b"export * as widgets from './widgets';\n";
        let mut parser = Parser::new();
        let result = parser
            .parse_bytes(Path::new("src/index.ts"), source)
            .unwrap();
        let fs = extract_symbols(&result.tree, source, Path::new("src/index.ts"), 0);

        let re: Vec<(&str, &str)> = fs
            .reexports
            .iter()
            .map(|r| (r.exported_name.as_str(), r.to_source.as_str()))
            .collect();
        assert_eq!(re, vec![("widgets", "./widgets")]);
        assert!(
            fs.symbols
                .iter()
                .any(|s| s.kind == SymbolKind::Export && s.name == "widgets"),
            "namespace re-export emits an Export symbol named `widgets`, got {:?}",
            fs.symbols
        );
        assert!(
            !fs.symbols.iter().any(|s| s.name == "*"),
            "no bare `*` symbol for a named namespace re-export"
        );
    }

    #[test]
    fn type_only_reexport_keeps_named_edges() {
        // `export type { T } from "m"` is a named re-export (the `type` keyword
        // is a token, not a different specifier kind) — it must not collapse to
        // a `*` wildcard.
        let source = b"export type { Props } from './types';\n";
        let mut parser = Parser::new();
        let result = parser
            .parse_bytes(Path::new("src/index.ts"), source)
            .unwrap();
        let fs = extract_symbols(&result.tree, source, Path::new("src/index.ts"), 0);

        let names: Vec<&str> = fs
            .reexports
            .iter()
            .map(|r| r.exported_name.as_str())
            .collect();
        assert_eq!(
            names,
            ["Props"],
            "type-only re-export keeps the named edge, got {:?}",
            fs.reexports
        );
    }

    /// K1 parity: the TS extractor, after the `LanguageExtractor`-trait
    /// refactor, must produce byte-for-byte the same `FileSymbols` (symbol ids,
    /// kinds, names, visibilities, and import edges with line numbers) as the
    /// pre-refactor walker. The expected values below were captured from the
    /// original `extract.rs` walker against this fixture; any drift here is a
    /// behavioural regression in the port, not a test that needs updating.
    #[test]
    fn ts_extractor_parity_snapshot() {
        // A fixture exercising the breadth of the walker: ESM function/class
        // exports, an internal symbol, an arrow-fn const, a named import, and
        // a re-export.
        let source = b"import { dep } from './dep';\n\
export function greet(name: string): string { return name; }\n\
function internal() {}\n\
export class Greeter {}\n\
const add = (a: number, b: number) => a + b;\n\
export { foo } from './other';\n";

        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("src/mod.ts"), source).unwrap();
        let fs = extract_symbols(&result.tree, source, Path::new("src/mod.ts"), 0);

        // --- Symbols: id / kind / name / visibility, in emission order ---
        let got: Vec<(u64, SymbolKind, &str, Visibility)> = fs
            .symbols
            .iter()
            .map(|s| (s.id, s.kind, s.name.as_str(), s.visibility))
            .collect();
        let expected: Vec<(u64, SymbolKind, &str, Visibility)> = vec![
            (0, SymbolKind::Function, "greet", Visibility::Public),
            (1, SymbolKind::Function, "internal", Visibility::Internal),
            (2, SymbolKind::Class, "Greeter", Visibility::Public),
            (3, SymbolKind::Function, "add", Visibility::Internal),
            (4, SymbolKind::Export, "foo", Visibility::Public),
        ];
        assert_eq!(
            got, expected,
            "symbol parity drifted from pre-refactor walker"
        );

        // Every symbol is attributed to the file.
        assert!(fs.symbols.iter().all(|s| s.file == "src/mod.ts"));

        // --- Imports: source + 1-based line, in emission order ---
        let imports: Vec<(&str, u32)> = fs
            .imports
            .iter()
            .map(|i| (i.to_source.as_str(), i.line))
            .collect();
        assert_eq!(
            imports,
            vec![("./dep", 1), ("./other", 6)],
            "import-edge parity drifted from pre-refactor walker"
        );
        assert!(fs.imports.iter().all(|i| i.from_file == "src/mod.ts"));

        // --- Re-exports (GV2-010): the `export { foo } from './other'` line ---
        let reexports: Vec<(&str, &str, u32)> = fs
            .reexports
            .iter()
            .map(|r| (r.exported_name.as_str(), r.to_source.as_str(), r.line))
            .collect();
        assert_eq!(
            reexports,
            vec![("foo", "./other", 6)],
            "re-export edge output must stay pinned"
        );
    }

    /// K1: the orchestrator routes through `TypeScriptExtractor` for every
    /// member of the TS family (.ts/.tsx/.js/.jsx) and yields identical results
    /// to calling the extractor directly.
    #[test]
    fn orchestrator_dispatch_matches_direct_extractor_call() {
        let source = b"export function f() {}\n";
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("a.ts"), source).unwrap();

        let via_orchestrator = extract_symbols(&result.tree, source, Path::new("a.ts"), 0);
        let via_trait = TypeScriptExtractor.extract(&result.tree, source, "a.ts", 0);

        assert_eq!(via_orchestrator.symbols.len(), via_trait.symbols.len());
        assert_eq!(via_orchestrator.symbols[0].name, "f");
        assert_eq!(via_trait.symbols[0].name, "f");
    }

    // --- GCALL-002 call-site extraction ---

    use anvil_kernel_types::{CalleeRef, FileSymbols, LocalSymbolRef, SymbolIdentity};

    fn extract(source: &[u8]) -> FileSymbols {
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("test.ts"), source).unwrap();
        extract_symbols(&result.tree, source, Path::new("test.ts"), 0)
    }

    /// Find the single call whose resolved callee `name` matches.
    fn call_to<'a>(fs: &'a FileSymbols, callee: &str) -> &'a anvil_kernel_types::CallSite {
        let matches: Vec<_> = fs
            .calls
            .iter()
            .filter(|c| c.callee.name == callee)
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "expected exactly one call to `{callee}`, got {}: {:#?}",
            matches.len(),
            fs.calls
        );
        matches[0]
    }

    fn fn_caller(name: &str, ordinal: u32) -> LocalSymbolRef {
        LocalSymbolRef {
            kind: SymbolKind::Function,
            name: name.to_string(),
            ordinal,
            module_scope: false,
        }
    }

    #[test]
    fn direct_same_file_call_attributes_caller_and_callee() {
        let fs = extract(b"function helper() {}\nfunction run() { helper(); }\n");
        let call = call_to(&fs, "helper");
        assert_eq!(call.from, fn_caller("run", 0));
        assert_eq!(
            call.callee,
            CalleeRef {
                name: "helper".into(),
                via_import: None
            }
        );
        assert_eq!(call.line, 2);
    }

    #[test]
    fn module_scope_call_has_module_scope_caller() {
        let fs = extract(b"function setup() {}\nsetup();\n");
        let call = call_to(&fs, "setup");
        assert!(call.from.module_scope, "top-level call is module-scoped");
        assert_eq!(call.from.kind, SymbolKind::Module);
    }

    #[test]
    fn imported_symbol_call_carries_export_name_and_specifier() {
        let fs = extract(b"import { foo } from './m';\nfunction run() { foo(); }\n");
        let call = call_to(&fs, "foo");
        assert_eq!(call.from, fn_caller("run", 0));
        assert_eq!(call.callee.via_import.as_deref(), Some("./m"));
    }

    #[test]
    fn aliased_import_resolves_to_export_name() {
        let fs = extract(b"import { foo as bar } from './m';\nbar();\n");
        // The callee is the export name `foo`, not the local alias `bar`.
        let call = call_to(&fs, "foo");
        assert_eq!(call.callee.via_import.as_deref(), Some("./m"));
        assert!(fs.calls.iter().all(|c| c.callee.name != "bar"));
    }

    #[test]
    fn namespace_member_call_resolves_member_against_specifier() {
        let fs = extract(b"import * as ns from './m';\nns.foo();\n");
        let call = call_to(&fs, "foo");
        assert_eq!(call.callee.via_import.as_deref(), Some("./m"));
    }

    #[test]
    fn default_import_call_is_named_default() {
        // Per ADR-086 a default-export callee is named `default` (Unresolved at
        // lift time).
        let fs = extract(b"import d from './m';\nd();\n");
        let call = call_to(&fs, "default");
        assert_eq!(call.callee.via_import.as_deref(), Some("./m"));
    }

    #[test]
    fn this_method_call_inside_class_resolves_to_owner_method() {
        let fs = extract(b"class Greeter {\n  greet() { this.helper(); }\n  helper() {}\n}\n");
        let call = call_to(&fs, "Greeter.helper");
        assert_eq!(
            call.from,
            LocalSymbolRef {
                kind: SymbolKind::Method,
                name: "Greeter.greet".into(),
                ordinal: 0,
                module_scope: false,
            }
        );
        assert_eq!(call.callee.via_import, None);
    }

    #[test]
    fn general_member_call_is_named_member_unresolved_specifier() {
        // `obj.save()` on an unknown receiver: named by member, no specifier —
        // lift-time will treat it as Unresolved unless a same-file match exists.
        let fs = extract(b"function run(obj) { obj.save(); }\n");
        let call = call_to(&fs, "save");
        assert_eq!(call.from, fn_caller("run", 0));
        assert_eq!(call.callee.via_import, None);
    }

    #[test]
    fn constructor_call_names_the_constructor() {
        let fs =
            extract(b"import { Widget } from './w';\nfunction make() { return new Widget(); }\n");
        let call = call_to(&fs, "Widget");
        assert_eq!(call.from, fn_caller("make", 0));
        assert_eq!(call.callee.via_import.as_deref(), Some("./w"));
    }

    #[test]
    fn call_inside_named_arrow_const_attributes_to_the_const() {
        let fs = extract(b"function helper() {}\nconst run = () => { helper(); };\n");
        let call = call_to(&fs, "helper");
        assert_eq!(call.from, fn_caller("run", 0));
    }

    #[test]
    fn require_is_not_emitted_as_a_call() {
        let fs =
            extract(b"const fs = require('node:fs');\nfunction run() { fs.readFileSync('x'); }\n");
        assert!(
            fs.calls.iter().all(|c| c.callee.name != "require"),
            "require() is a CJS import, not a symbol call: {:#?}",
            fs.calls
        );
    }

    #[test]
    fn anonymous_callback_calls_attribute_to_enclosing_named_scope() {
        // The arrow passed to `forEach` is anonymous → its body's call attributes
        // to `run`, not to a new scope and not to module scope.
        let fs = extract(
            b"function sink(x) {}\nfunction run(items) { items.forEach((i) => sink(i)); }\n",
        );
        let call = call_to(&fs, "sink");
        assert_eq!(call.from, fn_caller("run", 0));
    }

    /// ADR-086: every non-module caller ref is a real emitted symbol identity
    /// with the `for_file_symbols` ordinal — the consistency the lift relies on.
    #[test]
    fn caller_refs_match_for_file_symbols_identities() {
        let fs = extract(
            b"function a() { b(); }\nfunction b() { a(); }\nclass C {\n  m() { this.n(); }\n  n() {}\n}\nconst d = () => { a(); };\n",
        );
        let refs: Vec<&SymbolNode> = fs.symbols.iter().collect();
        let identities: std::collections::HashSet<(SymbolKind, String, u32)> =
            SymbolIdentity::for_file_symbols(&refs)
                .into_iter()
                .map(|id| (id.kind, id.name, id.ordinal))
                .collect();

        for call in &fs.calls {
            if call.from.module_scope {
                continue;
            }
            let key = (call.from.kind, call.from.name.clone(), call.from.ordinal);
            assert!(
                identities.contains(&key),
                "caller {:?} is not an emitted symbol identity; identities={identities:?}",
                call.from
            );
        }
    }

    #[test]
    fn extraction_is_deterministic() {
        let source =
            b"import { x } from './m';\nfunction a() { x(); helper(); }\nfunction helper() {}\n";
        assert_eq!(extract(source).calls, extract(source).calls);
    }

    #[test]
    fn call_in_nested_function_attributes_to_outer_emitted_symbol() {
        // Pass 1 does NOT emit `inner` (it never recurses a function body), so a
        // call inside it must attribute to `outer` — never a phantom `inner`
        // caller (the span model uses pass 1's actual symbol set).
        let fs =
            extract(b"function outer() {\n  function inner() { sink(); }\n}\nfunction sink() {}\n");
        let call = call_to(&fs, "sink");
        assert_eq!(call.from, fn_caller("outer", 0));
        assert!(fs.calls.iter().all(|c| c.from.name != "inner"));
    }

    #[test]
    fn call_in_anonymous_default_export_attributes_to_default_symbol() {
        // Pass 1 emits a synthetic Function "default" for an anonymous
        // `export default function(){}`; the call inside must attribute to it,
        // not to module scope.
        let fs = extract(b"export default function() {\n  sink();\n}\nfunction sink() {}\n");
        let call = call_to(&fs, "sink");
        assert!(!call.from.module_scope, "caller is the default-export fn");
        assert_eq!(call.from.kind, SymbolKind::Function);
        assert_eq!(call.from.name, "default");
    }

    #[test]
    fn call_in_class_field_initializer_attributes_to_class() {
        // A call in a class field initializer (not a method) has the class as its
        // nearest emitted enclosing symbol.
        let fs = extract(b"function seed() {}\nclass C {\n  x = seed();\n}\n");
        let call = call_to(&fs, "seed");
        assert_eq!(
            call.from,
            LocalSymbolRef {
                kind: SymbolKind::Class,
                name: "C".into(),
                ordinal: 0,
                module_scope: false,
            }
        );
    }

    #[test]
    fn call_in_nested_function_inside_arrow_const_attributes_to_nested() {
        // Pass 1 DOES recurse arrow-const bodies, so it emits the nested `inner`
        // Function; its span is innermost, so the call attributes to `inner`
        // (the span model tracks exactly what pass 1 emitted).
        let fs = extract(
            b"function sink() {}\nconst run = () => {\n  function inner() { sink(); }\n};\n",
        );
        let call = call_to(&fs, "sink");
        assert_eq!(call.from, fn_caller("inner", 0));
    }
}
