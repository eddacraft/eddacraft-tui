# sec-context Anti-Pattern Triage

| Field | Value |
| ----- | ----- |
| Date | 2026-06-18 |
| Author | anti-pattern-tracking |
| Status | Brainstorm — feeds ADR-087 and any future catalogue module |
| Source | [Arcanum-Sec/sec-context](https://github.com/Arcanum-Sec/sec-context) `ANTI_PATTERNS_BREADTH.md` (36 items) |

## Purpose

Decide, for each sec-context anti-pattern, **where it lands in Anvil's model**:

- **Covered** — an existing engine check (`secret`) already detects it; do not
  duplicate.
- **Route-out** — belongs to a different surface (dependency-advisory pack, the
  SEC CI pipeline module) or is not a static-code pattern at all.
- **New family** — a genuine catalogue candidate: a behavioural concept Anvil's
  regex or AST tier can detect deterministically in a single changed file.
- **Out of model** — requires taint/data-flow across functions, whole-program
  reachability, or detecting the *absence* of a control. Anvil is a
  deterministic single-file regex + AST-query scanner (ADR-071), **not** a
  taint-based SAST. These are not Anvil-shaped at any tier.

## Detection model recap (the filter)

| Tier | Mechanism | Sees | Cannot see |
| ---- | --------- | ---- | ---------- |
| Regex (save-time + gate) | line / multi-line text match + same-line post-filter | literal tokens, API names | data flow, cross-line context |
| AST (gate-time only, ADR-071) | tree-sitter S-expr query + Rust predicate over one file's tree | structure: attrs, ancestors, sibling nodes | taint across functions, reachability, absence-of-a-thing |
| `secret` check | entropy + named-pattern scan | hardcoded credentials | — |
| `command_safety` | dedicated | dangerous shell construction | — |

The load-bearing constraint: **no taint analysis, no whole-program reachability,
warnings-over-blocks (exit 0 by default), new-edges-only.** Any item whose true
detection needs "untrusted input reaches a dangerous sink" is at best a
*syntactic smell* (high false-positive), and many are simply infeasible.

## Triage table

Legend — **Disp**: C=Covered, R=Route-out, F=New family, X=Out of model.
**FP**: expected false-positive pressure against the §16.5 #9 bar.

| # | sec-context item | CWE | Disp | Target | Tier | FP |
|---|------------------|-----|------|--------|------|----|
| 1 | Hardcoded Passwords / API Keys | 798/259 | C | `secret` check | regex | — |
| 2 | Credentials in Config Files | 798/259 | C | `secret` check | regex | — |
| 3 | Secrets in Client-Side Code | 798 | C | `secret` check | regex | — |
| 4 | Insecure Credential Storage | 798 | X | — (storage mechanism is semantic) | — | — |
| 5 | Missing Secret Rotation | 798 | X | — (process, not code) | — | — |
| 6 | SQL Injection (string concat) | 89 | F | `injection-smell` (new) | AST | **high** |
| 7 | Command Injection | 78 | F | extend `command_safety` + `injection-smell` | AST | **high** |
| 8 | LDAP Injection | 90 | F | `injection-smell` (low priority) | AST | high |
| 9 | XPath Injection | 643 | F | `injection-smell` (low priority) | AST | high |
| 10 | NoSQL Injection | 943 | F | `injection-smell` (low priority) | AST | high |
| 11 | Template Injection (SSTI) | 1336 | F | extend `dynamic-execution` | regex/AST | med |
| 12 | Reflected XSS | 79 | X | — (needs taint) | — | — |
| 13 | Stored XSS | 79 | X | — (needs taint) | — | — |
| 14 | DOM-Based XSS (`innerHTML`, `document.write`) | 79 | F | `unsafe-rendering` (new) | regex | med |
| 15 | Missing Content-Security-Policy | 16 | X | — (absence of config) | — | — |
| 16 | Improper Output Encoding | 79/80 | X | — (context-sensitive, needs taint) | — | — |
| 17 | Weak Password Requirements | 521 | X | — (semantic) | — | — |
| 18 | Missing Rate Limiting | 770 | X | — (absence of control) | — | — |
| 19 | Insecure Session Token Generation | 330 | F | `weak-cryptography` (insecure-RNG rule) | regex | med |
| 20 | Session Fixation | 384 | X | — (flow) | — | — |
| 21 | JWT Misuse (`alg: none`, weak secret) | 287 | F | `jwt-misuse` (new, or rule in `weak-cryptography`) | regex | low |
| 22 | Missing MFA | 287 | X | — (architectural absence) | — | — |
| 23 | Insecure Password Reset Flows | 287 | X | — (flow) | — | — |
| 24 | Deprecated Algorithms (MD5/SHA1/DES) | 327/328 | F | `weak-cryptography` (new) | regex | med |
| 25 | Hardcoded Encryption Keys | 798 | C | `secret` check (extend patterns) | regex | — |
| 26 | ECB Mode Usage | 327 | F | `weak-cryptography` | regex | low |
| 27 | Missing / Weak IVs/Nonces | 330 | X | — (semantic) | — | — |
| 28 | Rolling Your Own Crypto | 327 | X | — (not generically detectable) | — | — |
| 29 | Insecure Random Number Generation | 330 | F | `weak-cryptography` (insecure-RNG) | regex | **high** |
| 30 | Improper Key Derivation | 326 | X | — (semantic) | — | — |
| 31 | Missing Server-Side Validation | 20 | X | — (absence) | — | — |
| 32 | Improper Type Checking | 1284 | X | — (semantic; overlaps `type-evasion`) | — | — |
| 33 | Missing Length Limits | 1284 | X | — (absence) | — | — |
| 34 | Regex Denial of Service (ReDoS) | 1333 | F | `redos` (niche; regex-over-regex) | AST | med |
| 35 | Accepting/Processing Untrusted Data | 20 | X | — (too broad) | — | — |
| 36 | Missing Canonicalization | 180 | X | — (semantic) | — | — |

## Tally

- **Covered (secret check): 4** — #1, 2, 3, 25. Possibly extend secret patterns
  for #25.
- **Out of model: 18** — taint-class (#12, 13, 16, 35), absence-class (#15, 18,
  22, 31, 33), and semantic/flow (#4, 5, 17, 20, 23, 27, 28, 30, 32, 36).
- **New-family candidates: 14** — but only a *minority* are low-FP enough to
  ship enabled-by-default:
  - **Low/med FP, regex, viable first wave:** #24 (deprecated algos), #26 (ECB),
    #21 (JWT none), #14 (DOM-XSS sinks), #11 (SSTI → extend `dynamic-execution`).
  - **High FP, AST, opt-in or deferred:** #6, 7, 8, 9, 10 (injection smells —
    syntactic only, no taint), #29/#19 (insecure RNG — most `Math.random` is
    non-security), #34 (ReDoS).

## Proposed family layout (feeds ADR-087)

Under one **new category** (name TBD in ADR — *not* `security`, which collides
with the SEC CI-pipeline module; proposed `insecure-construction`):

| Family | Concept (spectrum) | Items | Default tier |
| ------ | ------------------ | ----- | ------------ |
| `weak-cryptography` | Using broken/inappropriate crypto primitives | 24, 26, 19, 29 | regex, mostly enabled; insecure-RNG opt-in (FP) |
| `unsafe-rendering` | Writing untrusted data into a markup/exec sink | 14 | regex, enabled |
| `injection-smell` | Constructing a query/command by interpolation | 6, 7, 8, 9, 10 | AST, **opt-in** (smell, not proof) |
| `jwt-misuse` | Defeating token integrity | 21 | regex, enabled |
| extend `dynamic-execution` | (existing) eval/SSTI | 11 | regex |
| extend `command_safety` | (existing) shell construction | 7 | — |

## Honest scoping note

Roughly **half** of sec-context is not Anvil-shaped — it needs taint analysis
(Semgrep/CodeQL territory) or detects the absence of a control. The defensible
catalogue is the **syntactic-smell subset**: weak-crypto API names, insecure-RNG
calls, DOM-XSS sinks, JWT `none`, and (opt-in) interpolation-into-sink smells.
The injection class ships **opt-in** because without taint it is a smell, not a
finding, and would fail the false-positive bar enabled-by-default. This is the
warnings-over-blocks posture, not a gap to be closed later by adding taint —
adding taint would be a different product (see ADR-087 scope boundary).
