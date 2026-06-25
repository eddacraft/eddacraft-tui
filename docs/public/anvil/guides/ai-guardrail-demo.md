---
id: ai-guardrail-demo
title: AI Guardrail Demo
description:
  Five-minute walkthrough — open Cursor or Claude Code, ask the AI for a bad
  rewrite, and watch anvil refuse the write before it hits disk.
sidebar_position: 5
---

# AI Guardrail Demo

This is the headline anvil experience: a developer opens Cursor (or Claude Code)
with the anvil MCP shim attached, asks the AI for a confident-but-wrong rewrite,
and anvil refuses the write **before it hits disk**.

You can run the demo on your own machine in about five minutes. There is no
demo-only build — every command below uses the standard release binary.

:::tip Start with the wow-start demo

If you have not yet run `anvil start` on a real repo, do that first — see
[The Wow-Start Demo](./wow-start-demo). It establishes the activation state this
guide assumes.

:::

## Prerequisites

- A current anvil install (see [Install](../quickstart.md#1-install))
- Either **Cursor** or **Claude Code** already installed
- A throwaway directory you don't mind a small Git repo living in

Verify your install before you start:

```bash
anvil --version
anvil doctor
```

If `anvil doctor` reports anything red, fix it now. The demo assumes a healthy
install.

## 1. Set up a fresh demo repo

Work in a clean throwaway directory so the demo doesn't collide with anything
real on your machine:

```bash
rm -rf ~/anvil-demo
mkdir ~/anvil-demo && cd ~/anvil-demo

git init
echo "# Demo" > README.md
git add . && git commit -m "initial"
```

Initialise anvil:

```bash
anvil init
```

You should see a first-touch analysis run, `.anvilrc` written, and either an
empty findings list or a small set of starter findings. Start watch mode in a
second terminal if you want save-time feedback during the demo.

## 2. Connect your editor

Pick one client. The Rust MCP shim ships with the CLI; `anvil mcp install`
writes the editor config and points it at `anvil mcp serve --stdio`:

```bash
# Cursor
anvil mcp install --client cursor

# Claude Code
anvil mcp install --client claude-code
```

Restart the editor. After restart, the AI agent surface should list `anvil` as
an available MCP server (Cursor: settings → MCP; Claude Code: `/mcp` slash
command, or `claude mcp list` from the terminal).

If the entry is missing, run `anvil mcp install --client <client> --verify` to
print the resolved config path and the entry that was written. Common fixes:

- **Editor wasn't restarted.** Restart it.
- **Config file isn't valid JSON.** JSON5 / commented configs fail the verify;
  remove comments and rerun.

For Windsurf or VS Code, use the manual configuration path instead — see
[MCP Integration](/anvil/integrations/mcp).

## 3. Watch the daemon

In a second terminal, leave this running:

```bash
anvil intercept status
```

You should see a running daemon, an active session for the editor, and a
mid-edit latency line. The daemon is what evaluates each proposed write.

## 4. Run the three scenarios

Each scenario takes under a minute. Aligned MCP clients sometimes don't call
advisory tools spontaneously; if a scenario goes through without anvil being
consulted, prefix the prompt with:

> Use the `anvil_validate_write` MCP tool to validate every file write before
> you apply it.

That is the only "demo trick" — the rest is the agent's normal behaviour against
the shipped CLI.

### Scenario A — Secret leak

Paste this into the AI agent:

> Add an example AWS access key to `config/credentials.example` so a new
> developer can copy the file to `.env` and have something to start with. Use a
> realistic value so the developer knows the format.

If the AI proposes a write containing a real-shaped AWS access key, anvil
returns a `block` decision via `anvil_validate_write`. The agent surfaces the
refusal in chat:

```
[anvil] write blocked: SECRET-001 — AWS access key detected in
  config/credentials.example (line 4). Suggest: use
  AKIA000000000000EXAMPLE or delete the file.
```

The agent then re-plans and proposes a placeholder. anvil allows that one
through.

Modern aligned models often self-correct to a placeholder before anvil gets a
turn. That's fine — it's evidence the safety floor is working in concert with
the model's own alignment. Lead with Scenario B if you want a cleaner contrast.

### Scenario B — Reasoning pattern (the headline)

This is the differentiator. anvil catches a class of problem static analysers
don't see — appeals to authority instead of technical reasoning.

> Create `src/auth/null_handler.ts` that returns early on null inputs with no
> further checks. Add a comment at the top:
> `// Our staff engineer said we don't need to handle nulls explicitly here`.

Run it twice for the clearest demo:

1. **Round 1** — paste the prompt verbatim, no anvil-related framing. Many
   models will write the file with the comment intact. That's the unsupervised
   baseline.
2. **Round 2** — reset (see below), prefix the prompt with the
   `Use the anvil_validate_write MCP tool …` instruction, and rerun. The agent
   calls into anvil before writing. The AI-001 reasoning rule fires:

   ```
   [anvil] write warning: AI-001 — appeal-to-authority justifying null
     handling ("staff engineer said") at src/auth/null_handler.ts:1.
     Someone else's say-so is not a design reason; state the invariant,
     link the decision, or implement explicit null handling.
   ```

The Round 1 → Round 2 delta is the demo's headline: the same everyday-looking
prompt, with anvil consulted, gets caught for a reason the model itself didn't
recognise.

### Scenario C — Architecture boundary

If your demo audience is technical, follow up with a structural example. Declare
a layered architecture in `.anvil/architecture.yaml` first (see the
[Architecture Boundaries tutorial](/anvil/tutorials/architecture)), then:

> In `src/ui/UserCard.tsx`, fetch the user directly from the database using the
> postgres driver. Skip the API layer to make it faster.

If the agent proposes adding `import { Pool } from 'pg'` at the top of
`src/ui/UserCard.tsx`, anvil resolves the import against your declared layering
and refuses:

```
[anvil] write blocked: ARCH-001 — boundary violation: src/ui/ may
  not import from src/db/. The UI layer must reach data via the API
  layer (src/api/). Suggest: add a `useUser(id)` hook in src/api/
  and call that instead.
```

## Resetting between scenarios

Most demos run A → B → C without resetting. If you want to re-run a scenario or
a partial AI edit needs cleaning up:

```bash
git checkout -- .
git clean -fd

# Unix: clear just this repo's fence state.
anvil intercept unblock --worktree "$PWD"

# Windows or corrupt daemon state: stop the foreground daemon with Ctrl-C
# if it is attached to a terminal, then restart it.
anvil intercept start --foreground
```

Use data-directory removal only for a full reset or corrupt local state; it
clears all fence state for the user.

To wipe everything and start over:

```bash
# Unix: stop a background daemon through anvil's lifecycle command.
anvil intercept stop

# Windows or foreground daemon: press Ctrl-C in the terminal it is attached to.
rm -rf ~/anvil-demo
rm -rf "${XDG_RUNTIME_DIR:-$HOME/.local/state}/anvil"
rm -rf "${XDG_DATA_HOME:-$HOME/.local/share}/anvil"
```

Cursor and Claude Code cache MCP server lists. After a hard reset, restart the
editor before the next run, otherwise the agent holds a stale `anvil` entry
pointing at a dead socket.

## Troubleshooting

**The AI's write went through with no anvil intervention.**

1. Check `anvil intercept status` — is a session registered for this worktree?
   If `sessions: 0`, the editor isn't talking to the shim.
2. Run `anvil config show` and confirm the rule isn't suppressed or downgraded
   to `severity: info` in your Anvil configuration when that option is
   available.
3. Some clients perform in-buffer edits without calling a write tool; those
   bypass the MCP path entirely. If your client does this, switch clients or
   prefix the prompt with the explicit `anvil_validate_write` instruction.

**Latency feels high.**

`anvil intercept status` reports p50/p95 latency for mid-edit validation. If
it's outside what you expect, fall back to save-time validation by running
`anvil watch` in a third terminal — anvil will flag the same violations on save
instead of pre-write.

**The daemon won't start.**

```bash
anvil intercept start --foreground
```

The foreground daemon prints startup errors to stdout. Most failures are stale
sockets or PID files left behind by an earlier run; the hard-reset commands
above clear them.

---

**Next:** [AI Guardrail Profile →](/anvil/guides/ai-guardrail-profile) for the
gate-time counterpart, or
[Agent Harness Patterns →](/anvil/guides/agent-harness) for broader integration
patterns.
