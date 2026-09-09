# Graph Answer Attestation

| ID   | Owner       | Status | Progress |
| ---- | ----------- | ------ | -------- |
| GATT | @joshuaboys | Ready  | 0/6      |

**Status:** **Ready** (2026-09-07). Every gate is cleared —
[ADR-142](../decisions/142-graph-answer-attestation.md) is **Accepted**, the
owner is named, and the field shape is frozen by ADR-142 §1–§5. GATT-001..006
are Ready and executable in dependency order; GATT-001 is the entry point.

Accepting ADR-142 as drafted settled two open calls: the `truncated` bool is
**retained for one release** as a derived compatibility shim rather than cut
(§2), and **CIB-341/342 sequence before this work** (§Mitigations) — honest
disclosure over a graph that timed out during a full scan tells the user
precisely that the answer is bad, which is better than lying but is not a
product. Treat that ordering as binding, not advisory.

**Origin:** the ripwire borrow assessment
([`plans/brainstorms/2026-09-07-ripwire-borrow-assessment.md`](../brainstorms/2026-09-07-ripwire-borrow-assessment.md)),
which recommended exactly one primitive out of that repository and declined code
adoption and dependency. This module is that primitive and nothing else.

**Affinity, not membership:** this work sits next to the
[Graph Trust Surfaces](../index.aps.md#graph-trust-surfaces) programme in
intent — it makes a graph answer say what it does not know. It is **not** a
sixth track of that operator-approved shortlist; its home is Graph Substrate.

**Last reviewed:** 2026-09-07 — module created.

## Purpose

Make every graph-context answer state its own limits in-band, so a consumer can
tell a complete answer from a bounded one, an exact edge from an inferred one,
and a cheap call from an expensive one — without reading Anvil's source or
re-deriving anything.

Anvil already discloses two of the three classes an answer can be wrong about:
**privacy** (`RedactionSummary` — what egress removed) and **assurance**
(`workspace_assurance.state` + `reason` — whether the graph was warm). The third
class, **what the answer itself could not resolve or could not fit**, is
disclosed unevenly: present and good on `anvil_search_symbols`, thin or absent
on the reports where the stakes are highest. An assistant handed "these symbols
are affected" cannot currently tell which of those edges the resolver guessed at,
nor which of two caps bound the report.

## Background — what already exists

Stated precisely so no work item re-builds something shipped:

- **`RedactionSummary`** (`crates/anvil-gctx-types/src/lib.rs`) already carries
  `matched` (the pre-cap total), `returned`, `truncated`, `omitted_sensitive_paths`,
  `snippets_truncated`, `fully_suppressed_symbols` and `fields_suppressed`. The
  "count that reads like a total but is a floor" defect is **already solved** for
  `anvil_search_symbols`.
- **A versioned token estimator** ships in `crates/anvil-graph-cache/src/tokens.rs`
  (`estimate_gctx_tokens`, `TokenEstimate`, `GCTX_TOKEN_ESTIMATOR_VERSION`,
  `MAX_GCTX_TOKEN_ESTIMATOR_INPUT_BYTES`) from GCTX-020. It is used to drive the
  snippet budget; it is not reported to the caller on the other projections.
- **A `heuristic` marker** exists on callers and on test relevance, and `partial`
  exists on the caller and dependent projections — the vocabulary is there.
- **`ImpactSummary` / `AffectedTestsSummary`** count what was *returned*
  (`affected_symbols`, `dependent_files`, `known_tests`, `tests`,
  `evidence_edges`, `coverage_gaps`) plus a single `truncated: bool` whose own
  doc comment says the cap was "**either** the affected-symbol set or the
  dependent-closure walk" — the consumer cannot tell which, and there is no
  pre-cap total to compare against.
- **`crates/anvil-graph-cache/src/call_graph.rs`** computes per-edge ambiguity
  and then **OR-s it across all of a caller's edges** into one caller-level
  `heuristic` flag. The information is computed and then discarded.

## In Scope

- A single **self-attestation contract** covering all six graph-context tools
  and the `graph://` resources, additive to the existing egress DTOs.
- **Per-edge** call-resolution fidelity, replacing OR-ed per-caller marking.
- **Cap disclosure parity**: every capped section reports a pre-cap total, a
  returned count, and *which* cap fired — one reason per cap, separately counted.
- **Cost self-report**: the estimator's answer, and its version, on every
  projection envelope.
- A **gate harness that can observe what it asserts** — each disclosure forced
  true and proven visible — written before the fields it guards.
- Teaching the consumer to read and route on the disclosures (guide + the
  shipped `anvil-developer-functions` skill).

## Out of Scope

- **Ranking, relevance scoring, and task-shaped retrieval.** `SearchSymbolsQuery`
  is conjunctive filters over a keyset walk; there is no ranker, so there is no
  ranking confidence to report and none is invented here. Adding one is a
  different, unfiled proposal and would need its own scope-guard pass against
  [ADR-075](../decisions/075-v080-graph-product-scope.md).
- **Any change to GV2 schemas, hot-path admission, or persistence.** The GV2
  framing rule holds: Anvil-first, GCTX adapts (ADR-061/063/064/067/069).
- **Source-text egress posture.** The identity-only default (CE-1) and the
  `gctx.egress` opt-in are untouched; every field added here is counts-and-flags,
  never content.
- **The full-scan timeout and cold-serve failures** (CIB-341/342). They are
  upstream of this work's value — an honest disclosure on a graph that timed out
  just says precisely that the answer is bad — but they are not this module's to
  fix. Sequence them first.
- **New language support.** See [Languages](#languages-considered-and-declined).
- **An evaluation programme.** See [Deferred threads](#deferred-threads-not-owned-here).

## Interfaces

**Depends on:**

- [ADR-142](../decisions/142-graph-answer-attestation.md) — the decision this
  module implements. **Accepted** 2026-09-07.
- `crates/anvil-gctx-types` — the egress DTOs the contract extends.
- `crates/anvil-graph-cache` — `call_graph.rs` edge resolution; `tokens.rs` estimator.
- [`docs/architecture/graph-context-delivery-spec.md`](../../docs/architecture/graph-context-delivery-spec.md)
  — the frozen delivery contract this amends, and its CE-1..CE-12 conditions.
- [ADR-086](../decisions/086-symbol-call-graph-substrate.md) (CALL-1 heuristic
  marker), [ADR-084](../decisions/084-gctx-graph-handle-access.md),
  [ADR-095](../decisions/095-gctx-cli-secondary-surface.md) (the CLI secondary
  read surface the contract must also hold for).

**Exposes:**

- One self-attestation block on every GCTX projection, for MCP clients and the
  ADR-095 CLI secondary alike.

## Ready Checklist

All gates cleared 2026-09-07; module promoted to **Ready**.

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] At least one work item defined — six (statuses live in the table below)
- [x] **Owner named** — @joshuaboys (2026-09-07)
- [x] **ADR call made** — it warrants its own ADR:
      [ADR-142](../decisions/142-graph-answer-attestation.md) binds all six
      tools plus a rule for future ones, and extends a PV-9-conditioned surface
- [x] **Field shape frozen** — ADR-142 §1–§5. A shared `Attestation` struct held
      by every projection DTO, so half-adoption is a compile error rather than a
      review catch
- [x] **ADR-142 Accepted** — 2026-09-07 (owner). The decision is in effect and
      this module is authorised.

## Work Items

| ID       | Task                                                | Status | Depends on |
| -------- | --------------------------------------------------- | ------ | ---------- |
| GATT-001 | Self-attestation contract (spec amendment)          | Merged 2026-09-09 via PR [#4520](https://github.com/eddacraft/anvil-001/pull/4520) | —          |
| GATT-002 | Disclosure gate harness (written before the fields) | Merged 2026-09-10 via PR [#4525](https://github.com/eddacraft/anvil-001/pull/4525) | GATT-001   |
| GATT-003 | Per-edge call-resolution fidelity                   | Merged 2026-09-09 via PR [#4532](https://github.com/eddacraft/anvil-001/pull/4532) | GATT-002   |
| GATT-004 | Cap disclosure parity across the six tools          | In Progress | GATT-002   |
| GATT-005 | Cost self-report on every projection envelope       | Ready | GATT-002   |
| GATT-006 | Teach the consumer to read the disclosures          | Ready | GATT-003..005 |

### GATT-001 — Self-attestation contract

- **Intent:** Fix, in the delivery contract, what every graph-context answer
  must say about its own limits.
- **Expected Outcome:** The GCTX spec carries a self-attestation section
  implementing [ADR-142](../decisions/142-graph-answer-attestation.md) — the
  fields, their meaning per tool, and the rule that governs them; it records the
  CE-class analysis showing counts-and-enums stay inside the identity-only
  default (CE-1) and the counts-only egress posture (CE-11), states the CE-3
  post-deny-list counting constraint, and holds for both the MCP surface and the
  ADR-095 CLI secondary.
- **Validation:** `pnpm docs:check` — exit 0, 15/15 on
  `308e344b210d1cbc118a1af3e01a0f43fc4f2f17` (independent verify-loop pass).
  Merged 2026-09-09 via PR
  [#4520](https://github.com/eddacraft/anvil-001/pull/4520) (rebase). Original
  head is not an ancestor after rewrite; content probe: `## Self-attestation
  (ADR-142)` on `origin/main` (`ac0d73f3c` / tip `bf2391bbd`).
- **Non-scope:** No code. No GV2 schema change. No new identity or path is
  introduced by any field this section defines.
- **Files:** `docs/architecture/graph-context-delivery-spec.md`
- **Confidence:** high

### GATT-002 — Disclosure gate harness

- **Intent:** Prove each disclosure can be observed before any field depends on
  it being true.
- **Expected Outcome:** For every disclosure the contract defines, a test forces
  the condition and asserts the disclosure is visible on the projection — a cap
  actually fires, an ambiguous edge actually exists, an estimate is actually
  attached. Each gate is confirmed to fail when its subject is removed.
- **Validation:** `cargo test -p eddacraft-anvil-gctx-types -p eddacraft-anvil-graph-cache`
- **Non-scope:** Producer wiring — GATT-003..005 fill per-edge fidelity, cap
  counts, and cost self-report. This item ships the shared type and observer
  tests; projector envelopes currently attach `Attestation::default()`.
- **Files:** `crates/anvil-gctx-types/src/lib.rs`,
  `crates/anvil-gctx-egress/src/lib.rs`
- **Confidence:** medium

### GATT-003 — Per-edge call-resolution fidelity

- **Intent:** Stop a single ambiguous call from marking all of a caller's other
  edges as guesses.
- **Expected Outcome:** Ambiguity is reported per call edge. A caller with one
  unresolvable edge and three exact ones reports three exact edges. Any retained
  caller-level flag is derived from the edge set and documented as a summary of
  it, not as a property of the caller.
- **Validation:** `cargo test -p anvil-graph-cache`
- **Non-scope:** Improving the resolver. This item changes what is *reported*,
  not what is *resolved* — the per-edge decision is already computed and then
  OR-ed away.
- **Files:** `crates/anvil-graph-cache/src/call_graph.rs`,
  `crates/anvil-gctx-types/src/lib.rs`
- **Dependencies:** GATT-002
- **Confidence:** medium

### GATT-004 — Cap disclosure parity across the six tools

- **Intent:** Make every capped answer say how much it left out and which cap
  did it.
- **Expected Outcome:** Each capped section reports the budget that bound it,
  a returned count, and a **tagged** total — `Exact` or `AtLeast`, never a bare
  integer (ADR-142 §3), so a bounded traversal states a floor as a floor instead
  of walking past the node budget that bound it. `ImpactSummary` and
  `AffectedTestsSummary` no longer report a single `truncated` bool covering two
  different budgets; the bool is retained one release as a derived shim. Counts
  are taken **after** the CE-3 deny-list (ADR-142 §6). A parity test enumerates
  the tools so a future tool cannot ship without disclosure.
- **Validation:** `cargo test -p anvil-gctx-types -p anvil-cli`
- **Non-scope:** Changing any cap's value or the traversal budgets themselves.
- **Files:** `crates/anvil-gctx-types/src/lib.rs`, `crates/anvil-cli/src/mcp/tools/`
- **Dependencies:** GATT-002
- **Confidence:** high

### GATT-005 — Cost self-report on every projection envelope

- **Intent:** Let a caller budget an answer before spending its context on it.
- **Expected Outcome:** Every projection envelope carries the estimated token
  cost of the answer and the estimator version that produced it, so the number
  is comparable across releases and legible as an estimate rather than a
  measurement. The existing `estimate_gctx_tokens` is reused; no second
  estimator is introduced.
- **Validation:** `cargo test -p anvil-graph-cache -p anvil-cli`
- **Non-scope:** Token-budget *enforcement*. This reports cost; it does not
  bound it. The snippet budget keeps its existing behaviour.
- **Files:** `crates/anvil-graph-cache/src/tokens.rs`,
  `crates/anvil-gctx-types/src/lib.rs`
- **Dependencies:** GATT-002
- **Confidence:** high

### GATT-006 — Teach the consumer to read the disclosures

- **Intent:** Make the disclosures change assistant behaviour rather than
  decorate the payload.
- **Expected Outcome:** The AI-context-delivery guide and the shipped
  `anvil-developer-functions` skill tell an assistant what each disclosure means
  and what to do about it — when to page, when to treat an edge as unconfirmed,
  when an answer is a bounded prefix rather than a full set.
- **Validation:** `pnpm docs:check`
- **Files:** `docs/guides/ai-context-delivery.md`,
  `crates/anvil-cli/assets/skills/anvil-developer-functions/`
- **Dependencies:** GATT-003, GATT-004, GATT-005
- **Confidence:** high

## Languages — considered and declined

The assessment prompt asked whether ripwire's language coverage suggests work
here. It does not, on both halves of the question.

**Coverage.** ripwire carries twenty-one grammars; the ones Anvil lacks are
Objective-C/C++, Metal, CUDA, Swift, Ruby, PHP and Lua (Bash, JSON, TOML, YAML
and Markdown are Anvil *surfaces* — SURFSH, SURFGHA, MDGOV — not gaps). Anvil
already carries Dart, Kotlin, Zig and WebAssembly text, which ripwire does not.
None of the missing seven has an Anvil demand signal; Swift is on the
[§13 cut list](../specs/2026-04-08-language-and-coverage-design.md) explicitly. Promotion is
governed by the spec's demand × blast-radius × strategic-fit lever, and "a tool
we assessed supports it" is not a demand signal — adopting one would be the
precise anti-pattern that lever exists to prevent. **No language module is
proposed.**

**Onboarding cost.** ripwire advertises that adding a language is "a vendored
tree-sitter grammar plus one row in a declarative table". Anvil already meets
that bar: `Language::from_path` (`crates/anvil-kernel/src/parser/languages.rs`)
is a single extension match, and the LANGTS-005 extractor-trait refactor left
`Language::Rust` referenced at seven sites across three files. This is parallel
evolution with nothing to borrow. **No kernel work is proposed.**

## Deferred threads — not owned here

Recorded so they are not lost, and deliberately not filed as work items. Detail
and reasoning in
the assessment's [Suggested follow-ups](../brainstorms/2026-09-07-ripwire-borrow-assessment.md#suggested-follow-ups).

| Thread | Why not here |
| --- | --- |
| Real-corpus retrieval eval with source-authored held-out labels | A measurement programme, not a projection contract. Needs a corpus decision first. Would sit beside — not replace — the `token_reduction` bench scenario, which stays as a shape regression. |
| Publishing counterexamples where graph context loses to reading the file | A product and positioning call, not an engineering one. |
| Sibling-completeness as a standing Council review question ("which siblings does this fix not cover?") | Council checklist change; no tooling. CIB candidate — CIB is multi-writer, so it is filed on a bookkeeping branch, never here. |
| Cold-path serve time as evidence in the full-scan-timeout lane | Belongs to the existing CIB-341 lane as evidence; opening a parallel lane would fragment it. |
