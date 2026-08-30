# Intent Conformance

| ID   | Owner  | Status      | Progress |
| ---- | ------ | ----------- | -------- |
| CONF | @aneki | In Progress | 1/9      |

**Last reviewed:** 2026-08-31 — CONF-011 external PR declaration check Merged
via [#4245](https://github.com/eddacraft/anvil-001/pull/4245) as the first
production consumer of CONF-004/005 after implementation Council
`council-c6695499` converged and exact-head hosted CI passed. CONF-005 Verified
Change Declarations Merged via
[#4222](https://github.com/eddacraft/anvil-001/pull/4222) after implementation
Council `council-11195af4` converged with a binding PASS.
CONF-001 Merged via [#4174](https://github.com/eddacraft/anvil-001/pull/4174),
and CONF-002..004 implementation and executor proof Merged via
[#4190](https://github.com/eddacraft/anvil-001/pull/4190).
[ADR-134](../decisions/134-intent-conformance-gating.md) is accepted and pins
the deterministic Git, evidence-binding, and fail-honest outcome contract.
[ADR-138](../decisions/138-pin-git-administrative-state.md) is accepted and
narrowly amends ADR-134's Git environment: real repository administration is
observed under a three-value allowlist, then later Git reads use a five-value
allowlist with run-owned, never-written shallow/graft sentinel paths.
[ADR-139](../decisions/139-bound-conformance-range-evidence.md) is accepted and
adds one shared range-evidence envelope, extends the whole-run deadline through
report materialisation, and requires atomic bounded plain, JSON, and SARIF
output.
[ADR-135](../decisions/135-bounded-change-evidence-and-conformance-projections.md)
is accepted after scoped Council repair re-review `council-599eaa64`; the
[CEG design](../specs/2026-08-30-change-evidence-graph.md), graph-semantic
implementation and CONF-006..010 remain Proposed and default-off.

> **Origin (2026-06-11):** Product direction set during the graphify gap
> analysis: plan gates return as a conformance lint — "did the agent build
> what you were planning and what it said it did." This was anvil's original
> use case (see the ILGOV audit note: "all Anvil was going to be a system for
> proving the plan was followed"). The scope guard's "knowledge-graph doc
> search → Out" ruling (DocGraph assessment, 2026-05-31) does **not** apply
> here: this surface is detection + evidence at gate/closeout time, not
> retrieval — the lane the scope guard marks **In**. ADR-134 records that
> decision formally.
>
> **Naming:** deliberately "conformance", not "drift" —
> [ADR-052](../decisions/052-automated-drift-snapshots.md) owns code-edge
> drift and the DocGraph assessment §6 warns against overloading the term.

## Purpose

Check that a change matches its declared intent — what was planned, what was
asked, and what the author (human or agent) claimed was done — and attach the
result as gate/closeout evidence. Intent sources are tiered so the check is
generically useful with zero planning format and gets richer when one exists:

| Tier | Intent source                                        | Availability      |
| ---- | ---------------------------------------------------- | ----------------- |
| 0    | Commit `type(scope)`, PR-body claims                 | Universal         |
| 1    | Session intent events (Kindling, via ILGOV)          | Any wired agent   |
| 2    | Plan documents via adapters (APS first; OpenSpec, BMAD, SpecKit, issues) | Opt-in |

Conventional Commit extraction/evaluation is the first Tier-0 implementation
slice (CONF-003/004). Deterministic PR-body patterns landed under CONF-005;
CONF-011 exposes them through an advisory, planless CLI check over an explicit
Git range.

The first non-APS public use case is **Intent & Claim Integrity**: external
authors publish Verified Change Declarations under the machine tag
`intent-claim-integrity`.

All tiers normalise into one canonical conformance contract; policy predicates
("touched outside declared scope", "claimed X, delta shows Y") operate on the
contract, never on a source format. APS is the richest producer, not a
requirement — planless-first (ADR-001) is preserved because Tier 0 delivers
value with no plan at all.

## In Scope

- Canonical conformance contract: declared scope, claimed changes, acceptance
  assertions, source provenance with evidence grading
  ([ADR-062](../decisions/062-policy-evidence-drift-as-evidence.md))
- Tier-0 claim extraction staged as Conventional Commits first (CONF-003/004)
  and one deterministic PR-body `anvil-claims` block later (CONF-005), with
  `documentation-only`, `test-only`, `no-behaviour-change`,
  `refactor-only`, and `scope: path:<prefix>` vocabulary
- Conformance evaluation over the ADR-134 per-commit Git coverage contract:
  Git evidence for closed path/file-class claims; revision-bound GV2 evidence
  for graph-semantic claims
- Correlation join: plan/work-item ID ↔ commit trailer ↔ PR ↔ delta ↔ capsule
  ([ADR-072](../decisions/072-git-native-governance-substrate.md),
  [ADR-074](../decisions/074-review-capsule-v0-format.md))
- Gate or closeout evaluation/attachment timing; warnings-first and non-zero
  only under explicit ADR-002 opt-in, baselined per ADR-003
- Tier-2 adapter artifact contract so the TS adapters layer
  (`packages/adapters/`) can emit the canonical contract as a build-time JSON
  artifact (ADR-049 cross-language style) without a Rust port of the adapters

## Out of Scope

- Knowledge-graph search, retrieval, indexing, or FTS over docs/plans (scope
  guard: Out; unchanged by this module)
- Session intent capture and ledger integrity — owned by ILGOV
- Plan-format parsing itself — owned by the adapters layer (OPENSPEC, BMAD4)
  and the external `anvil-plan-spec` toolchain
- GV2 schemas, deltas, identities — owned by GV2
- LLM-inferred claim extraction; all extraction here is deterministic
- Blocking by default — exit-0 advisory posture until opt-in enforce

## Interfaces

**Depends on:**

- GV2 — per-file `GraphDelta`, `SymbolIdentity`, and file/symbol deltas;
  graph-semantic use requires repository/worktree, evaluated revision/blob,
  schema, generation, and run binding
- ILGOV — Tier-1 intent records and the `IntentLedgerRecord` canonical schema
  (ILGOV rescope item 1); CONF-002 must not fork that schema
- GITGOV — later capsule evidence attachment under ADR-134's minimisation and
  privacy-review gate
- `packages/adapters/` — Tier-2 plan-format normalisation (TS, consumed via
  artifact, never linked)
- `anvil-kernel-types` — contract type home (SCHEMA precedent)

**Exposes:**

- Conformance contract type (Rust) + JSON artifact schema for Tier-2 producers
- Conformance findings consumable by gate/closeout; later minimised capsule
  evidence owned by CONF-006/009
- Policy predicates over the contract for L4/policy-engine composition

## Ready Checklist

Change status to **Ready** when:

- [x] CONF-001 ADR accepted as
      [ADR-134](../decisions/134-intent-conformance-gating.md) (product lane,
      tier model, retrieval distinction, naming)
- [x] ADR-134 pins per-commit Git extraction, claim-appropriate evidence,
      per-member dispositions, monotonic outcome/evidence-strength aggregation,
      explicit `path:` scope authority, base-tree mappings, and capsule privacy
- [x] Tier 0 sequenced ahead of ILGOV rescope item 1; CONF-002 keeps a
      co-design seam and must not fork the future Rust `IntentLedgerRecord`
- [x] CONF-002..004 promoted 2026-08-28 under operator authority with
      implementation files and exact Rust validation commands
- [x] CONF-002..004 implement and prove exact Git extraction plus
      repository/worktree, revision/blob, schema, generation, and run binding;
      each dependency boundary has focused tests and full affected-crate proof
- [x] CONF-002..004 Merged via PR #4190
- [x] CONF-005 promoted under the approved deterministic grammar
- [x] CONF-006..010 remain Proposed for later waves

## Work Items

### CONF-001: Product-decision ADR for conformance gating

- **Status:** Merged 2026-08-27 via PR #4174
- **Intent:** Record the decision that intent/plan-conformance gating is
  in-lane, with the tier model and the retrieval-surface distinction.
- **Expected Outcome:** Accepted ADR in `plans/decisions/`; DECISION-LOG row;
  scope-guard borderline table cites it so the DocGraph "Out" ruling is not
  re-applied to this surface; module/programme/index truth records the cleared
  Tier-0 decision without promoting implementation work.
- **Files:** `plans/decisions/134-intent-conformance-gating.md`,
  `plans/decisions/DECISION-LOG.md`, `docs/vision/anvil-scope-guard.md`,
  this module, `plans/specs/2026-07-28-graph-trust-surfaces.md`, and
  `plans/index.aps.md`
- **Evidence:** ADR-134 pins base/head and per-commit Git extraction,
  first-parent merge footprints, raw status/rename/mode/type/gitlink and path
  handling, fixed extraction and rename budgets with fail-honest overflow,
  replacement-disabled sanitised Git execution, option-safe revision
  resolution, claim-appropriate evidence binding, per-member dispositions,
  monotonic outcomes, explicit `path:` authority, base-tree mapping
  authority, anti-self-authorisation, Conventional-Commit-first staging,
  ADR-002 posture, capsule minimisation/privacy ownership, and the Wave
  boundary. Implementation evidence remains with CONF-002..004; CONF-005..009
  remain Proposed.
- **Validation:** `pnpm adr:check`, `pnpm format:check`,
  `pnpm docs:check`, `pnpm aps:active-lint`, `pnpm aps:index:check`
- **Dependencies:** —
- **Confidence:** high

### CONF-002: Canonical conformance contract

- **Status:** Merged 2026-08-28 via PR #4190
- **Intent:** One Rust contract all tiers normalise into: declared scope,
  claimed changes, acceptance assertions, source provenance + evidence grade.
- **Expected Outcome:** Contract type in `anvil-kernel-types` with serde JSON
  schema for external producers; binds repository/worktree, evaluation run,
  revision/blob and graph schema/generation; extends (not forks) ILGOV's record
  schema.
- **Files:** `crates/anvil-kernel-types/src/conformance.rs`,
  `crates/anvil-kernel-types/src/lib.rs`, and
  `crates/anvil-kernel-types/README.md`,
  `crates/anvil-kernel-types/schema/intent-conformance-v1.schema.json`, and
  `crates/anvil-kernel-types/tests/conformance_contract.rs`
- **Evidence:** Canonical input and verdict round trips cover all binding axes;
  output validation proves raw-record/per-path separation, rename endpoints,
  canonical ordering, exact bidirectional endpoint references, and legal Git
  paths independent of scope-prefix grammar. Nine focused tests pass.
- **Validation:**
  `cargo test -p eddacraft-anvil-kernel-types --no-fail-fast`
- **Dependencies:** CONF-001
- **Confidence:** medium

### CONF-003: Tier-0 claim extraction — conventional commits

- **Status:** Merged 2026-08-28 via PR #4190
- **Intent:** Deterministically parse commit `type(scope)` and trailers into
  conformance contract claims as the first executable Tier-0 source.
- **Expected Outcome:** Each selected commit is bound to its own exact raw Git
  footprint and typed scope/kind claim under ADR-134's bounded,
  replacement-disabled Git contract; malformed, unknown, inapplicable, or
  option-shaped revision inputs yield reason-coded not-evaluated, never
  conformant. Tests cover option-shaped inputs, active replacement refs,
  legacy graft state, ambient Git overrides, and every budget reason.
- **Files:** `crates/anvil-checks/src/conformance/mod.rs`,
  `crates/anvil-checks/src/conformance/git.rs`,
  `crates/anvil-checks/tests/conformance_git.rs`,
  `crates/anvil-checks/src/lib.rs`, `crates/anvil-checks/Cargo.toml`,
  and `Cargo.lock`
- **Evidence:** Twenty-nine extractor tests cover option-safe selection,
  replacement/graft/shallow rejection, first-parent and ancestry semantics,
  raw record fidelity, base-tree authority, opaque identity verification, and
  stage-local last-complete-record budget diagnostics. Controlled Git wrappers
  prove preflight/final endpoint equality, timeout counters, descendant
  process-tree teardown, canonical absolute Git executable resolution, and
  canonical top-level worktree identity and command execution with explicit
  bare-repository rejection. Revision-list byte overflow retains the configured
  commit ceiling and the last complete commit count.
- **Validation:** `cargo test -p eddacraft-anvil-checks --no-fail-fast`
- **Dependencies:** CONF-002
- **Confidence:** high

### CONF-004: Tier-0 conformance check — claims vs delta

- **Status:** Merged 2026-08-28 via PR #4190
- **Intent:** Evaluate Tier-0 claims against the change delta and emit
  advisory findings.
- **Expected Outcome:** Closed path/file-class claims evaluate from Git;
  graph-semantic claims require bound GV2 evidence; every coverage member has
  a disposition; direct paths require `path:<prefix>`; mappings come only from
  versioned base-tree authority and mapping changes are `policy-change`;
  proven violations remain non-conformant under partial evidence; warnings exit
  0 by default; baselined new-edges-only.
- **Files:** `crates/anvil-checks/src/conformance/evaluate.rs`,
  `crates/anvil-checks/src/conformance/mod.rs`,
  `crates/anvil-checks/tests/conformance_evaluate.rs`,
  `crates/anvil-checks/README.md`, and
  `crates/anvil-checks/ARCHITECTURE.md`
- **Evidence:** Sixteen focused evaluator tests prove exact-case classification,
  mapped-prefix union authority, multi-source provenance remapping,
  policy-change independence, raw-record/per-path coverage, monotonic
  aggregation, and graph binding failure visibility across every identity
  axis; the ignored pinned-history dogfood also passes when selected exactly.
- **Validation:** `cargo test -p eddacraft-anvil-checks --no-fail-fast` plus
  a deterministic dogfood test over this repository's committed history
- **Dependencies:** CONF-003
- **Confidence:** high

### CONF-005: Tier-0 claim extraction — PR bodies

- **Status:** Merged 2026-08-30 via PR
  [#4222](https://github.com/eddacraft/anvil-001/pull/4222)
- **Intent:** Extract a single, explicit Verified Change Declaration from a PR
  description as weak-graded Tier-0 intent.
- **Expected Outcome:** Exactly one fenced `anvil-claims` block admits the
  closed vocabulary `claim: documentation-only`, `claim: test-only`,
  `claim: no-behaviour-change`, `claim: refactor-only`, and
  `scope: path:<prefix>`. Members are deterministically ordered and
  deduplicated. A second block makes the source not evaluated. Unknown or
  malformed members preserve any known extracted members but make the source
  reason-coded not evaluated. Extraction exits 0 and retains
  `IntentSourceKind::PullRequest`, an immutable source reference and body
  digest only — never the raw body. Documentation, test and direct-path
  predicates may evaluate from Git; no-behaviour/refactor predicates remain
  not evaluated until sufficient CEG evidence graduates under CONF-010.
- **Files:** `crates/anvil-checks/src/conformance/mod.rs`,
  `crates/anvil-checks/src/conformance/pr_body.rs`,
  `crates/anvil-checks/tests/conformance_pr_body.rs`,
  `crates/anvil-checks/README.md`, and
  `crates/anvil-checks/ARCHITECTURE.md`
- **Evidence:** Implementation Council `council-11195af4` converged with a
  binding PASS after scoped repairs for top-level Markdown/HTML-comment
  context, versioned body/reference/member/scope budgets, and fail-honest
  contract admission. Eleven focused extractor tests prove the closed
  vocabulary, exact-body digest binding, canonical ordering and deduplication,
  ambiguous/malformed source handling, nested-example isolation, every resource
  limit, and refusal to admit not-evaluated evidence. The full affected crate
  suite and strict all-target Clippy pass.
- **Integration:** Rebase-merged to `main` via
  [#4222](https://github.com/eddacraft/anvil-001/pull/4222) after exact-head
  hosted CI passed and all review conversations were resolved.
- **Validation:** `cargo test -p eddacraft-anvil-checks --no-fail-fast`
- **Dependencies:** CONF-002
- **Confidence:** high

### CONF-006: Correlation join across the git substrate

- **Status:** Proposed
- **Intent:** Join work-item IDs to their realised changes: plan/item ID ↔
  commit trailer ↔ PR ↔ `GraphDelta` ↔ capsule.
- **Expected Outcome:** Given a work-item ID, the join returns its commits,
  PRs, and touched files/symbols deterministically from local git state. Any
  later capsule attachment carries normalised claims and source reference +
  digest, never raw bodies, and waits for privacy review.
- **Validation:** `cargo test -p anvil-checks` + join replay against a known
  merged item in this repo
- **Dependencies:** CONF-002, GV2 delta surface
- **Confidence:** medium

### CONF-007: Closeout conformance check

- **Status:** Proposed
- **Intent:** Run or attach conformance evaluation at closeout as a timing
  surface, without creating a separate enforcement class.
- **Expected Outcome:** Closeout reports conformance findings; non-zero exit is
  available only through explicit ADR-002 enforcement opt-in; attachment
  preserves outcome/evidence separation and the privacy gate.
- **Validation:** dogfood closeout run on this repo
- **Dependencies:** CONF-004, CONF-006
- **Confidence:** medium

### CONF-008: Tier-2 plan-adapter artifact contract

- **Status:** Proposed
- **Intent:** Let the TS adapters layer emit the canonical contract as a JSON
  artifact the Rust gate consumes; APS adapter first.
- **Expected Outcome:** An APS module/work item round-trips to a conformance
  contract artifact; contract versioned; Rust side validates schema and never
  links TS.
- **Validation:** `pnpm --filter @eddacraft/anvil-adapters test` +
  `cargo test -p anvil-checks`
- **Dependencies:** CONF-002
- **Confidence:** low

### CONF-009: Intent-source evidence grading in verdicts

- **Status:** Proposed
- **Intent:** Carry source evidence grades through to findings so verdicts
  state what strength of intent they were checked against, and own the closed
  minimised capsule allowlist with CONF-006.
- **Expected Outcome:** Findings distinguish "conformant against plan
  acceptance criteria" from "conformant against commit-type claim only";
  capsule evidence excludes raw bodies, absolute paths, `GraphDelta.errors`,
  and graph baseline sets until a privacy review approves schema extension.
- **Validation:** `cargo test -p anvil-checks`
- **Dependencies:** CONF-004, CONF-008
- **Confidence:** medium

### CONF-010: CEG-backed graph-semantic predicates

- **Status:** Proposed
- **Intent:** Consume only graduated CEG evidence for exact graph-semantic
  declarations without broadening author prose into behavioural claims.
- **Expected Outcome:** CONF can evaluate
  `public-symbol-surface-unchanged`, `dependency-shape-unchanged`, and
  `privilege-surface-unchanged` only with a complete trusted snapshot vector.
  Unsupported, stale, mismatched, partial or budget-exceeded evidence is
  reason-coded not evaluated. `no-behaviour-change` and `refactor-only`
  remain not evaluated unless a later decision deliberately narrows them to a
  supported predicate.
- **Validation:** focused conformance/CEG parity, mismatch and completeness tests
- **Dependencies:** CONF-004, CEG-005
- **Confidence:** low

### CONF-011: External PR declaration check

- **Status:** Merged 2026-08-31 via PR
  [#4245](https://github.com/eddacraft/anvil-001/pull/4245)
- **Intent:** Give external, non-APS authors a production CLI surface that
  checks one Verified Change Declaration against the exact Git range proposed
  for review.
- **Expected Outcome:** `anvil conformance check` accepts explicit base and
  head revisions, a bounded PR-body file or stdin, and an immutable source
  reference. It runs CONF-005 extraction and the CONF-004 Git evaluator,
  reporting advisory plain, JSON, or SARIF output with separate declaration
  grade and evidence strength. The bounded Git footprint is claim-agnostic and
  does not require Conventional Commit syntax; CONF-003 commit-claim extraction
  remains separate. `documentation-only`, `test-only`, and
  explicit path scopes can evaluate from complete Git evidence;
  `no-behaviour-change` and `refactor-only` stay reason-coded
  `not-evaluated`. Missing, malformed, partial, over-budget, or mismatched
  evidence never becomes conformant. The evaluator rejects pre-existing
  replacement, graft, and shallow state under ADR-138's three-value
  administration-observation allowlist. Every post-admission Git read adds only
  run-owned, never-written shallow/graft sentinel paths, forming the exact
  five-value `GIT_*` allowlist and preventing ordinary concurrent repository
  administration from changing the admitted graph. This is not an
  operating-system immutability guarantee or a hostile same-user security
  boundary. ADR-139 caps the whole range at 100,000 raw records, 64 MiB raw Git
  diff bytes, and 64 MiB decoded Git path bytes; the five-minute deadline starts
  before bounded declaration input and spans identity, extraction, evaluation,
  and atomic report materialisation, whose output cap is 128 MiB.
  Overflow or timeout emits one complete reason-coded not-evaluated report.
- **Files:** `crates/anvil-checks/src/conformance/`,
  `crates/anvil-checks/tests/`, `crates/anvil-cli/src/commands/conformance.rs`,
  `crates/anvil-cli/src/{commands/mod.rs,main.rs}`,
  `crates/anvil-cli/tests/`, component documentation, the authoritative CLI
  references, the product feature catalogue,
  [ADR-134](../decisions/134-intent-conformance-gating.md),
  [ADR-138](../decisions/138-pin-git-administrative-state.md),
  [ADR-139](../decisions/139-bound-conformance-range-evidence.md), the decision
  log, and this module
- **Evidence:** Implementation Council `council-c6695499` converged after all
  nine findings were fixed, with independent verification returning a binding
  PASS. Exact-head hosted CI passed the Rust, Node, Windows Clippy, docs,
  security, APS, format, lint, and type-check gates with no unresolved review
  conversations.
- **Integration:** Rebase-merged to `main` via
  [#4245](https://github.com/eddacraft/anvil-001/pull/4245); rebased commit
  `dc06c358da05acea29f13d6e077d5b489191367c` is an ancestor of `origin/main`.
- **Validation:** `cargo test -p eddacraft-anvil-checks --no-fail-fast`;
  `cargo test -p eddacraft-anvil --no-fail-fast`; `cargo clippy -p
  eddacraft-anvil-checks -p eddacraft-anvil --all-targets -- -D warnings`;
  `pnpm docs:check`; `pnpm format:check`; `pnpm aps:active-lint`;
  `pnpm aps:index:check`; `pnpm adr:check`; `pnpm test:adr-integrity`
- **Dependencies:** CONF-004, CONF-005
- **Confidence:** high
