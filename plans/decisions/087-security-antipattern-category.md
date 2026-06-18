# ADR-087: A security anti-pattern category, scoped to syntactic smells

## Status

Proposed

## Date

2026-06-18

## Context

[Arcanum-Sec/sec-context](https://github.com/Arcanum-Sec/sec-context) catalogues
36 "AI code security anti-patterns" — the insecure code LLMs reproduce most
often (SQL injection, XSS, hardcoded secrets, weak crypto, missing validation,
…). It is an attractive input for Anvil's anti-pattern catalogue: the framing
("patterns agents emit") is exactly Anvil's `agent-output` thesis, and each item
carries a CWE reference and severity that map cleanly onto our `severity` /
`confidence` / `spectrum_position` fields.

The triage (`plans/brainstorms/2026-06-18-sec-context-antipattern-triage.md`)
sorted all 36 against Anvil's actual detection model:

- **4 are already covered** by the `secret` engine check (hardcoded
  credentials / keys) — duplicating them is pure cost.
- **18 are out of Anvil's model.** Anvil is a *deterministic single-file regex +
  AST-query scanner* (ADR-071), not a taint-based SAST. Items whose real
  detection needs untrusted-input-reaches-sink taint (reflected/stored XSS,
  output encoding) or the *absence* of a control (missing rate limiting, MFA,
  CSP, server-side validation) cannot be expressed at any Anvil tier.
- **~14 are new-family candidates,** but only a minority are low-false-positive
  enough to ship enabled-by-default; the injection class is a *syntactic smell*
  without taint and would fail the §16.5 #9 FP bar if on by default.

The forces requiring a decision now:

1. **Category fit.** Every existing `AntiPatternCategory`
   (`escape-hatch`, `error-handling`, `type-evasion`, `accountability`,
   `deferred-debt`, `code-quality`, `html`, `css`) is *behavioural*. None is
   security-class. Filing "weak crypto" under `code-quality` destroys the
   family-as-concept design (`guardrail-suppression/definition.anvil`) and
   produces meaningless nudges. A new category is needed — or the work is
   rejected.
2. **Identity / scope-guard.** Adding security detection edges Anvil toward SAST
   (Semgrep/CodeQL). `docs/vision/anvil-scope-guard.md` should bound this so we
   adopt the *agent-escape-hatch* slice without committing to taint analysis.
3. **Naming collision.** There is already a `security` (SEC) APS module — the CI
   *pipeline* (Trivy/TruffleHog/Semgrep, secret detection, token revocation). A
   category literally named `security` would be ambiguous against that module
   and the `security.yml` workflow.
4. **Posture tension.** Security findings *feel* like they must block; Anvil
   exits 0 by default. The decision must keep warnings-over-blocks.

## Decision

Adopt a **new `AntiPatternCategory` named `insecure-construction`**, populated
**only by the syntactic-smell subset** of sec-context, with the taint-class and
absence-class items explicitly out of scope.

1. **New category variant.** Add `InsecureConstruction` to
   `AntiPatternCategory` (`crates/anvil-checks/src/antipattern/types.rs`), the
   `"insecure-construction"` arm to `map_category`
   (`registry_loader.rs`), and the matching value to the TS compile schema
   (`packages/anvil/core/src/anvil-format/schemas.ts`). Name chosen over
   `security` to avoid collision with the SEC module / `security.yml`; it also
   reads as a *behaviour* ("constructing code insecurely"), consistent with the
   other category names.

2. **First-wave families (regex, low/med FP, enabled by default):**
   - `weak-cryptography` — broken/inappropriate primitives: deprecated algos
     (MD5/SHA1/DES, CWE-327/328), ECB mode (CWE-327), and JWT `alg: none`
     (CWE-287). API-name regex, low FP.
   - `unsafe-rendering` — untrusted data into a markup/exec sink: DOM-XSS sinks
     (`innerHTML =`, `document.write(`, `dangerouslySetInnerHTML`, CWE-79).
     Regex, med FP, allowlisted for test files.

3. **Deferred / opt-in families (AST or high FP):**
   - `injection-smell` — interpolation into a query/command (SQLi, command,
     LDAP, XPath, NoSQL). **Opt-in only**, AST tier (gate-time), because without
     taint it is a smell, not a finding. Command injection also extends the
     existing `command_safety` rule. Tracked as a follow-on, not first wave.
   - insecure-RNG (`Math.random()` / `new Random()` for security values,
     CWE-330) joins `weak-cryptography` **opt-in** — most uses are non-security.

4. **Reuse existing mechanisms — add no engine check.** These families compile
   through the existing registry (`.anvil` definition + rules) and run on the
   existing regex scanner and the existing `anvil-checks-ast` gate-time scanner
   (ADR-071). SSTI (CWE-1336) extends the existing `dynamic-execution` family
   rather than starting new. The `secret` check keeps ownership of all
   credential items.

5. **Scope boundary (the load-bearing constraint).** Anvil does **not** acquire
   taint analysis or whole-program reachability under this ADR. The taint-class
   (reflected/stored XSS, output encoding, "accepting untrusted data") and
   absence-class (missing rate limiting, MFA, CSP, server-side validation,
   length limits) items are **out of model** and will not be added by stretching
   regex/AST to approximate them. Detecting them properly is a different product;
   that decision, if ever made, is a separate ADR. A one-line scope note lands in
   `docs/vision/anvil-scope-guard.md`.

6. **Posture preserved.** All `insecure-construction` rules are warnings under
   the new-edges-only baseline, exit 0 by default, suppressible via the
   ADR-029 parser — identical to every other family. Severity from sec-context's
   CWE rating maps to `severity`/`spectrum_position`; it does not change the
   default block behaviour.

## Rationale

A new category is the only option that preserves the family-as-behavioural-
concept design while letting Anvil adopt the genuinely-detectable slice of
sec-context. Naming it `insecure-construction` (not `security`) avoids the SEC
module collision and stays consistent with behavioural category names. Scoping
*hard* to syntactic smells — and writing the taint/absence exclusion into the
ADR and scope guard — is what keeps this from drifting into a half-built SAST
that fails the FP bar and dilutes Anvil's identity. The injection class is
admitted only opt-in precisely because honest detection of it needs taint we
have deliberately chosen not to build.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **New `insecure-construction` category, syntactic-smell subset only (chosen)** | Preserves family design; adopts the detectable slice; no engine check; scope-guarded against SAST drift; no naming collision | A new enum variant + schema arm; only ~6 rules ship first wave (the rest deferred/out) |
| Reuse `security` as the category name | One obvious word | Collides with SEC module + `security.yml`; ambiguous in logs/docs |
| File security items under existing categories (`code-quality` etc.) | No enum change | Breaks family-as-concept; nonsensical nudges; agents game category boundaries |
| Adopt all 36 items, approximate taint with aggressive regex/AST | "Full coverage" | Fails §16.5 #9 FP bar; turns Anvil into a noisy pseudo-SAST; contradicts warnings-over-blocks |
| Reject — leave security to the SEC CI pipeline (Semgrep) | Zero new surface; Semgrep already in `security.yml` | Loses the agent-escape-hatch slice that *is* Anvil-shaped (weak-crypto/DOM-XSS/JWT smells at write-time) |
| New dedicated engine check (like `secret`) | Isolation | Unjustified — regex + AST tiers already cover these detections |

## Consequences

- **Positive:** Anvil gains a coherent, low-FP security slice (weak-crypto,
  DOM-XSS, JWT-none) at write-time and gate-time, reusing existing scanners. The
  scope boundary makes the "why not full SAST" answer explicit and durable. The
  sec-context CWE/severity data flows straight into existing fields.
- **Negative:** A new enum variant and TS schema arm to maintain. Only a small
  first wave ships; the larger sec-context list is deliberately *not* adopted,
  which may read as incompleteness without the scope note. The injection class
  is opt-in, so it will not protect users who do not enable it.
- **Risks:**
  - *FP creep* — insecure-RNG and injection smells are high-FP; mitigated by
    shipping them opt-in and AST-tiered, with allowlists, behind the dogfood FP
    bar.
  - *Scope drift* — pressure to "just add reflected XSS" will recur; mitigated
    by the explicit out-of-model list here and in the scope guard, requiring a
    new ADR to cross it.
  - *Category-name churn* — if `insecure-construction` is later disliked,
    renaming an enum variant + registry values is a coordinated change; mitigated
    by deciding the name now.
- **Mitigations:** opt-in + AST-tier for high-FP families; scope-guard note;
  dogfood FP bar (§16.5 #9) gates each family before enabled-by-default.

## References

- Related ADRs: ADR-071 (AST-aware anti-pattern detection — the gate-time tier
  these families reuse), ADR-029 (suppression parser authority), ADR-061
  (save-time vs gate-time tiering)
- APS modules: SEC (`plans/modules/security.aps.md` — the CI pipeline, distinct
  from this category), LANGTS-006 (`dynamic-execution` family precedent)
- Brainstorm: `plans/brainstorms/2026-06-18-sec-context-antipattern-triage.md`
- External: [Arcanum-Sec/sec-context](https://github.com/Arcanum-Sec/sec-context)
  `ANTI_PATTERNS_BREADTH.md`
