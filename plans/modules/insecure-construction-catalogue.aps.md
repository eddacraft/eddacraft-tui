<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Insecure-Construction Anti-Pattern Catalogue (First Wave)

| ID    | Owner | Status      | Progress |
| ----- | ----- | ----------- | -------- |
| INSEC | —     | In Progress | 6/8      |

**Last reviewed:** 2026-06-18 (created on operator direction after ADR-087 was
accepted. Realises the syntactic-smell subset of the sec-context triage
[`plans/brainstorms/2026-06-18-sec-context-antipattern-triage.md`]. First-wave
families `weak-cryptography` + `unsafe-rendering` are Ready; the deferred
opt-in items — `injection-smell` (AST) and insecure-RNG — stay Proposed per the
ADR's explicit deferral.)

## Purpose

Add the **`insecure-construction`** anti-pattern category and its first-wave
families, per **[ADR-087](../decisions/087-security-antipattern-category.md)**.
These are the items from
[Arcanum-Sec/sec-context](https://github.com/Arcanum-Sec/sec-context) that
Anvil's deterministic single-file regex + AST-query model can detect at an
acceptable false-positive rate — *syntactic smells*, not taint-based findings.

This is **not** a SAST/taint engine. The taint-class (reflected/stored XSS,
output encoding) and absence-class (rate limiting, MFA, CSP, server-side
validation) sec-context items are **out of model** by the ADR-087 scope
boundary and are deliberately absent here; crossing that line needs a new ADR.

## In Scope

- A new `InsecureConstruction` `AntiPatternCategory` variant, its
  `"insecure-construction"` wire value, and the matching TS compile-schema arm —
  reusing the existing registry → scanner pipeline (no new engine check).
- **`weak-cryptography`** family (regex, enabled): deprecated hash/cipher
  primitives (MD5 / SHA1-for-security / DES), ECB mode, JWT `alg: none`.
- **`unsafe-rendering`** family (regex, enabled): DOM-XSS sinks (`innerHTML =`,
  `document.write(`, `dangerouslySetInnerHTML`).
- SSTI coverage folded into the **existing `dynamic-execution`** family (extend,
  not new).
- The ADR-087 §5 scope-guard follow-up note in
  `docs/vision/anvil-scope-guard.md`.
- Dogfood + §16.5 #9 false-positive acceptance for the enabled families.

## Out of Scope

- Taint / data-flow / whole-program reachability analysis (ADR-087 boundary).
- Credential detection — owned by the existing `secret` engine check; not
  duplicated here.
- Slopsquatting / dependency risk — routed to the
  `save-time-dependency-advisory-pack` thesis, not this catalogue.
- The taint-class and absence-class sec-context items (see Purpose).

## Interfaces

**Depends on:**

- [ADR-087](../decisions/087-security-antipattern-category.md) (accepted) — the
  category + scope boundary.
- [ADR-071](../decisions/071-ast-aware-antipattern-detection.md) — the gate-time
  `anvil-checks-ast` scanner the deferred `injection-smell` family would reuse.
- Existing registry (`patterns/`, `patterns/compiled/registry.json`), the regex
  scanner (`crates/anvil-checks/src/antipattern/`), and the ADR-029 suppression
  parser.

**Exposes:**

- A security-class anti-pattern category usable across languages, reusing the
  existing save-time + gate-time scan paths and SARIF emission.

## Proposed rule-id prefixes

New registry `prefixes` entries (one per family), mirroring `GS`/`DD`/`PY`:

- `WC` → `weak-cryptography`
- `UR` → `unsafe-rendering`
- `INJ` → `injection-smell` (deferred)

## Work Items

### INSEC-001: `insecure-construction` category variant

- **Status:** Merged 2026-07-01 via PR #3028
- **Intent:** Make `insecure-construction` a first-class category so its families
  carry meaningful provenance instead of falling back to `code-quality`.
- **Expected Outcome:** A registry pattern tagged `category:
  insecure-construction` round-trips through compile → load → scanner as the new
  category; the TS schema accepts the value; the default `map_category` fallback
  is no longer reached for these rules.
- **Validation:** `cargo test -p eddacraft-anvil-checks` (category mapping +
  round-trip); `pnpm test` for the schema; `pnpm adr:check` unaffected.
- **Files:** `crates/anvil-checks/src/antipattern/types.rs` (enum),
  `crates/anvil-checks/src/antipattern/registry_loader.rs` (`map_category`),
  `packages/anvil/core/src/anvil-format/schemas.ts`
- **Dependencies:** —
- **Confidence:** high

---

### INSEC-002: `weak-cryptography` family (regex, enabled)

- **Status:** Merged 2026-07-01 via PR #3028
- **Intent:** Flag construction with broken/inappropriate crypto primitives.
- **Expected Outcome:** New `patterns/weak-cryptography/` family with rules for
  deprecated hash/cipher algos (MD5, SHA1-for-security, DES), ECB mode, and JWT
  `alg: none`; enabled by default, scanning the standard extension set; findings
  carry family provenance + a fix-oriented nudge.
- **Validation:** Per-rule positive + justified-negative tests; regex
  compiles under the registry diagnostics guard; dogfood FP check (INSEC-007).
- **Files:** `patterns/weak-cryptography/definition.anvil`,
  `patterns/weak-cryptography/*.anvil`, `patterns/compiled/registry.json`,
  `crates/anvil-checks/tests/`
- **Dependencies:** INSEC-001
- **Confidence:** medium
- **Risks:** MD5 used for non-security checksums is legitimate — nudge, do not
  block; allowlist or opt-in if FP pressure exceeds the bar.

---

### INSEC-003: `unsafe-rendering` family (regex, enabled)

- **Status:** Merged 2026-07-01 via PR #3028
- **Intent:** Flag writing untrusted data into a markup/exec sink.
- **Expected Outcome:** New `patterns/unsafe-rendering/` family for DOM-XSS sinks
  (`innerHTML =`, `document.write(`, `dangerouslySetInnerHTML`); enabled by
  default, scoped to web extensions; test-file allowlist to control FP.
- **Validation:** Per-rule positive + justified-negative tests; dogfood FP check
  (INSEC-007).
- **Files:** `patterns/unsafe-rendering/definition.anvil`,
  `patterns/unsafe-rendering/*.anvil`, `patterns/compiled/registry.json`,
  `crates/anvil-checks/tests/`
- **Dependencies:** INSEC-001
- **Confidence:** medium

---

### INSEC-004: SSTI coverage via the existing `dynamic-execution` family

- **Status:** Merged 2026-07-01 via PR #3028
- **Intent:** Cover server-side template injection without a new family, since
  `dynamic-execution` already owns the eval-class concept (AP-008).
- **Expected Outcome:** SSTI detection added as a rule in the existing
  `dynamic-execution` family rather than a new family.
- **Validation:** Positive + negative tests alongside the existing
  `dynamic-execution` rules.
- **Files:** `patterns/dynamic-execution/*.anvil`,
  `patterns/compiled/registry.json`, `crates/anvil-checks/tests/`
- **Dependencies:** INSEC-001
- **Confidence:** medium

---

### INSEC-005: ADR-087 scope-guard note

- **Status:** Merged 2026-07-01 via PR #3028
- **Intent:** Record the out-of-model boundary where the scope guard lives, so a
  future "just add reflected XSS" request hits the documented line (ADR-087 §5
  follow-up).
- **Expected Outcome:** A one-line note in `docs/vision/anvil-scope-guard.md`
  stating that taint-class / absence-class security patterns are out of model
  and crossing the boundary requires a new ADR.
- **Validation:** Doc review; `pnpm docs:check` if it gates the file.
- **Files:** `docs/vision/anvil-scope-guard.md`
- **Dependencies:** —
- **Confidence:** high

---

### INSEC-006: Dogfood + §16.5 #9 false-positive acceptance

- **Status:** Merged 2026-07-01 via PR #3028
- **Intent:** Prove the enabled families clear the false-positive bar on real
  code before they ship default-on (the gate every language anchor passes —
  cf. PYLAN-009).
- **Expected Outcome:** ≥1 external-codebase run for the enabled families with
  FP rate < N% (N operator-accepted), evidence recorded under `plans/reviews/`.
- **Validation:** TP-vs-FP classification in the evidence note; per-fix
  regression tests; full `anvil-checks` green.
- **Files:** `plans/reviews/<date>-insec-external-validation.md`, any precision
  fixes in `patterns/weak-cryptography/`, `patterns/unsafe-rendering/`
- **Dependencies:** INSEC-002, INSEC-003, INSEC-004
- **Confidence:** medium

---

### INSEC-007: `injection-smell` family (AST, gate-time, opt-in) — DEFERRED

- **Status:** Proposed
- **Intent:** Flag query/command construction by interpolation (SQLi, command,
  LDAP, XPath, NoSQL) as a *smell* — explicitly opt-in because, without taint,
  it is not a finding (ADR-087 §3).
- **Expected Outcome:** A `patterns/injection-smell/` family on the ADR-071
  gate-time AST tier, off by default; command injection also extends
  `command_safety`.
- **Validation:** AST predicate tests + a high-FP dogfood that justifies the
  opt-in default.
- **Files:** `patterns/injection-smell/*.anvil`,
  `crates/anvil-checks-ast/`, `patterns/compiled/registry.json`
- **Dependencies:** INSEC-001 (+ ADR-071 AST scanner)
- **Confidence:** low
- **Note:** Deferred per ADR-087; not authorised for execution until promoted to
  Ready.

---

### INSEC-008: insecure-RNG rule (opt-in) — DEFERRED

- **Status:** Proposed
- **Intent:** Flag `Math.random()` / `new Random()` used for security values
  (tokens, salts) — opt-in because most uses are non-security (high FP).
- **Expected Outcome:** An opt-in rule in `weak-cryptography`, off by default.
- **Validation:** Positive + negative tests; opt-in gating test.
- **Files:** `patterns/weak-cryptography/*.anvil`,
  `patterns/compiled/registry.json`
- **Dependencies:** INSEC-002
- **Confidence:** low
- **Note:** Deferred per ADR-087; not authorised for execution until promoted to
  Ready.

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Crypto-name regex FP (MD5 as checksum) | Medium | Nudge not block; allowlist; opt-in if over the bar |
| DOM-XSS sink FP in framework/test code | Medium | Extension scoping + test-file allowlist |
| Scope creep toward taint-based SAST | Medium | ADR-087 boundary + INSEC-005 scope-guard note; new ADR required to cross |
| Category-name churn | Low | Name fixed by ADR-087 (`insecure-construction`) |

## Open Questions

- [x] Which extension set should `unsafe-rendering` scan (web-only vs default)?
      Resolved (#3028): JS/TS only (`.ts/.tsx/.js/.jsx/.mjs/.cjs`); `.html/.css`
      retired from the scan set.
- [x] Should JWT `alg: none` live in `weak-cryptography` or its own `jwt-misuse`
      family? (ADR-087 left this open.) Resolved (#3028): stays in
      `weak-cryptography` (WC-003); no separate `jwt-misuse` family.
- [x] What N% FP bar does the operator accept for the enabled families?
      Resolved (#3031, 2026-07-02): **`N` = 5% per family**. All enabled families
      clear it; WC-001 stays default-on `warning` (MD5-as-checksum findings are
      suppressible true positives via `@anvil-ignore`, hard-FP rate 0%), not
      downgraded to opt-in. Evidence:
      [`plans/reviews/2026-07-01-insec-external-validation.md`](../reviews/2026-07-01-insec-external-validation.md).
