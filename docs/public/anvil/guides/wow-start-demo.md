---
id: wow-start-demo
title: The Wow-Start Demo
description:
  install → cd repo → anvil start. The first-minute story for v0.6.0-beta.
sidebar_position: 1
---

# The Wow-Start Demo

This is the canonical first-minute story for `v0.6.0-beta`:

```bash
curl -fsSL https://install.eddacraft.ai | sh   # install anvil
cd ~/Projects/your-real-repo                   # a real TS / JS repo
anvil start                                    # the wow-start
```

Three commands. After the third, anvil prints one literal protection state and
either tells you it is protecting your repo, or tells you the truthful reason it
is not. There is no auto-detected AI session, no rule-file injection, no demo
fixture. The whole story is the activation orchestrator composing what already
ships in the binary.

For the deeper Cursor / Claude Code interaction walkthrough — restart the
editor, ask the AI for a bad rewrite, watch `anvil_validate_write` refuse it —
see the [AI Guardrail Demo](./ai-guardrail-demo). This guide is the on-ramp; the
guardrail demo is what comes next.

## What this demo proves

After `anvil start` you land on exactly one of six literal states:

- **`protecting`** — pre-write MCP validation has been observed live in this
  repo. AI writes are being checked before they hit disk.
- **`ready_restart_required`** — MCP config was written safely and the server
  starts, but the editor or agent has to restart so the MCP entry attaches, or a
  daemon-state repair hint needs attention. This is **not** protection yet — the
  printed next step matters.
- **`watching`** — pre-write MCP attachment is not in evidence; the kernel
  watcher is running as a save-time fallback. Weaker than `protecting`, and the
  surface says so.
- **`needs_action`** — anvil cannot make a literal protection claim and you have
  a concrete next step (run `anvil init`, install a supported editor, etc.).
- **`unsupported`** — your repo's languages are out of scope for this release.
  The state names the gap honestly instead of pretending coverage.
- **`error`** — activation hit a hard error before any state could be
  established. The cause and a repair hint are printed.

The single load-bearing claim is: that printed word is the truth. If it says
`needs_action`, pre-write protection is **not** live — there is no second
sentence somewhere else that walks it back. Surfaces never invent ad-hoc phrases
like "config written" or "almost protected"; the six literals above are the only
allowed vocabulary.

## Prerequisites

- A current anvil install (see [Install](../quickstart.md#install)).
- **Cursor** or **Claude Code** installed — one is enough. v1 ships MCP install
  for Cursor and Claude Code only; nothing else is wired.
- A real TypeScript or JavaScript project. SQL and Markdown get partial
  coverage; Python and Rust are unsupported in this release and will land you on
  `unsupported` honestly.
- A clean working tree. `anvil start` is idempotent, but a clean tree makes the
  diff between "what was there before" and "what activation just wrote" obvious.

Verify the install before the demo:

```bash
anvil --version
```

## The first minute

Three commands, in any real repo on disk:

```bash
curl -fsSL https://install.eddacraft.ai | sh
cd ~/Projects/your-real-repo
anvil start
```

The third command prints an `ACTIVATION` block and ends on one literal `state:`
line. The exact lines vary by repo and editor coverage, but the shape is fixed.

Three representative outcomes:

### Outcome A — `protecting`

```
ACTIVATION
  state: protecting
  Protecting — pre-write validation is live in this repo.
  config: valid
  mcp:
    Cursor: live_validation
    Claude Code: live_validation
  watch: not_requested
  baseline: present (4 findings recorded; future scans will diff against this set as wiring lands)
  languages:
    TypeScript (37 files): supported — anchor + extension match
  install:
    Cursor: skipped — already up to date
    Claude Code: skipped — already up to date
```

The headline claim — "pre-write validation is live in this repo" — only fires
when anvil has live evidence from the daemon or an observed
`anvil_validate_write` call. Until that evidence arrives, the diagnostic stops
at `ready_restart_required`. This is by design: a `protecting` claim that hasn't
been earned would be the worst-case lie for a governance tool.

### Outcome B — `ready_restart_required`

```
ACTIVATION
  state: ready_restart_required
  Ready, restart required — restart your editor or agent so the MCP server attaches.
  config: valid
  mcp:
    Cursor: restart_required
    Claude Code: restart_required
  watch: not_requested
  baseline: present (0 findings recorded; future scans will diff against this set as wiring lands)
  languages:
    TypeScript (37 files): supported — anchor + extension match
  install:
    Cursor: installed at /home/you/.cursor/mcp.json (fresh)
    Claude Code: installed at /home/you/.claude.json (fresh)
  next: Restart Cursor (or Claude Code) so the MCP server attaches; re-run `anvil start --verify` to confirm.
```

This is the most common first-run outcome. anvil wrote `~/.cursor/mcp.json` and
`~/.claude.json`, the server starts cleanly, but the editor is holding the old
MCP server list. Restart it once. On `v0.7.1-beta` and newer, the next
`anvil start --verify` can move `ready_restart_required` → `protecting` when the
daemon attests the current worktree; if it cannot, read the printed repair hint.

### Outcome C — `watching`

```
ACTIVATION
  state: watching
  Watching — save-time fallback only; this is weaker than pre-write validation.
  config: valid
  mcp: not detected
  watch: offered
  note: MCP pre-write validation is not attached. Watch mode fallback validates saved file changes only — it cannot intercept MCP tool writes before they happen.
  baseline: present (0 findings recorded; future scans will diff against this set as wiring lands)
  languages:
    TypeScript (37 files): supported — anchor + extension match
  next: Install Cursor or Claude Code for pre-write protection, or run `anvil start --watch` for save-time fallback.
```

You will see this if neither Cursor nor Claude Code is detected (or if their
config is structurally unparseable and anvil correctly refused to touch it).
Watch fallback is offered; it is **not** running yet. To run it:
`anvil start --watch` (covered below).

> **Note on the sample output above.** The exact line spacing, label widths, and
> per-client tier strings come from `crates/anvil-cli/src/activation/render.rs`
> and the live `ProtectionState` vocabulary in `state.rs`. The blocks above are
> illustrative — copy on your machine may differ in punctuation or ordering as
> activation probes evolve. The `state:` literal and the six allowed values are
> pinned by tests and will not drift.

## What `anvil start` actually does

The orchestrator composes only **read-safe / idempotent** primitives. There is
no clever auto-detection or background process attach.

In order:

1. **Probe config status.** If `.anvilrc` is absent, run `anvil init` inline
   (which also runs the bounded first-scan from LAUNCH-004). If `.anvilrc` is
   valid, skip init — re-running `anvil start` is idempotent. If it is
   structurally invalid, surface that as `state: error` with the parse error,
   never overwrite.
2. **First-scan via the language profile.** Walk the working tree (excluding
   vendored and generated paths via the existing internal denylist), classify
   each detected language as `supported` / `partial` / `unsupported` against the
   registered language pack, and record a small baseline of existing findings so
   future runs can diff new edges only.
3. **Write MCP entries for the supported clients found.** v1 supports two
   clients:
   - Cursor → `~/.cursor/mcp.json`
   - Claude Code → `~/.claude.json`

   Existing config is parsed before modification, written atomically, and left
   untouched on parse failure or unsafe drift. Rule / instruction files
   (`.cursorrules`, `.clauderules`, global AI rules) are **not** edited.

4. **Run the activation diagnostic.** This is the read-only probe that produces
   the `ActivationDiagnostic` rendered above — config status, MCP tier per
   client, watch tier, baseline summary, language profile, `last_error` (if
   any).
5. **Emit one literal `state:`** from the six-word vocabulary, based on the
   diagnostic. The headline copy comes from `ProtectionState::headline()` so the
   wording cannot drift between `start`, `status`, and `doctor`.

What `anvil start` deliberately does **not** do:

- It does not auto-detect AI sessions running against this repo. anvil only
  knows what it wired itself — the entries it just wrote into
  `~/.cursor/mcp.json` and `~/.claude.json`.
- It does not inject `.cursorrules` / `.clauderules` / any global AI rule file.
- It does not modify your `git config` or install git hooks.
- It does not log in to a cloud, pull a team policy, or set up CI.
- It does not ship a demo fixture, a "challenge file", or a guaranteed-catch
  prompt catalogue. The catch comes from your real code being asked a real
  question by a real agent.

## Verifying without writing — `anvil start --verify`

When you want to inspect what `anvil start` **would do** without changing
anything on disk, use `--verify`:

```bash
anvil start --verify
```

`--verify` skips init, the first-scan, and the MCP install step. It forwards to
the same backend as `anvil status --verify` (LAUNCH-012) — the diagnostic is
read-only and idempotent, so re-running it is free.

Useful for:

- Confirming a fresh `anvil start` would land on the state you expect, before
  you let it write anything.
- Re-checking the state after restarting your editor (the most common path from
  `ready_restart_required` → `protecting`).
- CI / scripting: `anvil start --verify --json` produces a single
  `ActivationDiagnostic` JSON document on stdout that downstream consumers can
  parse for the `state` field.

`--json` implies `--verify` semantics — under JSON mode, `anvil start` behaves
like the read-only path so init's own JSON record cannot concatenate with the
activation diagnostic and break parseable consumers.

`--watch` and `--verify` are mutually exclusive: `--verify` is read-only and
cannot spawn the watcher. `--watch` and `--json` are also mutually exclusive
because the watcher streams event lines on stdout, breaking the
single-JSON-document contract. anvil rejects both combinations explicitly with a
hint.

## The watch fallback — `anvil start --watch`

When MCP pre-write validation is not live (no Cursor / Claude Code installed, or
the editor refused to load the server), the kernel watcher is the **save-time
fallback**:

```bash
anvil start --watch
```

This runs the activation orchestrator first, then hands off to the kernel
watcher inline, scoped to the current repo. The diagnostic block prints, the
synthetic `state: watching` literal matches the layer about to run, and the
watcher takes over the foreground until you press Ctrl-C.

**Watch is fallback, never primary.** The headline rule:

> Save-time fallback validates files **after** they are saved. It cannot
> intercept MCP tool writes **before** they happen. It is weaker than
> `protecting`, and the surface always says so.

`anvil start --watch` will refuse to spawn the watcher in five honest cases.
Each case prints a state-specific reason instead of running a watcher that would
generate noise without findings:

| Diagnostic state          | Watch decision                    | Why                                         |
| ------------------------- | --------------------------------- | ------------------------------------------- |
| MCP at `LiveValidation`   | `skipped — redundant`             | Pre-write already covers the save path.     |
| `.anvilrc` invalid        | `skipped — fix config first`      | Watcher can't honour an unparseable config. |
| `.anvilrc` absent         | `skipped — run anvil init first`  | No filters / extensions yet.                |
| `last_error` set          | `skipped — clear the error first` | Activation aborted upstream.                |
| All languages unsupported | `skipped — out of scope`          | Watcher would not produce findings.         |

If you want save-time signal anyway in a state where `--watch` has skipped, the
unconditional path is `anvil watch --source` (covered in
[Beta Quickstart: Turn On Watch Mode](../quickstart.md#turn-on-watch-mode-fallback)).

## The protection states, plainly

The six allowed literals, what each means, and the next user action:

| `state:`                 | Meaning                                                                                        | Next action                                                                   |
| ------------------------ | ---------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| `protecting`             | Pre-write validation has been observed live. AI writes hit `anvil_validate_write` before disk. | None — try the [AI Guardrail Demo](./ai-guardrail-demo).                      |
| `ready_restart_required` | MCP config written, but editor restart or daemon-state repair is still required.               | Follow the printed repair hint; re-run `anvil start --verify`.                |
| `watching`               | Pre-write MCP attachment not in evidence; save-time watcher running (weaker).                  | Install Cursor or Claude Code if you want pre-write; or accept the fallback.  |
| `needs_action`           | No literal protection claim possible; concrete next step exists.                               | Read the `next:` line below the diagnostic.                                   |
| `unsupported`            | Repo languages are out of scope for this release.                                              | Wait for the language pack to ship, or scope anvil to a TS / JS subdirectory. |
| `error`                  | Activation hit a hard error.                                                                   | Read `last_error:` and the `next:` repair hint.                               |

This is the same vocabulary used by `anvil status --verify`, `anvil doctor`, and
the protection-loop tutorial path. There is one renderer; surfaces cannot drift.

## Verifying the catch

Once `anvil start` reports `protecting`, the headline catch is your AI agent
attempting a write that hits an `anvil_validate_write` rejection. The full
Cursor / Claude Code restart and ask-the-AI-for-a-bad-rewrite walkthrough lives
in the [AI Guardrail Demo](./ai-guardrail-demo) — three scenarios (secret leak,
appeal-to-authority reasoning, architecture boundary violation), each under a
minute, each running against the standard release binary.

The wow-start gets you to `protecting` (or the honest reason you are not there).
The guardrail demo is what protection actually looks like when an AI agent
collides with it.

## What is honest in v0.7.x-beta

The wow-start is a real product surface, not a recording-only setup, but several
things you might expect from the brainstorm phase are explicitly **not
shipping** in `v0.7.x-beta`. Calling these out so the demo doesn't carry
implicit claims it can't back:

What `anvil start` does **not** do today:

- **No Windsurf or VS Code MCP install.** v1 supports Cursor and Claude Code
  only. The 2026-05-03 activation council banned the others until the
  editor-config writers can be proven correct end-to-end (the existing VS Code
  writer points at the wrong file under VS Code 1.99+; shipping it today would
  claim success while doing nothing).
- **No Copilot CLI / Codex CLI integration.** Out of scope for v1.
- **No process auto-attach.** anvil does not "find the AI session running in
  this repo". It writes MCP entries; the editor decides when to attach.
- **No rule-file injection.** `.cursorrules`, `.clauderules`, and global AI rule
  files are user-owned. anvil does not edit them.
- **No telemetry of which AI tool you are running.** anvil only knows what it
  wired itself — the entries in `~/.cursor/mcp.json` and `~/.claude.json` it
  just wrote.

Operator caveats carried forward from `v0.6.0-beta` (see the
[v0.7.0-beta release runbook](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/v0.7.0-beta-release-runbook.md)
for the current operator surface):

- **Foreground daemon only.** `anvil intercept start --foreground` is the only
  validated launch mode for v1. Operators running under systemd / launchd should
  run foreground under the manager's supervision.
- **`anvil intercept status` works on every supported target.** The Windows
  named-pipe client ships in `v0.6.0-beta`. `v0.7.1-beta` adds Windows MCP
  named-pipe parity too, so `anvil_validate_write` can report daemon status and
  `protection_claim` on Windows when the daemon is reachable.
- **macOS interrupt ladder is fence-first.** A daemon restart alone does not
  release fences. On Unix, use `anvil intercept unblock --worktree <PATH>` for
  worktree-scoped recovery; on Windows, stop and restart the daemon if every
  surface is quarantined. Remove `${XDG_DATA_HOME:-$HOME/.local/share}/anvil`
  only for a full reset or corrupt local state.
- **Fences survive daemon restart.** Use the same Unix unblock / Windows restart
  split as above.

The discipline holds across all of these: `anvil start` will not print
`protecting` unless pre-write validation has actually been observed. Every
weakness above shows up either as a state weaker than `protecting` or as a
caveat in the diagnostic, not as a hidden footnote.

## Repeat the demo cleanly

If you are recording the demo and want a fresh-state run, the reset path mirrors
the AI Guardrail Demo runbook. Run from inside the demo repo:

```bash
# Stop the foreground daemon (Ctrl-C in its terminal, or SIGTERM by PID).
# If you only need to clear a Unix fence, prefer:
# anvil intercept unblock --worktree "$PWD"

# Wipe runtime / data dirs for an absolutely-fresh run or corrupt local state.
# The data-dir removal clears all fence state for the user.
rm -rf "${XDG_RUNTIME_DIR:-$HOME/.local/state}/anvil"
rm -rf "${XDG_DATA_HOME:-$HOME/.local/share}/anvil"

# Optional — remove the MCP entries anvil wrote so the next start re-creates them
#   (skip if you want to demonstrate the idempotent "skipped — already up to date" path)
# rm ~/.cursor/mcp.json ~/.claude.json   # only if you have nothing else in them

# Re-run the wow-start
anvil start
```

Cursor and Claude Code cache MCP server lists. After a hard reset, restart the
editor before the next run, otherwise the agent holds a stale `anvil` entry
pointing at a dead socket.

For finer-grained per-scenario resets (between AI Guardrail Demo scenarios A / B
/ C without nuking everything), see
[AI Guardrail Demo § Resetting between scenarios](./ai-guardrail-demo#resetting-between-scenarios).

## Next steps

- [Beta Quickstart](/anvil/quickstart) — the full 10-minute install + activate
  - try-the-MCP-catch path.
- [AI Guardrail Demo](./ai-guardrail-demo) — three scenarios that exercise
  `anvil_validate_write` end-to-end inside Cursor / Claude Code.
- [Beta testing guide](/anvil/beta-testing-guide) — what we're asking testers to
  try and how to send feedback.
- [MCP Integration](/anvil/integrations/mcp) — the underlying transport, the
  `anvil_validate_write` tool shape, and the supported client list.
- [v0.7.0-beta release runbook](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/v0.7.0-beta-release-runbook.md)
  — operator-facing detail on the daemon-working surface and the caveats that
  still matter after the `v0.7.1-beta` activation-honesty patch.
