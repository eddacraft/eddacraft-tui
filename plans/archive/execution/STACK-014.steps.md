# STACK-014: CLI stack validate command

## Steps

1. [x] `apps/anvil-cli/src/commands/stack/validate.ts` exists with `anvil stack validate` subcommand
2. [x] Validate subcommand checks provenance integrity and reports broken chains
3. [x] Validation: `pnpm -F @eddacraft/anvil-cli run test -- run src/commands/stack.test.ts` passes (7 tests)
