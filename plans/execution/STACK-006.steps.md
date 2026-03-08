# STACK-006: Observation → Proposal type mapping

## Steps

1. [x] `packages/edda-stack/src/contracts/events.ts` contains observation event types covering Kindling→Ember transition
2. [x] `packages/edda-stack/src/contracts/events.test.ts` exists (60 tests pass)
3. [x] Observation hook tested via `packages/edda-stack/src/ember/observation-hook.test.ts` (6 tests pass)
4. [x] Validation: `pnpm nx test @eddacraft/anvil-edda-stack` passes (470 tests)

> **Note:** `observation-mappings.ts` + `observation-mappings.test.ts` (25 tests) created in PR #515 (`feat/edda-stack-reconciliation`), pending merge to main.
