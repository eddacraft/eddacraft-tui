<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if work items exist and status is Ready. -->

# Check Last-Run Report

| ID    | Owner | Status      | Progress |
| ----- | ----- | ----------- | -------- |
| CHKLR | —     | In Progress | 0/1      |

**Last reviewed:** 2026-08-28 — design approved in-thread; spec
[`2026-08-28-check-last-run-report.md`](../specs/2026-08-28-check-last-run-report.md)
accepted. Exclusive module: feature PRs update only this item's status;
stored `N/M` is reconciled separately under ADR-053. Not a release claim.

## Purpose

Make `anvil check` results durable so a person can work through findings in
an editor and then hand the same run to an agent, instead of relying on
terminal scrollback.

## In Scope

- Last-run files for `anvil check` only, per the accepted spec.
- Stdout/stderr contracts in that spec (human footer, JSON/SARIF purity,
  fail-open write).
- Public check docs (and clap long-help if that is the generated CLI
  authority) in the same change.

## Out of Scope

- `--output` / `--save-report`
- Last-run files for `anvil gate` or `anvil audit`
- Pager, SARIF-on-disk, HTML report, new finding model
- Activation, baseline, cache, or MCP state under `.anvil/`

## Interfaces

**Depends on:**

- `crates/anvil-cli/src/commands/check.rs` — existing `render_human` and
  `CheckOutput` JSON
- [ADR-056](../decisions/056-format-flag-output-selector.md) — `--format`
  remains the stdout selector
- `insights --output` write hardening (symlink refuse, `0o600`, overwrite
  regular file) as the file-write pattern

**Exposes:**

- Workspace-root `.anvil/last-check.txt` and `.anvil/last-check.json`
- Unchanged `anvil check` stdout and exit codes

## Ready Checklist

- [x] Design approved 2026-08-28
- [x] Spec accepted at
      [`plans/specs/2026-08-28-check-last-run-report.md`](../specs/2026-08-28-check-last-run-report.md)
- [x] CHKLR-001 has observable outcomes and exact validation commands
- [x] Gate/audit last-run and `--output` remain out of scope

## Work Items

### CHKLR-001: Persist last-run report files for `anvil check`

- **Status:** In Progress
- **Intent:** Every `anvil check` run that produces a result overwrites a
  human last-run file and a JSON last-run file under `.anvil/`.
- **Expected Outcome:**
  - `.anvil/last-check.txt` is the uncoloured human report (same text as
    plain stdout, including the blocking banner when it applies).
  - `.anvil/last-check.json` is the existing check JSON document,
    pretty-printed.
  - Files are written on clean, empty, and finding runs, including when
    stdout is JSON or SARIF.
  - Plain/TUI prints `Last run written to .anvil/last-check.txt` on
    stderr; JSON/SARIF stdout stays a single document.
  - Write failure warns on stderr and does not change the check result or
    exit code.
  - Creating `.anvil/` for these files is not activation.
  - Public check docs name the last-run paths.
- **Files:** `crates/anvil-cli/src/commands/check.rs`,
  `crates/anvil-cli/src/output/last_run.rs`,
  `docs/public/anvil/tutorials/rust-project.md`,
  `plans/specs/2026-08-28-check-last-run-report.md`
- **Validation:**
  `cargo test -p eddacraft-anvil --no-fail-fast last_run`
- **Dependencies:** —
- **Confidence:** high
- **Design:** [check last-run report](../specs/2026-08-28-check-last-run-report.md)
