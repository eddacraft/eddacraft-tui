# Clawpatch Techniques Adoption

| ID   | Owner  | Status   | Progress |
| ---- | ------ | -------- | -------- |
| CPTA | @aneki | Proposed | 0/7      |

**Status:** Proposed — direction reviewed via brainstorm against
[`openclaw/clawpatch`](https://github.com/openclaw/clawpatch) on 2026-05-16, but
execution is not yet operator-authorised. Promote to Ready per-task as scope
clarifies. CPTA-001 (discovery) is safe to authorise first; the remaining tasks
gate on its conclusions.

**Last reviewed:** 2026-05-16 — sibling to CGBDG under Dev Tooling Bridge.
Adoption targets the Claude `/council` developer-tooling layer first; product
adoption is gated through CPTA-007.

## Purpose

Lift the high-value techniques from clawpatch — a TypeScript-based LLM code
review tool that maps a repo into feature slices, reviews each with a strict
JSON contract, and persists findings under `.clawpatch/` — into Anvil's
developer-side review surface (`/council`, `/review`, reviewer agents) without
disturbing the deterministic save-time enforcement core.

This module is **dev-tooling-first**. Product-side adoption (whether Anvil's
own runtime can reuse any clawpatch pattern) is scoped to a single discovery
spike (CPTA-007) and gated by operator decision.

## Background

The user identified five techniques worth borrowing from clawpatch:

1. **Strict finding JSON schema** — `title`, `category` (bug / security /
   performance / concurrency / api-contract / data-loss / test-gap / docs-gap
   / build-release / maintainability), `severity`, `confidence`, `evidence[]`
   (file + line range + optional quote), `reasoning`, `reproduction`,
   `recommendation`, `minimumFixScope`, `suggestedRegressionTest`,
   `whyTestsDoNotAlreadyCoverThis`.
2. **Persistent finding lifecycle** — `.clawpatch/{features,findings,runs,
   patches,reports,locks}/` with each finding moving through
   `open → triaged → fixed → revalidated`.
3. **Map → review → report → fix → revalidate pipeline** — clean separation
   between what-to-review, what-was-found, what-was-done, is-it-still-broken.
4. **Prompt principles** — tests-as-evidence, root-cause deduplication,
   no-speculation, every claim must cite `path:line`, deterministic 24 KB
   per-file truncation with markers, `isInside()` path-validation rejecting
   any evidence outside repo root.
5. **Provider abstraction with mock / mock-fail** — lets council be
   dry-run-tested in CI without burning model calls.

Anvil already does better at: risk-tiered review (`quick`/`mini`/`full`),
deterministic path-based reviewer routing via `scripts/agent/guidance.sh`,
multi-persona council deduplication, and hook/skill/agent decomposition.
**Do not regress these on adoption.**

## Boundaries

**In scope:**

- Extend the five reviewer agent prompts under `.claude/agents/` to enforce
  the structured finding schema (1) and prompt principles (4)
- Optionally add a findings persistence store under `review/state/findings/`
  with the lifecycle from (2) (gated behind CPTA-001's outcome)
- Optionally add `/council fix --finding <id>` and `/council revalidate`
  subcommands (gated behind CPTA-004's outcome)
- Add a `mock` reviewer mode usable in CI for `/council` smoke tests (5)
- Document overlap and divergence with existing CGBDG work, so this module
  does not re-do council-judge → attestation mapping

**Out of scope:**

- Vendoring clawpatch as a runtime dependency of Anvil-the-product (Anvil's
  deterministic core principle forbids LLM-driven enforcement, per
  `docs/vision/anvil-scope-guard.md`)
- Modifying Anvil's attestation schema, witness chain, or ProtectionClaim
  shape — those are owned by MLP / MLP2 / CGBDG
- Replacing `scripts/agent/guidance.sh` or the existing risk-tiered routing
- Auto-commit, auto-PR, or auto-fix loops that bypass the existing
  Council/PR review gates

## Work Items

| ID       | Task                                                     | Status   |
| -------- | -------------------------------------------------------- | -------- |
| CPTA-001 | Discovery — confirm overlap with CGBDG / existing council | Ready    |
| CPTA-002 | Adopt structured finding schema in reviewer agents       | Proposed |
| CPTA-003 | Adopt prompt principles (tests-as-evidence, dedup, anchors) | Proposed |
| CPTA-004 | Findings persistence + lifecycle store                   | Proposed |
| CPTA-005 | `/council fix --finding <id>` + `/council revalidate`     | Proposed |
| CPTA-006 | Mock reviewer provider for CI dry-run                    | Proposed |
| CPTA-007 | Spike — is any pattern usable by Anvil-the-product?      | Ready    |

### CPTA-001 — Discovery — confirm overlap with CGBDG / existing council

- **Checkpoint:** Discovery memo lists each of the five clawpatch techniques
  and states, for each, whether it is (a) already covered by CGBDG /
  `plans/specs/2026-05-09-council-agent-skill-change-proposal.md` /
  existing reviewer agents, (b) net-new and worth adopting, or (c)
  net-new and worth declining.
- **Validate:** `plans/specs/YYYY-MM-DD-clawpatch-techniques-discovery.md`
  exists with per-technique verdict.
- **changeType:** docs
- **releaseIntent:** never

### CPTA-002 — Adopt structured finding schema in reviewer agents

- **Checkpoint:** Each of `council-reviewer.md`, `adversarial-reviewer.md`,
  `kernel-maintainer.md`, `operations-reviewer.md`, `pragmatic-lead.md`
  instructs the agent to emit findings in the clawpatch-style JSON shape
  (title / category / severity / confidence / evidence[] / reasoning /
  reproduction / recommendation / minimumFixScope / suggestedRegressionTest /
  whyTestsDoNotAlreadyCoverThis), with `evidence[]` requiring `path:line`
  anchors. `plan-synthesizer.md` is updated to deduplicate findings keyed
  by category + evidence path.
- **Validate:** Round-trip a `/council quick` against a known fixture
  branch; result deserialises against the documented schema.
- **Depends on:** CPTA-001
- **changeType:** internal
- **releaseIntent:** never

### CPTA-003 — Adopt prompt principles (tests-as-evidence, dedup, anchors)

- **Checkpoint:** Each reviewer agent prompt includes: "tests are evidence,
  not speculation"; "deduplicate by root cause — one finding with multiple
  evidence refs"; "every claim must cite `path:line`"; "do not speculate
  about code you have not read". `protocols.md` documents the per-file
  truncation budget (24 KB suggested) and the `isInside()` path-validation
  rule for any evidence ref.
- **Validate:** Diff `.claude/agents/*.md` shows the four principle clauses
  present in every reviewer; protocols carries the truncation + path rule.
- **Depends on:** CPTA-001
- **changeType:** internal
- **releaseIntent:** never

### CPTA-004 — Findings persistence + lifecycle store

- **Checkpoint:** Council runs persist findings under
  `review/state/findings/<id>.json` with `status ∈ {open, triaged, fixed,
  revalidated}` and a writer that refuses to overwrite a `fixed` /
  `revalidated` record without explicit re-open. Runs persist under
  `review/state/runs/<runId>.json`. Storage path layout documented in
  `protocols.md`. This is gated behind CPTA-001's recommendation —
  do not implement until CPTA-001 confirms it does not duplicate CGBDG.
- **Validate:** `/council quick <target>` produces durable IDs that
  `/council status` (existing subcommand) can list; a second run on the
  same target updates rather than duplicates findings.
- **Depends on:** CPTA-001, CPTA-002
- **changeType:** feature
- **releaseIntent:** never
- **Risk:** May overlap with CGBDG's attestation persistence design. If
  CPTA-001 finds overlap, fold this into CGBDG instead and close CPTA-004
  as superseded.

### CPTA-005 — `/council fix --finding <id>` + `/council revalidate`

- **Checkpoint:** Two new subcommands wired through `.claude/commands/council.md`:
  `fix --finding <id>` opens a focused edit session against a single finding
  with **dirty-worktree refusal**, **no auto-commit**, **no auto-PR**;
  `revalidate` re-runs the single-finding evidence anchor against current
  HEAD and updates lifecycle to `revalidated` (or back to `open` if the
  evidence re-fires).
- **Validate:** End-to-end on a synthetic finding: clean worktree → `fix
  --finding` produces an edit → manual commit → `revalidate` transitions
  status; dirty worktree → `fix` refuses with a clear error.
- **Depends on:** CPTA-004
- **changeType:** feature
- **releaseIntent:** never

### CPTA-006 — Mock reviewer provider for CI dry-run

- **Checkpoint:** `/council` accepts a `--provider mock` flag (or env
  override) that bypasses real reviewer agents and emits fixture findings.
  CI runs `/council quick --provider mock` against a fixture branch on
  every PR to lock the schema and the persistence contract.
- **Validate:** `pnpm test:council-mock` or equivalent passes in CI without
  spawning real agents.
- **Depends on:** CPTA-002
- **changeType:** internal
- **releaseIntent:** never

### CPTA-007 — Spike — is any pattern usable by Anvil-the-product?

- **Checkpoint:** Discovery memo answers, with evidence, three sub-questions:
  1. **Mapper heuristics seeding `anvil start` / CFGINT.** Today
     `crates/anvil-cli/src/activation/language_profile.rs` does extension-only
     language detection (TS/JS/HTML/CSS/SQL/Markdown); no manifest parsing,
     no ecosystem awareness. Architecture templates exist in
     `crates/anvil-architecture/src/definition.rs` (Starter / Layered /
     Hexagonal / Clean / DDD / Monorepo / Serverless / NxWorkspace / Custom)
     but none is auto-selected. The `config-intelligence` module (CFGINT,
     `plans/modules/config-intelligence.aps.md`, Draft, 0/7) is already
     chartered to extract dependency graphs from manifests
     (`package.json`, `Cargo.toml`, `go.mod`, `pyproject.toml`,
     `tsconfig.json`, etc.) but does not yet ship and has no design for
     auto-seeding `.anvil/architecture.yaml`. Clawpatch's mappers
     (`src/mappers/{node,next,rust,go,python,ruby,gradle,apple,swift,
     config}.ts`) produce dedup'd slice records with
     `{kind, source, entryPath, command|route|symbol}`.
     - Decide where this work belongs: **fold into CFGINT** (extend its
       task list to cover mapper heuristics + a template-picker), **slot
       into the activation orchestrator** (`crates/anvil-cli/src/activation/
       orchestrator/mod.rs` writes `.anvil/baseline.json` post-init — could
       additionally seed an architecture-yaml draft), or **stand up a new
       module**. Default expectation: extend CFGINT.
     - Decide what "seed" means: ecosystem detection only (npm-workspace,
       cargo-workspace, nx, next), or seed-plus-template-pick (detect
       monorepo shape → propose `NxWorkspace` template), or
       seed-plus-baseline-edges (detect slices → infer current dependency
       edges from manifests as a draft boundary set the user can refine).
     - Slices ≠ boundaries: feature slices say what units exist; boundaries
       say which units may depend on which. The memo must address how
       slices translate to (or fail to translate to) enforceable boundary
       rules without violating the "planless-first" and "warnings over
       blocks" principles in `docs/vision/anvil-scope-guard.md` and
       `.claude/rules/architecture.md`.
  2. **Finding schema for warning emission:** Anvil's save-time warnings
     are deterministic. Could the clawpatch finding shape (severity /
     evidence / recommendation / minimumFixScope / regressionTest) enrich
     `anvil status` JSON or the warning render? Cross-check against
     ProtectionClaim and existing diagnostic envelope.
  3. **Witness chain → clawpatch input:** could clawpatch (running adjacent
     to Anvil, not vendored into it) consume the witness chain to focus
     its slices on commits that crossed boundaries? This is the only
     adjacency that respects Anvil's determinism core.
- **Validate:** `plans/specs/YYYY-MM-DD-clawpatch-product-usability-spike.md`
  exists with a verdict per sub-question. For sub-question (1) the verdict
  must name the integration target (CFGINT extension / activation
  orchestrator slot / new module) and either (a) propose a follow-on task
  set against the chosen target, (b) file a discovery issue for further
  investigation, or (c) explicitly decline with rationale that survives the
  next planning council audit.
- **changeType:** docs
- **releaseIntent:** never
- **Risk:** Recommending adoption of any LLM-driven clawpatch component
  inside Anvil-the-product would violate the deterministic core principle
  in `docs/vision/anvil-scope-guard.md`. Sub-question (1) is the only one
  expected to land at (a). Sub-question (2) is expected at (c). Sub-question
  (3) is expected at (a) or (b).
- **Cross-reference:** `plans/modules/config-intelligence.aps.md` (CFGINT);
  `crates/anvil-cli/src/activation/language_profile.rs`;
  `crates/anvil-cli/src/activation/orchestrator/mod.rs`;
  `crates/anvil-architecture/src/definition.rs`.

## Dependencies

- **Sibling (dev-tooling layer):** CGBDG (council-gate-bridge) — both live
  under Dev Tooling Bridge. CPTA-001 must explicitly call out where the
  two modules overlap so neither re-does the council-judge → attestation
  work.
- **Sibling (product layer, CPTA-007 only):** CFGINT
  (`plans/modules/config-intelligence.aps.md`) — chartered to extract
  dependency graphs from manifests but not yet shipped. CPTA-007's
  default landing site for sub-question (1) is "extend CFGINT" rather
  than a new module.
- Existing: `.claude/agents/{council-reviewer,adversarial-reviewer,
  kernel-maintainer,operations-reviewer,pragmatic-lead,plan-synthesizer,
  protocols}.md` (reviewer prompt surface)
- Existing: `.claude/commands/{council,review}.md` (entrypoints)
- Existing: `scripts/agent/guidance.sh` (deterministic routing — must not regress)
- Existing (CPTA-007 only): `crates/anvil-cli/src/activation/`
  (language_profile + orchestrator — current `anvil start` flow);
  `crates/anvil-architecture/src/definition.rs` (template enum).
- Reference: `plans/specs/2026-05-09-council-agent-skill-change-proposal.md`
- Reference: `docs/vision/anvil-scope-guard.md` (binds CPTA-007 verdict —
  planless-first, deterministic, warnings over blocks)

## Risks

- **Schema drift vs CGBDG:** if CGBDG lands first with a different finding
  shape, this module's schema work would create two contracts. CPTA-001
  exists to detect that before CPTA-002 starts.
- **Determinism violation in product spike:** CPTA-007 must reject any
  pattern whose value depends on LLM output. Anvil's deterministic core
  is non-negotiable; only deterministic mapper heuristics or post-hoc
  adjacency (witness chain → external tool) are candidates.
- **Findings persistence becoming a third state store:** Anvil already has
  `.anvil/state.json` (execution) and the witness chain (provenance). A
  third store under `review/state/` for developer-tooling findings is
  acceptable only if CPTA-001 confirms no overlap with either. Otherwise
  fold into the existing surface.
- **Fix-loop bypassing council:** CPTA-005's `/council fix` must not bypass
  Council review or PR review — it is a focused edit-session helper, not
  an autonomy escalation. Dirty-worktree refusal and no-auto-commit are
  load-bearing safety properties, not nits.
