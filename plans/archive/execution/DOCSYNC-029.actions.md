# APS Public Documentation Rebuild Implementation Plan

**Goal:** Rebuild the public APS documentation so a first-time user can install
APS, create and validate a plan, and understand the next workflow step from
standalone, current guidance.

**Architecture:** Keep the existing public URLs where practical, but reorganise
the sidebar and page responsibilities around tutorials, how-to guides, concepts,
examples, and reference. Capture the audited anvil-plan-spec v0.6.0 CLI surface
in a source-pinned contract snapshot, then extend the existing public-doc checks
to validate APS casing, internal-reference boundaries, navigation coverage, and
fenced command examples.

**Tech Stack:** Markdown, Docusaurus sidebar configuration, Node.js validation
scripts, shell test fixtures, APS planning files.

---

## File Map

- `docs/public/aps/overview.md` — explain what APS does and route a new user.
- `docs/public/aps/getting-started.md` — own the canonical install-to-first-value journey.
- `docs/public/aps/installation.md` — cover alternative installation, updates, migration, and platforms without duplicating the tutorial.
- `docs/public/aps/workflow.md` — teach the day-to-day `next` → `start` → validate → `complete` loop.
- `docs/public/aps/terminology.md` — define public vocabulary without migration history or repository extensions.
- `docs/public/aps/spec/taxonomy.md` — explain the document hierarchy and authority model.
- `docs/public/aps/spec/file-layout.md` — describe generated and optional project files from current scaffold behaviour.
- `docs/public/aps/spec/determinism.md` — explain lint, audit, and CI safety boundaries.
- `docs/public/aps/schemas/json-schema.md` — provide the current Markdown document contract.
- `docs/public/aps/schemas/examples.md` — provide copyable, lint-compatible fragments.
- `docs/public/aps/examples/minimal-plan.md` — provide a small complete plan.
- `docs/public/aps/examples/multi-module.md` — provide a complete modular plan.
- `docs/public/aps/guides/ai-agents.md` — explain optional agent setup and safe execution.
- `docs/public/aps/guides/monorepo.md` — distinguish tagged and federated monorepo tiers.
- `docs/public/aps/tooling/validation.md` — provide the v0.6.0 CLI reference.
- `apps/docs-site/sidebars/aps.ts` — expose every APS page by user intent.
- `scripts/docs/aps-public-cli-contract.json` — pin the audited upstream version, commit, commands, and flags.
- `scripts/docs/check-aps-public-commands.mjs` — validate fenced APS command examples against the pinned contract.
- `scripts/docs/check-public-docs.mjs` — apply public-only, casing, and navigation checks to APS.
- `scripts/docs/docs-check.test.sh` — prove the APS public-doc and command boundaries fail closed.
- `package.json` — expose the APS command-example check for direct use.
- `plans/modules/documentation-sync.aps.md` — authorise DOCSYNC-029 and record its result.
- `plans/index.aps.md` — reconcile the DOCSYNC status summary at handoff.
- `plans/reviews/continuous-improvement-log.md` — record the required closeout improvement.

## Actions

### Action 1 — Lock the public APS trust boundary

**Purpose**
Make the desired newcomer and command-truth constraints executable before the
prose rewrite.

**Produces**
APS-aware validation fixtures, a source-pinned CLI contract, and a failing test
against the current v0.4.0 public section.

**Checkpoint**
Old APS guidance fails for the audited reasons.

**Validate**
`bash scripts/docs/docs-check.test.sh`

### Action 2 — Rebuild the first-success journey

**Purpose**
Give a reader a single path from prerequisites through a lint-clean first plan
and an explicit success state.

**Produces**
Rewritten overview, getting-started, installation, workflow, terminology, and
intent-based sidebar.

**Checkpoint**
A clean-room reader reaches `aps lint` success without external repository docs.

**Validate**
`pnpm docs:public:check`

### Action 3 — Rebuild concepts, examples, and reference

**Purpose**
Separate explanation and lookup material from the first-use tutorial while
covering the current CLI, scaffold, validation, agent, and monorepo contracts.

**Produces**
Rewritten specification, schema, examples, guides, and CLI reference pages.

**Checkpoint**
Every documented command and format claim matches the pinned v0.6.0 source audit.

**Validate**
`pnpm docs:public:aps-commands && pnpm docs:public:check`

### Action 4 — Verify the complete public surface

**Purpose**
Prove that docs, navigation, APS state, and the static site agree.

**Produces**
Fresh validation evidence and reconciled APS closeout.

**Checkpoint**
All targeted and repository documentation gates pass.

**Validate**
`pnpm docs:check && pnpm docs:public:aps-commands && pnpm aps:active-lint && pnpm aps:index:check && pnpm format:check && pnpm exec nx build docs-site`

## Completion

- [x] Public APS trust-boundary tests pass.
- [x] Every APS page is reachable from the sidebar.
- [x] Fenced APS commands match the pinned CLI contract.
- [x] The docs site builds with no broken links.
- [x] DOCSYNC-029 results and closeout evidence are recorded.
