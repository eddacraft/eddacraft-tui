# STACK-001: Common identifier schemas

## Steps

1. [x] `packages/edda-stack/src/contracts/identifiers.ts` exists with `ObservationId`, `ProposalId`, `MemoryId` Zod schemas
2. [x] Identifier tests covered in `packages/edda-stack/src/contracts/contracts.test.ts` (68 tests pass)
3. [x] Validation: `pnpm nx test @eddacraft/anvil-edda-stack` passes (470 tests)
