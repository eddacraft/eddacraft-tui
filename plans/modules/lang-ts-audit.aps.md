<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# TypeScript Audit & T3 Calibration (Track 1 anchor zero)

| ID     | Owner | Status | Done |
| ------ | ----- | ------ | ---- |
| LANGTS | —     | Ready  | 3/6  |

**Last reviewed:** 2026-04-26

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
[2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
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
  [2026-04-26-t3-acceptance-checklist.md](../specs/2026-04-26-t3-acceptance-checklist.md);
  audit-derived bar referenced from RSTLAN, PYLAN, every Track 4 pack module,
  and the pack registry per [ADR-027](../decisions/027-pack-architecture.md)).
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

- [LANGTS audit report](../specs/2026-04-26-langts-audit-report.md) —
  point-in-time evidence; current TS implementation state, named TS gaps
  (TS-G1..G7), named kernel-prereq gaps (K1..K5), and the recommended ADRs
  the audit believes are missing.
- [T3 acceptance checklist (v1)](../specs/2026-04-26-t3-acceptance-checklist.md)
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
- [ ] Re-scoring gate run per
      [docs/guides/anchor-rescoring-process.md](../../docs/guides/anchor-rescoring-process.md);
      session owner named for this invocation.
- [ ] Decision recorded on whether kernel prerequisite work (council §16.5 #3)
      is in-scope here or split into LANGTS-prereq submodule. *Audit
      recommendation: keep inside LANGTS-005 if K1..K4 fit one sprint; split
      otherwise.*
- [x] Decision recorded on T3 checklist artefact location — *companion
      spec at `plans/specs/2026-04-26-t3-acceptance-checklist.md`. Re-home
      to `docs/architecture/anvil-t3-acceptance.md` is a follow-up if the
      checklist is referenced from user-facing docs; the spec location
      keeps it next to the audit evidence for now.*

## Tasks

Tasks below are anticipated shape; promote each to a proper task with Intent
+ Validation when the module moves to Ready. Two of the five anticipated
tasks have evidence completed by the audit (LANGTS-001 audit pass and
LANGTS-003 publication of the checklist artefact).

- **LANGTS-001** (audit pass complete 2026-04-26): Enumerate current TS
  capability state across the seven T3 dimensions. *Evidence:
  [audit report §3](../specs/2026-04-26-langts-audit-report.md#3-current-ts-implementation-state).*
- LANGTS-002: Close identified TS gaps (TS-G1..G7 per
  [audit report §4](../specs/2026-04-26-langts-audit-report.md#4-ts-specific-gaps-langts-work-items)).
  Default scope per audit §7 OQ1: TS-G1 (interfaces / type aliases / enums)
  + TS-G2 (methods); defer TS-G3 / TS-G4 / TS-G6 with explicit follow-up
  notes; TS-G7 lives in checklist documentation only.
- **LANGTS-003** (publication complete 2026-04-26): Publish T3 acceptance
  checklist artefact. *Evidence:
  [`plans/specs/2026-04-26-t3-acceptance-checklist.md`](../specs/2026-04-26-t3-acceptance-checklist.md).*
- LANGTS-004: Add Zod-creep rules (`z.any()`, `z.unknown()`,
  `.passthrough()`) to TS T2 anti-pattern catalogue. Closes audit gap
  TS-G5; rules ship in `patterns/compiled/registry.json` with
  `definition_ref` to a new family entry.
- LANGTS-005: Kernel prerequisite work — extractor trait (K1), grammar
  version in AST cache key (K2), parser thread-safety strategy (K3), panic
  removal in `Parser::get_parser()` (K4), grammar maturity audit (K5)
  surfaced as a rubric in the T3 checklist. Or split into
  LANGTS-prereq-* if scope justifies (audit §7 OQ2). See
  [audit report §5](../specs/2026-04-26-langts-audit-report.md#5-kernel-prereq-gaps-council-165-3-work).
- LANGTS-006: Ship a TS antipattern rule for dynamic-eval shapes
  (`eval(<dynamic>)`, `new Function(...)`, `Function.prototype.constructor`
  string-source invocations). Severity `error` under the default profile;
  also wired into the `gate -p ai` curated set. **Merged 2026-05-21 via
  PR [#1820](https://github.com/eddacraft/anvil-001/pull/1820) at
  `bcb96175` — shipped AP-008 (eval dynamic + template-literal arg) and
  AP-009 (unconditional `new Function`) in the new `dynamic-execution`
  family; `Function.prototype.constructor` deferred to follow-up to
  avoid false positives on legitimate `.constructor` access without an
  AST-aware filter.** *Identified from
  [2026-05-21 new-user journey audit](../audits/2026-05-21-new-user-journey-audit.md)
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

- [ ] Is LANGTS a single module or LANGTS + LANGTS-prereq? Decide before
      moving to Ready. *Audit recommendation: keep inside LANGTS-005 if
      K1..K4 fit one sprint; split otherwise.*
- [x] Where does the T3 acceptance checklist live in the docs tree?
      Resolved: companion spec at
      `plans/specs/2026-04-26-t3-acceptance-checklist.md`. Re-home to
      `docs/architecture/` is a follow-up if user-facing reference needs
      it.
- [ ] Should the Zod-creep rules ship in a separate task, or fold into
      LANGTS-002 gap-close? *Audit recommendation: keep LANGTS-004 separate
      — registry edit + family doc page is naturally distinct from the
      extractor changes in LANGTS-002.*
- [ ] **New (audit §8):** Does the extractor trait shape (K1) need a
      dedicated ADR, or can it land inside LANGTS-005 with a section in
      ADR-026? Audit recommends a dedicated ADR before RSTLAN moves to
      Ready.
- [ ] **New (audit §8):** Should the grammar maturity rubric (K5) be
      lifted from a checklist section into its own ADR before the LANGTAIL
      tail wave kicks off? Audit recommends yes.
