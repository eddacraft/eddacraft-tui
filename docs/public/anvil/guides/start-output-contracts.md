---
id: start-output-contracts
title: Activation states
description: Look up every final state reported by anvil activation.
---

# Activation states

`anvil start`, `anvil start --verify`, and verified status output use the same
final vocabulary. Bare `anvil` (daily ensure) reports a separate ensure surface
for daemon, worktree, and MCP ensure outcomes; use `anvil --json` for automation
and `anvil start --verify` when you need the activation-state vocabulary below.

| State                    | Assurance                                        | Next action                                         |
| ------------------------ | ------------------------------------------------ | --------------------------------------------------- |
| `protecting`             | Supported pre-write validation is active         | Continue working                                    |
| `ready_restart_required` | Client configuration is installed but not active | Restart the named client and verify again           |
| `watching`               | The local daemon recognises the project          | Run `anvil watch` to prove a visible save-time loop |
| `needs_action`           | A named setup step is incomplete                 | Follow the displayed repair                         |
| `unsupported`            | Project or platform coverage is insufficient     | Check the support matrix                            |
| `error`                  | Activation could not complete                    | Run doctor and troubleshooting                      |

`watching` alone does not prove that a background save-time driver is attached.
Use the explicit watcher when save-time evidence matters.

For automation, use `--json`. Do not infer state by searching human-readable
prose.

## Interactive and plain output

In a genuine terminal, `anvil start` opens the interactive activation surface.
Everything else — including every scripted context — gets the same plain text as
before:

| Context                                             | Output                    |
| --------------------------------------------------- | ------------------------- |
| A terminal (all of stdin, stdout, stderr are a TTY) | Interactive surface       |
| `--no-tui`, or `ANVIL_NO_TUI=1`                     | Plain text                |
| `--verify` or `--json`                              | Plain text / JSON         |
| `anvil start --watch`                               | Plain text + event stream |
| Piped, redirected, or run under CI                  | Plain text                |

`--no-tui` and `ANVIL_NO_TUI=1` are the permanent escape hatches — reach for
either when a terminal session still needs plain output.

## Next step

Use [AI-assisted write protection](agent-harness.md) or
[save-time validation](save-time-validation.md).
