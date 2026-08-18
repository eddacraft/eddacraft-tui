//! GS-002 / TE-001: type assertions that manufacture evidence.
//!
//! Two rules ported from the `anti-slop` catalogue, reshaped to Anvil's
//! detection tier:
//!
//! - **GS-002** (`guardrail-suppression`) — `x as unknown as T`. The author
//!   knows the type and is overriding it; the detour through `unknown` exists
//!   only to defeat the compiler's overlap check.
//! - **TE-001** (`type-system-evasion`) — `JSON.parse(raw) as T`. Nothing was
//!   validated; the assertion claims a contract the parse never established.
//!
//! Positive fixtures are enumerated from each rule's *threat model* — the
//! shapes the rule exists to catch — not from the branches of the regex that
//! implements it (see `docs/guides/anvil-rule-authoring.md`, "Testing the
//! rule"). Each label names a shape; a shape without a fixture is impossible
//! by construction, and a narrowing reports every casualty in one run.

use anvil_checks::antipattern::{registry_compile_diagnostics, scan_file};

mod support;

use support::assert_rule_fires_on;

fn fires(path: &str, content: &str, id: &str) -> bool {
    scan_file(path, content, None)
        .warnings
        .iter()
        .any(|w| w.id == id)
}

// --- RE2 compile (no silent drop) ------------------------------------------

#[test]
fn assertion_laundering_rules_compile_under_re2() {
    // Lookaround in a pattern makes Rust's RE2 engine drop the rule silently.
    // Neither rule uses it; assert the loader agrees rather than trusting the
    // pattern by inspection.
    let dropped: Vec<String> = registry_compile_diagnostics()
        .into_iter()
        .filter(|d| d.pattern_id == "GS-002" || d.pattern_id == "TE-001")
        .map(|d| format!("{}: {}", d.pattern_id, d.error))
        .collect();
    assert!(
        dropped.is_empty(),
        "GS-002/TE-001 must compile under the Rust regex engine; dropped: {dropped:?}"
    );
}

// --- GS-002 threat model ---------------------------------------------------

#[test]
fn gs002_chained_assertion_shapes_fire() {
    // Threat model: an assertion routed through `unknown` so the compiler
    // cannot reject the conversion. Every syntactic position a TypeScript
    // expression can occupy is in scope — the laundering is the defect, the
    // surrounding position is incidental.
    assert_rule_fires_on(
        "src/app.ts",
        "GS-002",
        &[
            (
                "return position",
                "return obj as unknown as FeatureFlagSnapshot;\n",
            ),
            (
                "variable initialiser",
                "const cfg = raw as unknown as Config;\n",
            ),
            (
                "call argument",
                "buildOverview(rows as unknown as FleetBeaconRow[]);\n",
            ),
            (
                "object literal property",
                "return { kind: 'response', response: obj as unknown as JsonRpcResponse };\n",
            ),
            (
                "array element type",
                "const rows = data as unknown as FleetDailyInstallRow[];\n",
            ),
            (
                "awaited expression",
                "body = (await res.text()) as unknown as T;\n",
            ),
            (
                "single-letter type parameter",
                "return value as unknown as T;\n",
            ),
            (
                "generic type argument",
                "const m = obj as unknown as Map<string, number>;\n",
            ),
            (
                "indexed access type",
                "const s = obj as unknown as Proposal['signals'];\n",
            ),
            (
                "property access on the result",
                "const id = (raw as unknown as User).id;\n",
            ),
            (
                "extra internal whitespace",
                "const cfg = raw as  unknown   as Config;\n",
            ),
            (
                "assignment to declared binding",
                "let out: Config;\nout = raw as unknown as Config;\n",
            ),
        ],
    );
}

#[test]
fn gs002_stays_quiet_on_non_laundered_forms() {
    // A single assertion still faces the compiler's overlap check, so it is
    // not this rule's business — AP-004/GS-001 cover the other hatches.
    assert!(!fires(
        "src/app.ts",
        "const cfg = raw as Config;\n",
        "GS-002"
    ));
    // Widening alone discards evidence but manufactures none.
    assert!(!fires(
        "src/app.ts",
        "const raw = value as unknown;\n",
        "GS-002"
    ));
    // `as const` narrows; it is the opposite of this defect.
    assert!(!fires(
        "src/app.ts",
        "const modes = ['a', 'b'] as const;\n",
        "GS-002"
    ));
    // Identifiers that merely contain the tokens must not match.
    assert!(!fires(
        "src/app.ts",
        "const asUnknownAsType = 1;\nawait parseAsUnknownAsync(x);\n",
        "GS-002"
    ));
    // `unknown` reached via a named alias evades the token match by design —
    // pinned as a known gap in the rule body, not silently untested.
    assert!(!fires(
        "src/app.ts",
        "type Opaque = unknown;\nconst c = raw as Opaque as Config;\n",
        "GS-002"
    ));
}

#[test]
fn gs002_is_code_scoped() {
    // The phrase reads naturally in prose — this repo's own rule narratives
    // quote it — so comments and string literals must stay silent.
    assert!(!fires(
        "src/app.ts",
        "// never write `x as unknown as T` here\n",
        "GS-002"
    ));
    assert!(!fires(
        "src/app.ts",
        "const help = 'avoid as unknown as casts';\n",
        "GS-002"
    ));
    // ...while real code on the following line still fires.
    assert!(fires(
        "src/app.ts",
        "// never write `x as unknown as T` here\nconst c = raw as unknown as Config;\n",
        "GS-002"
    ));
}

// --- TE-001 threat model ---------------------------------------------------

#[test]
fn te001_boundary_assertion_shapes_fire() {
    // Threat model: a shape asserted onto a value whose only guarantee is
    // "was valid JSON". Both boundaries that produce such a value are in
    // scope, in every shape the asserted type can take.
    assert_rule_fires_on(
        "src/app.ts",
        "TE-001",
        &[
            (
                "JSON.parse into a named type",
                "const manifest = JSON.parse(content) as SignatureManifest;\n",
            ),
            (
                "JSON.parse into a type parameter",
                "return JSON.parse(content) as T;\n",
            ),
            (
                "JSON.parse wrapped in parens",
                "const m = (JSON.parse(row.metadata) as Record<string, unknown>);\n",
            ),
            (
                "JSON.parse into an indexed access type",
                "const s = JSON.parse(row.signals) as CandidateProposal['signals'];\n",
            ),
            (
                "JSON.parse into a generic wrapper",
                "const r = JSON.parse(row.resolution) as NonNullable<Proposal['resolution']>;\n",
            ),
            (
                "JSON.parse of a nested call",
                "const snap = JSON.parse(JSON.stringify(v)) as Snapshot;\n",
            ),
            (
                "bare response .json()",
                "const user = await res.json() as User;\n",
            ),
            (
                "parenthesised awaited .json()",
                "const user = (await res.json()) as User;\n",
            ),
            (
                "member-chained .json()",
                "const body = (await response.json()) as ApiResult;\n",
            ),
            (
                "assignment to a declared binding",
                "let minted: MintSessionResult;\nminted = JSON.parse(stored) as MintSessionResult;\n",
            ),
        ],
    );
}

#[test]
fn te001_stays_quiet_when_the_boundary_is_parsed() {
    // The recommended fix must not fire.
    assert!(!fires(
        "src/app.ts",
        "const raw: unknown = JSON.parse(text);\nconst cfg = ConfigSchema.parse(raw);\n",
        "TE-001"
    ));
    // A parse with no assertion at all.
    assert!(!fires(
        "src/app.ts",
        "const raw = JSON.parse(text);\n",
        "TE-001"
    ));
    // `as const` on a parse result narrows a literal; it asserts no shape.
    assert!(!fires(
        "src/app.ts",
        "const modes = JSON.parse(text) as const;\n",
        "TE-001"
    ));
    // A non-boundary value asserted normally is AP-003/GS-002 territory.
    assert!(!fires(
        "src/app.ts",
        "const cfg = raw as Config;\n",
        "TE-001"
    ));
}

#[test]
fn te001_defers_the_laundered_form_to_gs002() {
    // `JSON.parse(x) as unknown as T` is one defect, not two. TE-001's
    // capitalised-type convention leaves it to GS-002 so a single line does
    // not produce two findings in two different families.
    let src = "const cfg = JSON.parse(text) as unknown as Config;\n";
    assert!(!fires("src/app.ts", src, "TE-001"));
    assert!(fires("src/app.ts", src, "GS-002"));
}

#[test]
fn te001_is_code_scoped() {
    assert!(!fires(
        "src/app.ts",
        "// do not write JSON.parse(raw) as Config\n",
        "TE-001"
    ));
    assert!(!fires(
        "src/app.ts",
        "const msg = 'JSON.parse(raw) as Config is unsafe';\n",
        "TE-001"
    ));
}

// --- Scope: extensions, allowlists, suppression ----------------------------

#[test]
fn both_rules_are_typescript_scoped() {
    // Type assertions do not exist in JavaScript; running there is noise.
    for (path, src, id) in [
        (
            "src/app.js",
            "const c = raw as unknown as Config;\n",
            "GS-002",
        ),
        (
            "src/app.mjs",
            "const c = raw as unknown as Config;\n",
            "GS-002",
        ),
        (
            "src/app.js",
            "const c = JSON.parse(t) as Config;\n",
            "TE-001",
        ),
    ] {
        assert!(!fires(path, src, id), "{id} must not run on {path}");
    }
    // ...and both do run on .tsx.
    assert!(fires(
        "src/App.tsx",
        "const c = raw as unknown as Config;\n",
        "GS-002"
    ));
    assert!(fires(
        "src/App.tsx",
        "const c = JSON.parse(t) as Config;\n",
        "TE-001"
    ));
}

#[test]
fn test_doubles_are_allowlisted() {
    // Building a partial mock and presenting it as the full interface is the
    // canonical legitimate use; 57 of this repo's 63 chained assertions are
    // in test files.
    let src = "const svc = { get: vi.fn() } as unknown as Service;\n";
    for path in [
        "src/service.test.ts",
        "src/service.spec.ts",
        "src/__tests__/service.ts",
        "src/__mocks__/service.ts",
    ] {
        assert!(
            !fires(path, src, "GS-002"),
            "GS-002 must be silent in {path}"
        );
    }
    // Declaration files wrapping untyped libraries are allowlisted for TE-001.
    assert!(!fires(
        "types/vendor.d.ts",
        "const c = JSON.parse(t) as Config;\n",
        "TE-001"
    ));
}

#[test]
fn suppression_directives_are_honoured() {
    // A `// @anvil-ignore <ID> -- reason` on the preceding line marks the
    // finding suppressed rather than removing it, so the gate can filter on
    // it and an audit can still count the hatches in use.
    for (id, content, reason) in [
        (
            "GS-002",
            "// @anvil-ignore GS-002 -- branded id conversion, checked at the parse boundary\nconst id = raw as unknown as UserId;\n",
            "branded id conversion, checked at the parse boundary",
        ),
        (
            "TE-001",
            "// @anvil-ignore TE-001 -- round-trip of a value serialised by this module\nconst snap = JSON.parse(stored) as Snapshot;\n",
            "round-trip of a value serialised by this module",
        ),
    ] {
        let result = scan_file("src/app.ts", content, None);
        let warning = result
            .warnings
            .iter()
            .find(|w| w.id == id)
            .unwrap_or_else(|| panic!("{id} should still be reported, just suppressed"));
        let suppression = warning
            .suppressed
            .as_ref()
            .unwrap_or_else(|| panic!("{id} should be marked suppressed"));
        assert_eq!(suppression.reason, reason);
    }
}

#[test]
fn a_suppression_for_the_sibling_rule_does_not_silence_this_one() {
    // The two rules sit in different families and must not cross-suppress:
    // waiving the laundered form is not a waiver for an unvalidated boundary.
    let content =
        "// @anvil-ignore GS-002 -- unrelated\nconst snap = JSON.parse(stored) as Snapshot;\n";
    let warning = scan_file("src/app.ts", content, None)
        .warnings
        .into_iter()
        .find(|w| w.id == "TE-001")
        .expect("TE-001 should fire");
    assert!(
        warning.suppressed.is_none(),
        "a GS-002 suppression must not silence a TE-001 finding"
    );
}
