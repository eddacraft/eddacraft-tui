<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Tail Language T1 Wave 2 — WebAssembly text + Zig (Track 2)

| ID    | Owner      | Status      |
| ----- | ---------- | ----------- |
| LTW2  | joshuaboys | In Progress |

**Last reviewed:** 2026-06-29

> Re-entry / addition recorded in
> [ADR-093](../decisions/093-tail-wave-2-wasm-text-and-zig-reentry.md) (**Accepted**
> 2026-06-29) — an owner-directed addition of two languages to the Track 2 tail
> at **T1 (Parsed)**. Zig **re-enters** from the design §13 cut list; WebAssembly
> text was never previously a candidate.
>
> **Audit complete 2026-06-29 (LTW2-001 Done).** Both grammars bind + parse
> tree-sitter 0.26 — **wave membership is both languages**. Owner accepted
> including **WAT via a vendored, dormant-since-2022 ABI-13 grammar**
> ("better than nothing"); Zig ships from the published `tree-sitter-zig` crate.
>
> **Wiring merged:** **LTW2-003 (Zig) via #2996** and **LTW2-002 (WAT) via #3000**
> — both T1-parsed and graph-included on `main`. WAT's grammar FFI is isolated in
> the new `anvil-grammar-wat` crate so the kernel keeps `forbid(unsafe_code)`.
> **All work items merged** (LTW2-001..005); the external-corpus smoke
> (LTW2-004, #3013) ran ~2,527 real OSS files with **0 panics**. Module stays
> In Progress only pending a release tag → Complete.
> Module stays **In Progress** pending those + a release tag. See the audit table
> below for evidence.

## Grammar Maturity Audit (LTW2-001) — COMPLETE 2026-06-29

Per the LANGTAIL precedent (council finding C-005), no grammar ships until it
binds the workspace `tree-sitter` 0.26 runtime and parses a representative
multi-line fixture without an error tree. **Verdict: both bind and parse cleanly
— wave membership is both languages.** Verified by an isolated bind+parse spike
against tree-sitter 0.26 (`set_language` + parse a real fixture, assert no error
tree).

| Language | Grammar source | Version | ABI bind | Parse | Verdict |
|----------|----------------|---------|----------|-------|---------|
| WebAssembly text (`.wat`/`.wast`) | [`wasm-lsp/tree-sitter-wasm`](https://github.com/wasm-lsp/tree-sitter-wasm) `wat/parser.c` — **vendored** (no published crate) | repo @ last commit 2022-05-17, ABI 13 | ✓ `bind=OK` | ✓ `has_error=false` (`ROOT → module → module_field_func → …`) | **Include (vendored)** |
| Zig (`.zig`/`.zon`) | [`tree-sitter-zig`](https://crates.io/crates/tree-sitter-zig) (crates.io, `tree-sitter-grammars` org) | 1.1.2 (2024-12-22), `tree-sitter-language ^0.1` | ✓ `bind=OK` | ✓ `has_error=false` (`source_file → function_declaration → …`) | **Include** |

**Findings driving the wiring:**

- **WAT** is **capability-clean but a supply liability.** No published crate, so
  it is **vendored in-tree** (`parser.c` + `tree_sitter/parser.h`, ~40k lines,
  **no external scanner** — simple `cc` build). ABI 13 sits at tree-sitter
  0.26's supported floor (13–15), so it loads today; a future runtime that drops
  ABI 13 would force a regenerate. Upstream is **dormant since 2022**. Owner
  accepted the vendoring + maintenance liability (2026-06-29, "better than
  nothing"). Binding code confirmed by the spike:
  `tree_sitter_language::LanguageFn::from_raw(tree_sitter_wat)`.
- **Zig** is clean all round — published, maintained, modern binding
  (`tree_sitter_zig::LANGUAGE.into()`); standard LANGTAIL wiring shape.
- Pin the bind+parse check as a permanent regression guard alongside
  `parser::languages::tests::tail_wave_grammars_bind_and_parse` (LTW2-002/-003).

## Purpose

Bring two tail languages to **T1 (Parsed)** in a single batched wave, following
[`lang-tail-wave`](./lang-tail-wave.aps.md) (LANGTAIL, PR #2757) exactly. T1
means: tree-sitter grammar wired in `parser/languages.rs`, file detected via
`Language::from_path`, basic symbol extraction (functions, types, imports), file
appears in the symbol graph. **No** per-language anti-pattern catalogue,
suppression syntax, or policy hooks — that is T2/T3 anchor work, out of scope for
the tail per design §8.2.

Batching amortises the test harness, fixtures, and graph-inclusion validation
across both languages instead of paying it twice.

## In Scope

- **WebAssembly text format only** — extensions `.wat`, `.wast`. The binary
  format `.wasm` is **explicitly excluded** (not source; would feed binary bytes
  to a text grammar — ADR-093 §Decision point 2). `Language::from_path` must not
  map `.wasm`.
- **Zig** — extensions `.zig`, `.zon`. Import shape `@import(...)` /
  `@cImport(...)`; `pub` declarations.
- One `Language` arm per language + `from_path` mapping + `ts_language()`
  binding.
- Basic symbol/import extraction per language (`parser/extract/wat.rs`,
  `parser/extract/zig.rs`), reusing the `tail_common.rs` helpers.
- Inclusion in the `architecture-validate` symbol graph (automatic — the
  parseable gate already delegates to `Language::from_path`).
- One fixture per language under `crates/anvil-kernel/tests/fixtures/langtail2/`.
- Reconcile the stale `docs/public/anvil/overview.md:117` "Repo language
  profile" copy (LTW2-005).

## Out of Scope

- The binary `.wasm` format (see In Scope).
- T2 anti-pattern catalogues, suppression syntax, policy hooks, layer/boundary
  enforcement, drift baselines — for either language. Zig's archived module
  listed a 6-pattern T2 catalogue; **that does not re-enter here** (ADR-093
  §Decision point 4). T2 promotion is a separate future decision under a demand
  signal.
- Zig `build.zig` module-graph / comptime analysis, `@cImport` C-interop
  analysis.

## Interfaces

**Depends on:**

- [`lang-tail-wave`](./lang-tail-wave.aps.md) (LANGTAIL) — the extractor
  abstraction, `tail_common.rs` helpers, grammar-versioned AST cache key, and
  fixture/acceptance harness this wave reuses.
- Existing kernel parser and symbol graph.

**Exposes:**

- Up to two more tail languages parsed and graph-included.
- Per-grammar maturity findings (informs any future promotion decision).

## Prerequisites

- LANGTAIL complete (the wave-1 harness and extractor abstraction exist) — met.
- [ADR-093](../decisions/093-tail-wave-2-wasm-text-and-zig-reentry.md) Accepted.
- Grammar maturity audit (LTW2-001) passed for each candidate before it ships.

## Ready Checklist

Module **In Progress** — **all work items (LTW2-001..005) have merged**; the
only remaining step is a release tag before Complete:

- [x] ADR-093 Accepted by owner (2026-06-29).
- [x] Owner named.
- [x] LTW2-001 (grammar maturity audit) **Done** — both grammars bind + parse
      tree-sitter 0.26; wave membership is both languages.
- [x] LTW2-003 (Zig) **Merged via #2996** and LTW2-002 (WAT) **Merged via #3000**;
      both T1-parsed + graph-included on `main`, fixtures in
      `tests/fixtures/langtail/`.
- [x] LTW2-005 (doc reconciliation) **Merged via #3006** — `overview.md`
      language-profile copy reconciled.
- [x] LTW2-004 (external-corpus smoke) **Merged via #3013** — ~2,527 real OSS
      files, 0 panics; evidence under `plans/reviews/`.

## Work Items

Delivered as a single wave (one branch/PR) per the module's amortisation intent
— the languages share `Language`/dispatch hot files, so a batch lands cleaner
than per-language PRs.

### LTW2-001 — Grammar maturity audit; finalise wave membership

- **Status:** Done (2026-06-29 — both Include; see audit table)
- **Intent:** Confirm each candidate grammar binds tree-sitter 0.26 and parses a
  representative fixture, pinning version + verdict per language.
- **Expected Outcome:** The audit table is resolved (pinned grammar source +
  Include/Drop per language); a bind+parse regression guard exists; any dropped
  language is recorded as cut-from-wave. **Met 2026-06-29:** both Include, none
  dropped.
- **Validation:** `cargo test -p anvil-kernel parser::languages::tests`
- **Files:** `Cargo.toml`, `crates/anvil-kernel/Cargo.toml`,
  `crates/anvil-kernel/src/parser/languages.rs`
- **Confidence:** high (audit complete — both grammars bind + parse; ABI
  compatibility now verified)

### LTW2-002 — Wire WebAssembly text (`.wat`/`.wast`)

- **Status:** Merged 2026-06-30 via PR #3000
- **Intent:** WebAssembly text files are detected, parsed, and their symbols
  appear in the graph.
- **Expected Outcome:** `Language::Wat` arm + `from_path` mapping for
  `.wat`/`.wast` (**not** `.wasm`) + `ts_language()` binding + extractor +
  fixture; a `.wat` fixture's symbols appear in the kernel symbol graph.
- **Validation:** `cargo test -p anvil-kernel`
- **Files (as merged):** new crate `crates/anvil-grammar-wat/` (vendored
  `parser.c` + `tree_sitter/parser.h` + `LICENSE`, `build.rs` `cc` compile, safe
  `language()`); `crates/anvil-kernel/src/parser/{languages.rs,extract/wat.rs,extract/mod.rs}`;
  fixture `crates/anvil-kernel/tests/fixtures/langtail/calc.wat`; `ACKNOWLEDGEMENTS`.
- **Dependencies:** LTW2-001 (Done)
- **Confidence:** high — Council-reviewed (kernel-maintainer/adversarial/operations)
- **Note:** WAT has **no published crate**, so `wat/parser.c` (ABI 13, no external
  scanner) is vendored from `wasm-lsp/tree-sitter-wasm` (**Apache-2.0 WITH
  LLVM-exception**, corrected from the audit's MIT) in a dedicated crate that
  isolates the grammar FFI so the kernel keeps `forbid(unsafe_code)`. Extractor
  covers standalone **and** inline import/export forms. Accepted maintenance
  liability (upstream dormant since 2022; `parser.c` SHA-256 pinned in the vendor
  README).

### LTW2-003 — Wire Zig (`.zig`/`.zon`)

- **Status:** Merged 2026-06-29 via PR #2996
- **Intent:** Zig files are detected, parsed, and their symbols appear in the
  graph.
- **Expected Outcome:** `Language::Zig` arm + `from_path` mapping for
  `.zig`/`.zon` + `ts_language()` binding + extractor (`@import` edges, `pub`
  declarations) + fixture; symbols appear in the kernel symbol graph.
- **Validation:** `cargo test -p anvil-kernel`
- **Files (as merged):** `crates/anvil-kernel/src/parser/{languages.rs,extract/zig.rs,extract/mod.rs}`,
  `Cargo.toml` + `crates/anvil-kernel/Cargo.toml` (`tree-sitter-zig = "1.1.2"`),
  `ACKNOWLEDGEMENTS`, fixture `crates/anvil-kernel/tests/fixtures/langtail/greeter.zig`
- **Dependencies:** LTW2-001 (Done)
- **Confidence:** high — published crate; reviewed (fixed the type-annotation vs
  value bug + chained-import capture before merge)

### LTW2-004 — Wave acceptance

- **Status:** Merged 2026-06-30 via PR #3013
- **Intent:** Every included language parses a real-world fixture without
  panicking and appears in the graph via `architecture-validate`.
- **Expected Outcome:** All included-language fixtures green; an external-corpus
  smoke run (the LANGTAIL-008 equivalent) shows 0 panics; evidence recorded
  under `plans/reviews/`.
- **Met 2026-06-30:** the in-harness half landed with the wiring; the external
  smoke ran the production kernel path over ~2,527 real OSS files (zls +
  zig-stdlib; wasm-tools `.wat` + the WebAssembly testsuite `.wast`) — **0
  panics**; ~26k symbols extracted; Zig 6.7% error-trees, WAT/WAST higher
  (script syntax + newer proposals + invalid-by-design fixtures, like LANGTAIL-008
  C/C++). Evidence:
  `plans/reviews/2026-06-30-ltw2-004-external-validation.md`.
- **Validation:** `cargo test -p anvil-kernel` (wave-2 acceptance test) + smoke
  evidence file present
- **Dependencies:** LTW2-002 (Merged), LTW2-003 (Merged)
- **Confidence:** high — 0 panics across the corpus
- **Finding:** `.zon` (build.zig.zon, a data manifest) 100%-error-trees with 0
  symbols under the Zig source grammar — see Open Questions.
- **Note:** the in-harness half landed with the wiring — `calc.wat` + `greeter.zig`
  join the shared `langtail_wave_acceptance` corpus (parse-clean + graph
  inclusion via the production embedded scan), and the `ltw2_grammars_bind_and_parse`
  guard is pinned. **Remaining:** the external real-OSS-corpus smoke (0 panics)
  with evidence under `plans/reviews/`.

### LTW2-005 — Reconcile public language-profile copy

- **Status:** Merged 2026-06-30 via PR #3006
- **Intent:** The public "Repo language profile" copy reflects what the feature
  actually reports.
- **Expected Outcome:** `docs/public/anvil/overview.md` matches the CLI
  **language-profile registry** (`activation/language_profile.rs`) — the feature's
  actual output — adding the missing **Web (HTML/CSS) → supported** and naming the
  unsupported set concretely (Python, Go, C/C++, …). SQL & Markdown stay
  **partial** (both are in the registry).
- **Note (course-correction):** the registry's tiers track **shipped governance
  checks, not parser capability**, so it is **independent of** the kernel parser.
  An initial edit wrongly claimed "Python fully supported" + the tail "parsed"
  (parser capability); a reviewer caught the divergence and it was corrected to
  match the registry. **Finding → follow-up:** the parser now parses Python +
  the LANGTAIL/LTW2 tail, but the registry still reports them `unsupported`
  (Python's entry looks stale post-PYLAN). Reconciling the registry to shipped
  parser/check coverage is a **separate code work item** (`language_profile.rs`),
  not a doc edit — see Open Questions.
- **Validation:** `pnpm docs:check`
- **Files:** `docs/public/anvil/overview.md`
- **Confidence:** high
- **Note:** the doc line is already stale independent of this wave (it predates
  Python T3 + LANGTAIL); can land standalone if the wave is shelved.

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| `wasm-lsp/tree-sitter-wasm` not ABI-compatible with tree-sitter 0.26 (low maintenance activity) | High | LTW2-001 gate; drop WAT from the wave rather than stall it (LANGTAIL hard rule) |
| `tree-sitter-zig` maintenance / regression risk | Medium | Pin a maintained fork version; fixture must include realistic multi-line syntax |
| Someone maps `.wasm` binary expecting symbols | Medium | ADR-093 + In Scope both forbid it; `from_path` test asserts `.wasm` → `None` |
| Half-wave ships and the wave never closes | Low | Wave acceptance is all-or-some; dropped languages re-enter only on a new signal |

## Open Questions

- [x] Does an ABI-0.26-compatible Zig grammar exist as a published crate, or does
      it need vendoring? — **published**: `tree-sitter-zig` 1.1.2 (LTW2-001).
- [x] Are `.wast` (script/test) files in scope, or `.wat` only? — the
      `wasm-lsp/tree-sitter-wasm` repo ships **both** `wat/` and `wast/`
      grammars; LTW2-002 vendors `wat/` first (`.wat`/`.wast` → `Language::Wat`),
      `wast/` reserved if script-format demand appears.
- [x] **Registry vs parser divergence (surfaced by LTW2-005).** The CLI
      language-profile registry (`crates/anvil-cli/src/activation/language_profile.rs`)
      reported Python + the LANGTAIL/LTW2 tail as `unsupported` even though the
      parser handles them (tiers gate on shipped governance, not parsing).
      **Resolved via CIB-123 (owner tier call, Merged via PR #3011):** Python →
      Supported (PYLAN shipped the catalogue, same bar as Rust); the tail stays
      Unsupported but is now *listed* (a tier = shipped language-specific
      governance, not parser capability); no new tier added.
- [ ] **`.zon` mapping adds no value (surfaced by LTW2-004).** `.zon`
      (`build.zig.zon`) is a Zig data-manifest format, not a source file, so it
      100%-error-trees with 0 symbols under the `source_file` grammar. LTW2-003
      mapped `.zon → Language::Zig`; recommend dropping it from
      `Language::from_path` (1-line + the `detects_zig` test) or filing a small
      CIB. No panic risk — purely a no-value mapping. Owner call: fix inline or
      file CIB.
