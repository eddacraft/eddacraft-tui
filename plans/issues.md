# Issues (running list)

| ID | Title | Status | Area | Severity | Owner | Next action |
|---:|---|---|---|---|---|---|
| ISS-006 | `preserve-caught-error` warnings across CLI, core, and runtime (9 total) | new | cli, core, runtime | low | unassigned | Add `{ cause }` to re-thrown errors in catch blocks |
| ISS-005 | TUI lint warnings: `no-array-index-key` (17), `no-direct-set-state-in-use-effect` (11), leaked timeouts (2) | new | cli/tui | low | unassigned | Batch-fix React lint warnings in TUI components before stable release |
| ISS-004 | Pulumi Preview CI check failing on main (pre-existing) | new | infra | medium | unassigned | Investigate Pulumi Preview failures — blocks all PR CI checks |
| ISS-003 | Windows CI: git-status test path separator fragility | resolved | runtime | medium | unassigned | Fixed in feat/edda-stack (path.sep instead of hardcoded '/') — merge to main |
| ISS-002 | Welcome quick-start uses fire-and-forget spawn (exit/failure not surfaced) | new | cli | medium | unassigned | Decide whether to await child completion and propagate status |
| ISS-001 | `anvil tutorial --no-tui` advertises plain text but exits with unsupported message | new | cli | medium | unassigned | Align CLI option description with behaviour or implement plain-text mode |
