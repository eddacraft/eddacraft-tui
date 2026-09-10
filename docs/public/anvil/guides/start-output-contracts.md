---
id: start-output-contracts
title: Activation states
description: Look up every final state reported by anvil activation.
owner: ACTTUI
upstream:
  - crates/anvil-cli/src/activation/state.rs
  - crates/anvil-cli/src/activation/render.rs
  - crates/anvil-cli/src/commands/start.rs
  - crates/anvil-cli/src/activation/receipt.rs
verified_against: 0.9.7-beta
public_unlisted: true
---

# Activation states

`anvil start`, `anvil start --verify`, and verified status output use the same
final vocabulary. After a mutating start, the closing receipt names project,
selected coverage, client, policy, last proof and the next command;
`anvil status` and `anvil doctor` consume those facts. Bare `anvil` (daily
ensure) reports a separate ensure surface for daemon, worktree, and MCP ensure
outcomes; use `anvil --json` for automation and `anvil start --verify` when you
need the activation-state vocabulary below.

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
Presentation cannot change intended mutations.

| Context                                                            | Output                                                                                 |
| ------------------------------------------------------------------ | -------------------------------------------------------------------------------------- |
| A terminal (stdin, stdout, and stderr are TTYs)                    | Interactive TUI                                                                        |
| `--no-tui` / `ANVIL_NO_TUI=1` on a real TTY (stdin and stderr TTY) | Interactive **plain** consent, not auto-install                                        |
| `--verify` or `--json`                                             | Plain text / JSON (read-only; `--json` never installs)                                 |
| `anvil start --watch`                                              | Plain text + event stream                                                              |
| Piped, CI, or `ANVIL_NO_PROMPT` / `NONINTERACTIVE`                 | Unattended plain. New MCP needs `--mcp-client` / `--all-mcp-clients`; `--no-mcp` skips |

Stdout redirect alone (`anvil start \| tee`) is presentation, not unattended.
`--no-tui` remains the escape hatch for plain output on a terminal.

## Next step

Use [AI-assisted write protection](agent-harness.md) or
[save-time validation](save-time-validation.md).
