# Pragmatic Lead — Ship/No-Ship, Test Quality, Completeness

**Reviewer:** pragmatic-lead
**Scope:** f0d34257..HEAD (7 commits, RCLI-015a/015b/022/023/030)
**Verdict:** SHIP with one major to fix

## Ship Assessment

This is a well-executed batch of Rust CLI cutover work. Five features land
together with solid test coverage and a clean Node.js archival. The work is
ready to ship with one fix.

## Findings

### MAJOR-PL-001: process::exit in gate command prevents terminal teardown
- **File:** `crates/anvil-cli/src/commands/gate.rs:759`
- **Category:** correctness
- **Severity:** major
- **Detail:** Same as KM-001. If `anvil gate` is ever invoked from the TUI hub
  (which it already is via `collect_gate_data` + welcome surface), and the
  standalone path is later unified, `process::exit` would corrupt the terminal.
  Must return an error instead. Quick fix: return
  `Err(anyhow::anyhow!("gate checks failed"))` and let main() set exit code 2.

### Test Coverage Assessment

| Area | Tests | Quality |
|------|-------|---------|
| credentials.rs | 14 tests | Excellent -- covers all 4 priority levels, migration, permissions, edge cases |
| main.rs (auth enforcement) | 18 tests | Excellent -- every command classified, evaluate_auth thoroughly tested |
| output/mod.rs | 6 tests | Good -- all resolution paths covered |
| gate.rs | 14 tests | Good -- profiles, coverage, dependency, architecture, policy checks tested |
| welcome.rs | 4 tests | Adequate -- basic path coverage |
| anvil-architecture/lib.rs | 3 tests | Adequate for a YAML-parse stub |
| welcome/mod.rs (TUI) | 6 tests | Good -- navigation, selection, quit |

Total: ~65 new/modified tests. Coverage is strong where it matters most
(credential handling, auth enforcement).

### MINOR-PL-002: No integration test for auth-gated command flow
- **File:** n/a
- **Category:** test gap
- **Severity:** minor
- **Detail:** Unit tests cover `requires_auth` and `evaluate_auth` independently
  but there is no test that exercises the full flow: parse command -> check auth
  -> reject/proceed. This could be a follow-up.

### MINOR-PL-003: Archival config cleanup is thorough
- **Files:** `pnpm-workspace.yaml`, `nx.json`, `eslint.config.mjs`, `vitest.config.ts`, `tsconfig.json`, `package.json`
- **Category:** completeness
- **Severity:** positive (no issue)
- **Detail:** All build system references to `apps/anvil-cli` are properly
  removed or redirected. Nx plugins exclude `archive/**`. pnpm workspace
  excludes `archive/**`. ESLint, vitest, and tsconfig references updated.
  Link/unlink scripts removed from package.json. Clean work.

## Summary

Ship it. Fix the `process::exit` in gate.rs (MAJOR-PL-001) before merge.
Everything else is minor/follow-up quality.
