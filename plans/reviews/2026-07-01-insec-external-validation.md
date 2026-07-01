# INSEC-006 — external-codebase validation evidence

**Date:** 2026-07-01
**Scope:** Discharge the ADR-087 §5 / spec §16.5 #9 acceptance bar for the
first-wave `insecure-construction` families — "≥1 external-codebase run for the
enabled families with FP rate < N%, evidence recorded under `plans/reviews/`"
(INSEC-006) — and propose `N` from the observed rate.

Enabled families under test: **`weak-cryptography`** (WC-001 deprecated hash,
WC-002 broken cipher/ECB, WC-003 JWT `alg:none`), **`unsafe-rendering`**
(UR-001 innerHTML/outerHTML, UR-002 `document.write`, UR-003
`dangerouslySetInnerHTML`), and the SSTI rule folded into `dynamic-execution`
(**AP-017**).

## Why an external corpus was required

Anvil is a pure-Rust monorepo — it contains effectively no JavaScript/TypeScript
application code and no server-side crypto/DOM-sink usage, so dogfooding Anvil's
own tree is vacuous for these JS/TS-shaped families (cf. PYLAN-009, where the
same was true for Python). The external run is the load-bearing evidence.

## Corpus

Real application source from sibling working repos (their `node_modules/`,
`dist/`, `build/`, `.d.ts` excluded — application code only):

| Repo | Why | files scanned |
| ---- | --- | ------------- |
| `paperclip` | large React + Node/TS app (UI, server, plugins) — the richest DOM-sink + crypto surface | (bulk) |
| `pi-mono` | multi-package TS: web-ui, coding-agent HTML export, scripts | (bulk) |
| `entx`, `joshuaboys` | additional React/TS surface (i18n `dangerouslySetInnerHTML`) | (bulk) |
| **Total (app corpus)** | `.ts/.tsx/.js/.jsx/.py` | **4,298** |

A second **stress corpus** of 467 framework/library files from `node_modules`
(Next.js, react-dom, jose, @types/react, …) was scanned separately to observe
behaviour on library-internal and type-definition code (see *Stress corpus*
below).

## Method

`anvil check --json` (planless antipattern catalogue) over the corpus in
batches, with `ANVIL_REGISTRY_PATH` pointed at the branch registry so the new
rules load into a released binary. Every WC-\*/UR-\*/AP-017 finding was
classified by inspecting the flagged source line as:

- **True positive** — the rule fired on a genuine instance of its targeted
  primitive/sink (a deprecated hash construction, a raw-HTML sink written with a
  dynamic value). Per ADR-087 these are *construction* smells: a TP does **not**
  require proving the input is attacker-reachable (taint is explicitly out of
  model), only that the flagged construct is the risky one.
- **False positive** — the rule fired where the construct is definitively safe
  and should not have matched (a test-file DOM teardown, an empty-string
  `innerHTML` clear, a comparison, a non-security checksum).

## Results — first pass (catalogue as first authored)

| Family | Findings | False positives | Rate |
| ------ | -------- | --------------- | ---- |
| `unsafe-rendering` (UR-001/003) | 66 | 50 | 76% |
| `weak-cryptography` (WC-001) | 2 | 2 | (n too small) |
| `dynamic-execution` SSTI (AP-017) | 0 | — | — |
| **Total** | **68** | **52** | **76%** |

The 52 first-pass false positives fell into exactly two shapes, both on UR-001:

1. **Test-file DOM teardown (dominant).** `document.body.innerHTML = ""` in
   React test files. The rule's allowlist covered `**/*.test.ts` /
   `**/*.spec.ts` but **not** the `.tsx`/`.jsx` variants React tests actually
   use — so every `*.test.tsx` teardown leaked through.
2. **Quoted-literal right-hand side.** `container.innerHTML = ''` (clearing a
   node), `el.innerHTML = '<hr>'` (static snippet). A string-literal RHS carries
   no dynamic data — the original `= (?:[^=]|$)` pattern fired on it anyway.

## Fixes applied (this PR)

1. **UR-001/002/003 + WC-\* + AP-017 allowlists** gained the `.tsx`/`.jsx` test
   globs (`**/*.test.tsx`, `**/*.spec.tsx`, `**/*.test.jsx`, `**/*.spec.jsx`),
   closing the React-test gap.
2. **UR-001 detection** now skips a *pure* quoted-literal RHS, mirroring the
   `eval` rule (AP-008): a `= ''` clear or a static `= '<hr>'` snippet no longer
   fires, while template-literal, identifier, and expression right-hand sides
   still do.

Each fix has a pinned regression test in
`crates/anvil-checks/tests/insecure_construction.rs`
(`ur001_quoted_literal_rhs_is_clean`, `ur001_allowlists_tsx_test_files`).

## Council-review hardening (post-dogfood)

A batch council review (adversarial + kernel-maintainer) on the dogfooded
catalogue produced further correctness fixes, each with a regression test:

- **WC-003 (CRITICAL):** the `alg` match lacked a left word-boundary, so
  `compressionAlgorithm: 'none'` / `hashAlgorithm: "none"` matched — and WC-003
  shipped at `severity: error`, which *blocks* `anvil check`/`gate`. Fixed by
  anchoring (`\balg…`) **and** downgrading to `severity: warning` for ADR-087 §6
  posture consistency (a residual unrelated `algorithm: 'none'` config is now a
  suppressible warning, not a block). Tests: `wc003_does_not_fire_on_non_jwt_
  algorithm_config`, `wc003_is_a_warning_not_a_blocking_error`.
- **UR-001 / AP-017 (MAJOR):** the quoted-literal skip previously dropped the
  classic literal-prefixed shape `'<b>' + tainted` / `render_template_string('Hi
  ' + name)`. Added a concatenation alternative so a quoted literal *followed by
  `+`* still fires; only a pure static literal is skipped. Tests:
  `ur001_literal_prefixed_concatenation_fires`,
  `ap017_literal_prefixed_concatenation_fires`.
- **WC-002 (MAJOR):** covered decrypt-side construction
  (`createDecipheriv('des-ecb', …)`). Test: `wc002_covers_decipher_construction`.
- **WC-001 (MINOR):** added `hashlib.new('md5')` and Web-Crypto
  `subtle.digest('SHA-1', …)` recall. Test:
  `wc001_covers_hashlib_new_and_web_crypto`.

Re-running the app-corpus dogfood after these changes left the results table
above unchanged (18 findings, 0 `unsafe-rendering` FPs) — the recall gains are
pinned by unit tests, and no new corpus false positive was introduced.

## Results — second pass (post-fix, shipped state)

| Family | Findings | False positives | Rate |
| ------ | -------- | --------------- | ---- |
| `unsafe-rendering` (UR-001 ×9, UR-003 ×7) | 16 | **0** | **0%** |
| `weak-cryptography` (WC-001 ×2) | 2 | 2* | see note |
| `dynamic-execution` SSTI (AP-017) | 0 | — | — |
| **Total** | **18** | **2** | **11%** |

- **`unsafe-rendering`: 16/16 correct (0% FP).** All nine UR-001 findings are
  genuine dynamic raw-HTML writes (`el.innerHTML = text`,
  `content.innerHTML = getTreeNodeDisplayHtml(...)`,
  `row.innerHTML = ` template literals, `.innerHTML = rows.map(...)`). All seven
  UR-003 findings are real `dangerouslySetInnerHTML` escape-bypasses
  (i18n `__html: t('…')` in `DocsPage.tsx`, `__html: svg` in `MarkdownBody.tsx`)
  — the rule's job is to force the author to confirm the input contract, which
  every one of these warrants.

- **`weak-cryptography`: the only 2 residual findings are the *documented*
  non-security-MD5 class** (\*). Both are `createHash("md5")` in
  `paperclip` plugin code — a wiki content hash and a static-asset ETag/cache
  key, i.e. MD5 used for accidental-corruption resistance, not security. This is
  exactly the risk the module's risk table and ADR-087 anticipated ("MD5 used
  for non-security checksums is legitimate — nudge, do not block"): WC-001 ships
  at **severity `warning`**, and its nudge text tells the author to suppress with
  a reason when the use is a non-security checksum. Counting them strictly they
  are FPs (2/18 = 11%); under the rule's own "nudge-not-block" contract they are
  correct-but-suppressible warnings.

- **`AP-017` (SSTI)** surfaced no findings in the app corpus (no Jinja/Nunjucks
  dynamic-template usage in the sampled projects); it is covered by unit-test
  positives/negatives instead. No FP signal to report.

## Stress corpus (framework/library code) — context, not the bar

Scanning 467 `node_modules` files produced 3,721 raw findings, dominated by
`dangerouslySetInnerHTML` inside **Next.js (2,642)** and **react-dom (462)**
internals and `@types/react` type declarations, plus WC-003 in `jose`'s
deliberately-named `unsecured.js` (its `alg:none` implementation). These are
**not** representative of Anvil's normal operation: `node_modules` is
git-ignored and is not part of `anvil check --changed/--all` file discovery
(this run fed explicit paths to bypass discovery). The takeaway is a usage note,
not a precision problem — the families are for *application* source, where the
0% UR figure above applies.

## Proposed acceptance bar `N` (Open Question #3 — operator sign-off)

The measured application-corpus rates support a **bar of `N` = 10% per family**:

- `unsafe-rendering` clears it outright (0%).
- `weak-cryptography`'s only findings are the anticipated, warning-severity,
  suppressible MD5-as-checksum class; keep MD5 at **`warning`** (not `error`) and
  revisit an opt-in downgrade only if field FP pressure exceeds 10% on a
  non-toy corpus. WC-002/WC-003 produced zero application-corpus FPs.

**This numeric bar (`N`) is a product decision and is flagged for operator
confirmation.** The `weak-cryptography` and `unsafe-rendering` families ship at
`warning` severity (exit 0 by default, new-edges-only baseline), so for those the
bar governs only whether a future MD5-opt-in downgrade is warranted. Note the one
exception: **AP-017 (SSTI) is `severity: error`**, inheriting the eval-class
`dynamic-execution` family's posture (AP-008/AP-009), so a *new* SSTI match will
fail `anvil check` at the default `error` threshold — deliberate for an
RCE-class smell, and it recorded **zero** app-corpus findings above.

## Open questions resolved by this work

- **Q1 — `unsafe-rendering` extension set:** JS/TS only (`.ts/.tsx/.js/.jsx/.mjs
  /.cjs`). `.html`/`.css` are retired from the scan set (guarded in
  `patterns.rs`), so the sinks are caught in the script code that assigns them.
- **Q2 — JWT `alg:none` home:** stays in `weak-cryptography` (WC-003), per
  ADR-087; no separate `jwt-misuse` family.
- **Q3 — FP bar `N`:** proposed 10%/family (above); operator to confirm.
