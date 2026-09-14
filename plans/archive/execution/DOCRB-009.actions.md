# DOCRB-009 — Mandatory diagram review and enforcement

## ReadyItem

- **Target:** DOCRB-009
- **Decision:** ready
- **Goal:** Activate the proven ADR-123 diagram-impact rule as a mandatory, change-scoped maintenance check without adding noise to unaffected changes.
- **Expected behaviour:**
  - Root agent and contributor contracts require an update-or-unaffected diagram-impact disposition for the existing architecture and public-flow triggers.
  - A pinned Mermaid CLI 11.16.0 renders live, affected internal diagrams with file-level diagnostics.
  - Existing public Draw.io/SVG parity and declared file-level freshness checks remain authoritative and continue to fail closed.
  - One trusted change-routing signal selects diagram-impact validation in existing CI and local validation surfaces.
  - Fixtures prove relevant declared-upstream drift fails, an updated/renderable owning diagram passes, malformed affected Mermaid fails, and irrelevant changes pass without a waiver.
- **Dependencies:** DOCRB-003, DOCRB-004, DOCRB-005, DOCRB-006, DOCRB-007, and DOCRB-008 are Merged on `origin/main` at `89a6d2050b7a93e69ac6ea99848d9dfa7c63be3c`.
- **Scope:** The exact 35 paths named by the DOCRB-009 module item: four policy/guide paths, three pinned-toolchain paths, four docs-check implementation/test paths, seven existing CI/local routing and contract-test paths, two APS lifecycle paths, this action plan, thirteen triggered freshness/review metadata closeouts, and one evidence report.
- **Approved freshness closeout:** `plans/specs/2026-08-19-anvil-docs-definition-layer.md`, `apps/docs-shell/ARCHITECTURE.md`, `docs/architecture/docs-delivery.md`, `docs/architecture/overview.md`, `docs/architecture/trust-and-deployment-boundaries.md`, `docs/reviews/shipped-codebase-review-checklist.md`, `docs/README.md`, `docs/architecture/README.md`, `docs/governance/tags-catalogue.md`, `docs/guides/README.md`, `docs/guides/adapters/README.md`, `docs/reviews/README.md`, and `docs/guides/testing.md` are metadata-only review dispositions; no diagram, topology, authority, navigation, or product prose changed.
- **Approved toolchain:** On 2026-08-21 the operator directly approved exact `@mermaid-js/mermaid-cli@11.16.0` as a root development dependency plus only the required Puppeteer build allowance. No other dependency or build-script approval is authorised.
- **Approved routing:** On 2026-08-21 the operator directly approved one `diagram-impact` routing signal through the existing trusted classifier, Docs Lint workers, and local validator. It creates no new job or required status.
- **Capacity exception:** On 2026-08-21 the operator directly approved one fresh DOCRB-009 Worktrunk despite the WIP count after a conservative cleanup dry-run found zero eligible removals; nothing was deleted.
- **Non-scope:** DOCRB-010; new or rewritten diagrams; diagram-content repair not caused by this diff; public IA, content, or navigation; PR #4050 absorption; DOCFRESH mechanics or baseline changes; Draw.io exporter/checker changes; PR-template or PR-body parsing; new CI jobs or required statuses; release gating; administrator/policy bypass; generated indexes without changed output; product runtime code; sibling-module lifecycle.
- **Validation:** `node --test scripts/docs/check-diagram-impact.test.mjs && pnpm test:docs-check && pnpm docs:check && pnpm docs:public:diagrams && pnpm docs:owed --since 89a6d2050b7a93e69ac6ea99848d9dfa7c63be3c --fail-on-owed && pnpm test:ci-classify && pnpm test:validate-local && pnpm test:ci-integration && pnpm format:check && pnpm lint:check && pnpm aps:active-lint && pnpm aps:index:check && pnpm aps:drift --json && git diff --check`

## Actions

### 1. Activate the update-or-unaffected contract

- **Checkpoint:** `AGENTS.md`, `CONTRIBUTING.md`, and the two owning guides state the existing ADR-123 triggers as mandatory and keep detailed procedure in documentation governance.
- **Validate:** `pnpm docs:check`

### 2. Pin and prove Mermaid rendering

- **Depends on:** 1
- **Checkpoint:** The exact Mermaid CLI 11.16.0 dependency renders affected live non-archive Mermaid fences and reports the owning file and fence on failure.
- **Validate:** `node --test scripts/docs/check-diagram-impact.test.mjs && pnpm docs:check`

### 3. Enforce declared-upstream diagram impact

- **Depends on:** 2
- **Checkpoint:** A relevant changed upstream with an untouched owning diagram fails; an updated and renderable owner passes; irrelevant changes pass without a marker or waiver; existing public parity and docs-owed gates remain composed rather than duplicated.
- **Validate:** `node --test scripts/docs/check-diagram-impact.test.mjs && pnpm docs:public:diagrams && pnpm docs:owed --since 89a6d2050b7a93e69ac6ea99848d9dfa7c63be3c --fail-on-owed`

### 4. Route the one signal through existing CI and local validation

- **Depends on:** 3
- **Checkpoint:** The trusted classifier exposes one `diagram-impact` required-check signal; existing Docs Lint corpus/tooling and local changed validation honour it; classifier failure remains fail-closed; no new CI job or status exists.
- **Validate:** `pnpm test:ci-classify && pnpm test:validate-local && pnpm test:ci-integration`

### 5. Record exact-head evidence

- **Depends on:** 1, 2, 3, 4
- **Checkpoint:** The evidence report records RED/GREEN fixtures, exact renderer version, affected/unaffected classification, public parity, freshness, APS, formatting, lint, and diff gates at one clean head.
- **Validate:** `pnpm test:docs-check && pnpm docs:check && pnpm format:check && pnpm lint:check && pnpm aps:active-lint && pnpm aps:index:check && pnpm aps:drift --json && git diff --check`
