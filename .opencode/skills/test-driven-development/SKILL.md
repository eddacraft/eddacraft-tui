---
name: test-driven-development
description: Anvil TDD workflow for planning-workflow/dev-workflow authorised work: red, green, refactor, targeted verification, and evidence capture.
---

# Test-Driven Development Skill

## Source And Variant

This is the Anvil vendored variant of the neutral EddaCraft skill at
`eddacraft-skills/skills/eddacraft/test-driven-development`. Keep the
red-green-refactor contract aligned, but preserve Anvil-specific APS entry
conditions, validation commands, and closeout evidence here.

## OpenCode Surface

Use OpenCode shell/tool execution for targeted tests and invoke `task` with a
TDD/debugging agent only when the work needs delegation. Do not assume Claude
slash commands are available from this copy.

## Entry Conditions

Use this skill only for the Code stage after `planning-workflow` has handed off
`ready-for-dev` or `aps-planning` has returned `valid`, and any required APS
updates have been made.

Before code:

1. Confirm the APS work item, expected outcome, and validation command.
2. Confirm the worktree and branch follow Anvil's Worktrunk/main-first rules.
3. Identify the smallest behaviour that proves progress.
4. Choose the narrowest useful test surface.
5. Confirm no unresolved `needs-design`, `needs-plan-update`, or `blocked`
   decision remains from planning.

If the work cannot reasonably be test-first, record why in the APS item or PR
test plan and define replacement evidence before implementation.

If expected behaviour, scope, or validation is unclear, stop and return to
`planning-workflow` or `aps-planning`. Do not resolve planning ambiguity in the
implementation loop.

## Red-Green-Refactor

1. **Red:** write or update the smallest failing test for the behaviour. Run the
   targeted command and confirm it fails for the expected reason.
2. **Green:** make the smallest correct implementation change. Run the same
   targeted test and confirm it passes.
3. **Refactor:** simplify names, structure, and duplication without changing
   behaviour. Re-run targeted tests.

Repeat this loop for each behaviour slice. Do not batch broad implementation
before proving the first red/green cycle.

## Anvil Test Surfaces

- TypeScript unit: `pnpm exec nx run <project>:test` or `pnpm test`
- TypeScript E2E: `pnpm --filter @eddacraft/anvil-e2e test`
- Rust unit: `cargo test --workspace` or targeted `cargo test -p <crate>`
- Rego: `opa test --verbose policies/fixtures/`
- Full closeout:
  `pnpm format:check && pnpm lint:check && pnpm typecheck && pnpm test`

Use the APS item's `Validation:` command when present. If it is stale, return to
`aps-planning` and update the plan before continuing.

For Anvil closeout, the normal full local gate is
`pnpm format:check && pnpm lint:check && pnpm typecheck && pnpm test`; add
`cargo test --workspace` and policy/E2E commands when the touched surfaces
require them.

## Test Design

- Prefer behaviour tests over implementation-detail tests.
- Cover failure paths and boundary conditions, not only happy paths.
- Use existing fixtures and helpers before creating new ones.
- Keep tests deterministic: avoid real time, network, randomness, and shared
  mutable state unless the product behaviour requires them.
- Mock external boundaries, not the code under test.

## Debug Escalation

If a test fails unexpectedly, stop normal TDD and invoke `systematic-debugging`.
Do not stack speculative fixes.

## Completion Evidence

Before leaving the Code stage, capture:

- The failing test or changed test that drove the implementation
- The targeted command proving it now passes
- Any broader validation needed by APS or repo rules
- Any APS validation command that changed and needs plan reconciliation
- Whether the APS item can move to reconciliation or still needs follow-up work
