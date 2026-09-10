# Wow Start Brainstorm - OpenCode

## Context

A senior influencer/developer tested Anvil and ended the session with:

> give me a version that people like me can just install and use

The important read is not that Anvil lacks installers. It already has a native
binary, install site, Homebrew, WinGet, Scoop, and platform-specific scripts.
The gap is that install does not yet collapse into immediate protection inside
the user's real AI/editor workflow.

## Core Pitch

Ship **Anvil Activate**: one command that turns an installed binary into a
protected project.

```bash
curl -fsSL https://install.eddacraft.ai | sh
anvil activate
```

The message should be simple:

> Install Anvil. Run `anvil activate`. Your AI coding tools are now governed.

The wow moment should happen in the user's own repo, not a demo project:

> I installed Anvil, opened Claude Code or Cursor, asked it to make a risky
> change, and Anvil stopped it before it wrote the file.

## Product Thesis

We already have installation. We need activation.

Today a new user has to understand several surfaces before they experience the
core value:

- `anvil auth login`
- `anvil init`
- `anvil check --all`
- `anvil watch --source`
- `anvil mcp install --client ...`
- editor-specific config and verification

That is reasonable for contributors and beta testers, but not for a senior
developer evaluating whether this belongs in their daily workflow. They should
not have to learn Anvil's command taxonomy before Anvil proves itself.

`anvil activate` should choose the shortest safe path from current directory to
save-time or pre-write protection.

## Proposed Behaviour

`anvil activate` should:

- detect the repo, Git state, package manager, framework signals, and source
  layout
- run in local-only mode by default so first value does not require login
- create the smallest useful `.anvilrc`
- initialise `.anvil/` only when needed
- run a first high-signal scan immediately
- detect installed AI/editor clients where possible
- install the best available integration automatically or offer a single clear
  choice
- verify the integration it just wrote
- start watch mode or print the exact next command when a persistent session is
  not possible
- end with a clear status line: `Anvil is now protecting this repo`

The command should avoid asking questions unless there is a real fork:

- multiple supported AI clients detected
- unsafe dirty worktree state before writing config
- existing config drift that requires overwrite confirmation
- no supported editor or AI client found, in which case watch mode becomes the
  default fallback

## Experience Shape

Ideal first run:

```text
$ anvil activate

Analysing project...
Detected: pnpm TypeScript repo, Git clean, Claude Code installed

Creating local Anvil config...
Running first protection scan...
Installing Claude Code MCP guardrail...
Verifying MCP connection...

Anvil is now protecting this repo.

Try it: ask Claude Code to edit a file containing "ChatGPT said this was fine".
Anvil will flag the write before it lands.
```

If no AI client is detected:

```text
Anvil is ready for this repo.

No supported AI client was detected, so Anvil started save-time watch mode.
Keep this terminal open while you code.
```

## Why This Could Wow New Users

- It reframes Anvil from a CLI toolkit into a protective layer that switches on.
- It makes the first value event happen before docs, accounts, dashboards, or CI.
- It lets influencers describe the product in one sentence and one command.
- It shows the category difference from SAST: Anvil intervenes while code is
  being created, not after a scan report.
- It supports the strongest demo loop: install, activate, prompt an agent, see a
  deterministic intervention.

## Scope Fit

This stays inside the Anvil scope guard because it directly increases prevention
capability at the point of change creation.

It is not generic onboarding polish. It wires deterministic validation into the
workflow where unsafe changes are produced.

It should not become:

- a generic project bootstrapper
- an IDE productivity assistant
- a dashboard-first onboarding tour
- an agent orchestration layer
- a cloud account sales funnel before local value

## MVP Cut

Smallest valuable slice:

- add `anvil activate`
- local-only activation, no account required
- wrap existing `init` plus first scan
- detect Claude Code and Cursor where feasible
- call the existing MCP install path for the detected client
- verify the generated config
- fall back to `anvil watch --source`
- print one high-confidence next action

This can be mostly composition over existing surfaces. The value is in ruthless
decisioning and copy, not a new engine.

## Follow-On Ideas

After the MVP proves itself:

- `anvil activate --agent claude-code|cursor|windsurf|vscode`
- `anvil activate --team` for authenticated policy/profile pulls
- `anvil activate --ci` to add the gate after local protection is working
- install-site copy that ends with `anvil activate`, not a menu of commands
- first-run challenge fixture injected into the user's repo only with explicit
  consent, for a reliable demonstration of pre-write validation
- activation health shown in `anvil status` as a single protected/not-protected
  summary

## Risks

- Over-promising editor support if MCP clients do not consistently call the tool
  without prompt instructions.
- Writing editor config too aggressively and damaging trust.
- Local-only mode creating confusion later when team/cloud features require auth.
- False positives during the first scan weakening the wow moment.
- Watch fallback feeling weaker than pre-write interception.

Mitigations:

- activation must verify what it changed and clearly name degraded modes
- all writes need preview or path-safety prompts when overwriting existing config
- first scan should bias toward high-signal checks, especially secrets and AI
  reasoning anti-patterns
- the final line should distinguish `protected`, `ready`, and `needs action`

## Decision Recommendation

Prioritise `anvil activate` as the next user-facing launch bet.

Do not spend the next cycle adding more install channels. The install surface is
already credible. The missing product move is a command that converts install
into protection with almost no user knowledge.

If this lands well, the influencer quote becomes the release headline:

> You can just install Anvil and use it. Run `anvil activate` in any repo and it
> starts protecting your AI coding workflow.
