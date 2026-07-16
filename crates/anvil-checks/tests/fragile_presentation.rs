//! CIB-198 / ADR-110: the `fragile-presentation` category and its first
//! rule, FRAG-001 — content authored invisible (`opacity: 0`) whose
//! visibility depends on an entrance animation firing.
//!
//! Exercises the family through the public scanner API: the rule
//! compiles cleanly under the RE2 engine (no silent drop), carries its
//! own `AntiPatternCategory` variant rather than the `code-quality`
//! fallback, is enabled by default, fires on the single-line motion
//! entrance idiom, and stays quiet on visible-by-default constructions,
//! partial opacity, exit fades, and non-web extensions.

use anvil_checks::antipattern::{
    AntiPatternCategory, get_default_patterns, get_pattern, registry_compile_diagnostics, scan_file,
};

fn fires(path: &str, content: &str, id: &str) -> bool {
    scan_file(path, content, None)
        .warnings
        .iter()
        .any(|w| w.id == id)
}

// --- RE2 compile (no silent drop) ------------------------------------------

#[test]
fn frag_rules_compile_under_re2() {
    // A lookahead/backreference in a pattern makes Rust's RE2 engine drop
    // the rule silently; assert none of the family's rules are in the
    // compile diagnostics so a future regex edit can't quietly disable
    // enforcement.
    let dropped: Vec<String> = registry_compile_diagnostics()
        .into_iter()
        .filter(|d| d.pattern_id.starts_with("FRAG-"))
        .map(|d| format!("{}: {}", d.pattern_id, d.error))
        .collect();
    assert!(
        dropped.is_empty(),
        "fragile-presentation rules must compile under RE2: {dropped:?}"
    );
}

// --- Category variant -------------------------------------------------------

#[test]
fn frag001_carries_the_fragile_presentation_category() {
    let p = get_pattern("FRAG-001").expect("FRAG-001 must exist in the registry");
    assert_eq!(
        p.category,
        AntiPatternCategory::FragilePresentation,
        "FRAG-001 should carry the fragile-presentation category, not the code-quality fallback"
    );
}

#[test]
fn frag001_is_default_on() {
    let defaults = get_default_patterns();
    assert!(
        defaults.iter().any(|p| p.id == "FRAG-001"),
        "FRAG-001 must be enabled by default (opt_in: false)"
    );
}

// --- FRAG-001: invisible initial gated on an entrance animation -------------

#[test]
fn frag001_invisible_initial_fires() {
    assert!(fires(
        "src/components/Hero.tsx",
        "<motion.section initial={{ opacity: 0, y: 20 }} whileInView={{ opacity: 1, y: 0 }}>\n",
        "FRAG-001"
    ));
}

#[test]
fn frag001_compact_solo_opacity_fires() {
    assert!(fires(
        "src/App.jsx",
        "<motion.div initial={{opacity:0}} animate={{opacity:1}}>\n",
        "FRAG-001"
    ));
}

#[test]
fn frag001_spaced_assignment_fires() {
    // JSX permits whitespace around `=` and before the colon; the trap is
    // the same construction.
    assert!(fires(
        "src/components/Hero.tsx",
        "<motion.section initial = {{ opacity : 0 }} animate={{ opacity: 1 }}>\n",
        "FRAG-001"
    ));
}

#[test]
fn frag001_visible_initial_stays_quiet() {
    assert!(!fires(
        "src/components/Hero.tsx",
        "<motion.section initial={{ opacity: 1, y: 20 }} whileInView={{ y: 0 }}>\n",
        "FRAG-001"
    ));
    assert!(!fires(
        "src/components/Hero.tsx",
        "<motion.section initial={{ y: 20 }} whileInView={{ y: 0 }}>\n",
        "FRAG-001"
    ));
}

#[test]
fn frag001_partial_opacity_stays_quiet() {
    assert!(!fires(
        "src/components/Hero.tsx",
        "<motion.div initial={{ opacity: 0.4 }} animate={{ opacity: 1 }}>\n",
        "FRAG-001"
    ));
}

#[test]
fn frag001_exit_fade_stays_quiet() {
    // Fading out on unmount is not the trap — the content was visible.
    assert!(!fires(
        "src/components/Modal.tsx",
        "<motion.div exit={{ opacity: 0 }}>\n",
        "FRAG-001"
    ));
}

#[test]
fn frag001_is_web_extension_scoped() {
    // The idiom is a JSX prop; the same bytes in a Python string must not
    // fire.
    assert!(!fires(
        "scripts/render.py",
        "template = '<motion.div initial={{ opacity: 0 }}>'\n",
        "FRAG-001"
    ));
}
