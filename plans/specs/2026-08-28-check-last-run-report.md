# anvil check last-run report

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Authoritative for CHKLR-001 | CHKLR | Accepted | 2026-08-28 — design approved in-thread; implementation via CHKLR-001 |

| Upstream | Downstream |
| -------- | ---------- |
| Operator design (2026-08-28): editor-then-agent last-run files; [ADR-056](../decisions/056-format-flag-output-selector.md); `crates/anvil-cli/src/commands/check.rs`; `insights --output` write hardening | [CHKLR-001](../archive/modules/check-last-run.aps.md); public `anvil check` docs in the same change |

**Design approved 2026-08-28.** Execution authority is CHKLR-001. This
specification does not authorise product code on its own.

## 1. Problem

`anvil check` only writes to stdout. Human mode is a linear dump (and `tui` is
the same `print_human` path as `plain`), so the only way back into a large
result is terminal scrollback. `--format json` and `--format sarif` already
exist, but they still go to stdout; there is no durable last-run artefact.

The job is sequential: work through findings in an editor, then hand the same
run to an agent without scanning again.

## 2. Decision

Every `anvil check` run that produces a result overwrites two last-run files
under the workspace root:

| Path | Contents |
| ---- | -------- |
| `.anvil/last-check.txt` | Uncoloured human report (same text as plain stdout, including the blocking banner when it applies) |
| `.anvil/last-check.json` | Existing check JSON schema (`CHECK_OUTPUT_VERSION` / `CheckOutput`), pretty-printed |

`--format` continues to select **stdout only**. The files are written on every
such run, including `--format json` and `--format sarif`.

## 3. Behaviour

### When to write

Write after any path that produces a check result, including:

- findings present
- clean (no warnings)
- empty / no files / no planless-eligible checks / no analysable files

Do **not** write when the command fails before a result exists (for example a
flag conflict). Concurrent runs may race; last writer wins.

### Stdout and stderr

- Stdout shape, exit code, and JSON/SARIF purity are unchanged (ADR-056).
- Plain/TUI: one stderr success line, exactly
  `Last run written to .anvil/last-check.txt`.
- JSON/SARIF: no success footer.
- Write failure: warn on stderr, do not change the check result or exit code.
  The previous last-run files may then be stale.

### Location and hardening

- Resolve paths from the same workspace root `check` already uses for
  `.anvilrc` / path relativisation.
- Create `.anvil/` if needed. That is local runtime state, not activation: do
  not write `first-run`, baseline, cache, or MCP state.
- Follow the `insights --output` write rules: refuse a symlink destination,
  overwrite a regular file, Unix mode `0o600`.
- `.anvil/` is already gitignored (ADR-073).

### Content rules

- Human file: `render_human` output, uncoloured. Include the blocking banner
  when the run has blocking findings. Empty/early messages use the same text
  stdout already prints.
- JSON file: the same `CheckOutput` document stdout would emit under
  `--format json`, including empty documents. Pretty-print. No new schema.
- Secret findings stay redacted as they are on stdout. Do not add snippets.

## 4. Non-goals

- `--output PATH` / `--save-report`
- Last-run files for `anvil gate` or `anvil audit`
- A pager, SARIF-on-disk, HTML report, or new finding model
- Changing `--format`, `--json`, or exit codes

## 5. Docs impact

This is a user-visible `anvil check` behaviour change. In the same
implementation change, update the public check reference/how-to (and clap
long-help if that is the generated CLI authority) so the last-run paths are
discoverable. Do not invent a second output selector beside ADR-056.

## 6. Decision log

| ID | Decision |
| -- | -------- |
| D-1 | Human first, then agent, same run |
| D-2 | Dual files: existing human text + existing check JSON |
| D-3 | Always overwrite, including clean and empty runs |
| D-4 | Stdout unchanged; files independent of `--format` |
| D-5 | Plain/TUI stderr success footer; JSON/SARIF silent on success |
| D-6 | Write failure is advisory (warn, same result and exit code) |
| D-7 | No `--output` and no gate/audit last-run in this slice |
