//! PYLAN-003/-004/-007: Python reliability anti-pattern catalogue.
//!
//! Exercises the `python-reliability` family (PY-001..007) through the public
//! scanner API: each rule fires on a realistic positive fixture and stays
//! quiet on the justified/negative form, the rules compile cleanly under the
//! RE2 engine (no silent-drop), `#`-comment suppression works (PYLAN-004), the
//! rules are `.py`-extension-scoped, and `.py` is in the default scan set
//! (PYLAN-007).

use anvil_checks::antipattern::{
    AntipatternCheckConfig, ScanOptions, registry_compile_diagnostics, scan_file,
};

fn fires(path: &str, content: &str, id: &str) -> bool {
    scan_file(path, content, None)
        .warnings
        .iter()
        .any(|w| w.id == id)
}

fn fires_opt_in(path: &str, content: &str, id: &str) -> bool {
    let opts = ScanOptions {
        patterns: None,
        include_opt_in: true,
    };
    scan_file(path, content, Some(&opts))
        .warnings
        .iter()
        .any(|w| w.id == id)
}

// --- RE2 compile (no silent drop) ------------------------------------------

#[test]
fn python_reliability_rules_compile_under_re2() {
    // A lookahead in a pattern would make Rust's RE2 engine drop the rule
    // silently; assert none of the PY-* rules are in the compile-diagnostics.
    let dropped: Vec<String> = registry_compile_diagnostics()
        .into_iter()
        .filter(|d| d.pattern_id.starts_with("PY-"))
        .map(|d| format!("{}: {}", d.pattern_id, d.error))
        .collect();
    assert!(
        dropped.is_empty(),
        "PY rules must compile under RE2 (no lookahead): {dropped:?}"
    );
}

// --- PY-001: # type: ignore without an error code --------------------------

#[test]
fn py001_bare_type_ignore_fires() {
    assert!(fires(
        "src/app.py",
        "def f(x):  # type: ignore\n    return x\n",
        "PY-001"
    ));
}

#[test]
fn py001_scoped_type_ignore_is_clean() {
    assert!(!fires(
        "src/app.py",
        "def f(x):  # type: ignore[arg-type]\n    return x\n",
        "PY-001"
    ));
}

// --- PY-002: bare # noqa without a rule code -------------------------------

#[test]
fn py002_bare_noqa_fires() {
    assert!(fires("src/app.py", "import os  # noqa\n", "PY-002"));
}

#[test]
fn py002_coded_noqa_is_clean() {
    assert!(!fires("src/app.py", "import os  # noqa: F401\n", "PY-002"));
}

// --- PY-003: # pylint: disable ---------------------------------------------

#[test]
fn py003_pylint_disable_fires() {
    assert!(fires(
        "src/app.py",
        "obj.dynamic  # pylint: disable=no-member\n",
        "PY-003"
    ));
}

#[test]
fn py003_pylint_enable_is_clean() {
    // Review: a clean/negative case — `# pylint: enable` re-enables a rule and
    // must not be flagged as a suppression.
    assert!(!fires(
        "src/app.py",
        "obj.dynamic  # pylint: enable=no-member\n",
        "PY-003"
    ));
}

// --- PY-004: bare except / except ...: pass --------------------------------

#[test]
fn py004_bare_except_fires() {
    assert!(fires(
        "src/app.py",
        "try:\n    risky()\nexcept:\n    pass\n",
        "PY-004"
    ));
}

#[test]
fn py004_inline_except_pass_fires() {
    assert!(fires(
        "src/app.py",
        "try:\n    risky()\nexcept Exception: pass\n",
        "PY-004"
    ));
}

#[test]
fn py004_arbitrary_inline_handler_pass_fires() {
    // Review: the rule intentionally flags ANY inline `except <x>: pass`
    // swallow, not just `except Exception:` — the title reflects this.
    assert!(fires(
        "src/app.py",
        "try:\n    f()\nexcept (KeyError, IndexError): pass\n",
        "PY-004"
    ));
}

#[test]
fn py004_named_handler_is_clean() {
    assert!(!fires(
        "src/app.py",
        "try:\n    risky()\nexcept ValueError as e:\n    raise\n",
        "PY-004"
    ));
}

#[test]
fn py004_attribute_named_except_is_not_a_false_positive() {
    // Review regression: `\bexcept` must not match an identifier ending in
    // "except" (`last_except: int = 0`).
    assert!(!fires(
        "src/app.py",
        "class C:\n    last_except: int = 0\n",
        "PY-004"
    ));
    // ...while a real bare except still fires.
    assert!(fires(
        "src/app.py",
        "try:\n    f()\nexcept:\n    g()\n",
        "PY-004"
    ));
}

#[test]
fn py004_except_inside_a_string_literal_is_not_flagged() {
    // PYLAN-009 validation regression: rich's traceback.py has the literal
    // `"... not called in except: block"`. PY-004 is code-scoped (runs against
    // the comment/string-masked view), so a string occurrence does not fire.
    assert!(!fires(
        "src/app.py",
        "raise ValueError(\"Value for 'trace' required if not called in except: block\")\n",
        "PY-004"
    ));
}

// --- PY-005: wildcard import -----------------------------------------------

#[test]
fn py005_wildcard_import_fires() {
    assert!(fires("src/app.py", "from os.path import *\n", "PY-005"));
}

#[test]
fn py005_relative_wildcard_import_fires() {
    assert!(fires("src/app.py", "from .config import *\n", "PY-005"));
}

#[test]
fn py005_explicit_import_is_clean() {
    assert!(!fires(
        "src/app.py",
        "from os.path import join, dirname\n",
        "PY-005"
    ));
}

#[test]
fn py005_commented_out_wildcard_is_not_flagged() {
    // Review regression: anchoring to `^\s*from` excludes a commented-out
    // import and a string-literal occurrence.
    assert!(!fires("src/app.py", "# from os import *\n", "PY-005"));
    assert!(!fires(
        "src/app.py",
        "doc = \"from os import *\"\n",
        "PY-005"
    ));
    // An indented (in-block) real wildcard import still fires.
    assert!(fires(
        "src/app.py",
        "if True:\n    from os import *\n",
        "PY-005"
    ));
}

#[test]
fn py005_init_file_wildcard_reexport_is_allowlisted() {
    // PYLAN-009 validation: `__init__.py` re-exporting a package API with
    // `from .sub import *` is the one conventional wildcard use, so it is
    // allowlisted — no day-one noise.
    assert!(!fires(
        "pkg/__init__.py",
        "from ._api import *\nfrom ._client import *\n",
        "PY-005"
    ));
    // The same wildcard in a non-__init__ module still fires.
    assert!(fires("pkg/api.py", "from ._client import *\n", "PY-005"));
}

// --- PY-006: print() (opt-in) ----------------------------------------------

#[test]
fn py006_print_fires_only_when_opt_in_enabled() {
    let content = "def main():\n    print(\"debug\", value)\n";
    assert!(
        !fires("src/app.py", content, "PY-006"),
        "PY-006 is opt-in and must be off by default"
    );
    assert!(
        fires_opt_in("src/app.py", content, "PY-006"),
        "PY-006 fires when opt-in is enabled"
    );
}

#[test]
fn py006_method_named_print_is_not_flagged() {
    // Review improvement: `(^|[^.\w])print` excludes a `.print()` method call
    // (e.g. rich's `console.print(...)`) while still catching the builtin.
    assert!(!fires_opt_in(
        "src/app.py",
        "    console.print(panel)\n",
        "PY-006"
    ));
    assert!(fires_opt_in("src/app.py", "    print(value)\n", "PY-006"));
}

// --- PY-007: Any annotation (opt-in) ---------------------------------------

#[test]
fn py007_any_annotation_fires_when_opt_in() {
    assert!(fires_opt_in(
        "src/app.py",
        "def f(x: Any) -> Any:\n    return x\n",
        "PY-007"
    ));
    assert!(fires_opt_in(
        "src/app.py",
        "data: Dict[str, Any] = {}\n",
        "PY-007"
    ));
}

#[test]
fn py007_qualified_typing_any_fires() {
    // Review: the qualified form `typing.Any` is as common as the bare import.
    assert!(fires_opt_in(
        "src/app.py",
        "def f(x: typing.Any) -> typing.Any:\n    return x\n",
        "PY-007"
    ));
    // A name that merely ends in `Any` (e.g. a user type) is not flagged.
    assert!(!fires_opt_in(
        "src/app.py",
        "def f(x: MyAny) -> None:\n    return None\n",
        "PY-007"
    ));
}

#[test]
fn py007_any_in_import_statement_is_not_flagged() {
    // PYLAN-009 validation regression: `from typing import ..., Any, ...`
    // imports the NAME, it is not a type annotation. The subscript-context
    // branch (`\[[^\]]*\bAny\b`) no longer matches the import-list comma.
    assert!(!fires_opt_in(
        "src/app.py",
        "from typing import IO, TYPE_CHECKING, Any, Callable, Optional\n",
        "PY-007"
    ));
    assert!(!fires_opt_in("src/app.py", "import typing.Any\n", "PY-007"));
    // ...but a real subscripted `Any` (incl. with a leading comma) still fires.
    assert!(fires_opt_in(
        "src/app.py",
        "data: Dict[str, Any] = {}\n",
        "PY-007"
    ));
    assert!(fires_opt_in(
        "src/app.py",
        "items: List[Any] = []\n",
        "PY-007"
    ));
}

#[test]
fn py007_string_subscript_key_any_is_not_flagged() {
    // PR #2740 review regression: `x["Any"]` is a runtime subscript with the
    // string key "Any", not a type annotation. PY-007 is code-scoped (masked
    // view), so the string occurrence does not fire...
    assert!(!fires_opt_in(
        "src/app.py",
        "value = config[\"Any\"]\n",
        "PY-007"
    ));
    // ...while a real bare `Any` in a subscript annotation still fires.
    assert!(fires_opt_in(
        "src/app.py",
        "mapping: Dict[str, Any] = {}\n",
        "PY-007"
    ));
}

// --- PYLAN-004: # @anvil-ignore suppression --------------------------------

#[test]
fn anvil_ignore_comment_suppresses_python_rule() {
    // A `# @anvil-ignore <ID> -- reason` on the preceding line marks the
    // finding suppressed (not removed), mirroring the TS/Rust behaviour.
    let content = "try:\n    risky()\n# @anvil-ignore PY-004 -- top-level daemon guard, logs and re-raises\nexcept:\n    pass\n";
    let result = scan_file("src/daemon.py", content, None);
    let warning = result
        .warnings
        .iter()
        .find(|w| w.id == "PY-004")
        .expect("PY-004 should still be reported, just suppressed");
    let suppression = warning
        .suppressed
        .as_ref()
        .expect("PY-004 should be marked suppressed");
    assert_eq!(
        suppression.reason,
        "top-level daemon guard, logs and re-raises"
    );
}

#[test]
fn anvil_ignore_for_other_rule_does_not_suppress() {
    let content = "# @anvil-ignore PY-001 -- unrelated\nfrom os import *\n";
    let result = scan_file("src/app.py", content, None);
    let warning = result.warnings.iter().find(|w| w.id == "PY-005").unwrap();
    assert!(
        warning.suppressed.is_none(),
        "a PY-001 suppression must not silence a PY-005 finding"
    );
}

// --- PY-008 / PY-009: dynamic execution (Dave SEC-COV-1) --------------------

#[test]
fn py008_eval_with_dynamic_argument_fires() {
    assert!(fires("src/app.py", "result = eval(user_input)\n", "PY-008"));
    assert!(fires("src/app.py", "exec(payload)\n", "PY-008"));
    assert!(fires(
        "src/app.py",
        "code = compile(src, '<str>', 'exec')\n",
        "PY-008"
    ));
    // Identifier start `i` must still fire — do not subtract prefix
    // letters from the first-character class.
    assert!(fires("src/app.py", "eval(input())\n", "PY-008"));
    assert!(fires("src/app.py", "eval(item)\n", "PY-008"));
    assert!(fires("src/app.py", "exec(imported)\n", "PY-008"));
    assert!(fires("src/app.py", "eval(user_input.strip())\n", "PY-008"));
    assert!(fires("src/app.py", "eval(user_input[0])\n", "PY-008"));
    assert!(fires("src/app.py", "exec(f'pass')\n", "PY-008"));
    assert!(fires("src/app.py", "exec(f\"{user_input}\")\n", "PY-008"));
}

/// Council review of PR #3880: the assertions above all terminate the first
/// identifier with one of `,` `)` `(` `.` `[`, so they mirrored the detector's
/// delimiter class instead of the injection shapes the rule exists to catch —
/// the suite stayed green (33/33) while the rule silently stopped firing on
/// every operator form. Each case below fired on the pre-#3880 rule, went
/// clean on the first #3880 pattern, and must fire again. Reverting
/// `PY-008.anvil`'s pattern to
/// `\b(eval|exec|compile)\s*\(\s*(?:[fF]['"]|[A-Za-z_][A-Za-z0-9_]*\s*[,().\[])`
/// turns every assertion in this test RED.
#[test]
fn py008_composed_and_operator_terminated_arguments_fire() {
    // Symbolic operators — concatenating or formatting untrusted input into
    // the payload is the canonical eval-injection shape.
    assert!(fires("src/app.py", "eval(a + b)\n", "PY-008"));
    assert!(fires("src/app.py", "eval(a+b)\n", "PY-008"));
    assert!(fires("src/app.py", "eval(user + \"x\")\n", "PY-008"));
    assert!(fires("src/app.py", "exec(code % vars)\n", "PY-008"));
    assert!(fires("src/app.py", "eval(base**exp)\n", "PY-008"));

    // Keyword operators are identifier-shaped, so the terminator has to allow
    // the space before them — an allowlist of punctuation silently misses all
    // of these.
    assert!(fires(
        "src/app.py",
        "eval(cmd if trusted else safe)\n",
        "PY-008"
    ));
    assert!(fires("src/app.py", "eval(x and y)\n", "PY-008"));
    assert!(fires("src/app.py", "eval(x or fallback)\n", "PY-008"));
    assert!(fires("src/app.py", "eval(not flag)\n", "PY-008"));
    assert!(fires("src/app.py", "eval(x is None)\n", "PY-008"));
    assert!(fires("src/app.py", "eval(x in allowed)\n", "PY-008"));
    assert!(fires("src/app.py", "eval(await coro)\n", "PY-008"));
    assert!(fires("src/app.py", "eval(target := build())\n", "PY-008"));

    // Keyword arguments and unpacking.
    assert!(fires("src/app.py", "eval(code, key=val)\n", "PY-008"));
    assert!(fires(
        "src/app.py",
        "compile(source=src, filename=\"<s>\", mode=\"exec\")\n",
        "PY-008"
    ));
    assert!(fires("src/app.py", "eval(*args)\n", "PY-008"));
    assert!(fires("src/app.py", "exec(**kwargs)\n", "PY-008"));

    // Python identifiers may hold non-ASCII characters; the terminator must
    // treat the first non-ASCII byte as "not an identifier char", not give up.
    assert!(fires("src/app.py", "eval(naïve_input)\n", "PY-008"));

    // A hand-wrapped call whose first argument ends the line. The scanner
    // matches per line, so end-of-line has to count as a terminator.
    assert!(fires(
        "src/app.py",
        "value = eval(user_input\n             + suffix)\n",
        "PY-008"
    ));
}

/// f-strings interpolate at run time, so every prefix ordering is dynamic.
/// The first #3880 pattern matched only a bare `f`/`F`, so `rf"{user_input}"`
/// fell through to the identifier arm, hit the quote, and was classified
/// static — while the rule body shipped in the same commit promised the
/// opposite.
#[test]
fn py008_f_string_prefixes_fire_in_every_ordering() {
    for src in [
        "eval(f\"{user_input}\")\n",
        "eval(F\"{user_input}\")\n",
        "eval(rf\"{user_input}\")\n",
        "eval(fr\"{user_input}\")\n",
        "eval(Rf'{user_input}')\n",
        "eval(fR\"{user_input}\")\n",
        "eval(FR\"{user_input}\")\n",
        "eval(RF\"{user_input}\")\n",
    ] {
        assert!(
            fires("src/app.py", src, "PY-008"),
            "f-string prefix must fire (it interpolates at run time): {src:?}"
        );
    }
}

#[test]
fn py008_eval_with_string_literal_does_not_fire() {
    assert!(!fires("src/app.py", "result = eval(\"1 + 1\")\n", "PY-008"));
    assert!(!fires("src/app.py", "exec('pass')\n", "PY-008"));
}

#[test]
fn py008_prefixed_string_literals_do_not_fire() {
    // Prefixed literals are static, same as a bare quote. The previous
    // detector treated the prefix letter as an identifier and false-fired.
    assert!(!fires(
        "src/app.py",
        "pattern = re.compile(r'^\\d+$')\n",
        "PY-008"
    ));
    assert!(!fires("src/app.py", "compile(r\"print(1)\")\n", "PY-008"));
    assert!(!fires(
        "src/app.py",
        "compile(rb\"print(1)\", \"<s>\", \"exec\")\n",
        "PY-008"
    ));
    assert!(!fires("src/app.py", "eval(u\"1\")\n", "PY-008"));
    assert!(!fires("src/app.py", "eval(b'1')\n", "PY-008"));
    // Every raw/bytes/unicode ordering and casing, not just `rb`. A terminator
    // class that excludes quotes but not identifier characters re-fires all of
    // these: the engine backtracks the prefix to one letter and accepts the
    // second letter as the terminator.
    for src in [
        "eval(R\"x\")\n",
        "eval(U\"x\")\n",
        "eval(B'x')\n",
        "eval(rb'x')\n",
        "eval(br\"x\")\n",
        "eval(Rb\"x\")\n",
        "eval(bR\"x\")\n",
        "eval(BR\"x\")\n",
        "eval(RB\"x\")\n",
        "eval(Br'x')\n",
    ] {
        assert!(
            !fires("src/app.py", src, "PY-008"),
            "prefixed literal is static and must not fire: {src:?}"
        );
    }
}

/// CIB-332: `compile` is not only the Python builtin. `re.compile`,
/// `torch.compile`, and any `self.compile` / `Pattern.compile` method are
/// ordinary calls that take a variable, and PY-008 fired on all of them at
/// `severity: error, confidence: high`. CIB-322 fixed only the inline
/// prefixed-literal half (`re.compile(r'^\d+$')`); the named-constant idiom
/// (`DATE_RE = r'...'` then `re.compile(DATE_RE)`) is the common real shape and
/// still false-fired. A dotted receiver means the call is not the builtin.
#[test]
fn py008_dotted_compile_receiver_does_not_fire() {
    for src in [
        // The named-constant regex idiom — the most common real-world shape.
        "pattern = re.compile(pattern_var)\n",
        "rx = re.compile(user_supplied_regex, re.I)\n",
        // Non-stdlib receivers named `compile` are just as ordinary.
        "model = torch.compile(model)\n",
        "self.compile(src)\n",
        "Pattern.compile(name)\n",
    ] {
        assert!(
            !fires("src/app.py", src, "PY-008"),
            "a dotted `compile` receiver is not the builtin and must not fire: {src:?}"
        );
    }
}

/// The receiver gate applies to `compile` only. CIB-322's Non-scope clause is
/// explicit that receiver resolution must not become the whole fix: an
/// unqualified `compile(...)` is the builtin and must still fire. `eval` and
/// `exec` keep their word boundary deliberately — `df.eval(expr)` (pandas) and
/// an `obj.exec(cmd)` wrapper are genuine execution surfaces, unlike
/// `re.compile`.
#[test]
fn py008_unqualified_compile_and_attribute_eval_exec_still_fire() {
    for src in [
        // Unqualified `compile` is the builtin — dotting the receiver must not
        // be a way to lose the builtin case.
        "compile(src, '<str>', 'exec')\n",
        "compile(source=src, filename=\"<s>\", mode=\"exec\")\n",
        // Attribute access on eval/exec still fires.
        "builtins.eval(user_input)\n",
        "df.eval(expr)\n",
        "obj.exec(cmd)\n",
    ] {
        assert!(
            fires("src/app.py", src, "PY-008"),
            "must still fire: {src:?}"
        );
    }

    // `^` in the receiver gate is a per-line anchor, not a per-file one: the
    // scanner runs the regex line by line. Asserting only on line 1 (where the
    // two anchors coincide) would pass even if the rule silently degraded to
    // matching the first line of a file.
    assert!(fires(
        "src/app.py",
        "import dis\ncompile(src, '<str>', 'exec')\n",
        "PY-008"
    ));
}

#[test]
fn py008_explanation_describes_eval_exec_compile_not_family_boilerplate() {
    let result = scan_file("src/app.py", "eval(user_input)\n", None);
    let warning = result
        .warnings
        .iter()
        .find(|w| w.id == "PY-008")
        .expect("PY-008 should fire on eval(user_input)");
    let explanation = warning.explanation.to_lowercase();
    assert!(
        explanation.contains("eval")
            && explanation.contains("exec")
            && explanation.contains("compile"),
        "PY-008 explanation must describe eval/exec/compile, got: {}",
        warning.explanation
    );
    for boilerplate in ["# type: ignore", "import *", "except:", "print()", "Any"] {
        assert!(
            !warning.explanation.contains(boilerplate),
            "PY-008 explanation must not reuse family boilerplate ({boilerplate}): {}",
            warning.explanation
        );
    }
}

#[test]
fn py009_shell_true_and_pickle_fire() {
    assert!(fires(
        "src/app.py",
        "subprocess.call(\"ls \" + user_input, shell=True)\n",
        "PY-009"
    ));
    assert!(fires("src/app.py", "os.system(cmd)\n", "PY-009"));
    assert!(fires("src/app.py", "obj = pickle.loads(blob)\n", "PY-009"));
    assert!(fires("src/app.py", "data = yaml.load(stream)\n", "PY-009"));
}

// --- Extension scoping ------------------------------------------------------

#[test]
fn python_rules_do_not_fire_on_non_python_files() {
    // PY rules are `.py`-scoped; matching text in a `.ts` file must not fire.
    assert!(!fires(
        "src/app.ts",
        "const x = 1;  # type: ignore\n",
        "PY-001"
    ));
    assert!(!fires("src/app.ts", "from os import *\n", "PY-005"));
}

// --- PYLAN-007: .py in the default scan set --------------------------------

#[test]
fn py_extension_is_in_default_scan_set() {
    assert!(
        AntipatternCheckConfig::default()
            .extensions
            .iter()
            .any(|e| e == ".py"),
        "`.py` must be in the default antipattern/drift scan extensions"
    );
}
