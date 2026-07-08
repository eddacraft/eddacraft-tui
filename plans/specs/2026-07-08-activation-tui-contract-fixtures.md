# Activation TUI Contract Fixtures (ACTTUI-000)

| Type   | Authority | Owner  | Status | Freshness |
| ------ | --------- | ------ | ------ | --------- |
| Spec   | Proposal  | ACTTUI | Ready  | Authored 2026-07-08 against ACTTUI-000, ADR-103, `crates/anvil-cli/src/commands/start.rs`, and `crates/anvil-cli/src/activation/render.rs` |

| Upstream | Downstream |
| -------- | ---------- |
| [`ACTTUI-000`](../modules/activation-tui.aps.md), [`ADR-103`](../decisions/103-tty-default-activation-tui.md), `crates/anvil-cli/src/commands/start.rs`, `crates/anvil-cli/src/activation/render.rs` | ACTTUI-001, ACTTUI-007, `crates/anvil-cli/tests/fixtures/start-activation/`, `docs/public/anvil/guides/start-output-contracts.md` |

This spec pins the machine-readable and non-interactive output contracts that
must survive the Activation TUI rollout. ACTTUI changes only the default
interactive terminal path; `--verify`, `--json`, and piped compact output remain
plain and script-safe.

## Fixture home

Golden inputs and outputs for the rollout live under:

```text
crates/anvil-cli/tests/fixtures/start-activation/
```

The directory is intentionally named `start-activation` rather than
`start-verify` because it covers all three non-TUI contracts:

1. read-only verify stdout,
2. JSON stdout,
3. compact plain stdout for piped / `--no-tui` consumers.

ACTTUI-001 may add the harness that reads these files; ACTTUI-007 must make the
fixture comparison mandatory before the TTY-default flip.

## Normalisation

Fixture assertions compare bytes after only these normalisations:

| Field | Normalisation |
| ----- | ------------- |
| Absolute repository path | `<WORKTREE>` |
| User home | `<HOME>` |
| OS path separator | `/` |
| Timestamps / durations | `<TIME>` / `<DURATION>` |
| Process identifiers | `<PID>` |
| Platform-specific editor path | `<EDITOR_PATH>` |

No wording, line ordering, state literal, consent default, or exit-code
normalisation is allowed. A fixture change is a public contract change and needs
an explicit ACTTUI closeout note.

## Matrix

| Fixture | Command shape | Expected surface | Contract |
| ------- | ------------- | ---------------- | -------- |
| `verify-protecting.stdout` | `anvil start --verify` in a protecting fixture repo | plain stdout | Byte-stable read-only diagnostic; no TUI, no writes |
| `verify-ready-restart-required.stdout` | `anvil start --verify` after MCP config write but before editor restart | plain stdout | State literal and restart hint preserved |
| `json-protecting.stdout` | `anvil start --json` in a protecting fixture repo | JSON stdout | Single parseable document; no stderr human block in JSON mode |
| `compact-protecting.stdout` | `anvil start --no-tui` or piped default in a protecting fixture repo | compact plain stdout | Bounded summary, target ≤10 lines |
| `compact-gated-anvil-home.stdout` | `ANVIL_HOME=<tmp> anvil start --no-tui` under gated project writes | compact plain stdout | Persistent gated-posture message; no repo-scoped write claim |
| `pty-opt-in.transcript` | `ANVIL_ACTIVATION_TUI=1 anvil start` under a PTY, then `q` | PTY transcript | Enters TUI, exits cleanly, raw mode restored |

`verify-*` and `json-*` are the highest-stability fixtures. `compact-*` fixtures
may gain or lose non-contract explanatory lines only before ACTTUI-007; after
ACTTUI-007 they are pinned until a new ADR or work item changes the compact
contract.

## Read-only contract

`--verify` and `--json` remain read-only. They do not:

- install MCP entries,
- write workflows or hooks,
- seed `.anvilrc`, baseline, witness, or project state,
- start the daemon,
- enter the TUI.

`--watch --verify` and `--watch --json` remain rejected because watch fallback is
not a read-only single-document probe.

## Compact plain contract

Compact plain output is for scripts, logs, CI, and users who opt out of TUI:

- it prints one literal state (`protecting`, `ready_restart_required`,
  `watching`, `needs_action`, `unsupported`, or `error`);
- it prints one next step when the state is not `protecting`;
- it avoids terminal control sequences;
- it does not wait for input;
- on a `protecting` re-run, it is bounded to a short summary (target ≤10 lines).

`render_compact()` remains unchanged until ACTTUI-007 owns the byte fixtures.

## TTY opt-in transcript

The first release uses:

```bash
ANVIL_ACTIVATION_TUI=1 anvil start
# or
anvil start --tui
```

The PTY transcript proves only the terminal lifecycle in ACTTUI-001:

1. the surface opens on the interactive path;
2. the shell chrome names the current state phase;
3. pressing `q` exits without orphan raw mode;
4. `--watch` tears down the alternate screen before handing off to the watcher
   (mandatory in ACTTUI-007).

The transcript is not a substitute for snapshot tests of the TUI widgets; those
belong to `crates/anvil-tui`.

## Release-note template

Use this template for the first release that contains ACTTUI-001:

````markdown
### `anvil start` activation TUI (opt-in)

`anvil start` now has an opt-in terminal UI for the activation flow:

```bash
ANVIL_ACTIVATION_TUI=1 anvil start
# or
anvil start --tui
```

Scripting contracts are unchanged. Use `anvil start --verify` for read-only
state probes, `anvil start --json` for machine output, and `anvil start
--no-tui` (or `ANVIL_NO_TUI=1`) for compact plain output.
````

For the release that flips TTY-default after ACTTUI-007:

````markdown
### `anvil start` now opens the activation TUI on interactive terminals

On an interactive terminal, `anvil start` now opens the activation TUI by
default. Scripts and CI are unchanged: `--verify` and `--json` stay byte-stable,
and `--no-tui` / `ANVIL_NO_TUI=1` force compact plain output.
````

## WOW coordination

The same fixture and trust-boundary language is downstream input for first-run
welcome work:

- ACTTUI-008 consumes the shared chrome for the welcome hub.
- PR #3231 records that WOW-005 cannot close its first-fix write design gate
  until ACTTUI-000 and ACTTUI-004 land.
- PR #3231 records that WOW-006 cannot close its autoplay demo design gate until
  the ACTTUI foundation and shared-widget extract land.

This branch deliberately does **not** edit `plans/modules/first-run-wow.aps.md`
while PR #3231 is open; it records ACTTUI-side obligations and avoids a
same-file APS race.
