# ADR-093: Tail T1 wave 2 — WebAssembly text + Zig re-entry

## Status

**Accepted** — 2026-06-29, Josh (owner). Records an owner-directed
addition of two languages to the Track 2 tail at **T1 (Parsed)**:

1. **WebAssembly text format** (`.wat` / `.wast`) — never previously a
   candidate in the
   [2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md).
2. **Zig** (`.zig` / `.zon`) — **re-entry** from the §13 cut list
   (archived 2026-04-22 at
   [`lang-zig.aps.md`](../archive/modules/lang-zig.aps.md), "zero demand").

Both land as a single batched wave following the
[LANGTAIL precedent](../modules/lang-tail-wave.aps.md) (PR #2757), gated on a
grammar maturity audit per council finding C-005.

**Audit outcome (LTW2-001, 2026-06-29):** the gate passed for **both** —
each grammar binds tree-sitter 0.26 and parses a real fixture with no error tree
(spike-verified). Wave membership is both languages. Zig ships from the
published `tree-sitter-zig` 1.1.2 crate. **WAT has no published crate**, so the
owner accepted **including it via a vendored grammar** (`wasm-lsp/tree-sitter-wasm`
`wat/parser.c`, ABI 13, no external scanner; upstream dormant since 2022) —
capability is clean, the cost is an in-tree maintenance liability. See the
[LTW2 module](../modules/lang-tail-wave-2.aps.md) audit table for evidence.

## Date

2026-06-29

## Context

The design's §13 re-entry rule reads: *"Items on the cut list stay cut until a
demand signal appears. No silent re-adds. If a new early-access user brings one
back, it re-scores under the §6 criteria like any other candidate."* WebAssembly
text was never even a candidate (it is neither in §7.2 anchors nor §8.2 tail).

This ADR exists so neither addition is a *silent* re-add. The trigger here is
**not** a new early-access user — it is a direct **owner (Josh) strategic
directive** (2026-06-29). The §6 re-score is recorded honestly below: confirmed
user demand for both is **0**; the decision rests on owner direction plus a thin
"governs systems / edge code" strategic narrative, not on invented demand.

### §6 re-score (honest)

| Candidate | Demand | Blast | Strategic | Pack unlock | Note |
|---|---|---|---|---|---|
| WebAssembly text (`.wat`/`.wast`) | 0 confirmed (Anvil has no `.wat` files) | Low–medium | supports (edge/Wasm narrative) | 0 | Owner-directed. Source-format only; see scoping below. |
| Zig (`.zig`/`.zon`) | 0 confirmed | Medium (systems code) | supports ("governs systems code"; bun, TigerBeetle) | 0 | Owner-directed re-entry; supersedes the §13 cut. |

Neither would clear the §6 bar on demand alone. They proceed because the owner
has authority to override the cut list; this record makes that override explicit
rather than dressing it as user demand.

## Decision

1. **Add WebAssembly *text* format and Zig to the Track 2 tail at T1**, batched
   as a second wave (`lang-tail-wave-2`, scope `LTW2`), amortising the test
   harness and fixtures exactly as LANGTAIL did.

2. **WebAssembly scope is the text format only — `.wat` and `.wast`.** The
   binary format `.wasm` is **explicitly excluded**: it is not source, and
   feeding binary bytes to a text grammar yields error trees, not symbols.
   `Language::from_path` must **not** map `.wasm`.

3. **Grammar maturity audit is the gate (C-005 precedent).** Before either
   language ships, its candidate grammar must bind the workspace `tree-sitter`
   0.26 runtime and parse a representative multi-line fixture without an error
   tree, pinned as a regression guard (as
   `parser::languages::tests::tail_wave_grammars_bind_and_parse` does for wave
   1). Candidate crates:
   [`wasm-lsp/tree-sitter-wasm`](https://github.com/wasm-lsp/tree-sitter-wasm)
   (WAT + WAST) and a maintained `tree-sitter-zig`. ABI compatibility with
   tree-sitter 0.26 was UNVERIFIED at authoring — if a grammar failed to bind it
   would be **dropped from the wave** rather than allowed to stall it (the
   LANGTAIL hard rule), and the module stayed **Proposed** until the audit
   passed. **Resolved 2026-06-29 (see the Status audit-outcome note above):**
   both grammars bound + parsed cleanly, so neither was dropped and the wiring
   items are now Ready.

4. **T1 only.** No per-language anti-pattern catalogue, suppression syntax, or
   policy hooks — that is T2/T3 anchor work, out of scope for the tail, per
   design §8.2. Zig's archived module listed a 6-pattern T2 catalogue; that is
   **not** re-entering with it — only the T1 parse/extract/graph-inclusion slice
   does. Promotion to T2 remains a separate future decision under a demand
   signal.

5. **Supersede the §13 cut for Zig.** The design's "Cut entirely" line and the
   archived `lang-zig.aps.md` are annotated as re-entered via this ADR. The
   archived module content folds into `lang-tail-wave-2` (mirroring how the six
   per-language `lang-*` modules folded into LANGTAIL), so no `lang-zig.aps.md`
   is un-archived as a standalone active module.

## Consequences

- Two new `Language` arms, `from_path` extension mappings, `ts_language()`
  bindings, extractors (`parser/extract/{wat,zig}.rs`), and one fixture each —
  the standard LANGTAIL per-language shape.
- `+2` grammar crates in `Cargo.toml` (subject to the audit), an
  `ACKNOWLEDGEMENTS` regen, and a `Language::from_path` parseable-gate extension
  (the gate already delegates to `from_path`, so graph inclusion is automatic).
- The public `docs/public/anvil/overview.md:117` "Repo language profile" copy is
  already stale (it omits Python T3 and the wave-1 tail) and is reconciled as a
  work item in the new module — independent of, but landed with, this wave.
- If either grammar fails the 0.26 ABI audit, the wave ships with one language
  (or is shelved), and this ADR is amended to record the drop.

## Alternatives considered

- **Map `.wasm` binary too.** Rejected — binary is not parseable as text; would
  produce noise and false "supported" signals.
- **Standalone per-language modules.** Rejected — the LANGTAIL amortisation
  argument (share the harness/fixtures/validation) applies equally to a
  two-language wave.
- **Skip the ADR, just edit the enum.** Rejected — violates the §13 "no silent
  re-adds" rule and the design's "no mystery modules / named reason" success
  criterion.
