//! INSEC-001..004: the `insecure-construction` security anti-pattern category
//! and its first-wave families.
//!
//! Exercises the `weak-cryptography` (WC-*) and `unsafe-rendering` (UR-*)
//! families plus the SSTI rule folded into `dynamic-execution` (AP-017)
//! through the public scanner API: each rule fires on a realistic positive
//! fixture and stays quiet on the justified/negative form, the rules compile
//! cleanly under the RE2 engine (no silent drop), the new category maps to its
//! own `AntiPatternCategory` variant rather than the `code-quality` fallback,
//! the enabled families are default-on, and `unsafe-rendering` is web-extension
//! scoped.

use anvil_checks::antipattern::{
    AntiPatternCategory, ScanOptions, WarningSeverity, get_default_patterns, get_pattern,
    registry_compile_diagnostics, scan_file,
};

fn fires(path: &str, content: &str, id: &str) -> bool {
    scan_file(path, content, None)
        .warnings
        .iter()
        .any(|w| w.id == id)
}

// --- RE2 compile (no silent drop) ------------------------------------------

#[test]
fn insec_rules_compile_under_re2() {
    // A lookahead/backreference in a pattern makes Rust's RE2 engine drop the
    // rule silently; assert none of the new rules are in the compile
    // diagnostics so a future regex edit can't quietly disable enforcement.
    let dropped: Vec<String> = registry_compile_diagnostics()
        .into_iter()
        .filter(|d| {
            d.pattern_id.starts_with("WC-")
                || d.pattern_id.starts_with("UR-")
                || d.pattern_id == "AP-017"
        })
        .map(|d| format!("{}: {}", d.pattern_id, d.error))
        .collect();
    assert!(
        dropped.is_empty(),
        "insecure-construction rules must compile under RE2: {dropped:?}"
    );
}

// --- INSEC-001: category variant -------------------------------------------

#[test]
fn insec_families_carry_the_new_category_variant() {
    // The whole point of INSEC-001: these rules resolve to their own
    // first-class category instead of falling back to `code-quality`.
    for id in ["WC-001", "WC-002", "WC-003", "UR-001", "UR-002", "UR-003"] {
        let p = get_pattern(id).unwrap_or_else(|| panic!("{id} must exist in the registry"));
        assert_eq!(
            p.category,
            AntiPatternCategory::InsecureConstruction,
            "{id} should carry the insecure-construction category"
        );
    }
}

#[test]
fn insec_enabled_families_are_default_on() {
    let defaults = get_default_patterns();
    for id in [
        "WC-001", "WC-002", "WC-003", "UR-001", "UR-002", "UR-003", "AP-017",
    ] {
        assert!(
            defaults.iter().any(|p| p.id == id),
            "{id} must be enabled by default (opt_in: false)"
        );
    }
}

// --- INSEC-002: weak-cryptography ------------------------------------------

#[test]
fn wc001_deprecated_hash_fires() {
    assert!(fires(
        "src/token.ts",
        "const t = createHash('md5').update(secret).digest('hex');\n",
        "WC-001"
    ));
    assert!(fires(
        "src/sig.py",
        "import hashlib\ndigest = hashlib.sha1(payload).hexdigest()\n",
        "WC-001"
    ));
}

#[test]
fn wc001_modern_hash_is_clean() {
    assert!(!fires(
        "src/token.ts",
        "const t = createHash('sha256').update(secret).digest('hex');\n",
        "WC-001"
    ));
}

#[test]
fn wc002_broken_cipher_and_ecb_fire() {
    assert!(fires(
        "src/enc.ts",
        "const c = crypto.createCipheriv('des-ecb', key, null);\n",
        "WC-002"
    ));
    assert!(fires(
        "src/enc.ts",
        "const c = crypto.createCipheriv('aes-256-ecb', key, null);\n",
        "WC-002"
    ));
    assert!(fires(
        "src/Enc.java",
        "Cipher c = Cipher.getInstance(\"DES\");\n",
        "WC-002"
    ));
}

#[test]
fn wc002_authenticated_mode_is_clean() {
    assert!(!fires(
        "src/enc.ts",
        "const c = crypto.createCipheriv('aes-256-gcm', key, iv);\n",
        "WC-002"
    ));
}

#[test]
fn wc003_jwt_alg_none_fires() {
    assert!(fires(
        "src/auth.ts",
        "const decoded = jwt.verify(raw, key, { algorithms: ['none'] });\n",
        "WC-003"
    ));
    assert!(fires(
        "src/auth.ts",
        "const header = { alg: 'none', typ: 'JWT' };\n",
        "WC-003"
    ));
}

#[test]
fn wc003_pinned_algorithm_is_clean() {
    assert!(!fires(
        "src/auth.ts",
        "const decoded = jwt.verify(raw, key, { algorithms: ['RS256'] });\n",
        "WC-003"
    ));
}

// --- INSEC-003: unsafe-rendering -------------------------------------------

#[test]
fn ur001_inner_html_assignment_fires() {
    // Identifier right-hand side.
    assert!(fires(
        "src/view.ts",
        "el.innerHTML = userProvidedMarkup;\n",
        "UR-001"
    ));
    // Template-literal right-hand side (the ergonomic dynamic form) still fires.
    assert!(fires(
        "src/view.ts",
        "row.innerHTML = `<td>${cell}</td>`;\n",
        "UR-001"
    ));
    // A call returning HTML fires.
    assert!(fires(
        "src/view.ts",
        "content.innerHTML = renderMarkup(node);\n",
        "UR-001"
    ));
}

#[test]
fn ur001_text_content_and_comparison_are_clean() {
    assert!(!fires(
        "src/view.ts",
        "el.textContent = userProvidedMarkup;\n",
        "UR-001"
    ));
    // Equality comparison must not be read as an assignment.
    assert!(!fires(
        "src/view.ts",
        "if (el.innerHTML === '') resetView();\n",
        "UR-001"
    ));
}

#[test]
fn ur001_quoted_literal_rhs_is_clean() {
    // INSEC-006 precision fix: like the `eval` rule, a single/double-quoted
    // literal right-hand side is skipped — the dominant real-world benign
    // form is `innerHTML = ''` to clear a node, plus static snippets.
    assert!(!fires(
        "src/view.ts",
        "container.innerHTML = '';\n",
        "UR-001"
    ));
    assert!(!fires("src/view.ts", "el.innerHTML = \"\";\n", "UR-001"));
    assert!(!fires("src/view.ts", "el.innerHTML = '<hr>';\n", "UR-001"));
}

#[test]
fn ur001_allowlists_tsx_test_files() {
    // INSEC-006 precision fix: React test files use `.test.tsx`; the DOM
    // teardown idiom `document.body.innerHTML = value` in a test must not fire.
    assert!(!fires(
        "src/App.test.tsx",
        "afterEach(() => { document.body.innerHTML = cleanup; });\n",
        "UR-001"
    ));
}

#[test]
fn ur002_document_write_fires() {
    assert!(fires(
        "src/boot.ts",
        "document.write(location.hash);\n",
        "UR-002"
    ));
}

#[test]
fn ur003_dangerously_set_inner_html_fires() {
    assert!(fires(
        "src/Comment.tsx",
        "return <div dangerouslySetInnerHTML={{ __html: comment.body }} />;\n",
        "UR-003"
    ));
}

#[test]
fn ur001_is_web_extension_scoped() {
    // The DOM-XSS sinks are meaningless outside a browser context, so the
    // family is scoped to web extensions — an `.innerHTML =` inside a Python
    // file (e.g. a templated string) must not fire.
    assert!(!fires(
        "src/render.py",
        "page.innerHTML = user_input\n",
        "UR-001"
    ));
}

// --- INSEC-004: SSTI via dynamic-execution ---------------------------------

#[test]
fn ap017_dynamic_template_string_fires() {
    assert!(fires(
        "src/app.py",
        "return render_template_string(request.args.get('tpl'))\n",
        "AP-017"
    ));
    assert!(fires(
        "src/app.py",
        "html = env.from_string(user_template).render()\n",
        "AP-017"
    ));
}

#[test]
fn ap017_static_template_literal_is_clean() {
    assert!(!fires(
        "src/app.py",
        "return render_template_string('<h1>Static</h1>')\n",
        "AP-017"
    ));
}

// --- Council review hardening (adversarial + kernel findings) --------------

#[test]
fn wc003_does_not_fire_on_non_jwt_algorithm_config() {
    // Adversarial CRITICAL: without a left `\b` anchor, `alg` matched the tail
    // of unrelated camelCase keys (`compressionAlgorithm: 'none'`), producing a
    // blocking false positive on non-JWT config. The `\b` anchor must keep
    // those clean while still catching a standalone JWT `alg`/`algorithm(s)`.
    assert!(!fires(
        "src/zlib.ts",
        "const config = { compressionAlgorithm: 'none' };\n",
        "WC-003"
    ));
    assert!(!fires(
        "src/hash.ts",
        "const opts = { hashAlgorithm: \"none\" };\n",
        "WC-003"
    ));
    // Real JWT shapes must still fire.
    assert!(fires("src/a.ts", "const h = { alg: 'none' };\n", "WC-003"));
    assert!(fires(
        "src/a.py",
        "tok = jwt.encode(p, k, algorithm=\"none\")\n",
        "WC-003"
    ));
}

#[test]
fn wc003_is_a_warning_not_a_blocking_error() {
    // ADR-087 §6 warnings-over-blocks: every insecure-construction rule is a
    // warning so a match cannot fail `anvil check` at the default error
    // threshold. WC-003 was downgraded from error to warning in review.
    let p = get_pattern("WC-003").expect("WC-003 exists");
    assert_eq!(p.severity, WarningSeverity::Warning);
}

#[test]
fn ur001_literal_prefixed_concatenation_fires() {
    // Adversarial MAJOR: the classic literal-prefixed DOM-XSS shape
    // `'<b>' + tainted` must fire — only a *pure* static literal is skipped.
    assert!(fires(
        "src/view.ts",
        "el.innerHTML = '<b>' + userName + '</b>';\n",
        "UR-001"
    ));
    assert!(fires(
        "src/view.ts",
        "el.innerHTML = \"\" + tainted;\n",
        "UR-001"
    ));
    // A pure static literal is still skipped.
    assert!(!fires("src/view.ts", "el.innerHTML = '<hr>';\n", "UR-001"));
}

#[test]
fn ap017_literal_prefixed_concatenation_fires() {
    // Same literal-prefixed shape for SSTI.
    assert!(fires(
        "src/app.py",
        "return render_template_string('Hello ' + name)\n",
        "AP-017"
    ));
    assert!(!fires(
        "src/app.py",
        "return render_template_string('Hello world')\n",
        "AP-017"
    ));
}

#[test]
fn wc002_covers_decipher_construction() {
    // Adversarial MAJOR: decrypt-side construction with the same broken
    // primitive (`createDecipheriv('des-ecb', …)`) must not be a blind spot.
    assert!(fires(
        "src/dec.ts",
        "const d = crypto.createDecipheriv('des-ecb', key, null);\n",
        "WC-002"
    ));
}

#[test]
fn wc_and_ap_allowlist_js_and_jsx_test_files() {
    // Copilot review: WC-*/AP-017 scan .js/.jsx but the allowlist originally
    // only covered .ts/.tsx/.py test globs, so a weak primitive in a JS test
    // fixture leaked through. JS/JSX test files must be allowlisted too.
    assert!(!fires(
        "src/crypto.test.js",
        "const h = createHash('md5').update(x).digest('hex');\n",
        "WC-001"
    ));
    assert!(!fires(
        "src/render.spec.jsx",
        "return render_template_string(tpl);\n",
        "AP-017"
    ));
}

#[test]
fn wc001_covers_hashlib_new_and_web_crypto() {
    // Adversarial MINOR: Python's generic `hashlib.new('md5')` dispatcher and
    // Web Crypto's `subtle.digest('SHA-1', …)` are common real forms.
    assert!(fires(
        "src/h.py",
        "digest = hashlib.new('md5', data).hexdigest()\n",
        "WC-001"
    ));
    assert!(fires(
        "src/h.ts",
        "const d = await crypto.subtle.digest('SHA-1', bytes);\n",
        "WC-001"
    ));
}

// --- Suppression (ADR-029 parser) works on the new rules -------------------

#[test]
fn wc001_respects_inline_suppression() {
    let opts = ScanOptions {
        patterns: None,
        include_opt_in: false,
    };
    // Anvil's native directive sits on the line *above* the finding.
    let content = "// @anvil-ignore WC-001 -- cache key, non-security\nconst t = createHash('md5').update(x).digest('hex');\n";
    let unsuppressed = scan_file("src/cache.ts", content, Some(&opts))
        .warnings
        .into_iter()
        .filter(|w| w.id == "WC-001" && w.suppressed.is_none())
        .count();
    assert_eq!(
        unsuppressed, 0,
        "an inline @anvil-ignore WC-001 must suppress the finding"
    );
}
