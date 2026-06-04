//! Rule-specific, Rust-coded predicates (ADR-071 §4).
//!
//! A rule's `ast_query` (stored in the compiled registry, the single source of
//! truth) selects candidate nodes; the context the query language cannot
//! cleanly express is decided here. Each predicate operates on the `@target`
//! node a query captured plus the file's source bytes and path.

use tree_sitter::Node;

/// The four AST-detection rules this crate implements. Each maps 1:1 to a
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
        _ => None,
    }
}

/// Every rule id the predicate table knows about (for the completeness guard).
#[must_use]
pub fn known_rule_ids() -> &'static [&'static str] {
    &["RS-001", "RS-002", "RS-003", "RS-004"]
}

#[must_use]
pub(crate) fn node_text<'a>(node: Node, src: &'a [u8]) -> &'a str {
    node.utf8_text(src).unwrap_or("")
}

/// A Cargo integration-test / bench / example target — a separate crate with no
/// `#[cfg(test)]` ancestor, so the cfg walk alone can't exclude it (ADR-071 §4).
#[must_use]
pub(crate) fn path_is_test_target(path: &str) -> bool {
    path.replace('\\', "/")
        .split('/')
        .any(|seg| matches!(seg, "tests" | "benches" | "examples"))
}

/// True when `node` (or any ancestor) is gated by a `#[cfg(test)]`-style
/// attribute: `cfg(test)`, `cfg(all(test, …))`, `cfg(any(test, …))` exclude;
/// `cfg(not(test))` does not (ADR-071 §4 — a substring check is too broad and
/// too narrow, so the predicate parses the cfg tree and tracks negation depth).
#[must_use]
pub(crate) fn in_cfg_test(node: Node, src: &[u8]) -> bool {
    let mut cur = Some(node);
    while let Some(n) = cur {
        if preceding_attrs_have_test_cfg(n, src) {
            return true;
        }
        cur = n.parent();
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
            "identifier" if node_text(child, src) == "test" && !negated => return true,
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

/// RS-003 — true when the `unsafe` block already carries a `// SAFETY:` comment
/// on its immediately-preceding sibling line (AST-sibling semantics, not byte
/// proximity, so a blank line does not defeat it but an intervening statement
/// does). The predicate fires when this returns `false`.
#[must_use]
pub(crate) fn has_preceding_safety_comment(unsafe_block: Node, src: &[u8]) -> bool {
    let anchor = statement_anchor(unsafe_block);
    let Some(prev) = anchor.prev_named_sibling() else {
        return false;
    };
    if !matches!(prev.kind(), "line_comment" | "block_comment") {
        return false;
    }
    is_safety_comment(node_text(prev, src))
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
    // Strip the comment delimiters (`//`, `///`, `/*`) and leading whitespace,
    // then match `SAFETY` case-insensitively — mirrors ADR-071's
    // `(?i)^\s*//+\s*SAFETY`.
    let t = text
        .trim_start_matches('/')
        .trim_start_matches('*')
        .trim_start();
    t.len() >= "SAFETY".len() && t[..6.min(t.len())].eq_ignore_ascii_case("SAFETY")
}

/// RS-004 — true when `struct_item` derives `Deserialize` but no preceding
/// attribute supplies `#[serde(deny_unknown_fields)]`.
#[must_use]
pub(crate) fn struct_lacks_deny_unknown(struct_item: Node, src: &[u8]) -> bool {
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
