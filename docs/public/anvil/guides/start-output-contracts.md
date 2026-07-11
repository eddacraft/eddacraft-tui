---
id: start-output-contracts
title: anvil start Output Contracts
description:
  How to use anvil start from terminals, scripts, CI, and the activation TUI
  rollout.
sidebar_position: 9
---

# `anvil start` Output Contracts

`anvil start` has two audiences:

- humans in a terminal who want the activation flow to explain itself; and
- scripts, CI jobs, and probes that need stable output.

The activation TUI rollout keeps those audiences separate. Interactive terminals
can opt in to the new TUI during the first release, while scripted surfaces keep
their existing plain contracts.

## Which mode should I use?

| Need                                             | Command                                                     |
| ------------------------------------------------ | ----------------------------------------------------------- |
| Try the activation TUI during the opt-in release | `ANVIL_ACTIVATION_TUI=1 anvil start` or `anvil start --tui` |
| Read-only state probe                            | `anvil start --verify`                                      |
| Machine-readable state                           | `anvil start --json`                                        |
| Plain output in a terminal                       | `anvil start --no-tui` or `ANVIL_NO_TUI=1 anvil start`      |
| CI / piped output                                | Pipe or run under CI; anvil chooses compact plain output    |

`--verify` and `--json` never enter the TUI. They are the surfaces to use for
health checks, scripts, dashboards, and release gates.

## Rollout ladder

The activation TUI rolls out in two steps:

1. **Opt-in release.** Use `ANVIL_ACTIVATION_TUI=1 anvil start` or
   `anvil start --tui` to open the TUI. The default `anvil start` terminal path
   stays plain until the contract matrix is green.
2. **TTY-default release.** After the verify/json/plain/PTY matrix passes and
   the welcome surface ships in the same release cohort, interactive terminals
   open the TUI by default.

After the default flip, `--no-tui` and `ANVIL_NO_TUI=1` remain permanent escape
hatches for plain output.

`ANVIL_NO_TUI` follows the same convention as the other `ANVIL_NO_*` hatches
(`ANVIL_NO_DAEMON`, `ANVIL_NO_MCP`): any non-empty value opts out, and an empty
value (`ANVIL_NO_TUI=`) is treated as unset.

## Trust boundary

The TUI can render only when the process is genuinely interactive:

- stdin, stdout, and stderr are TTYs;
- `CI` is not set;
- `ANVIL_NO_PROMPT` is not set;
- `--verify`, `--json`, `--no-tui`, and `ANVIL_NO_TUI=1` are not active; and
- the TUI rollout gate is active for the current release.

Every other context gets deterministic plain output with no keypress wait and no
terminal control sequences.

## Scripting contracts

For scripts and CI:

- prefer `anvil start --json` when you need structured data;
- prefer `anvil start --verify` when you need a human-readable read-only probe;
- use `--no-tui` when a human terminal still needs the compact plain view;
- never scrape the interactive TUI transcript.

`--verify` and `--json` are read-only. They do not install MCP entries, write
workflows or hooks, seed project state, start the daemon, or enter the TUI.

## Release-note template

For the opt-in release:

````markdown
### `anvil start` activation TUI (opt-in)

`anvil start` now has an opt-in terminal UI for the activation flow:

```bash
ANVIL_ACTIVATION_TUI=1 anvil start
# or
anvil start --tui
```

Scripting contracts are unchanged. Use `anvil start --verify` for read-only
state probes, `anvil start --json` for machine output, and
`anvil start --no-tui` (or `ANVIL_NO_TUI=1`) for compact plain output.
````

For the TTY-default release:

```markdown
### `anvil start` now opens the activation TUI on interactive terminals

On an interactive terminal, `anvil start` now opens the activation TUI by
default. Scripts and CI are unchanged: `--verify` and `--json` stay byte-stable,
and `--no-tui` / `ANVIL_NO_TUI=1` force compact plain output.
```
