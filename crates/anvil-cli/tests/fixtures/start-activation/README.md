# `anvil start` activation output contracts

This directory is the fixture home for ACTTUI-000 / ADR-103. It pins the
non-interactive contracts that must not regress while `anvil start` grows an
interactive TUI:

- `anvil start --verify`
- `anvil start --json`
- compact plain output (`--no-tui`, piped output, CI)
- PTY lifecycle transcript for the opt-in TUI path

The fixture matrix, normalisation rules, and release-note template live in
[`plans/specs/2026-07-08-activation-tui-contract-fixtures.md`](../../../../../plans/specs/2026-07-08-activation-tui-contract-fixtures.md).

ACTTUI-001 may add the first test harness that reads this directory. ACTTUI-007
must make the fixture comparison mandatory before the TTY-default flip.
