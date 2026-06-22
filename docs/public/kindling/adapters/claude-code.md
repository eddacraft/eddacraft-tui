---
id: claude-code
title: Claude Code Adapter
description: Automatic session memory and context injection for Claude Code.
sidebar_position: 1
---

# Claude Code Adapter

The Claude Code plugin gives you **automatic memory across sessions**. It
captures what happens in a session — tool calls, file edits, commands, errors,
your prompts, and subagent results — and injects relevant prior context when you
start a new session in the same project.

All data is stored in the project-scoped SQLite database described in
[Storage](/kindling/concepts/storage). Capture hooks shell out to the `kindling`
binary with no npm dependencies. The `/memory` slash-command scripts are thin
Node.js wrappers (requires **Node.js >= 18** on `PATH`).

## Prerequisite: the `kindling` binary

The hooks run `kindling hook <type>`, so the `kindling` binary must be on your
`PATH`. Install it through any channel from the
[install guide](/kindling/quickstart/install):

```bash
cargo install eddacraft-kindling   # the published crate; the binary is `kindling`
# verify
kindling --version
```

The hooks **fail open** — if the binary is missing or errors, they no-op and
never block your session.

## Install the plugin

Add the marketplace, then install:

```text
/plugin marketplace add eddacraft/kindling
/plugin install kindling@kindling-plugins
```

Or load it directly for development:

```bash
claude --plugin-dir ./plugins/kindling-claude-code
```

## What it captures

When you start a session, the plugin:

1. **opens a session capsule** to track all activity;
2. **injects prior context** from previous sessions in this project;
3. **captures** tool calls, commands, file edits, errors, and your messages as
   observations;
4. **captures subagent results**; and
5. **closes the capsule** when the session ends.

## Hook contract

The plugin registers a hook per Claude Code lifecycle event. Each runs
`kindling hook <type>`, reads the hook context as JSON on stdin, and always
exits 0.

| Claude Code event    | Command                               | Purpose                                 |
| -------------------- | ------------------------------------- | --------------------------------------- |
| `SessionStart`       | `kindling hook session-start`         | Open the capsule, inject prior context. |
| `UserPromptSubmit`   | `kindling hook user-prompt-submit`    | Capture your prompt as an observation.  |
| `PostToolUse`        | `kindling hook post-tool-use`         | Capture tool calls, edits, commands.    |
| `PostToolUseFailure` | `kindling hook post-tool-use-failure` | Capture tool failures as errors.        |
| `SubagentStop`       | `kindling hook subagent-stop`         | Capture subagent completion.            |
| `PreCompact`         | `kindling hook pre-compact`           | Capture context before compaction.      |
| `Stop`               | `kindling hook stop`                  | Close the capsule.                      |

You normally never configure these by hand — installing the plugin wires them
up. The same binary can also be invoked as `kindling-hook <type>` (drop-in
program name).

## `recall` skill

The plugin ships a **`recall` skill** that teaches Claude when and how to search
prior memory proactively — before implementing features, on repeated errors,
when you reference past work, or in unfamiliar code areas. Invoke it with
`/kindling:recall <query>` for targeted deep searches.

## `/memory` slash commands

The plugin adds slash commands for working with memory from inside a session:

| Command                         | Description                                               |
| ------------------------------- | --------------------------------------------------------- |
| `/memory search <query>`        | Search past sessions in this project.                     |
| `/memory status`                | Show database statistics.                                 |
| `/memory pin [note] [--ttl 7d]` | Pin the last observation, optionally with a note and TTL. |
| `/memory pins`                  | List all pins.                                            |
| `/memory unpin <id>`            | Remove a pin.                                             |
| `/memory forget <id>`           | Redact an observation.                                    |

## Configuration

The hooks honour these environment variables:

| Variable               | Default                     | Effect                                                                                                      |
| ---------------------- | --------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `KINDLING_REPO_ROOT`   | detected                    | Override the project root used to select the database. Only applied when the working directory is under it. |
| `KINDLING_MAX_CONTEXT` | `10`                        | Maximum number of recent observations injected on `SessionStart`.                                           |
| `KINDLING_SOCK`        | `~/.kindling/kindling.sock` | Daemon socket path override.                                                                                |
| `KINDLING_DB_PATH`     | per-project                 | Explicit database path override.                                                                            |

See [Configuration](/kindling/reference/config) for the full list.

`kindling init --claude-code` detects whether `~/.claude/` exists and prints
plugin install next steps. It does **not** install or configure the plugin for
you — use the marketplace commands above.

## Next

- [OpenCode adapter →](/kindling/adapters/opencode)
- [VS Code adapter →](/kindling/adapters/vscode)
- [Retrieval — how injected context is ranked →](/kindling/concepts/retrieval)
