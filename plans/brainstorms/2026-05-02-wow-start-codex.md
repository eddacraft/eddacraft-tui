# Wow Start Brainstorm - Codex

## Context

A senior influencer/developer tested Anvil and ended the session with:

> give me a version that people like me can just install and use

The important read is not that Anvil lacks installers. It already ships a native
binary and has credible install paths: install site, shell installer, PowerShell
installer, Homebrew, WinGet, and Scoop.

The gap is that installation does not yet become immediate protection in the
user's real coding workflow. A senior developer should not have to learn Anvil's
command map before Anvil proves value.

## Core Pitch

Ship **Anvil Start**: one command from installed binary to live protection.

```bash
cd my-repo
anvil start
```

The release message should be:

> Install Anvil. Run `anvil start`. Your AI coding tool is now guarded.

The wow moment should happen in the user's own repository, not in a tutorial:

> I installed Anvil, opened Cursor or Claude Code, asked the agent to make a
> risky change, and Anvil intercepted it before the unsafe write landed.

## Product Thesis

We already have install distribution. We need activation.

Today, a serious evaluator still has to understand too many surfaces before the
core value appears:

- `anvil auth login`
- `anvil init`
- `anvil check --all`
- `anvil watch --source`
- `anvil mcp install --client cursor`
- `anvil mcp install --client claude-code`
- `anvil hooks install`
- `anvil doctor`

Those are useful primitives, but they should not be the first-run product. The
first-run product should make the shortest safe path from "binary exists" to
"unsafe AI-generated changes are being checked at creation time".

`anvil start` already exists as an alias for `welcome`. We should make that
alias earn its name.

## Proposed Behaviour

`anvil start` should:

- detect the repository, package manager, Git state, language signals, and
  source layout
- detect supported AI clients, starting with Cursor and Claude Code
- initialise Anvil config if missing
- baseline existing findings so first run does not punish the user for old
  problems
- run a first high-signal scan immediately
- install the MCP write-validation guardrail for detected clients
- verify the MCP config it just wrote
- offer one-key Git hook installation when Git is available
- start save-time watch mode when an editor guardrail cannot be installed
- finish with a plain protection summary

The final state should be explicit:

```text
Anvil is protecting this repo

AI write validation: on for Cursor
Save-time checks: on
Commit gate: installed
Baseline: 42 existing findings ignored
New unsafe changes: will be warned before they leave your editor

First signal:
  AP-006 empty catch block
  src/auth/session.ts:88
```

If the ideal path is not available, `anvil start` should degrade honestly:

```text
Anvil is ready for this repo

No supported AI client was detected, so Anvil started save-time watch mode.
Keep this terminal open while you code.
```

## User Experience Principles

- Ask questions only when there is a real fork: multiple clients, overwrite
  risk, dirty config, or missing permissions.
- Prefer local value before cloud value. Login can be required for team policy,
  but the first protective moment should work locally where possible.
- Name the protection mode clearly: protected, ready, degraded, or needs action.
- Verify every integration write instead of assuming success.
- Make the next thing to try concrete, preferably against the user's own repo.

## Why This Could Wow New Users

- It reframes Anvil from a CLI toolkit into a control layer that switches on.
- It makes the first value event happen before docs, dashboards, CI, or team
  rollout.
- It gives influencers a one-command story they can repeat.
- It separates Anvil from normal SAST: Anvil intervenes while code is being
  created, not after a report is generated.
- It leans into the strongest demo loop: install, start, prompt an agent, see a
  deterministic intervention.

## Scope Fit

This fits the Anvil scope guard because it directly increases prevention at the
point of change creation.

It is not generic onboarding polish. It composes existing enforcement surfaces
so deterministic validation is present where unsafe changes are produced.

It should not become:

- a generic project bootstrapper
- an IDE productivity assistant
- a dashboard-first onboarding tour
- an agent orchestration layer
- a cloud account sales funnel before local value

## Repo Fit

This aligns with current repo direction:

- ADR-001 says Anvil should be planless-first and deliver value without plans or
  config.
- ADR-002 and ADR-003 support a low-friction first run: warnings over blocks and
  new edges only.
- ADR-012 makes the Rust `anvil` binary the primary entry point.
- ADR-030 / ADR-033 point future surfaces toward thin drivers over the daemon,
  while the current MCP launch shim already provides write validation.
- `LAUNCH` already owns start-flow polish and explicitly tracks the missing
  shortcut from onboarding into watch.

The implementation should be mostly composition over existing parts, not a new
engine.

## MVP Cut

Smallest valuable slice:

- make `anvil start` a non-demo activation path
- initialise config when missing
- run the existing first scan
- detect Cursor and Claude Code where feasible
- call the existing `anvil mcp install` path for the detected client
- verify the generated MCP config
- offer hook install when Git is available
- fall back to `anvil watch --source`
- print the protection summary

This is the release candidate shape:

```bash
curl -fsSL https://install.eddacraft.ai | sh
cd my-repo
anvil start
```

## Follow-On Ideas

- `anvil start --client cursor|claude-code|windsurf|vscode`
- `anvil start --team` for authenticated policy/profile pulls
- `anvil start --ci` after local protection is already active
- install-site copy that ends with `anvil start`, not a menu of commands
- `anvil status` showing one protected/not-protected summary
- a reversible "try the guardrail" challenge that asks consent before adding any
  demonstration file to the repo

## Risks

- Editor support can be over-promised if MCP clients do not reliably call the
  validation tool before writes.
- Aggressively writing editor config can damage trust.
- Local-only value can create confusion later when team policy needs auth.
- A noisy first scan can weaken the wow moment.
- Watch fallback is less impressive than pre-write interception.

Mitigations:

- verify integrations and clearly name degraded modes
- preview or prompt before overwriting existing editor config
- bias the first scan toward high-signal checks
- baseline old findings so the first run focuses on new unsafe changes
- keep the final status line brutally literal

## Decision Recommendation

Prioritise `anvil start` as the next user-facing launch bet.

Do not spend the next cycle primarily adding install channels. The install
surface is already credible. The missing product move is converting install into
protection with almost no user knowledge.

The headline should be:

> You can just install Anvil and use it. Run `anvil start` in any repo and it
> starts protecting your AI coding workflow.
