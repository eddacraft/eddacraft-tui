//! Rule-specific, Rust-coded predicates (ADR-071 §4).
//!
//! A rule's `ast_query` (stored in the compiled registry, the single source of
//! truth) selects candidate nodes; the context the query language cannot
//! cleanly express is decided here. Each predicate operates on the `@target`
//! node a query captured plus the file's source bytes and path.

use tree_sitter::Node;

/// The AST-detection rules this crate implements (RS-001..RS-008). Each maps 1:1 to a
/// `Detection::Ast` rule id in the compiled registry; [`kind_for`] is the
/// predicate table ADR-071 §3 requires every registry AST rule to have an entry
/// in (enforced by the registry-completeness guard test).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AstRuleKind {
    /// RS-001 — `.unwrap()` / `.expect()` outside test code.
    UnwrapExpect,
    /// RS-002 — `panic!()` reached from non-test code.
    Panic,
    /// RS-003 — `unsafe { … }` without a preceding `// SAFETY:` comment.
    UnsafeNoSafety,
    /// RS-004 — `Deserialize` struct missing `#[serde(deny_unknown_fields)]`.
    SerdeDenyUnknown,
    /// RS-005 — `todo!()` / `unimplemented!()` reached from non-test code.
    TodoMacro,
    /// RS-006 — catch-all `#[serde(flatten)]` field without a validation boundary.
    SerdeFlattenUnvalidated,
    /// RS-007 — `Deserialize` plaintext field carrying a high-confidence secret.
    SecretDeserialize,
    /// RS-008 — `.clone()` inside a syntactic loop.
    CloneInLoop,
    /// PY-010 — named `except` handler whose block body is only `pass`.
    ExceptBlockPass,
}

/// Predicate table (ADR-071 §3/§4): rule id → (kind, expected `ast_query`).
///
/// The second tuple element is a drift snapshot of the registry query for that
/// id; [`crate`]'s snapshot test fails if the compiled registry diverges from
/// it, so a grammar/query change can't silently desync the predicate from the
/// nodes it assumes.
#[must_use]
pub fn kind_for(id: &str) -> Option<(AstRuleKind, &'static str)> {
    match id {
        "RS-001" => Some((
            AstRuleKind::UnwrapExpect,
            "(call_expression function: (field_expression field: (field_identifier) @method)) @target",
        )),
        "RS-002" => Some((
            AstRuleKind::Panic,
            "(macro_invocation macro: (identifier) @name) @target",
        )),
        "RS-003" => Some((AstRuleKind::UnsafeNoSafety, "(unsafe_block) @target")),
        "RS-004" => Some((AstRuleKind::SerdeDenyUnknown, "(struct_item) @target")),
        "RS-005" => Some((
            AstRuleKind::TodoMacro,
            "(macro_invocation macro: (identifier) @name) @target",
        )),
        "RS-006" => Some((
            AstRuleKind::SerdeFlattenUnvalidated,
            "(attribute_item (attribute (identifier) @attr arguments: (token_tree) @serde_args)) @target",
        )),
        "RS-007" => Some((
            AstRuleKind::SecretDeserialize,
            "(field_declaration name: (field_identifier) @field) @target",
        )),
        "RS-008" => Some((
            AstRuleKind::CloneInLoop,
            "(call_expression function: (field_expression field: (field_identifier) @method)) @target",
        )),
        "PY-010" => Some((AstRuleKind::ExceptBlockPass, "(except_clause) @target")),
        _ => None,
    }
}

/// Every rule id the predicate table knows about (for the completeness guard).
#[must_use]
pub fn known_rule_ids() -> &'static [&'static str] {
    &[
        "RS-001", "RS-002", "RS-003", "RS-004", "RS-005", "RS-006", "RS-007", "RS-008", "PY-010",
    ]
}

/// True when `node` is a Python `except_clause` that names an exception type
/// and whose **indented** suite contains only `pass` (the PY-004 regex-blind
/// shape).
///
/// Bare `except:` has no type child and is left to regex PY-004. Inline
/// `except Exception: pass` is the same line as the colon, so tree-sitter
/// still wraps it as a `block` — require `pass` on a later row so PY-010
/// does not duplicate PY-004. A handler that logs, re-raises, or does more
/// than `pass` is clean.
#[must_use]
pub(crate) fn except_block_is_only_pass(node: Node<'_>) -> bool {
    if node.kind() != "except_clause" {
        return false;
    }
    let mut saw_type = false;
    let mut block: Option<Node<'_>> = None;
    let mut walk = node.walk();
    for child in node.children(&mut walk) {
        match child.kind() {
            "block" => block = Some(child),
            // `*` is `except*` (PEP 654); it is not an exception type.
            "except" | ":" | "as" | "comment" | "*" => {}
            _ => saw_type = true,
        }
    }
    if !saw_type {
        return false;
    }
    let Some(block) = block else {
        return false;
    };
    let mut named = block.walk();
    let mut pass: Option<Node<'_>> = None;
    let mut extra = false;
    for child in block.named_children(&mut named) {
        if child.kind() == "pass_statement" && pass.is_none() {
            pass = Some(child);
        } else {
            extra = true;
        }
    }
    if extra {
        return false;
    }
    let Some(pass) = pass else {
        return false;
    };
    pass.start_position().row > node.start_position().row
}

#[must_use]
pub(crate) fn node_text<'a>(node: Node, src: &'a [u8]) -> &'a str {
    node.utf8_text(src).unwrap_or("")
}

/// Paths the unwrap/expect/panic rules treat as not shipped non-test runtime —
/// where a panic is either a test failure or an idiomatic build-time abort, so
/// flagging it is noise (RSTLAN-008 dogfood finding):
///
/// - Cargo integration-test / bench / example targets (`tests/`, `benches/`,
///   `examples/`) — separate crates with no `#[cfg(test)]` ancestor the cfg walk
///   could see.
/// - Separate test / bench module files included via `#[cfg(test)] mod tests;`
///   (`tests.rs` / `test.rs` / `bench.rs`) — the file itself carries no in-file
///   `cfg(test)` marker, so the cfg walk can't reach it.
/// - Build scripts (`build.rs`) — panicking / `unwrap()` is the idiomatic
///   build-time error path and the script is not shipped runtime code.
#[must_use]
pub(crate) fn path_is_test_target(path: &str) -> bool {
    let norm = path.replace('\\', "/");
    let file = norm.rsplit('/').next().unwrap_or(&norm);
    let is_py = std::path::Path::new(file)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("py"));
    let stem = std::path::Path::new(file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    norm.split('/')
        .any(|seg| matches!(seg, "tests" | "benches" | "examples"))
        || matches!(
            file,
            "tests.rs" | "test.rs" | "bench.rs" | "build.rs" | "conftest.py"
        )
        || (is_py && (stem.starts_with("test_") || stem.ends_with("_test")))
}

/// True when `node` (or any ancestor) is gated by a `#[cfg(test)]`-style
/// non-shipped attribute: `cfg(test)`, `cfg(doc)`, `cfg(docsrs)`,
/// `cfg(all(test, …))`, and `cfg(any(doc, …))` exclude; `cfg(not(test))` and
/// `cfg(not(doc))` do not (ADR-071 §4 / CIB-081 — a substring check is too broad
/// and too narrow, so the predicate parses the cfg tree and tracks negation
/// depth).
#[must_use]
pub(crate) fn in_cfg_test(node: Node, src: &[u8]) -> bool {
    let mut cur = Some(node);
    while let Some(n) = cur {
        // Outer attributes (`#[cfg(test)] mod tests { … }`) are preceding
        // siblings of the gated item.
        if preceding_attrs_have_test_cfg(n, src) {
            return true;
        }
        // Inner attributes (`mod tests { #![cfg(test)] … }`) are leading
        // children of the gated scope's body.
        if has_inner_test_cfg(n, src) {
            return true;
        }
        cur = n.parent();
    }
    false
}

/// Check a scope body for a leading `#![cfg(test)]` inner attribute. Inner
/// attributes must precede all items, so the scan stops at the first
/// non-attribute, non-comment child.
fn has_inner_test_cfg(node: Node, src: &[u8]) -> bool {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            // `inner_attribute_item` carries the same `(attribute …)` child as
            // the outer `attribute_item`, so the cfg parser is shared.
            "inner_attribute_item" => {
                if attr_item_is_test_cfg(child, src) {
                    return true;
                }
            }
            "line_comment" | "block_comment" => {}
            _ => break,
        }
    }
    false
}

fn preceding_attrs_have_test_cfg(node: Node, src: &[u8]) -> bool {
    let mut sib = node.prev_named_sibling();
    while let Some(s) = sib {
        match s.kind() {
            "attribute_item" => {
                if attr_item_is_test_cfg(s, src) {
                    return true;
                }
            }
            // Comments are `extras` that can interleave the attribute run.
            "line_comment" | "block_comment" => {}
            // First non-attribute, non-comment sibling ends the decorating run.
            _ => break,
        }
        sib = s.prev_named_sibling();
    }
    false
}

fn attr_item_is_test_cfg(attr_item: Node, src: &[u8]) -> bool {
    let Some(attr) = attr_item.named_child(0) else {
        return false;
    };
    if attr.kind() != "attribute" {
        return false;
    }
    let Some(name) = attr.named_child(0) else {
        return false;
    };
    if node_text(name, src) != "cfg" {
        return false;
    }
    let Some(args) = attr.child_by_field_name("arguments") else {
        return false;
    };
    token_tree_has_unnegated_test(args, src, false)
}

fn token_tree_has_unnegated_test(tt: Node, src: &[u8], negated: bool) -> bool {
    let mut cursor = tt.walk();
    for child in tt.named_children(&mut cursor) {
        match child.kind() {
            "identifier" if is_non_shipped_cfg_identifier(node_text(child, src)) && !negated => {
                return true;
            }
            "token_tree" => {
                let neg = match child.prev_named_sibling() {
                    Some(p) if p.kind() == "identifier" && node_text(p, src) == "not" => !negated,
                    _ => negated,
                };
                if token_tree_has_unnegated_test(child, src, neg) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn is_non_shipped_cfg_identifier(ident: &str) -> bool {
    matches!(ident, "test" | "doc" | "docsrs")
}

/// RS-003 — true when the `unsafe` block already carries a `// SAFETY:` comment.
///
/// Credited in three positions (external-FP dogfood widened this from the
/// original single-sibling check, which tokio/alacritty's idioms defeated):
/// - anywhere in the **contiguous run** of comment siblings immediately
///   preceding the block at any level up to its enclosing statement — so a
///   multi-line `// SAFETY:` block whose keyword is on the first line, with
///   continuation lines below, still counts (AST-sibling semantics, not byte
///   proximity, so a blank line does not defeat it but an intervening statement
///   does);
/// - inside a `match` arm (the comment precedes the `match_arm`, not the whole
///   `match`), covered by walking the chain rather than just the anchor;
/// - as the **first statement inside** the block (`unsafe { // SAFETY: … }`).
///
/// The predicate fires when this returns `false`.
#[must_use]
pub(crate) fn has_preceding_safety_comment(unsafe_block: Node, src: &[u8]) -> bool {
    if block_opens_with_safety_comment(unsafe_block, src) {
        return true;
    }
    let anchor = statement_anchor(unsafe_block);
    let mut cur = unsafe_block;
    loop {
        // Scan the whole contiguous run of preceding comment siblings — a
        // multi-line `// SAFETY:` rationale is several `line_comment` nodes, and
        // the keyword may sit on any of them, not just the line above.
        let mut sib = cur.prev_named_sibling();
        while let Some(prev) = sib {
            if !matches!(prev.kind(), "line_comment" | "block_comment") {
                break;
            }
            if is_safety_comment(node_text(prev, src)) {
                return true;
            }
            sib = prev.prev_named_sibling();
        }
        if cur.id() == anchor.id() {
            return false;
        }
        match cur.parent() {
            Some(p) => cur = p,
            None => return false,
        }
    }
}

/// True when a `// SAFETY:` comment is the leading content inside the block of
/// an `unsafe_block` (`unsafe { // SAFETY: … }`). The scan stops at the first
/// non-comment child so only a genuinely-leading safety rationale counts.
fn block_opens_with_safety_comment(unsafe_block: Node, src: &[u8]) -> bool {
    let mut top = unsafe_block.walk();
    let Some(block) = unsafe_block
        .named_children(&mut top)
        .find(|n| n.kind() == "block")
    else {
        return false;
    };
    let mut cursor = block.walk();
    for child in block.named_children(&mut cursor) {
        match child.kind() {
            "line_comment" | "block_comment" => {
                if is_safety_comment(node_text(child, src)) {
                    return true;
                }
            }
            _ => break,
        }
    }
    false
}

/// Climb to the node that sits directly in a statement list (`block`,
/// `declaration_list`, or `source_file`), so a `// SAFETY:` comment that
/// precedes the whole statement is found whether the `unsafe` block is a bare
/// statement (`unsafe { … };`) or a sub-expression (`let x = unsafe { … };`).
fn statement_anchor(node: Node) -> Node {
    let mut cur = node;
    while let Some(p) = cur.parent() {
        if matches!(p.kind(), "block" | "declaration_list" | "source_file") {
            return cur;
        }
        cur = p;
    }
    cur
}

fn is_safety_comment(text: &str) -> bool {
    // Strip the comment delimiters (`//`, `///`, `/*`, `/*!`) and leading
    // whitespace, then require the literal word `SAFETY` (ASCII,
    // case-insensitive) — mirrors ADR-071's `(?i)^\s*//+\s*SAFETY`. Compared at
    // the byte level so a multi-byte char straddling index 6 cannot panic, and
    // the following byte must be a non-alphanumeric word boundary so `SAFETYFOO`
    // does not count.
    let t = text
        .trim_start_matches('/')
        .trim_start_matches(['*', '!'])
        .trim_start();
    let bytes = t.as_bytes();
    bytes
        .get(..6)
        .is_some_and(|b| b.eq_ignore_ascii_case(b"SAFETY"))
        && bytes.get(6).is_none_or(|c| !c.is_ascii_alphanumeric())
}

/// RS-004 — true when a named-field `struct_item` derives `Deserialize` but no
/// preceding attribute supplies `#[serde(deny_unknown_fields)]`.
///
/// Tuple structs and unit structs are skipped: `deny_unknown_fields` is a no-op
/// on them (serde only applies it to named fields), so flagging them would give
/// misleading advice (council adversarial MINOR).
#[must_use]
pub(crate) fn struct_lacks_deny_unknown(struct_item: Node, src: &[u8]) -> bool {
    if struct_item
        .child_by_field_name("body")
        .is_none_or(|body| body.kind() != "field_declaration_list")
    {
        return false;
    }
    let attrs = preceding_attribute_items(struct_item);
    let derives_deserialize = attrs.iter().any(|a| attr_derive_contains(*a, src));
    if !derives_deserialize {
        return false;
    }
    if struct_has_flatten_field(struct_item, src) {
        return false;
    }
    let has_deny = attrs.iter().any(|a| attr_serde_deny_unknown(*a, src));
    !has_deny
}

/// RS-006 — true when a `#[serde(flatten)]` attribute decorates a catch-all map
/// field on a `Deserialize` struct, with no detectable validation boundary.
///
/// Deliberately opt-in: typed flatten composition (`common: CommonConfig`) is
/// clean, and validation that is not mechanically visible should be expressed
/// with a local `@anvil-ignore` reason rather than guessed.
#[must_use]
pub(crate) fn serde_flatten_without_validation(attr_item: Node, src: &[u8]) -> bool {
    if !attr_item_is_serde_with_identifier(attr_item, src, "flatten") {
        return false;
    }
    let Some(field) = decorated_field_for_attr(attr_item) else {
        return false;
    };
    if !field_type_is_catch_all_map(field, src) {
        return false;
    }
    if field_attrs_have_serde_identifier(field, src, "deserialize_with") {
        return false;
    }
    let Some(struct_item) = ancestor_kind(field, "struct_item") else {
        return false;
    };
    let attrs = preceding_attribute_items(struct_item);
    attrs.iter().any(|a| attr_derive_contains(*a, src))
        && !attrs
            .iter()
            .any(|a| attr_item_is_serde_with_identifier(*a, src, "try_from"))
}

/// RS-007 — true when a named field on a `Deserialize` struct deserialises a
/// high-confidence secret into a plaintext-ish type. The rule intentionally
/// ignores manual `Deserialize` implementations; CIB-079 scopes this first wave
/// to derive-backed structs visible through the AST rule table.
#[must_use]
pub(crate) fn secret_deserialize_field(field: Node, src: &[u8]) -> bool {
    let Some(struct_item) = ancestor_kind(field, "struct_item") else {
        return false;
    };
    if !preceding_attribute_items(struct_item)
        .iter()
        .any(|a| attr_derive_contains(*a, src))
    {
        return false;
    }
    if field_attrs_have_serde_identifier(field, src, "skip_deserializing") {
        return false;
    }
    if field_attrs_have_serde_identifier(field, src, "deserialize_with") {
        return false;
    }
    let Some(type_node) = field.child_by_field_name("type") else {
        return false;
    };
    let type_text = node_text(type_node, src);
    if type_is_secret_wrapper(type_text) || !type_is_plaintextish(type_text) {
        return false;
    }
    let Some(name_node) = field.child_by_field_name("name") else {
        return false;
    };
    let field_name = node_text(name_node, src);
    is_high_confidence_secret_name(field_name) || field_attrs_have_secret_rename(field, src)
}

/// RS-008 — syntactic loop only. Iterator-adapter closures and UFCS
/// `Clone::clone` are out of scope until the rule has type/cost context.
#[must_use]
pub(crate) fn inside_syntactic_loop(node: Node) -> bool {
    let mut cur = Some(node);
    while let Some(n) = cur {
        if matches!(
            n.kind(),
            "for_expression" | "while_expression" | "loop_expression"
        ) {
            return true;
        }
        cur = n.parent();
    }
    false
}

fn preceding_attribute_items(node: Node) -> Vec<Node> {
    let mut out = Vec::new();
    let mut sib = node.prev_named_sibling();
    while let Some(s) = sib {
        match s.kind() {
            "attribute_item" => out.push(s),
            "line_comment" | "block_comment" => {}
            _ => break,
        }
        sib = s.prev_named_sibling();
    }
    out
}

fn attr_with_name<'a>(attr_item: Node<'a>, src: &[u8], want: &str) -> Option<Node<'a>> {
    let attr = attr_item.named_child(0)?;
    if attr.kind() != "attribute" {
        return None;
    }
    let name = attr.named_child(0)?;
    if node_text(name, src) != want {
        return None;
    }
    attr.child_by_field_name("arguments")
}

fn attr_derive_contains(attr_item: Node, src: &[u8]) -> bool {
    attr_with_name(attr_item, src, "derive")
        .is_some_and(|args| subtree_has_identifier(args, src, "Deserialize"))
}

fn attr_serde_deny_unknown(attr_item: Node, src: &[u8]) -> bool {
    attr_with_name(attr_item, src, "serde")
        .is_some_and(|args| subtree_has_identifier(args, src, "deny_unknown_fields"))
}

fn attr_item_is_serde_with_identifier(attr_item: Node, src: &[u8], ident: &str) -> bool {
    attr_with_name(attr_item, src, "serde")
        .is_some_and(|args| subtree_has_identifier(args, src, ident))
}

fn decorated_field_for_attr(attr_item: Node) -> Option<Node> {
    if matches!(
        attr_item.parent().map(|p| p.kind()),
        Some("field_declaration")
    ) {
        return attr_item.parent();
    }
    let mut sib = attr_item.next_named_sibling();
    while let Some(s) = sib {
        match s.kind() {
            "attribute_item" | "line_comment" | "block_comment" => sib = s.next_named_sibling(),
            "field_declaration" => return Some(s),
            _ => return None,
        }
    }
    None
}

fn ancestor_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cur = Some(node);
    while let Some(n) = cur {
        if n.kind() == kind {
            return Some(n);
        }
        cur = n.parent();
    }
    None
}

fn field_attrs_have_serde_identifier(field: Node, src: &[u8], ident: &str) -> bool {
    field_attribute_items(field)
        .iter()
        .any(|a| attr_item_is_serde_with_identifier(*a, src, ident))
}

fn field_attribute_items(field: Node) -> Vec<Node> {
    let mut out = Vec::new();
    let mut cursor = field.walk();
    for child in field.named_children(&mut cursor) {
        if child.kind() == "attribute_item" {
            out.push(child);
        }
    }
    if out.is_empty() {
        let mut sib = field.prev_named_sibling();
        while let Some(s) = sib {
            match s.kind() {
                "attribute_item" => out.push(s),
                "line_comment" | "block_comment" => {}
                _ => break,
            }
            sib = s.prev_named_sibling();
        }
    }
    out
}

fn field_type_is_catch_all_map(field: Node, src: &[u8]) -> bool {
    let Some(type_node) = field.child_by_field_name("type") else {
        return false;
    };
    let text = node_text(type_node, src);
    let lower = text.to_ascii_lowercase();
    (text.contains("HashMap") || text.contains("BTreeMap") || text.contains("IndexMap"))
        && (lower.contains("serde_json::value")
            || lower.contains("serde_json :: value")
            || text.ends_with("Value>")
            || text.contains("Value,")
            || text.contains("Value >"))
}

fn struct_has_flatten_field(struct_item: Node, src: &[u8]) -> bool {
    let mut cursor = struct_item.walk();
    struct_item
        .named_children(&mut cursor)
        .filter(|n| n.kind() == "field_declaration_list")
        .flat_map(|list| {
            let mut list_cursor = list.walk();
            list.named_children(&mut list_cursor).collect::<Vec<_>>()
        })
        .any(|field| {
            field.kind() == "field_declaration"
                && field_attrs_have_serde_identifier(field, src, "flatten")
        })
}

fn type_is_secret_wrapper(type_text: &str) -> bool {
    [
        "SecretString",
        "SecretVec",
        "Secret<",
        "Sensitive<",
        "Redacted<",
        "SecretBox<",
        "Masked<",
    ]
    .iter()
    .any(|needle| type_text.contains(needle))
}

fn type_is_plaintextish(type_text: &str) -> bool {
    let compact: String = type_text.chars().filter(|c| !c.is_whitespace()).collect();
    compact == "String"
        || compact == "std::string::String"
        || compact == "Vec<u8>"
        || compact == "std::vec::Vec<u8>"
        || compact == "Bytes"
        || compact == "serde_json::Value"
        || compact == "Value"
        || compact.contains("Option<String>")
        || compact.contains("Option<std::string::String>")
}

fn is_high_confidence_secret_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if lower.contains("public_key") || lower.contains("token_count") || lower.contains("key_path") {
        return false;
    }
    matches!(
        lower.as_str(),
        "password"
            | "passwd"
            | "passphrase"
            | "private_key"
            | "client_secret"
            | "access_token"
            | "refresh_token"
            | "auth_token"
            | "token"
            | "api_key"
            | "api_token"
    ) || lower.ends_with("_password")
        || lower.ends_with("_private_key")
        || lower.ends_with("_client_secret")
        || lower.ends_with("_access_token")
        || lower.ends_with("_refresh_token")
        || lower.ends_with("_auth_token")
        || lower.ends_with("_api_key")
        || lower.ends_with("_api_token")
}

fn field_attrs_have_secret_rename(field: Node, src: &[u8]) -> bool {
    field_attribute_items(field).iter().any(|a| {
        attr_with_name(*a, src, "serde").is_some_and(|args| {
            let text = node_text(args, src).to_ascii_lowercase();
            [
                "client_secret",
                "access_token",
                "refresh_token",
                "api_key",
                "api_token",
                "password",
            ]
            .iter()
            .any(|secret| text.contains(secret))
        })
    })
}

/// Recursively scan a node's subtree for an `identifier` whose text equals
/// `ident`. Robust to `derive(serde::Deserialize)` (the `Deserialize`
/// identifier appears as a token regardless of path qualification).
fn subtree_has_identifier(node: Node, src: &[u8], ident: &str) -> bool {
    if node.kind() == "identifier" && node_text(node, src) == ident {
        return true;
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| subtree_has_identifier(child, src, ident))
}
