# DOCRB-010 — Independent documentation re-baseline verification

## ReadyItem

- **Target:** DOCRB-010
- **Decision:** ready
- **Goal:** Independently prove from a clean checkout that the documentation system is navigable, accurate, accessible, maintainable, and correctly enforced before the DOCRB programme is called complete.
- **Expected behaviour:**
  - A clean Worktrunk installs exactly from the lockfile and verifies Mermaid CLI 11.16.0 before inspecting the corpus.
  - Representative maintainer, contributor, operator, and public-reader journeys locate the right authority and trace documented nodes, arrows, routes, and public flows to current source and contracts.
  - Every governed Mermaid fence renders; public Draw.io/SVG source, export, accessibility, mounting, and parity checks pass.
  - Relevant source changes and deletions fail when an owning diagram is untouched; an updated owner and an unrelated change pass without a waiver.
  - All three production documentation applications build, and the two curated public journey diagrams are inspected on their mounted routes in light and dark modes.
  - Residual gaps are recorded as bounded follow-up proposals rather than repaired or silently accepted inside DOCRB-010; the operator separately authorised #4114, #4115, and #4116 on 2026-08-24.
- **Dependencies:** DOCRB-009 is Merged via PR #4099 and its reconciliation is Merged via PR #4102 on the pinned starting receipt `abe6be8b657b8be68565aace3aada6056323ae61`.
- **Scope:** Exactly four repository paths: `plans/modules/docs-rebaseline.aps.md`, `plans/index.aps.md`, this action plan, and `plans/reviews/2026-08-23-docrb-010-clean-room-verification.md`.
- **Capacity exception:** On 2026-08-23 the operator approved one additional DOCRB-010 Worktrunk after the conservative cleanup dry-run found zero eligible removals. No existing worktree may be force-cleaned or removed by this item.
- **Risk:** High. This verification decides whether the DOCRB programme can be called complete; it requires fresh independent verification and Council review.
- **Non-scope:** Product, documentation, diagram, checker, build, or workflow repairs; new diagrams; changing DOCFRESH, DOCSYNC, DOCDEF, DSITE, or another module's lifecycle; release claims or release gating; automatic GitHub/APS follow-up creation; administrator or policy bypass. A discovered gap blocks completion until it is reported and separately authorised.
- **Validation:** `pnpm install --frozen-lockfile && pnpm exec mmdc --version && node --test scripts/docs/check-diagram-impact.test.mjs && node scripts/docs/check-diagram-impact.mjs --json && pnpm test:docs-check && pnpm docs:check && pnpm docs:public:check && pnpm docs:public:diagrams && pnpm docs:owed --since abe6be8b657b8be68565aace3aada6056323ae61 --fail-on-owed && pnpm docs:index:check && pnpm test:ci-classify && pnpm test:validate-local && pnpm test:ci-integration && pnpm exec nx test docs-shell && pnpm --filter @eddacraft/anvil-docs-private build && pnpm --filter @eddacraft/docs-public build && pnpm --filter @eddacraft/docs-shell build && pnpm validate:changed && pnpm format:check && pnpm lint:check && pnpm aps:active-lint && pnpm aps:index:check && pnpm aps:drift --json && git diff --check`

## Actions

### 1. Establish the clean-room baseline

- **Checkpoint:** Record exact base, clean status, frozen-lockfile install, Node/pnpm versions, exact Mermaid 11.16.0 output, and inherited warnings before lifecycle or evidence edits.
- **Validate:** `pnpm install --frozen-lockfile && pnpm exec mmdc --version && pnpm docs:check`

### 2. Exercise authority, navigation, and source accuracy

- **Depends on:** 1
- **Checkpoint:** A maintainer locates component-local authority and traces representative nodes/arrows to source; an operator traces the live docs-shell/private/public topology to deployment and proxy truth; a public reader reaches the anvil detect/fix/verify and APS work-item lifecycle pages through the mounted discovery surfaces.
- **Validate:** `pnpm docs:public:check && pnpm docs:index:check && pnpm docs:check`

### 3. Exercise maintenance enforcement

- **Depends on:** 1
- **Checkpoint:** Fresh fixtures prove relevant changes and deletions fail with the owning document and upstream named; updating the owner passes; an unrelated change passes without a marker; classifier, local validation, and CI routing agree.
- **Validate:** `node --test scripts/docs/check-diagram-impact.test.mjs && pnpm test:ci-classify && pnpm test:validate-local && pnpm test:ci-integration`

### 4. Exercise rendering and accessibility

- **Depends on:** 2, 3
- **Checkpoint:** Corpus Mermaid rendering is complete; public diagram parity is clean; private, public, and shell builds pass; mounted anvil and APS journey diagrams return 200, load completely, expose meaningful text, and remain legible and unclipped in light and dark modes.
- **Validate:** `node scripts/docs/check-diagram-impact.mjs --json && pnpm docs:public:diagrams && pnpm --filter @eddacraft/anvil-docs-private build && pnpm --filter @eddacraft/docs-public build && pnpm --filter @eddacraft/docs-shell build`

### 5. Record independent evidence and disposition gaps

- **Depends on:** 1, 2, 3, 4
- **Checkpoint:** The exact-head report distinguishes passes, inherited baselines, environment limitations, and blocking residual gaps. No gap is repaired or silently accepted in this item; ADR/topology and diagram-enforcement residuals are separately authorised as #4114, #4115, and #4116.
- **Validate:** `pnpm docs:owed --since abe6be8b657b8be68565aace3aada6056323ae61 --fail-on-owed && pnpm format:check && pnpm lint:check && pnpm aps:active-lint && pnpm aps:index:check && pnpm aps:drift --json && git diff --check`
