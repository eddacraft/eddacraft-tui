<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# TypeScript Audit & T3 Calibration (Track 1 anchor zero)

| ID     | Owner | Status | Done |
| ------ | ----- | ------ | ---- |
| LANGTS | —     | Complete | 6/6  |

**Last reviewed:** 2026-06-08 — release-tag reconciliation sweep: LANGTS-002
(#2106 `a34b6231`), -004 (#2125 `79863927`), -005 (#2096 `2fc6b41f`), and
-006 (#1820 `bcb96175`) merge commits all confirmed in the `v0.7.3-beta` tag
(2026-05-31), advancing each to **Released/Shipped via v0.7.3-beta**;
LANGTS-001/-003 are terminal Done audit/checklist artefacts. Module advances
**In Progress → Complete**. Prior review 2026-05-30 — LANGTS-004 (Zod-creep
rules) Merged via PR #2125 as AP-015 (`z.any()` + Zod `.passthrough()`, on by
default) + AP-016 (`z.unknown()`, opt-in), advancing the done count to
**6/6**. The Council renumbered off the retired
`AP-010..AP-013` range and split `z.unknown()` to opt-in (idiomatic + the
recommended `any` alternative) — see the LANGTS-004 spec reconciliation.
2026-05-29 — LANGTS-002 (TS extraction gaps TS-G1/TS-G2:
interface/type/enum + class-method symbols) Merged via PR #2106, advancing
the done count to **5/6** (LANGTS-001, -002, -003, -005, -006 done; -004
done 2026-05-30). Earlier the same day LANGTS-005 (kernel-prerequisite refactor,
K1–K4) Merged via PR #2096 (4/6). Earlier, 2026-05-28:
the two bounded open questions were resolved inline (single module, no
`lang-ts-prereq` split; K1 extractor-trait ADR deferred to RSTLAN per the
audit §8 decision), promoting LANGTS-002, -004, and -005 from
anticipated-shape bullets to Ready work items with grounded Intent / Outcome
/ Scope / Validation.

> **Anchor re-scoring gate run 2026-04-26 (solo, self-review):**
> - **TS still anchor zero** — confirmed. Demand profile unchanged since
>   2026-04-08 spec; Anvil itself is the heaviest TS consumer; Track 4
>   packs (PACKPUL, PACKLLM, PACKDRZ, PACKNXT, PACKHON) all gate on TS
>   T3 substrate.
> - **Rust is catching up faster than the spec assumed** — observed
>   signal. Where the 2026-04-08 design parked RSTLAN at Tier B / Phase 2
>   behind LANGTS + Track 4 packs, the dogfood case for Rust → T3 has
>   strengthened: Anvil's own crates are Rust-substrate-heavy, the
>   intercept daemon is Rust-only, and post-rust-migration coverage gaps
>   on Anvil itself are accumulating. Implication: RSTLAN may warrant
>   earlier promotion than its current Tier B parking suggests.
>   **Action:** flagged for re-evaluation — see followup task. Does not
>   change LANGTS sequencing; LANGTS is still the Phase-1 prerequisite.
> - **Spec bar still applies** — T3 acceptance checklist (just landed at
>   `plans/specs/2026-04-26-t3-acceptance-checklist.md`) is the canonical
>   bar for any anchor; nothing in the gate run suggests loosening.
>
> Status promoted Draft → In Progress → **Ready** 2026-04-26.

## Purpose

Audit TypeScript's current tier and produce the **T3 acceptance checklist**
that Rust and Python anchors must pass. TS is the language Anvil already
partially supports (`crates/anvil-kernel/src/parser/languages.rs`); without an
explicit calibration pass, "T3" is a made-up label. This module is the literal
first work item in the language-and-coverage plan set per
[2026-04-08 Language and Coverage Design](../../specs/2026-04-08-language-and-coverage-design.md)
§7.3, §8.1, §17.3.

The audit also folds in **Zod-creep rules** (`z.any()`, `z.unknown()`,
`.passthrough()`) into the TS T2 anti-pattern catalogue alongside existing
rules for `any`, `as any`, `@ts-ignore`. Zod is cross-cutting infrastructure
used by every TS framework — language-level concern, not pack concern.

This module is the single point of failure flagged by council finding C-007 —
five Track 4 packs gate on TS → T3. Audit must complete before any pack
module starts execution.

## In Scope

- Enumerate current TS capabilities against the nine T3 dimensions
  (grammar, symbol/import extraction, anti-pattern catalogue, suppression,
  entry-point detection, layer/boundary enforcement, policy hook integration,
  drift baseline, `architecture-validate` inclusion) per spec §7.3.
- Identify and close TS-specific gaps surfaced during the audit.
- Publish a **T3 acceptance checklist** as a checked-in artefact (companion
  spec at
  [2026-04-26-t3-acceptance-checklist.md](../../specs/2026-04-26-t3-acceptance-checklist.md);
  audit-derived bar referenced from RSTLAN, PYLAN, every Track 4 pack module,
  and the pack registry per [ADR-027](../../decisions/027-pack-architecture.md)).
- Add Zod-creep rules to TS T2 catalogue.
- Optional: split into a sub-module for the kernel prerequisite work
  identified by council §16.5 #3 (extractor refactor, grammar version in
  cache key, parser thread-safety, panic removal, grammar maturity audit) —
  this becomes Track 1 item 0.5 if the work is large enough to justify a
  module boundary. Audit recommendation (audit §7 OQ2): keep K1..K4 inside
  LANGTS-005 if the trait can land alongside K2..K4 within one sprint;
  split into `lang-ts-prereq.aps.md` if scope grows.

## Out of Scope

- Any other anchor language (Rust, Python — separate modules).
- Pack work (substrate-gated on this module's completion).
- Tooling churn unrelated to T3 calibration.
- Type checker replacement (Anvil is governance, not typing).

## Interfaces

**Depends on:**

- Existing kernel parser (`crates/anvil-kernel/src/parser/`).
- Existing architecture analysis (`crates/anvil-architecture/`).
- Existing OPA policy pipeline.
- Existing drift-baseline mechanism.

**Exposes:**

- [LANGTS audit report](../../specs/2026-04-26-langts-audit-report.md) —
  point-in-time evidence; current TS implementation state, named TS gaps
  (TS-G1..G7), named kernel-prereq gaps (K1..K5), and the recommended ADRs
  the audit believes are missing.
- [T3 acceptance checklist (v1)](../../specs/2026-04-26-t3-acceptance-checklist.md)
  — the durable, re-usable bar every Track 4 pack module and future anchor
  (RSTLAN, PYLAN) references. Load-bearing for Track 1 items 1 and 2 plus
  all Track 4 packs.
- Updated TS extraction layer (LANGTS-002 — closes audit gaps TS-G1..G6 to
  the extent decided per audit §7 OQ1).
- Updated TS T2 anti-pattern catalogue including Zod-creep rules (LANGTS-004
  — closes audit gap TS-G5).
- Extractor trait refactor + cache-key fix + parser thread-safety strategy
  + panic removal underpinning subsequent anchors (LANGTS-005 — closes
  kernel-prereq gaps K1..K4; K5 is recurring governance referenced from
  the T3 checklist §1).

## Prerequisites

None — this is anchor item zero.

## Ready Checklist

Change status to **Ready** when:

- [x] Audit owner named (single accountable owner for the T3 checklist)
      — *audit produced 2026-04-26, see audit report header.*
- [x] Re-scoring gate run per
      [docs/guides/anchor-rescoring-process.md](../../../docs/guides/anchor-rescoring-process.md);
      session owner named for this invocation. *Run 2026-04-26 (solo,
      self-review) — see the gate-run callout at the top of this module.*
- [x] Decision recorded on whether kernel prerequisite work (council §16.5 #3)
      is in-scope here or split into LANGTS-prereq submodule. **Resolved
      2026-05-28 — keep K1..K4 inside LANGTS-005 (single module); the audit
      §5.6 sizing puts K1..K4 within one sprint. Split only if K1 grows past a
      sprint during execution (Risks-table escape hatch).** See Open
      Questions.
- [x] Decision recorded on T3 checklist artefact location — *companion
      spec at `plans/specs/2026-04-26-t3-acceptance-checklist.md`. Re-home
      to `docs/architecture/anvil-t3-acceptance.md` is a follow-up if the
      checklist is referenced from user-facing docs; the spec location
      keeps it next to the audit evidence for now.*

## Work Items

LANGTS-001 and LANGTS-003 carry audit-completed evidence. LANGTS-002, -004,
and -005 were promoted from anticipated-shape bullets to Ready work items
2026-05-28 once the two bounded open questions (single-module-vs-split; the
K1 ADR) were resolved inline above.

### LANGTS-001: Audit current TS capability state across the T3 dimensions — Done

- **Status:** Done (audit pass complete 2026-04-26).
- **Intent:** Enumerate current TS capability state across the nine T3
  dimensions so "T3" is a measured bar, not a label.
- **Evidence:**
  [audit report §3](../../specs/2026-04-26-langts-audit-report.md#3-current-ts-implementation-state).

### LANGTS-002: Close identified TS extraction gaps (TS-G1, TS-G2) — Merged

- **Status:** Released/Shipped via v0.7.3-beta (2026-05-31). Merged 2026-05-29 via PR #2106
- **Intent:** Close the two extraction gaps the audit marked Medium so the TS
  symbol graph carries the shapes Track 4 packs will reason about, and record
  the deferral decision for the rest.
- **Expected Outcome:** The TS extractor in
  `crates/anvil-kernel/src/parser/extract.rs` emits interface / type-alias /
  enum symbols (TS-G1) and method-level symbols on classes (TS-G2). TS-G3
  (dynamic `import()` edges), TS-G4 (namespace re-exports), and TS-G6
  (per-specifier re-export names) are deferred with an explicit
  "deferred for the first T3 iteration" note in the T3 acceptance checklist,
  so RSTLAN/PYLAN are not required to extract analogous shapes while the
  deferral note stands. TS-G7 (entry-point auto-detection) remains
  documentation-only in the checklist.
- **Scope:** `crates/anvil-kernel/src/parser/extract.rs`,
  `crates/anvil-kernel-types/src/graph.rs` (additive `SymbolKind` variants if
  the audit's predicted `SymbolKind::Method` / `TypeAlias` additions are
  needed), the T3 checklist deferral note.
- **Non-scope:** The extractor-trait refactor (that is LANGTS-005); TS-G3 /
  TS-G4 / TS-G6 implementation; type checking.
- **Dependencies:** LANGTS-001 (Done). Sequence-coupled with LANGTS-005 — if
  LANGTS-005 lands first, TS-G1/TS-G2 are implemented against the new
  `LanguageExtractor` TS impl rather than the current monolithic walker; the
  outcome is the same either way.
- **Validation:** `cargo test -p eddacraft-anvil-kernel` passes with new
  extraction fixtures asserting an `interface` / `type` / `enum` declaration
  and a class method each appear in the extracted `FileSymbols`.
- **Confidence:** medium — additive extraction over the existing walker; the
  `SymbolKind` additions are the only contract-surface risk.

### LANGTS-003: Publish the T3 acceptance checklist artefact — Done

- **Status:** Done (publication complete 2026-04-26).
- **Intent:** Publish the durable T3 bar every Track 4 pack and future anchor
  references.
- **Evidence:**
  [`plans/specs/2026-04-26-t3-acceptance-checklist.md`](../../specs/2026-04-26-t3-acceptance-checklist.md).

### LANGTS-004: Add Zod-creep rules to the TS T2 anti-pattern catalogue — Merged

- **Status:** Released/Shipped via v0.7.3-beta (2026-05-31; merge commit
  `79863927` confirmed in tag). Merged 2026-05-30 via PR
  [#2125](https://github.com/eddacraft/anvil-001/pull/2125) — AP-015
  (`z.any()` + Zod `.passthrough()`, on by default) and AP-016
  (`z.unknown()`, opt-in) ship in the `type-system-evasion` family.
- **Intent:** Close audit gap TS-G5 by adding the cross-cutting Zod-creep
  rules so escape hatches in schema definitions trip the gate the same way
  the existing `any` / `as any` / `@ts-ignore` rules do.
- **Spec reconciliation (2026-05-30):** Council review during
  implementation corrected two points before merge:
  - **Renumbered to AP-015 / AP-016.** The obvious next id `AP-010` is a
    retired HTML/CSS pattern id (`AP-010..AP-013`) guarded by
    `crates/anvil-checks/src/antipattern/patterns.rs::retired_html_css_patterns_are_absent`,
    and is independently claimed by a different rule in
    `crates/anvil-policy/src/library.rs`. `AP-014` is a TUI test fixture.
    The new rules take the next clear ids, AP-015 and AP-016.
  - **`z.unknown()` split to an opt-in rule.** Flagging all three patterns
    on by default mis-calibrates: `z.unknown()` has 16 idiomatic
    first-party uses (`z.record(z.string(), z.unknown())`) and is the
    *recommended* safe alternative to `any` — the antipattern scanner does
    not baseline, so an on-by-default rule would warn on legitimate code
    from day one. So **AP-015** (`z.any()` + a Zod-anchored `.passthrough()`)
    ships on by default at `warning`, and **AP-016** (`z.unknown()`) ships
    `opt_in: true` (off by default, `confidence: medium`) for teams that
    want every schema field to carry a contract. All three patterns the
    spec names are still detectable. `.passthrough()` is anchored to a Zod
    receiver on the same line to avoid firing on non-Zod `.passthrough()`
    (streams, mocks).
- **Expected Outcome:** New rules detect `z.any()`, `z.unknown()`, and
  `.passthrough()` and ship in the compiled pattern registry under the
  `type-system-evasion` family. AP-015 (`z.any()` + Zod `.passthrough()`)
  is on by default at `severity: warning` (matching AP-003's TS T2
  placement); AP-016 (`z.unknown()`) is opt-in.
- **Scope:** `patterns/type-system-evasion/AP-015.anvil` + `AP-016.anvil`,
  the family `definition.anvil` rule list/prose, the regenerated
  `patterns/compiled/registry.json`, and scanner code-scoping /
  eslint-suppression parity for the new ids.
- **Non-scope:** Extractor changes (LANGTS-002); the kernel-prereq work
  (LANGTS-005).
- **Dependencies:** LANGTS-001 (Done).
- **Validation:** Gate + scanner fixture tests assert `z.any()` and a Zod
  `.passthrough()` fire AP-015 by default, `z.unknown()` fires AP-016 only
  under opt-in (and is quiet by default), a non-Zod `.passthrough()` does
  not fire, and a plain typed schema (`z.object({ id: z.string() })`) is
  clean — same fixture-pair shape the LANGTS-006 `dynamic-execution` rules
  used to avoid false positives.
- **Confidence:** high — registry edit + family doc page is a well-trodden
  path (FLAGCAT-independent; LANGTS-006 is the working precedent).

### LANGTS-005: Kernel-prerequisite refactor (K1–K4) — Merged

- **Status:** Released/Shipped via v0.7.3-beta (2026-05-31). Merged 2026-05-29 via PR #2096
- **Intent:** Land the load-bearing kernel-prereq work (council §16.5 #3) so
  the parser layer can host three anchor languages plus a tail wave without an
  `if lang == …` cascade, without latent cache corruption, without a parse-path
  panic, and with a stated concurrency posture. Single module — K1..K4 stay
  here (decision recorded in Open Questions).
- **Expected Outcome:**
  - **K1 — extractor trait:** `crates/anvil-kernel/src/parser/extract.rs` is
    reshaped into a `LanguageExtractor` trait plus per-language modules; the
    orchestration layer (`extract_symbols`) is language-agnostic and dispatches
    on the parsed language. Existing TS extraction behaviour is preserved by
    porting today's walker to the TS impl. The trait shape is captured inline +
    against the T3 checklist §3 suggested interface; the durable cross-anchor
    ADR is deferred to RSTLAN per the audit §8 decision (no LANGTS ADR).
  - **K2 — grammar version in cache key:** the AST cache at
    `crates/anvil-kernel/src/parser/cache.rs` keys on grammar version in
    addition to content hash, so a tree-sitter grammar bump cannot serve a
    stale cached tree.
  - **K3 — parser thread-safety:** a stated strategy (audit option (1)
    `thread_local!` is the default) with a regression net; no ADR unless
    INTD-001 review makes the choice contentious (audit §8 conditional defer).
  - **K4 — panic removal:** the parse path no longer panics on language
    mismatch — `Parser::get_parser` surfaces the
    `expect("language version mismatch")` at
    `crates/anvil-kernel/src/parser/mod.rs:54` as a `Result`/`ParseError`
    instead of panicking, which is load-bearing for daemon mode.
  - **K5** stays a rubric referenced from the T3 checklist §1 (not executable
    scope here; folds into OPSUP per the audit §8 decision).
- **Scope:** `crates/anvil-kernel/src/parser/extract.rs` (+ new
  `extract/typescript.rs` module per the audit's recommended shape),
  `crates/anvil-kernel/src/parser/cache.rs`,
  `crates/anvil-kernel/src/parser/mod.rs`,
  `crates/anvil-kernel-types/src/graph.rs` (additive `SymbolKind` variants if
  needed).
- **Non-scope:** Rust / Python extractor implementations (per-anchor work in
  RSTLAN / PYLAN); the cross-anchor extractor-trait ADR (RSTLAN); K5 rubric
  promotion (OPSUP).
- **Dependencies:** LANGTS-001 (Done).
- **Validation:** `cargo test -p eddacraft-anvil-kernel` passes with: a
  parity test proving the TS extractor produces identical `FileSymbols`
  before/after the trait refactor; a cache test proving a grammar-version
  change invalidates a cached tree on the same content hash; and a parse-path
  test asserting a language mismatch returns a `ParseError` rather than
  panicking.
- **Confidence:** medium — the trait refactor is ~1 sprint (audit §5.6);
  the escape hatch is splitting into `lang-ts-prereq.aps.md` if K1 grows.

### LANGTS-006: TS dynamic-eval antipattern rule — Merged

- **Status:** Released/Shipped via v0.7.3-beta (2026-05-31; commit
  `bcb96175` confirmed in tag). Merged 2026-05-21 via PR
  [#1820](https://github.com/eddacraft/anvil-001/pull/1820) (`bcb96175`).
- **Intent:** Ship a TS antipattern rule for dynamic-eval shapes
  (`eval(<dynamic>)`, `new Function(...)`, `Function.prototype.constructor`
  string-source invocations). Severity `error` under the default profile;
  also wired into the `gate -p ai` curated set. **Merged 2026-05-21 via
  PR [#1820](https://github.com/eddacraft/anvil-001/pull/1820) at
  `bcb96175` — shipped AP-008 (eval dynamic + template-literal arg) and
  AP-009 (unconditional `new Function`) in the new `dynamic-execution`
  family; `Function.prototype.constructor` deferred to follow-up to
  avoid false positives on legitimate `.constructor` access without an
  AST-aware filter.** *Identified from
  [2026-05-21 new-user journey audit](../../audits/2026-05-21-new-user-journey-audit.md)
  finding #7 — a planted `export function unsafe(input:any){ return eval(input); }`
  was not caught by any of `check`, `audit`, `gate`, `watch`, or MCP, even
  though `antipattern-scan` is wired and PASSes under `gate` on the same
  file. Tracking: GH issue
  [#1801](https://github.com/eddacraft/anvil-001/issues/1801).
  Implementation note: rule ships in `patterns/compiled/registry.json`
  with a new family entry, mirroring LANGTS-004's Zod-creep approach.
  Validation: fixture test that asserts the rule fires on `eval(x)` and
  `new Function(s)()` while not firing on a literal `eval("1+1")` call
  with a static string argument (the static-eval case is rare but
  benign, and false positives there would erode trust in the rule).*

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Audit reveals months of TS gaps; pack ROI argument collapses (council C-007) | High | Re-score whole spec before committing to Track 4; surface gap depth inside this module before opening pack modules. **Audit complete 2026-04-26 — surfaced TS-G1..G7 (bounded, none individually blocking) and kernel-prereq K1..K5 (load-bearing, named in §5 of audit report).** |
| Kernel prerequisite work entangles anchor zero indefinitely | Medium | Split into LANGTS-prereq submodule the moment scope grows beyond a sprint. Audit recommends keep-or-split decision at LANGTS Ready-flip. |
| T3 checklist becomes a moving target as Rust/Python uncover edge cases | Medium | Version the checklist (v1 published 2026-04-26); treat changes as ADR-level decisions per checklist §10. |

## Open Questions

- [x] **Resolved 2026-05-28 — single module.** Is LANGTS a single module or
      LANGTS + LANGTS-prereq? The audit §5.6 sizes the kernel-prereq gaps as
      K1 medium (~1 sprint), K2 small (< 1 day), K3 small wrapper, K4 trivial,
      K5 per-grammar — i.e. K1..K4 fit inside one sprint. Per the audit §7 OQ2
      recommendation, **keep K1..K4 inside LANGTS-005; do not split into
      `lang-ts-prereq.aps.md`.** The Risks-table escape hatch stands: split out
      only if K1's trait-port scope grows past a sprint during execution. This
      closes the matching Ready-Checklist box.
- [x] Where does the T3 acceptance checklist live in the docs tree?
      Resolved: companion spec at
      `plans/specs/2026-04-26-t3-acceptance-checklist.md`. Re-home to
      `docs/architecture/` is a follow-up if user-facing reference needs
      it.
- [x] **Resolved — separate task.** Should the Zod-creep rules ship in a
      separate task, or fold into LANGTS-002 gap-close? Per the audit
      recommendation, **keep LANGTS-004 separate** — the registry edit + family
      doc page (a `patterns/<family>/` directory + `patterns/compiled/registry.json`
      entry, mirroring LANGTS-006's `dynamic-execution` family) is naturally
      distinct from the extractor changes in LANGTS-002.
- [x] **Resolved 2026-05-28 — no standalone ADR now; defer to RSTLAN start.**
      Does the extractor trait shape (K1) need a dedicated ADR? The audit's
      §7 OQ3 said "probably yes", but the audit's own **§8 Decision
      (2026-04-26)** refined it to: *"Extractor trait shape — defer until
      RSTLAN starts. Author at the point RSTLAN's first work item surfaces
      real shape requirements; premature locking risks the wrong
      abstraction."* So LANGTS-005 ships the `LanguageExtractor` trait with
      its shape captured inline in the implementation and the T3 checklist §3
      suggested interface; the durable cross-anchor ADR is authored by RSTLAN,
      not LANGTS. (This supersedes the earlier "dedicated ADR before RSTLAN
      moves to Ready" wording, which conflicted with §8.)
- [x] **Resolved — fold into OPSUP, no standalone ADR.** Should the grammar
      maturity rubric (K5) be lifted into its own ADR before LANGTAIL? Per the
      audit §8 Decision, **fold K5 into `operational-supplement` (OPSUP)** as a
      slice when LANGTAIL admission becomes a real ask; no standalone ADR.
      Inside LANGTS, K5 stays a rubric referenced from the T3 checklist §1 (it
      is not part of LANGTS-005's executable scope).
