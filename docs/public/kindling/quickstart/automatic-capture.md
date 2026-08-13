---
id: automatic-capture
title: Automatic Capture
description:
  Capture context hands-free through an adapter instead of logging manually.
sidebar_position: 3
owner: DOCSYNC
verified_against: 0.2.0
---

# Automatic Capture

Logging observations by hand is useful, but the real value of Kindling comes
from capturing context _automatically_ as you work. Adapters hook into a tool's
lifecycle, map its events to observations, and manage capsules for you.

## Adapters at a glance

| Adapter                                       | Captures                                                                                                       |
| --------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| [Claude Code](/kindling/adapters/claude-code) | Tool calls, file edits, commands, your prompts, subagent results — and injects prior context on session start. |
| [VS Code](/kindling/adapters/vscode)          | File saves, manual selection logging, and editor session lifecycle.                                            |
| [OpenCode](/kindling/adapters/opencode)       | Session events and tool activity, with secret redaction.                                                       |
| [PocketFlow](/kindling/adapters/pocketflow)   | Workflow node lifecycle and outputs, with intent inference.                                                    |
| [Custom](/kindling/adapters/custom)           | Build your own on the Rust or TypeScript thin-client APIs.                                                     |

See the full [integrations matrix](/kindling/reference/integrations).

## No adapter yet?

Use the standalone path — `kindling demo`, `kindling search`, and
`kindling browse` work without any IDE or agent integration:

[Quickstart without Claude Code](/kindling/quickstart/without-claude-code)

## The fastest path: Claude Code

If you use Claude Code, the plugin gives you automatic memory in two steps.

First, make sure the `kindling` binary is on your `PATH` (see
[Install](/kindling/quickstart/install)) — the capture hooks shell out to it:

```bash
kindling --version
```

Then add the marketplace and install the plugin:

```text
/plugin marketplace add eddacraft/kindling
/plugin install kindling@kindling-plugins
```

From then on, every Claude Code session:

1. **opens a session capsule** to track activity,
2. **injects prior context** from previous sessions in this project,
3. **captures** tool calls, commands, errors, and your messages, and
4. **closes the capsule** when the session ends.

Hooks **fail open** — if the binary is missing or errors, they no-op and never
block your session.

See the [Claude Code adapter](/kindling/adapters/claude-code) for the hook
contract, the `recall` skill, the `/memory` slash commands, and configuration.

## Capturing from your own code

If you are building an integration rather than using an existing tool, capture
through the SDK instead of an adapter:

- **Rust** — use [`kindling-client`](/kindling/reference/crates) to talk to the
  daemon, or `kindling-service` for embedded in-process access.
- **TypeScript** — see the [custom adapter guide](/kindling/adapters/custom).

## Next steps

- [Claude Code adapter →](/kindling/adapters/claude-code)
- [VS Code adapter →](/kindling/adapters/vscode)
- [Core concepts: capsules →](/kindling/concepts/capsules)
