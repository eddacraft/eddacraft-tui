# Wow Start Brainstorm — `anvil connect`

## Context

A senior influencer/developer tested Anvil and ended the session with:

> give me a version that people like me can just install and use

The important read is not that Anvil lacks installers. It already has a native
binary, install site, Homebrew, WinGet, Scoop, and platform-specific scripts.
The gap is that install does not yet collapse into immediate, visible protection
inside the user's real AI/editor workflow.

## What Already Exists

Anvil has every piece needed:

- native binary with one-line install
- `anvil init` — project config
- `anvil mcp-config --target <editor> --write` — per-editor MCP entry
- `anvil doctor` — integration health check
- `anvil watch` — file-save watcher fallback
- `anvil tutorial` — interactive TUI walkthrough
- `anvil wizard` — project template selector

The problem is the user has to know to ask for each one. There is no single
moment that goes from "installed" to "protected".

## Core Pitch: `anvil connect`

```bash
curl -fsSL https://install.eddacraft.ai | sh
anvil connect
```

One command that self-assembles the right integration for whatever editors and
AI tools it finds on the machine.

The message is simple:

> Install Anvil. Run `anvil connect`. Your AI coding tools are now governed.

The wow moment happens in the user's own repo, not a demo project:

> I ran `anvil connect`, it found my editors, wrote the configs, and told me to
> restart. I did. Then I asked my AI tool to make a risky change and Anvil
> stopped it before it wrote the file.

## Proposed Behaviour

`anvil connect` should:

1. Detect installed AI editors and clients by checking well-known config paths
   (Claude Code, Cursor, Windsurf, VS Code — in order of MCP maturity).
2. Run `mcp-config --write` for every editor found, reusing the existing
   surface.
3. Run `anvil init` in the current directory if no `.anvilrc` exists.
4. Run `anvil doctor` silently and surface any unresolved issues.
5. Fall back to `anvil watch --source` and say so clearly if no supported
   editor is found.
6. Print one clear status line per editor — found and wired, or not installed.
7. End with a single terminal line: `Anvil is now protecting this repo`.

The command should make zero decisions on behalf of the user unless a real fork
exists (existing config drift, conflicting entries). It detects and writes; it
does not ask.

## Experience Shape

Ideal first run:

```text
$ anvil connect

  Scanning your machine...

  ✓ Claude Code   — MCP entry written
  ✓ Cursor        — MCP entry written
  ○ Windsurf      — not installed
  ○ VS Code       — not installed

  Initialised .anvilrc in current project
  Running first scan...  4 checks, 0 issues

  ─────────────────────────────────────────────
  Anvil is now protecting this repo.

  Restart your editors to activate the connection.
  Then ask your AI tool to edit a file — Anvil will
  intercept the write and run your policies.
  ─────────────────────────────────────────────
```

If no editor is detected:

```text
  ○ No supported AI editor detected.
  ✓ Watch mode started — save-time protection is active.

  Keep this terminal open while you code.
  Run `anvil mcp-config --help` to connect an editor manually.
```

## Why This Wows

- The output is the demo. The user sees their actual editors listed and checked.
  That is the moment of belief — not a docs page, not a dashboard.
- Zero decisions required. The influencer can tweet the exact two-command
  install. Other developers copy-paste it in under 60 seconds.
- It reframes Anvil from a CLI toolkit that requires configuration into a
  protective layer that switches on.
- It shows the category difference from SAST: Anvil intervenes while code is
  being created, not after a scan report lands in a PR queue.
- It supports the strongest demo loop: install, connect, prompt an agent, see a
  deterministic intervention.

## Relationship to `anvil activate`

The OpenCode brainstorm (`2026-05-02-wow-start-opencode.md`) proposes `anvil
activate` with a project-analysis-first framing. The distinction here:

- `anvil connect` leads with editor detection — the first output line is about
  the user's tools, not their codebase. That ordering matches the influencer
  persona: they care about "did it wire into my editor" before they care about
  "did it analyse my project".
- `anvil connect` is the more neutral name — it signals wiring rather than
  configuration or setup.

Both commands could be aliases of each other, or distinct surfaces with
different primary emphasis. That is a design decision for the APS work item.
The core engine is the same: detect, write, verify, fall back.

## Scope Fit

Inside the Anvil scope guard: this directly wires deterministic validation into
the workflow where AI agents produce unsafe changes.

Not in scope:

- Becoming a generic project bootstrapper
- IDE productivity features unrelated to governance
- Requiring a cloud account before local protection is working
- Adding editor-specific UI beyond the MCP server entry

## MVP Cut

Smallest slice that wows the influencer:

- `anvil connect` command in the CLI
- Editor detection for Claude Code, Cursor, Windsurf, VS Code (check well-known
  config paths)
- `mcp-config --write` call for each detected editor (reuses existing surface)
- `anvil init` call if config missing (reuses existing surface)
- `anvil doctor` call for verification (reuses existing surface)
- Watch fallback if no editor detected (reuses `anvil watch`)
- Per-editor status lines with check/circle symbols
- Final `Anvil is now protecting this repo` or `Anvil is ready` line

Implementation is mostly composition over existing commands. The value is in
ruthless decisioning and output design, not a new engine.

## Follow-On Ideas

After the MVP:

- `anvil connect --editor claude-code|cursor|windsurf|vscode` for explicit
  targeting
- `anvil connect --verify` to recheck without rewriting
- Connection health shown in `anvil status` as `editor: connected | disconnected`
- Install-page copy that ends with `anvil connect`, not a menu of commands
- First-run scan result shown in the connect output as social proof

## Risks

- Editor config paths vary across platforms and install methods. Must handle
  gracefully and bail clearly rather than writing to the wrong location.
- Writing into editor config files (JSON, YAML) risks corrupting user content if
  the merge is not safe. All writes must be atomic; bail with manual instruction
  on parse failure.
- If MCP is not actually invoked by the editor after restart, the "protected"
  claim is misleading. `anvil connect --verify` and `anvil doctor` must be able
  to confirm live invocation, not just config presence.
- The restart step breaks the flow. The terminal output must make this the only
  memorable action item so users do not skip it.

Mitigations:

- Never overwrite an existing matching entry without confirming it changed.
- Verification must distinguish config-written from MCP-confirmed-live.
- Final status should be `Anvil is now protecting` only when doctor passes;
  otherwise `Anvil is ready — restart your editor to activate`.

## Decision Recommendation

Prioritise `anvil connect` as the next user-facing launch bet.

The install surface is already credible. The missing product move is a single
command that converts install into protection with almost no user knowledge. When
it works, the influencer quote writes itself:

> Run `anvil connect` in any repo. It finds your editors, wires in, and tells
> you when it's done. Then ask your AI tool to make a change — Anvil's already
> watching.
