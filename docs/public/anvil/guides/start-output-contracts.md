---
id: start-output-contracts
title: Activation states
description: Look up every final state reported by anvil activation.
---

# Activation states

`anvil start`, `anvil start --verify`, and verified status output use the same
final vocabulary.

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

## Next step

Use [AI-assisted write protection](agent-harness.md) or
[save-time validation](save-time-validation.md).
