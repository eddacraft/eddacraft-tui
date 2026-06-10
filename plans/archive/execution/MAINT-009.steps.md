# MAINT-009 — Edda list filters parity with release claims

## Goal
Ensure `anvil edda list` capabilities and documentation/release messaging are consistent.

## Preconditions
- Workspace builds (`pnpm nx run-many -t build --all`)
- CLI runnable from `apps/anvil-cli/dist/index.js`

## Steps
1. Confirm current CLI behavior
   - Run: `node apps/anvil-cli/dist/index.js edda list --help`
   - Capture current supported filters.
2. Confirm release/docs claims
   - Inspect v0.2.1-beta release notes and relevant docs.
3. Decide parity strategy
   - Option A: implement missing filters (`--confidence`, age filter), OR
   - Option B: correct release/docs copy to current behavior.
4. If Option A selected
   - Add flags and query support in `commands/edda/list.ts`.
   - Add tests for parsing + filtering semantics.
5. If Option B selected
   - Update docs/release templates to remove unsupported claims.
6. Verify
   - `edda list --help` matches docs/release text exactly.
   - Tests pass for changed paths.
7. Commit with MAINT reference
   - Example: `fix(maint-009): align edda list filters with release/docs`

## Exit Criteria
- No mismatch between `edda list --help` and shipped release/docs language.
