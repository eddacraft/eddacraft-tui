//! Rule-specific, Rust-coded predicates (ADR-071 §4).
//!
//! A rule's `ast_query` (stored in the compiled registry, the single source of
//! truth) selects candidate nodes; the context the query language cannot
//! cleanly express is decided here. Each predicate operates on the `@target`
//! node a query captured plus the file's source bytes and path.

use tree_sitter::Node;

/// The AST-detection rules this crate implements (RS-001..RS-005). Each maps 1:1 to a
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
        _ => None,
    }
}

/// Every rule id the predicate table knows about (for the completeness guard).
#[must_use]
pub fn known_rule_ids() -> &'static [&'static str] {
    &["RS-001", "RS-002", "RS-003", "RS-004", "RS-005"]
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
    norm.split('/')
        .any(|seg| matches!(seg, "tests" | "benches" | "examples"))
        || matches!(
            norm.rsplit('/').next(),
            Some("tests.rs" | "test.rs" | "bench.rs" | "build.rs")
        )
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
    let has_deny = attrs.iter().any(|a| attr_serde_deny_unknown(*a, src));
    !has_deny
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
