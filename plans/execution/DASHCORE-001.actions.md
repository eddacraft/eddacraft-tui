# Dashboard Current-State Health Cards Implementation Plan

**Goal:** Deliver DASHCORE-001 as an honest current-state metric-card row over
the existing typed `ProtectionOverview` resource.
**Architecture:** A pure React adapter derives display values from the generated
OpenAPI type already returned by the Protection Overview query. It renders
inside the existing module after the protection/attention summary and makes no
new API request or domain decision. Historical data remains owned by
DASHCORE-002.
**Tech Stack:** React 19, TypeScript, Vitest, Tailwind v4/CSS, generated OpenAPI
types, existing shadcn Card primitive.

---

## File Map

- `plans/modules/dashboard-core-views.aps.md` — authoritative split, source
  mapping, statuses, and acceptance.
- `plans/index.aps.md` — current NBI and module status.
- `plans/specs/2026-07-18-dashboard-core-wave-2-split.md` — approved design and
  historical-range semantics.
- `apps/dashboard/src/components/primitives/metric-card.tsx` — accessible
  presentational card API and state hook.
- `apps/dashboard/src/modules/core/overview/current-health-cards.tsx` — pure
  derivation and five-card layout.
- `apps/dashboard/src/modules/core/overview/current-health-cards.test.tsx` —
  complete/partial/unavailable boundary coverage.
- `apps/dashboard/src/modules/protection/protection-overview.tsx` — place cards
  in the approved reading order.
- `apps/dashboard/src/modules/protection/protection-overview.test.tsx` — module
  integration assertion.
- `apps/dashboard/src/styles.css` — responsive card grid and state styling.

## Task 1: Pin honest current-state derivation

**Files:**

- Create: `apps/dashboard/src/modules/core/overview/current-health-cards.test.tsx`
- Create: `apps/dashboard/src/modules/core/overview/current-health-cards.tsx`

- [x] Write a failing test for the five current facts from a complete typed
      fixture.
- [x] Add failing cases for partial warnings, unavailable warnings, absent
      gate evidence, absent assurance, and absent timestamps.
- [x] Run
      `pnpm exec nx run dashboard:test --skip-nx-cache -- --run current-health-cards`
      and verify failure because the component does not exist.
- [x] Implement pure derivation helpers and the minimal component.
- [x] Run the targeted test and verify all cases pass.

## Task 2: Integrate the cards into Protection Overview

**Files:**

- Modify: `apps/dashboard/src/modules/protection/protection-overview.tsx`
- Modify: `apps/dashboard/src/modules/protection/protection-overview.test.tsx`
- Modify: `apps/dashboard/src/components/primitives/metric-card.tsx`
- Modify: `apps/dashboard/src/styles.css`

- [x] Add a failing integration assertion that the card group follows the
      protection/attention summary and precedes detailed tables.
- [x] Add the responsive five-card grid with direct text labels and explicit
      state attributes.
- [x] Preserve the existing desktop/mobile reading order and ensure no
      horizontal page overflow.
- [x] Run the dashboard test target and verify it passes.

## Task 3: Validate and review

**Files:**

- Modify only if validation or Council exposes a defect.

- [x] Run dashboard test, lint, typecheck, and build targets without Nx cache.
- [x] Run `pnpm validate:changed`, `pnpm aps:active-lint`,
      `pnpm aps:index:check`, `pnpm docs:check`, `pnpm format:check`, and
      `git diff --check`.
- [x] Run desktop and mobile visual QA; retain evidence without committing
      generated output unless governance requires it.
- [x] Run Council against the worktree diff and resolve all critical/major
      findings.
- [x] Record the continuous-improvement closeout in the pending queue.
