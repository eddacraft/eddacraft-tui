<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Tail Language T1 Wave 2 — WebAssembly text + Zig (Track 2)

| ID    | Owner      | Status   |
| ----- | ---------- | -------- |
| LTW2  | joshuaboys | Proposed |

**Last reviewed:** 2026-06-29

> Re-entry / addition recorded in
> [ADR-093](../decisions/093-tail-wave-2-wasm-text-and-zig-reentry.md) — an
> owner-directed addition of two languages to the Track 2 tail at **T1
> (Parsed)**. Zig **re-enters** from the design §13 cut list; WebAssembly text
> was never previously a candidate. Module stays **Proposed** until the grammar
> maturity audit (LTW2-001) passes; see ADR-093 §Decision point 3.

## Grammar Maturity Audit (LTW2-001) — PENDING

Per the LANGTAIL precedent (council finding C-005), no grammar ships until it
binds the workspace `tree-sitter` 0.26 runtime and parses a representative
multi-line fixture without an error tree. **ABI compatibility with tree-sitter
0.26 is UNVERIFIED for both candidates** — this audit is the gate. If a grammar
fails to bind, that language is **dropped from the wave** rather than allowed to
stall it.

| Language | Candidate grammar crate | Version | ABI bind | Verdict |
|----------|-------------------------|---------|----------|---------|
| WebAssembly text (`.wat`/`.wast`) | [`wasm-lsp/tree-sitter-wasm`](https://github.com/wasm-lsp/tree-sitter-wasm) (WAT + WAST) | TBD | ⏳ pending | TBD |
| Zig (`.zig`/`.zon`) | maintained `tree-sitter-zig` | TBD | ⏳ pending | TBD |

Resolve to a pinned crate version + verdict per language before promoting the
module to Ready, and pin the bind+parse check as a permanent regression guard
alongside `parser::languages::tests::tail_wave_grammars_bind_and_parse`.

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

Promote Proposed → Ready (then In Progress) only when all are met:

- [ ] ADR-093 Accepted by owner.
- [ ] LTW2-001 grammar maturity audit complete; final wave membership confirmed
      (both, one, or shelved).
- [ ] At least one fixture identified per included language.
- [x] Owner named.

## Work Items

Delivered as a single wave (one branch/PR) per the module's amortisation intent
— the languages share `Language`/dispatch hot files, so a batch lands cleaner
than per-language PRs.

### LTW2-001 — Grammar maturity audit; finalise wave membership

- **Status:** Proposed
- **Intent:** Confirm each candidate grammar binds tree-sitter 0.26 and parses a
  representative fixture, pinning version + verdict per language.
- **Expected Outcome:** The audit table above is resolved (pinned crate version +
  Include/Drop per language); a bind+parse regression guard exists; any dropped
  language is recorded as cut-from-wave.
- **Validation:** `cargo test -p anvil-kernel parser::languages::tests`
- **Files:** `Cargo.toml`, `crates/anvil-kernel/Cargo.toml`,
  `crates/anvil-kernel/src/parser/languages.rs`
- **Confidence:** medium (ABI compat unverified — this item resolves it)

### LTW2-002 — Wire WebAssembly text (`.wat`/`.wast`)

- **Status:** Proposed
- **Intent:** WebAssembly text files are detected, parsed, and their symbols
  appear in the graph.
- **Expected Outcome:** `Language::Wat` arm + `from_path` mapping for
  `.wat`/`.wast` (**not** `.wasm`) + `ts_language()` binding + extractor +
  fixture; a `.wat` fixture's symbols appear in the kernel symbol graph.
- **Validation:** `cargo test -p anvil-kernel`
- **Files:** `crates/anvil-kernel/src/parser/languages.rs`,
  `crates/anvil-kernel/src/parser/extract/wat.rs`,
  `crates/anvil-kernel/src/parser/extract/mod.rs`,
  `crates/anvil-kernel/tests/fixtures/langtail2/`
- **Dependencies:** LTW2-001
- **Confidence:** medium

### LTW2-003 — Wire Zig (`.zig`/`.zon`)

- **Status:** Proposed
- **Intent:** Zig files are detected, parsed, and their symbols appear in the
  graph.
- **Expected Outcome:** `Language::Zig` arm + `from_path` mapping for
  `.zig`/`.zon` + `ts_language()` binding + extractor (`@import` edges, `pub`
  declarations) + fixture; symbols appear in the kernel symbol graph.
- **Validation:** `cargo test -p anvil-kernel`
- **Files:** `crates/anvil-kernel/src/parser/languages.rs`,
  `crates/anvil-kernel/src/parser/extract/zig.rs`,
  `crates/anvil-kernel/src/parser/extract/mod.rs`,
  `crates/anvil-kernel/tests/fixtures/langtail2/`
- **Dependencies:** LTW2-001
- **Confidence:** medium

### LTW2-004 — Wave acceptance

- **Status:** Proposed
- **Intent:** Every included language parses a real-world fixture without
  panicking and appears in the graph via `architecture-validate`.
- **Expected Outcome:** All included-language fixtures green; an external-corpus
  smoke run (the LANGTAIL-008 equivalent) shows 0 panics; evidence recorded
  under `plans/reviews/`.
- **Validation:** `cargo test -p anvil-kernel` (wave-2 acceptance test) + smoke
  evidence file present
- **Dependencies:** LTW2-002, LTW2-003
- **Confidence:** medium

### LTW2-005 — Reconcile public language-profile copy

- **Status:** Proposed
- **Intent:** The public "Repo language profile" copy reflects current coverage.
- **Expected Outcome:** `docs/public/anvil/overview.md:117` no longer claims only
  "TS / JS / Rust supported" — it accounts for Python (T3) and the tail T1
  languages (wave 1, plus any wave-2 additions), honestly. `pnpm docs:index`
  regenerated.
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

- [ ] Does an ABI-0.26-compatible Zig grammar exist as a published crate, or does
      it need vendoring? — resolved by LTW2-001.
- [ ] Are `.wast` (script/test) files in scope, or `.wat` only? Design leans
      both (same grammar); confirm during LTW2-001.
