# Issues (running list)

| ID | Title | Status | Area | Severity | Owner | Next action |
|---:|---|---|---|---|---|---|
| ISS-007 | `preserve-caught-error` warnings across CLI (4 remaining) | resolved | cli | low | aneki | Fixed — PR #513. Added `{ cause }` to re-thrown errors in all catch blocks. Subsumed by ISS-006 fix. |
| ISS-006 | `preserve-caught-error` warnings across CLI, core, and runtime (9 total) | resolved | cli, core, runtime | low | aneki | Fixed — PR #513. All 9 warnings resolved by adding `{ cause }` to re-thrown errors. |
| ISS-005 | TUI lint warnings: `no-array-index-key` (17), `no-direct-set-state-in-use-effect` (11), leaked timeouts (2) | resolved | cli/tui | low | aneki | Fixed — warnings reduced from 38 to 4; remaining warnings tracked under ISS-007 and resolved in PR #513. |
| ISS-004 | Pulumi Preview CI check failing on main (pre-existing) | archived | infra | medium | — | Not a code bug. `infra.yml` has `check-secrets` guard that skips Pulumi when Azure creds aren't configured. Requires credential provisioning in GitHub secrets, not code changes. |
| ISS-003 | Windows CI: git-status test path separator fragility | resolved | runtime | medium | aneki | Fixed in feat/edda-stack (path.sep instead of hardcoded '/') — merged to main |
| ISS-002 | Welcome quick-start uses fire-and-forget spawn (exit/failure not surfaced) | resolved | cli | medium | aneki | Fixed — `runCommand` now awaits child process and propagates exit code |
| ISS-001 | `anvil tutorial --no-tui` advertises plain text but exits with unsupported message | resolved | cli | medium | aneki | Fixed — removed `--no-tui` option from tutorial command |
