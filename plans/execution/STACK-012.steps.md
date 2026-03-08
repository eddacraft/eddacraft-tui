# STACK-012: Stack configuration schema

## Steps

1. [x] `packages/edda-stack/src/config.ts` exists with stack-wide Zod configuration schema
2. [x] `packages/edda-stack/src/config.test.ts` exists (24 tests pass)
3. [x] Validation: `pnpm nx test @eddacraft/anvil-edda-stack --testNamePattern="stack.*config"` passes
