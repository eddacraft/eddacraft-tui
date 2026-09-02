//! The tier-1 prefix gate must never hide a rule that would have matched.
//!
//! `vendored_patterns_for` compiles and runs a tier-1 rule only when the line
//! carries that rule's literal provider prefix. That is sound because of two
//! structural properties of the vendored data, both asserted here so a ruleset
//! refresh cannot quietly invalidate the reasoning:
//!
//! 1. every rule's pattern contains its prefix as a literal, and
//! 2. no rule has a top-level alternation, so no branch can match without it.
//!
//! Property 3 — the one that actually matters — is checked directly: for real
//! content, any rule whose regex matches must be among those the gate returned.

use std::fs;
use std::path::Path;

use anvil_checks::secret::{
    VENDORED_RULESET, vendored_patterns_for, vendored_prefix_gate_is_active,
};
use regex::Regex;

/// `true` when `pattern` has a `|` outside every group, which would let a branch
/// match without the prefix.
///
/// Character classes are skipped: `(`, `)` and `|` are literals inside `[...]`,
/// and treating them as structure would mis-track the depth — a refresh could
/// then introduce a real top-level alternation while this check still passed,
/// silently weakening the gate's safety proof.
fn has_top_level_alternation(pattern: &str) -> bool {
    let mut depth = 0i32;
    let mut in_class = false;
    let mut chars = pattern.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                chars.next();
            }
            '[' if !in_class => in_class = true,
            ']' if in_class => in_class = false,
            '(' if !in_class => depth += 1,
            ')' if !in_class => depth -= 1,
            '|' if !in_class && depth == 0 => return true,
            _ => {}
        }
    }
    false
}

#[test]
fn top_level_alternation_scan_understands_groups_classes_and_escapes() {
    assert!(has_top_level_alternation("a|b"));
    assert!(has_top_level_alternation("[a]|b"));
    assert!(has_top_level_alternation("(a)(b)|c"));
    assert!(!has_top_level_alternation("(a|b)"));
    assert!(!has_top_level_alternation("x(?:a|b)y"));
    // `|` and parens are literals inside a class, so neither opens a group nor
    // counts as an alternation.
    assert!(!has_top_level_alternation("[(|)]x"));
    assert!(!has_top_level_alternation("[|]"));
    // An unbalanced-looking `)` inside a class must not push depth negative and
    // make a later real `|` look nested.
    assert!(has_top_level_alternation("[)]a|b"));
    // Escaped delimiters are literals too.
    assert!(!has_top_level_alternation("a\\|b"));
    // An escaped `[` does not open a class, so the `|` here really is top level.
    assert!(has_top_level_alternation("\\[a|b\\]"));
}

fn corpus_lines() -> Vec<String> {
    let mut lines: Vec<String> = vec![
        String::new(),
        "let x = 1;".to_owned(),
        "// ordinary source with no provider prefix".to_owned(),
        "url = \"https://example.com/path?q=1\"".to_owned(),
    ];
    // Every rule's own prefix, so the positive side of the gate is exercised —
    // and each in upper and lower case, because seven tier-1 rules are `(?i)`
    // and a case-sensitive gate would hide exactly those. Without the case
    // variants this check passes against a gate that drops the `(?i)` handling.
    for rule in &VENDORED_RULESET.rules {
        let body = "abcdefghijklmnopqrstuvwxyz0123456789";
        for prefix in [
            rule.prefix.clone(),
            rule.prefix.to_uppercase(),
            rule.prefix.to_lowercase(),
        ] {
            lines.push(format!("token = \"{prefix}{body}\""));
        }
    }
    let cases = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus/secret/cases");
    if let Ok(entries) = fs::read_dir(&cases) {
        for entry in entries.flatten() {
            if let Ok(text) = fs::read_to_string(entry.path()) {
                lines.extend(text.lines().map(str::to_owned));
            }
        }
    }
    lines
}

#[test]
fn the_prefix_gate_is_active() {
    // The fallback path is correct but compiles all 27 rules per process, which
    // is the cost this gate exists to remove. If it ever goes inactive the
    // budget regression returns silently.
    assert!(vendored_prefix_gate_is_active());
}

#[test]
fn every_rule_is_anchored_on_its_declared_prefix() {
    for rule in &VENDORED_RULESET.rules {
        assert!(
            !rule.prefix.is_empty(),
            "tier-1 rule `{}` has no prefix; the gate cannot reason about it",
            rule.id
        );
        assert!(
            rule.pattern.contains(&rule.prefix),
            "tier-1 rule `{}` does not contain its declared prefix {:?} literally, so gating on \
             that prefix could hide a real match",
            rule.id,
            rule.prefix
        );
        assert!(
            !has_top_level_alternation(&rule.pattern),
            "tier-1 rule `{}` has a top-level alternation, so a branch could match without the \
             prefix the gate filters on",
            rule.id
        );
    }
}

#[test]
fn the_gate_returns_every_rule_that_would_match() {
    let compiled: Vec<(String, Regex)> = VENDORED_RULESET
        .rules
        .iter()
        .map(|r| {
            (
                r.id.clone(),
                Regex::new(&r.pattern).expect("vendored rule compiles"),
            )
        })
        .collect();

    let lines = corpus_lines();
    assert!(
        lines.len() > 25,
        "corpus did not load; the check would be vacuous"
    );

    let mut matched_at_least_once = 0usize;
    for line in &lines {
        let gated: Vec<&str> = vendored_patterns_for(line)
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        for (id, regex) in &compiled {
            if regex.is_match(line) {
                matched_at_least_once += 1;
                assert!(
                    gated.contains(&id.as_str()),
                    "rule `{id}` matches line {line:?} but the prefix gate did not return it — \
                     gating dropped a real detection"
                );
            }
        }
    }
    assert!(
        matched_at_least_once > 0,
        "no rule matched any sample line; the check proved nothing"
    );
}

#[test]
fn the_gate_matches_case_insensitively_for_case_insensitive_rules() {
    // Checked directly on the gate rather than through a full regex match: a
    // synthetic line that carries the prefix rarely satisfies the rest of the
    // pattern (several tier-1 rules want hex or a fixed length), so routing this
    // through `is_match` would assert nothing and hide a case-sensitive gate.
    let mut checked = 0usize;
    for rule in &VENDORED_RULESET.rules {
        if !rule.pattern.contains("(?i)") {
            continue;
        }
        let upper = rule.prefix.to_uppercase();
        let lower = rule.prefix.to_lowercase();
        if upper == lower {
            continue; // no cased characters to vary
        }
        for variant in [upper, lower] {
            let line = format!("token = \"{variant}payload\"");
            let gated: Vec<&str> = vendored_patterns_for(&line)
                .iter()
                .map(|p| p.name.as_str())
                .collect();
            assert!(
                gated.contains(&rule.id.as_str()),
                "rule `{}` is case-insensitive but the gate missed its prefix as {variant:?}; \
                 a real credential in that case would go undetected",
                rule.id
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "no case-insensitive rule was exercised");
}
