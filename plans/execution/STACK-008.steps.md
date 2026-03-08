# STACK-008: Event bus for layer communication

## Steps

1. [x] `packages/edda-stack/src/contracts/events.ts` exists with observation, proposal, and promotion event schemas
2. [x] `packages/edda-stack/src/contracts/events.test.ts` exists (60 tests pass)
3. [x] Validation: `pnpm nx test @eddacraft/anvil-edda-stack --testNamePattern="events"` passes
