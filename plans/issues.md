# Issues (running list)

| ID | Title | Status | Area | Severity | Owner | Next action |
|---:|---|---|---|---|---|---|
| ISS-007 | `preserve-caught-error` warnings across CLI (4 remaining) | new | cli | low | unassigned | Add `{ cause }` to re-thrown errors in catch blocks |
| ISS-006 | `preserve-caught-error` warnings across CLI, core, and runtime (9 total) | new | cli, core, runtime | low | unassigned | Add `{ cause }` to re-thrown errors in catch blocks |
| ISS-005 | TUI lint warnings: `no-array-index-key` (17), `no-direct-set-state-in-use-effect` (11), leaked timeouts (2) | resolved | cli/tui | low | aneki | Fixed — warnings reduced from 38 to 4 (all remaining are ISS-007) |
| ISS-004 | Pulumi Preview CI check failing on main (pre-existing) | new | infra | medium | unassigned | Investigate Pulumi Preview failures — blocks all PR CI checks |
| ISS-003 | Windows CI: git-status test path separator fragility | resolved | runtime | medium | aneki | Fixed in feat/edda-stack (path.sep instead of hardcoded '/') — merged to main |
| ISS-002 | Welcome quick-start uses fire-and-forget spawn (exit/failure not surfaced) | resolved | cli | medium | aneki | Fixed — `runCommand` now awaits child process and propagates exit code |
| ISS-001 | `anvil tutorial --no-tui` advertises plain text but exits with unsupported message | resolved | cli | medium | aneki | Fixed — removed `--no-tui` option from tutorial command |
