<!--
APS Steps: HTMLCSS-001
======================
Make analysable extensions configurable.
See: plans/aps-rules.md
-->

# Steps: HTMLCSS-001

| Field  | Value                                                                      |
| ------ | -------------------------------------------------------------------------- |
| Source | [../modules/html-css-support.aps.md](../modules/html-css-support.aps.md)   |
| Task   | HTMLCSS-001 — Make analysable extensions configurable                      |
| Status | Draft                                                                      |

## Prerequisites

- [ ] `pnpm build` succeeds
- [ ] `pnpm test` passes

## Steps

### 1. Add `extensions` field to Anvil config schema

- **Checkpoint:** `packages/platform/config/src/schema.ts` has `extensions`
  field with default `['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs']`
- **Validate:** `pnpm -F config test`
- **Pattern:** Existing config fields in same file

### 2. Update check command to read extensions from config

- **Checkpoint:** `apps/anvil-cli/src/commands/check.ts` reads extensions from
  loaded config instead of hard-coded `ANALYSABLE_EXTENSIONS`
- **Validate:** `pnpm -F anvil-cli test -- --testNamePattern="check"`
- **Pattern:** Other config usage in check.ts

### 3. Add `--extensions` CLI flag override

- **Checkpoint:** `anvil check --extensions .html,.css` overrides config
- **Validate:**
  `pnpm -F anvil-cli test -- --testNamePattern="check.*extension"`

### 4. Update antipattern check config

- **Checkpoint:** `packages/anvil/runtime/src/gate/checks/antipattern.check.ts`
  reads extensions from config instead of DEFAULT_CONFIG hard-coded list
- **Validate:** `pnpm -F anvil-runtime test`

### 5. Update git status checker

- **Checkpoint:** `getChangedFiles` in
  `packages/anvil/runtime/src/watch/git-status.ts` passes config-sourced
  extensions instead of hard-coded list
- **Validate:** `pnpm -F anvil-runtime test`

### 6. Add tests for new config field

- **Checkpoint:** Tests verify: default extensions, config override, CLI
  override, CLI + config merge behaviour
- **Validate:** `pnpm test`

## Completion

- [ ] All checkpoints validated
- [ ] `pnpm test` passes
- [ ] No hard-coded extension lists remain outside of default values
- [ ] Task marked complete in source module
