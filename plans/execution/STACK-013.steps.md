# STACK-013: CLI stack status command

## Steps

1. [x] `apps/anvil-cli/src/commands/stack.ts` exists with `anvil stack status` subcommand
2. [x] `apps/anvil-cli/src/commands/stack/status.ts` implements layer health display
3. [x] `apps/anvil-cli/src/commands/stack.test.ts` exists (7 tests pass)
4. [x] Validation: `pnpm -F @eddacraft/anvil-cli run test -- run src/commands/stack.test.ts` passes
