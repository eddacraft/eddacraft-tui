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

use std::collections::HashMap;
use std::ops::Range;

use anvil_kernel_types::{
    ByteRange, CallSite, CalleeRef, LocalSymbolRef, SymbolIdentity, SymbolKind, SymbolNode,
    TrustLevel, Visibility, content_hash,
};

use super::{FileSymbols, ImportEdge, LanguageExtractor, ReexportEdge};

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
        let mut spans = Vec::new();
        let mut imports = Vec::new();
        let mut reexports = Vec::new();
        let mut next_id = id_offset;

        extract_from_node(
            tree.root_node(),
            source,
            file,
            &mut symbols,
            &mut spans,
            &mut imports,
            &mut reexports,
            &mut next_id,
        );

        debug_assert_eq!(
            spans.len(),
            symbols.len(),
            "GCALL-004: every emitted symbol must record a defining-node span",
        );

        // Pass 2 (GCALL-004): a separate walk emits call sites without touching
        // symbol/import/reexport emission, so the Pass 1 parity story is unchanged.
        let calls = extract_call_sites(tree.root_node(), source, &symbols, &spans);

        // GV2-032: lift each symbol's defining-node byte span onto the symbol
        // (offsets only, never text — PV-7(e)) so it rides into the resident graph.
        for (symbol, span) in symbols.iter_mut().zip(&spans) {
            symbol.span = Some(ByteRange::from_range(span.clone()));
        }

        FileSymbols {
            file: file.to_string(),
            symbols,
            imports,
            reexports,
            calls,
            calls_partial: false,
            // Rust has no JS/TS dynamic require()/import() (CIB-093 N1).
            has_unresolved_dynamic_import: false,
            // GV2-032: the freshness key for the bytes we just parsed (CE-7).
            content_hash: Some(content_hash(source)),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn extract_from_node(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    spans: &mut Vec<Range<usize>>,
    imports: &mut Vec<ImportEdge>,
    reexports: &mut Vec<ReexportEdge>,
    next_id: &mut u64,
) {
    match node.kind() {
        "function_item" => {
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
        // A struct or union is the nominal type carrying members — the closest
        // language-agnostic kind is `Class`.
        "struct_item" | "union_item" => {
            push_named(
                node,
                source,
                file,
                symbols,
                spans,
                next_id,
                SymbolKind::Class,
            );
        }
        "enum_item" => {
            push_named(
                node,
                source,
                file,
                symbols,
                spans,
                next_id,
                SymbolKind::Enum,
            );
        }
        "type_item" => {
            push_named(
                node,
                source,
                file,
                symbols,
                spans,
                next_id,
                SymbolKind::TypeAlias,
            );
        }
        // A trait is Rust's interface contract; surface its methods as
        // `Trait.method` (mirrors TS-G2 class methods) from both default-bodied
        // `function_item`s and bodyless `function_signature_item`s.
        "trait_item" => {
            let owner = field_name_text(node, source);
            push_named(
                node,
                source,
                file,
                symbols,
                spans,
                next_id,
                SymbolKind::Interface,
            );
            if let Some(owner) = owner {
                emit_assoc_fns(node, source, file, symbols, spans, next_id, &owner);
            }
        }
        // An `impl` block emits no symbol of its own; its associated functions
        // become `Type.method`. The Self type comes from the `type` field
        // (generics stripped: `Foo<T>` → `Foo`).
        "impl_item" => {
            if let Some(ty) = node.child_by_field_name("type") {
                let owner = base_type_name(&node_text(ty, source));
                emit_assoc_fns(node, source, file, symbols, spans, next_id, &owner);
            }
        }
        // A module declaration is a structural symbol; recurse into an inline
        // body so nested items + `use` edges are captured. A bare `mod foo;`
        // (no body) just declares the child — file resolution is RSTLAN-005.
        "mod_item" => {
            push_named(
                node,
                source,
                file,
                symbols,
                spans,
                next_id,
                SymbolKind::Module,
            );
            if let Some(body) = node.child_by_field_name("body") {
                recurse_children(
                    body, source, file, symbols, spans, imports, reexports, next_id,
                );
            }
        }
        "use_declaration" => extract_use(node, source, file, imports, reexports),
        "extern_crate_declaration" => extract_extern_crate(node, source, file, imports),
        // Everything else (the `source_file` root, attribute items, comments,
        // top-level statics/consts, expression statements) is traversed
        // generically — its children may hold items we do handle.
        _ => recurse_children(
            node, source, file, symbols, spans, imports, reexports, next_id,
        ),
    }
}

/// Recurse into a node's named children with the top-level dispatcher.
#[allow(clippy::too_many_arguments)]
fn recurse_children(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    symbols: &mut Vec<SymbolNode>,
    spans: &mut Vec<Range<usize>>,
    imports: &mut Vec<ImportEdge>,
    reexports: &mut Vec<ReexportEdge>,
    next_id: &mut u64,
) {
    for i in 0..u32::try_from(node.named_child_count()).unwrap_or(0) {
        if let Some(child) = node.named_child(i) {
            extract_from_node(
                child, source, file, symbols, spans, imports, reexports, next_id,
            );
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
    spans: &mut Vec<Range<usize>>,
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
        span: None,
    });
    spans.push(node.byte_range());
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
                span: None,
            });
            spans.push(member.byte_range());
            *next_id += 1;
        }
    }
}

/// One resolved leaf of a `use` tree.
struct UseLeaf {
    /// The imported path — `ImportEdge.to_source` and, for a re-export, the
    /// `ReexportEdge.to_source` (Rust's full-path convention).
    source: String,
    /// The name a consumer binds to when this leaf is re-exported: the `as`
    /// alias if present, `*` for a glob, else the path's last `::` segment.
    name: String,
}

/// Last `::` segment of a path (`a::b::C` → `C`); the path itself if unscoped.
fn last_segment(path: &str) -> String {
    path.rsplit("::").next().unwrap_or(path).to_string()
}

/// Flatten a `use_declaration` into one [`ImportEdge`] per leaf path.
///
/// A bare `pub use` (not the crate-internal `pub(crate)` / `pub(super)`)
/// re-exports: it widens the module's public surface, so each leaf also emits a
/// [`ReexportEdge`] alongside its dependency [`ImportEdge`]. The re-exported
/// name is the consumer-visible binding — the `as` alias
/// (`pub use a::B as C` → `C`), `*` for a glob (`pub use a::*`), else the
/// path's last segment (`pub use a::b::C` → `C`). `to_source` carries the full
/// path, matching Rust's `ImportEdge` convention.
fn extract_use(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    imports: &mut Vec<ImportEdge>,
    reexports: &mut Vec<ReexportEdge>,
) {
    let Some(arg) = node.child_by_field_name("argument") else {
        return;
    };
    let mut leaves = Vec::new();
    collect_use_paths(arg, source, "", &mut leaves);
    let line = node_line(node);
    let is_reexport = item_visibility(node, source) == Visibility::Public;
    for leaf in leaves {
        if leaf.source.is_empty() {
            continue;
        }
        if is_reexport {
            reexports.push(ReexportEdge {
                from_file: file.to_string(),
                exported_name: leaf.name.clone(),
                to_source: leaf.source.clone(),
                line,
            });
        }
        imports.push(ImportEdge {
            from_file: file.to_string(),
            to_source: leaf.source,
            line,
        });
    }
}

/// Recursively flatten a `use` tree, accumulating the `::`-joined prefix so a
/// grouped `foo::{a, b}` yields `foo::a` and `foo::b`.
fn collect_use_paths(node: tree_sitter::Node, source: &[u8], prefix: &str, out: &mut Vec<UseLeaf>) {
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
        // `foo::bar as baz` — the import edge tracks the source path, but a
        // re-export binds the *alias* (`baz`), which is the public name. A
        // `self as x` leaf still means "the parent module itself".
        "use_as_clause" => {
            let path = node
                .child_by_field_name("path")
                .map(|p| node_text(p, source))
                .unwrap_or_default();
            let alias = node
                .child_by_field_name("alias")
                .map(|a| node_text(a, source));
            if path == "self" {
                push_module_self(prefix, alias, out);
            } else {
                let source_path = join_path(prefix, &path);
                let name = alias.unwrap_or_else(|| last_segment(&source_path));
                out.push(UseLeaf {
                    source: source_path,
                    name,
                });
            }
        }
        // `foo::*` — record the module the glob reaches into; a re-export of it
        // re-publishes the whole surface, so the name is the `*` wildcard.
        "use_wildcard" => {
            let child = node
                .named_child(0)
                .map(|p| node_text(p, source))
                .unwrap_or_default();
            let full = join_path(prefix, &child);
            if !full.is_empty() {
                out.push(UseLeaf {
                    source: full,
                    name: String::from("*"),
                });
            }
        }
        // A bare `self` leaf in a group — `use foo::{self, bar}` — imports the
        // parent module itself, so the edge is the prefix, not `prefix::self`.
        _ => {
            let text = node_text(node, source);
            if text == "self" {
                push_module_self(prefix, None, out);
            } else {
                // Leaf paths: `foo`, `crate::a::B`, `super::x`, `self::y`.
                let source_path = join_path(prefix, &text);
                let name = last_segment(&source_path);
                out.push(UseLeaf {
                    source: source_path,
                    name,
                });
            }
        }
    }
}

/// `self` as a use-list leaf refers to the enclosing module path. Emit that
/// prefix as the edge (a top-level bare `use self;` has no prefix → no edge).
/// A `self as x` re-export binds the alias `x`; otherwise the module's last
/// segment is the bound name.
fn push_module_self(prefix: &str, alias: Option<String>, out: &mut Vec<UseLeaf>) {
    if !prefix.is_empty() {
        let name = alias.unwrap_or_else(|| last_segment(prefix));
        out.push(UseLeaf {
            source: prefix.to_string(),
            name,
        });
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

/// Best-effort owner name for an `impl`'s `Owner.method` symbols. Strips a
/// leading reference / raw-pointer prefix (`impl Trait for &Foo` → `Foo`) so the
/// owner is the nominal type, then drops generic arguments (`Foo<T>` → `Foo`).
/// Exotic Self types (`[T]`, `dyn Trait`, tuples, lifetime-annotated refs) keep
/// their text — a slightly odd owner name is harmless (an odd edge, never a
/// false violation), and proc-macro/lifetime-aware resolution is out of scope.
fn base_type_name(ty: &str) -> String {
    let ty = ty.trim();
    let ty = ty
        .strip_prefix("&mut ")
        .or_else(|| ty.strip_prefix('&'))
        .or_else(|| ty.strip_prefix("*const "))
        .or_else(|| ty.strip_prefix("*mut "))
        .unwrap_or(ty);
    ty.split('<').next().unwrap_or(ty).trim().to_string()
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

// --- Pass 2: call-site extraction (GCALL-004) ---

/// Emit symbol-level call sites for a Rust file (ADR-086 §2).
///
/// Mirrors the TS/JS extractor: build a local-binding table from `use`
/// declarations, assign stable caller ordinals with
/// [`SymbolIdentity::for_file_symbols`] over the same `symbols` slice Pass 1
/// emitted, then walk every `call_expression`, attributing it to the innermost
/// enclosing emitted symbol (by span) and resolving its callee best-effort and
/// statically. Cross-file callee resolution is lift-time (`re_resolve_calls`,
/// GCALL-003).
fn extract_call_sites(
    root: tree_sitter::Node,
    source: &[u8],
    symbols: &[SymbolNode],
    spans: &[Range<usize>],
) -> Vec<CallSite> {
    let bindings = build_use_bindings(root, source);
    let refs: Vec<&SymbolNode> = symbols.iter().collect();
    let identities = SymbolIdentity::for_file_symbols(&refs);
    let mut calls = Vec::new();
    walk_calls(root, source, &bindings, spans, &identities, &mut calls);
    calls
}

/// A name brought into file scope by a `use`: the target's export name (the
/// path's last segment) and the module path it lives in (the specifier).
struct UseBinding {
    export_name: String,
    specifier: String,
}

/// Build the local-name → [`UseBinding`] table from every `use_declaration`,
/// reusing the Pass 1 leaf flattener. The local binding is the `as` alias when
/// present, else the path's last segment ([`UseLeaf`]`::name`); the export name
/// is the path's last segment and the specifier is the path prefix. A glob
/// (`use a::*`) binds no nameable symbol and is skipped.
fn build_use_bindings(root: tree_sitter::Node, source: &[u8]) -> HashMap<String, UseBinding> {
    let mut bindings = HashMap::new();
    collect_use_bindings(root, source, &mut bindings);
    bindings
}

fn collect_use_bindings(
    node: tree_sitter::Node,
    source: &[u8],
    bindings: &mut HashMap<String, UseBinding>,
) {
    if node.kind() == "use_declaration" {
        if let Some(arg) = node.child_by_field_name("argument") {
            let mut leaves = Vec::new();
            collect_use_paths(arg, source, "", &mut leaves);
            for leaf in leaves {
                if leaf.name == "*" || leaf.source.is_empty() {
                    continue;
                }
                let export_name = last_segment(&leaf.source);
                let specifier = path_prefix(&leaf.source);
                bindings.insert(
                    leaf.name,
                    UseBinding {
                        export_name,
                        specifier,
                    },
                );
            }
        }
        return;
    }
    for i in 0..u32::try_from(node.named_child_count()).unwrap_or(0) {
        if let Some(child) = node.named_child(i) {
            collect_use_bindings(child, source, bindings);
        }
    }
}

/// Everything before the last `::` segment (`a::b::c` → `a::b`); empty for a
/// single-segment path.
fn path_prefix(path: &str) -> String {
    path.rsplit_once("::")
        .map_or_else(String::new, |(prefix, _)| prefix.to_string())
}

/// `Some(s)` when `s` is non-empty, else `None` — the same-file
/// (`via_import: None`) vs module-qualified callee discriminator.
fn non_empty(s: &str) -> Option<String> {
    (!s.is_empty()).then(|| s.to_string())
}

/// The caller for a call at byte `pos`: the innermost emitted-symbol span
/// containing it (smallest width wins, so a method beats its enclosing module),
/// or the module-scope placeholder when none contains it.
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

/// Recursively walk, emitting call sites. Each `call_expression` is attributed
/// to its enclosing emitted symbol via [`caller_at`] over the parallel `spans` /
/// `identities` — no scope tracking, so it cannot mint a caller Pass 1 did not
/// emit. `macro_invocation` (`println!()`) is a distinct node kind and is never
/// matched, so macro calls are invisible (consistent with the Pass 1 docs).
fn walk_calls(
    node: tree_sitter::Node,
    source: &[u8],
    bindings: &HashMap<String, UseBinding>,
    spans: &[Range<usize>],
    identities: &[SymbolIdentity],
    calls: &mut Vec<CallSite>,
) {
    if node.kind() == "call_expression"
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

/// The owner type/trait of the enclosing caller, for `self.method()` /
/// `Self::method()` resolution: a `Method` caller's `Owner.method` prefix, or a
/// `Class`/`Interface` caller's own name.
fn enclosing_owner_name(caller: &LocalSymbolRef) -> Option<String> {
    if caller.module_scope {
        return None;
    }
    match caller.kind {
        SymbolKind::Class | SymbolKind::Interface => Some(caller.name.clone()),
        SymbolKind::Method => caller.name.split('.').next().map(ToString::to_string),
        _ => None,
    }
}

/// Resolve a Rust callee expression (the `function` of a `call_expression`) to a
/// [`CalleeRef`], or `None` when there is no statically nameable callee (a macro,
/// a computed/closure callee). Best-effort and static (ADR-086 §1); cross-file
/// resolution is lift-time.
fn resolve_callee(
    func: tree_sitter::Node,
    source: &[u8],
    bindings: &HashMap<String, UseBinding>,
    caller: &LocalSymbolRef,
) -> Option<CalleeRef> {
    match func.kind() {
        // `foo()` — a `use`-bound name (alias reverse-mapped to its export name +
        // module), else a same-file callee.
        "identifier" => {
            let name = node_text(func, source);
            match bindings.get(&name) {
                Some(UseBinding {
                    export_name,
                    specifier,
                }) => Some(CalleeRef {
                    name: export_name.clone(),
                    via_import: non_empty(specifier),
                }),
                None => Some(CalleeRef {
                    name,
                    via_import: None,
                }),
            }
        }
        // `a::b::c()` — `Self::m()` resolves to the enclosing owner's `Owner.m`;
        // any other qualified path is the namespace-member shape (export name +
        // the qualifier as the module specifier), resolved cross-module at lift.
        "scoped_identifier" => {
            let name = func
                .child_by_field_name("name")
                .map(|n| node_text(n, source))?;
            let path = func
                .child_by_field_name("path")
                .map(|p| node_text(p, source))
                .unwrap_or_default();
            if path == "Self" {
                let owner = enclosing_owner_name(caller)?;
                return Some(CalleeRef {
                    name: format!("{owner}.{name}"),
                    via_import: None,
                });
            }
            Some(CalleeRef {
                name,
                via_import: non_empty(&path),
            })
        }
        // `recv.method()` — `self.method()` resolves to `Owner.method`; any other
        // receiver names the bare method, leaving cross-file resolution to lift.
        "field_expression" => {
            let field = func.child_by_field_name("field")?;
            if field.kind() != "field_identifier" {
                return None;
            }
            let method = node_text(field, source);
            let receiver = func.child_by_field_name("value")?;
            let recv_is_self = receiver.kind() == "self"
                || (receiver.kind() == "identifier" && node_text(receiver, source) == "self");
            if recv_is_self && let Some(owner) = enclosing_owner_name(caller) {
                return Some(CalleeRef {
                    name: format!("{owner}.{method}"),
                    via_import: None,
                });
            }
            Some(CalleeRef {
                name: method,
                via_import: None,
            })
        }
        // `foo::<T>()` — turbofish wraps the real callee in the `function` field.
        "generic_function" => {
            let inner = func.child_by_field_name("function")?;
            resolve_callee(inner, source, bindings, caller)
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

    use super::{LanguageExtractor, RustExtractor};
    use crate::parser::Parser;
    use crate::parser::extract::extract_symbols;

    fn extract(source: &[u8]) -> anvil_kernel_types::FileSymbols {
        let mut parser = Parser::new();
        let result = parser.parse_bytes(Path::new("src/lib.rs"), source).unwrap();
        extract_symbols(&result.tree, source, Path::new("src/lib.rs"), 0)
    }

    #[test]
    fn populates_symbol_spans_and_content_hash_gv2_032() {
        let source = b"pub fn alpha() {\n    let _ = 1;\n}\n";
        let fs = extract(source);
        assert!(
            fs.symbols.iter().all(|s| s.span.is_some()),
            "GV2-032: every Rust symbol must carry a span",
        );
        let alpha = fs
            .symbols
            .iter()
            .find(|s| s.name == "alpha")
            .expect("alpha symbol");
        let span = alpha.span.expect("span populated");
        let text = std::str::from_utf8(&source[span.start as usize..span.end as usize]).unwrap();
        assert!(
            text.starts_with("pub fn alpha"),
            "span locates the item: {text:?}"
        );
        assert_eq!(
            fs.content_hash,
            Some(anvil_kernel_types::content_hash(source)),
            "GV2-032: content_hash is the hash of the parsed source",
        );
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
    fn pub_use_emits_a_reexport_edge_plain_use_does_not() {
        // `pub use` widens the public surface → ReexportEdge (name = last
        // segment) + the dependency ImportEdge. `pub(crate) use` and plain
        // `use` are not re-exports.
        let fs = extract(
            b"
pub use crate::internal::Widget;
pub use foo::bar::{Alpha, Beta};
pub(crate) use crate::hidden::Secret;
use std::fmt::Debug;
",
        );
        let got: Vec<(&str, &str)> = fs
            .reexports
            .iter()
            .map(|r| (r.exported_name.as_str(), r.to_source.as_str()))
            .collect();
        assert_eq!(
            got,
            vec![
                ("Widget", "crate::internal::Widget"),
                ("Alpha", "foo::bar::Alpha"),
                ("Beta", "foo::bar::Beta"),
            ],
            "only bare `pub use` re-exports; name = last `::` segment"
        );
        assert!(fs.reexports.iter().all(|r| r.from_file == "src/lib.rs"));
        // pub(crate) use and plain use are dependency edges but not re-exports.
        assert!(
            !fs.reexports
                .iter()
                .any(|r| r.to_source.contains("Secret") || r.to_source.contains("Debug")),
            "pub(crate)/plain use must not re-export: {:?}",
            fs.reexports
        );
    }

    #[test]
    fn pub_use_alias_reexports_the_alias_name() {
        // `pub use foo::Bar as Baz` re-publishes the binding as `Baz` (what
        // consumers see), not the source-side `Bar`.
        let fs = extract(b"pub use crate::internal::Widget as Gadget;\n");
        let got: Vec<(&str, &str)> = fs
            .reexports
            .iter()
            .map(|r| (r.exported_name.as_str(), r.to_source.as_str()))
            .collect();
        assert_eq!(got, vec![("Gadget", "crate::internal::Widget")]);
    }

    #[test]
    fn pub_use_glob_reexports_wildcard() {
        // `pub use foo::*` re-publishes the whole module surface → name `*`,
        // consistent with the TS `export * from` convention.
        let fs = extract(b"pub use crate::prelude::*;\n");
        let got: Vec<(&str, &str)> = fs
            .reexports
            .iter()
            .map(|r| (r.exported_name.as_str(), r.to_source.as_str()))
            .collect();
        assert_eq!(got, vec![("*", "crate::prelude")]);
    }

    #[test]
    fn use_list_self_imports_the_parent_module() {
        // `use foo::{self, bar}` — the `self` leaf is the parent module itself,
        // so the edge is `foo`, not `foo::self`. Common idiom; regression guard.
        let fs = extract(b"use std::fs::{self, OpenOptions};\nuse crate::util::{self};\n");
        let s = sources(&fs);
        assert!(s.contains(&"std::fs"), "`{{self}}` ⇒ parent module: {s:?}");
        assert!(s.contains(&"std::fs::OpenOptions"), "sibling leaf: {s:?}");
        assert!(
            s.contains(&"crate::util"),
            "lone `{{self}}` ⇒ parent: {s:?}"
        );
        assert!(
            !s.iter().any(|p| p.ends_with("::self")),
            "no edge should end in ::self, got {s:?}"
        );
    }

    #[test]
    fn trait_impl_methods_are_qualified_by_the_implementing_type() {
        // `impl Trait for Type` qualifies methods by the *implementing type*,
        // not the trait — the owner comes from the `type` field.
        let fs = extract(
            b"
struct Printer;
impl std::fmt::Display for Printer {
    fn fmt(&self, _: &mut std::fmt::Formatter) -> std::fmt::Result { Ok(()) }
}
",
        );
        let methods = names_of(&fs, SymbolKind::Method);
        assert!(methods.contains(&"Printer.fmt"), "got {methods:?}");
        assert!(
            !methods.iter().any(|m| m.starts_with("Display")),
            "owner must be the impl type, not the trait: {methods:?}"
        );
    }

    #[test]
    fn impl_for_reference_type_strips_the_borrow_prefix() {
        // `impl Trait for &Foo` ⇒ owner `Foo`, never `&Foo` (no phantom symbol).
        let fs = extract(
            b"
struct Foo;
impl std::fmt::Debug for &Foo {
    fn fmt(&self, _: &mut std::fmt::Formatter) -> std::fmt::Result { Ok(()) }
}
",
        );
        let methods = names_of(&fs, SymbolKind::Method);
        assert!(methods.contains(&"Foo.fmt"), "got {methods:?}");
        assert!(
            !methods.iter().any(|m| m.contains('&')),
            "borrow prefix must be stripped from the owner: {methods:?}"
        );
    }

    #[test]
    fn const_and_static_items_emit_no_symbol() {
        // Per the module scope, top-level const/static are not symbols. This
        // pins the invariant so a future change can't silently start emitting
        // them (they would land as the wrong `SymbolKind`).
        let fs = extract(b"const MAX: usize = 8;\nstatic NAME: &str = \"anvil\";\n");
        assert!(fs.symbols.is_empty(), "got {:?}", fs.symbols);
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

    // --- Pass 2: call-site extraction (GCALL-004) ---

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
        let fs = extract(b"fn helper() {}\nfn run() { helper(); }\n");
        let call = call_to(&fs, "helper");
        assert_eq!(call.from, fn_caller("run", 0));
        assert_eq!(call.callee.via_import, None);
        assert_eq!(call.line, 2);
    }

    #[test]
    fn use_bound_call_reverse_maps_alias_to_export_name_and_module() {
        let fs = extract(b"use crate::util::helper as h;\nfn run() { h(); }\n");
        let call = call_to(&fs, "helper");
        assert_eq!(
            call.callee,
            CalleeRef {
                name: "helper".to_string(),
                via_import: Some("crate::util".to_string()),
            }
        );
        assert_eq!(call.from, fn_caller("run", 0));
    }

    #[test]
    fn scoped_path_call_names_the_qualifier_as_module() {
        let fs = extract(b"fn run() { std::mem::swap(); }\n");
        let call = call_to(&fs, "swap");
        assert_eq!(call.callee.via_import, Some("std::mem".to_string()));
    }

    #[test]
    fn self_method_call_resolves_to_owner_method() {
        let fs = extract(b"struct S;\nimpl S { fn a(&self) {} fn b(&self) { self.a(); } }\n");
        let call = call_to(&fs, "S.a");
        assert_eq!(call.from, method_caller("S.b"));
        assert_eq!(call.callee.via_import, None);
    }

    #[test]
    fn self_assoc_fn_call_resolves_to_owner_method() {
        let fs =
            extract(b"struct S;\nimpl S { fn new() -> S { S } fn make() -> S { Self::new() } }\n");
        let call = call_to(&fs, "S.new");
        assert_eq!(call.from, method_caller("S.make"));
        assert_eq!(call.callee.via_import, None);
    }

    #[test]
    fn module_scope_call_attributes_to_synthetic_file_caller() {
        // A call outside any emitted symbol (here, a const initializer) has no
        // enclosing function span, so it binds to the module-scope placeholder.
        let fs = extract(b"fn compute() -> u32 { 0 }\nstatic X: u32 = compute();\n");
        let call = call_to(&fs, "compute");
        assert!(call.from.module_scope);
        assert_eq!(call.from.kind, SymbolKind::Module);
    }

    #[test]
    fn macro_invocation_is_not_a_call() {
        let fs = extract(b"fn run() { println!(\"hi\"); }\n");
        assert!(fs.calls.is_empty());
    }

    #[test]
    fn bare_method_on_arbitrary_receiver_names_the_method_only() {
        let fs = extract(b"fn run(x: Thing) { x.frob(); }\n");
        let call = call_to(&fs, "frob");
        assert_eq!(call.callee.via_import, None);
        assert_eq!(call.from, fn_caller("run", 0));
    }

    #[test]
    fn call_in_method_body_attributes_to_the_method() {
        // The method's span (inside the `impl`) is the innermost emitted symbol
        // containing the call, so the caller is `S.m`, not the file scope.
        let fs = extract(b"fn free() {}\nstruct S;\nimpl S { fn m(&self) { free(); } }\n");
        let call = call_to(&fs, "free");
        assert_eq!(call.from, method_caller("S.m"));
    }

    #[test]
    fn call_in_nested_fn_attributes_to_nearest_emitted_ancestor() {
        // Rust Pass 1 does not emit nested `function_item`s as symbols, so a call
        // in a nested fn binds to the nearest emitted enclosing symbol (`outer`) —
        // never a caller Pass 1 did not emit.
        let fs = extract(b"fn outer() { fn inner() { helper(); } }\nfn helper() {}\n");
        let call = call_to(&fs, "helper");
        assert_eq!(call.from, fn_caller("outer", 0));
    }

    #[test]
    fn call_extraction_is_deterministic() {
        let src = b"use a::b::c;\nfn run() { c(); helper(); self_free(); }\nfn helper() {}\nfn self_free() {}\n";
        assert_eq!(extract(src).calls, extract(src).calls);
    }
}
