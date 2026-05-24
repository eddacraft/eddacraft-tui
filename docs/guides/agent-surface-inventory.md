# Agent Surface Inventory

| Type  | Authority     | Owner | Status | Freshness                                                                                 |
| ----- | ------------- | ----- | ------ | ----------------------------------------------------------------------------------------- |
| Guide | Authoritative | CIB   | Live   | Last reviewed 2026-05-25 against `AGENTS.md`, `.claude/`, `.opencode/`, and CIB-002 scope |

| Upstream                                                                                    | Downstream                                               |
| ------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| `AGENTS.md`, `.claude/skills/`, `.claude/agents/`, `.claude/commands/`, `.opencode/skills/` | `docs/guides/documentation-governance.md`, agent routing |

Authoritative inventory of the skills, agents, and commands the anvil workflow
depends on. Each entry names a canonical source so drift between the inventory,
`.claude/` and `.opencode/`, and external skill repositories is detectable.

Status: **In Progress** (CIB-002). Until automated validation lands, the
[Drift Detection](#drift-detection) section below describes the manual
cross-check.

## Purpose

- Make the set of skills and agents anvil expects to be available explicit.
- Distinguish repo-local definitions from globally-available ones.
- Identify the canonical source for each global entry so syncing has a target.
- Provide a single doc that contributors and agents can read to answer "is `X`
  something this repo defines, or is it expected to come from somewhere else?".

## Out of Scope

- Listing every global skill or agent that exists in `joshuaboys/code-env`. This
  inventory covers only what anvil's workflow actually references.
- Defining what each skill/agent _does_ — that's the skill or agent file's own
  description. This inventory is a routing index, not a re-description.
- Automating drift detection. That's a follow-up once the manual check
  stabilises.

## Canonical Sources

| Source                             | Role                                                                                | Path                                                                                                                      |
| ---------------------------------- | ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| **Repo-local Claude Code**         | Anvil-specific overrides and additions                                              | `.claude/skills/`, `.claude/agents/`, `.claude/commands/`                                                                 |
| **Repo-local OpenCode**            | Anvil-specific OpenCode parallel surface                                            | `.opencode/skills/`                                                                                                       |
| **Repo-local Codex**               | Codex-facing companion skills where anvil needs the same project-local workflow     | `.codex/skills/`                                                                                                          |
| **Global (`joshuaboys/code-env`)** | Cross-project skills, agents, commands the user maintains as their personal toolkit | `https://github.com/joshuaboys/code-env` — `.claude/skills/`, `.claude/agents/`, `.claude/commands/`; `.opencode/skills/` |
| **Claude Code built-ins**          | Skills shipped by Claude Code itself (e.g. `commit`, `loop`, `schedule`, `init`)    | Loaded by the Claude Code runtime; no source file in either repo                                                          |

When a name exists in both repo-local and global, the repo-local entry
**overrides** the global. The override pattern is intentional: anvil tunes the
surface for its `quick|mini|full` Council tiers, main-first branching, and
Rust + TypeScript polyglot conventions.

## Skills

### Repo-local skills

| Name                               | Location                                           | Purpose                                                                                                                                            | Notes                                                                                                                                                                           |
| ---------------------------------- | -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `dev-workflow`                     | `.claude/skills/dev-workflow/SKILL.md`             | Lifecycle routing tuned to anvil (main-first Worktrunk branches, cleanup offer, `/council [quick\|mini\|full]`, repo-local Surface Inventory link) | Vendored snapshot. Companion: `.opencode/skills/dev-workflow/SKILL.md`. Tracked via `!.claude/skills/dev-workflow/` and `!.opencode/skills/dev-workflow/` gitignore exceptions. |
| `addressing-pr-reviews`            | `.claude/skills/addressing-pr-reviews/SKILL.md`    | PR remediation workflow tuned to anvil CI, review ordering, rebase preference, bot mention rules, and docs closeout                                | Project-local override. Companion: `.opencode/skills/addressing-pr-reviews/SKILL.md`. Tracked via matching `.gitignore` exceptions.                                             |
| `planning-council`                 | `.claude/skills/planning-council/`                 | Multi-persona planning for anvil with project-specific playbooks (`direction-validate`, `plan-amend`, `plan-create`, `pre-execution-validate`)     | Anvil-specific — does not exist in `code-env`.                                                                                                                                  |
| `release`                          | `.claude/skills/release/`                          | Agent-driven release operator wrapper around `scripts/release/*`                                                                                   | Anvil-specific. Tracked via `!.claude/skills/release/` gitignore exception.                                                                                                     |
| `dependabot`                       | `.claude/skills/dependabot` (symlink → `code-env`) | Dependency-update triage                                                                                                                           | Symlink to the global `code-env` skill; tracked via `!.claude/skills/dependabot` gitignore exception.                                                                           |
| `dev-workflow` (OpenCode)          | `.opencode/skills/dev-workflow/SKILL.md`           | OpenCode-native parallel of the Claude `dev-workflow` skill                                                                                        | Same routing knowledge, including Worktrunk and cleanup-offer rules; OpenCode skill schema per `https://opencode.ai/docs/skills/`.                                              |
| `addressing-pr-reviews` (OpenCode) | `.opencode/skills/addressing-pr-reviews/SKILL.md`  | OpenCode-native parallel of the Claude `addressing-pr-reviews` skill                                                                               | Same anvil PR remediation workflow; OpenCode skill schema per `https://opencode.ai/docs/skills/`.                                                                               |
| `addressing-pr-reviews` (Codex)    | `.codex/skills/addressing-pr-reviews/SKILL.md`     | Codex-facing parallel of the PR remediation workflow                                                                                               | Same closure-loop contract for CI, unresolved review threads, and mergeability.                                                                                                 |

### Global skills the anvil workflow references

These are not vendored under `.claude/` or `.opencode/` here. They are expected
to be available via the agent runtime (Claude Code globals from
`~/.claude/skills/` or `joshuaboys/code-env`, or OpenCode-native skills loaded
on demand via the `skill` tool).

| Name                             | Canonical source                                          | Where anvil references it                                                       |
| -------------------------------- | --------------------------------------------------------- | ------------------------------------------------------------------------------- |
| `aps-planning`                   | `code-env/.claude/skills/aps-planning/`                   | `/plan` and `/plan-status` command notes; `anvil-plan-spec` agent docs          |
| `brainstorming`                  | `code-env/.claude/skills/brainstorming/`                  | `dev-workflow` Stage Map (Idea / spec)                                          |
| `writing-plans`                  | `code-env/.claude/skills/writing-plans/`                  | `dev-workflow` Stage Map (Plan)                                                 |
| `using-git-worktrees`            | `code-env/.claude/skills/using-git-worktrees/`            | `dev-workflow` Stage Map (Branch)                                               |
| `test-driven-development`        | `code-env/.claude/skills/test-driven-development/`        | `dev-workflow` Stage Map (Code)                                                 |
| `systematic-debugging`           | `code-env/.claude/skills/systematic-debugging/`           | `dev-workflow` Stage Map (Debug)                                                |
| `verification-before-completion` | `code-env/.claude/skills/verification-before-completion/` | `dev-workflow` Stage Map (Verify)                                               |
| `council`                        | `code-env/.claude/skills/council/`                        | Used in conjunction with the repo-local `/council` command for the Review stage |
| `finishing-a-branch`             | `code-env/.claude/skills/finishing-a-branch/`             | `dev-workflow` Stage Map (Finish)                                               |
| `parallel-agents`                | `code-env/.claude/skills/parallel-agents/`                | `dev-workflow` Stage Map (Parallelise)                                          |
| `commit`                         | Claude Code built-in or `code-env/.claude/skills/commit/` | `dev-workflow` Stage Map (Finish)                                               |

## Agents

### Repo-local agents (`.claude/agents/`)

| Name                   | Role                                                                                                             | Used by                                  |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------- | ---------------------------------------- |
| `council-reviewer`     | General correctness, maintainability, test coverage (and the `security` role per `commands/council.md` Role Map) | `/council`                               |
| `adversarial-reviewer` | Edge cases, failure paths, abuse cases                                                                           | `/council`                               |
| `operations-reviewer`  | CI, release, deployment, observability, recovery                                                                 | `/council`                               |
| `pragmatic-lead`       | Proportionality, scope, ship-readiness                                                                           | `/council`                               |
| `kernel-maintainer`    | Rust kernel correctness and parity reviews                                                                       | `/council` (full tier)                   |
| `anvil-plan-spec`      | APS plan authoring and validation                                                                                | `/plan`, `dev-workflow` Stage Map (Plan) |
| `plan-synthesizer`     | Multi-persona planning synthesis                                                                                 | `planning-council` skill                 |
| `tdd-coach`            | Test-first guidance                                                                                              | `dev-workflow` Stage Map (Code)          |

`protocols.md` in the same directory is a shared protocol fragment, not an agent
— it documents conventions consumed by the agents above.

### Global agents the anvil workflow references

| Name         | Canonical source                        | Where anvil references it                                                |
| ------------ | --------------------------------------- | ------------------------------------------------------------------------ |
| `debugger`   | `code-env/.claude/agents/debugger.md`   | `dev-workflow` Stage Map (Debug), `AGENTS.md` quick-reference table      |
| `autonomous` | `code-env/.claude/agents/autonomous.md` | `dev-workflow` Stage Map (Parallelise), repo-local `/autonomous` command |

Several global agents (`code-reviewer`, `architect`, `librarian`, `planner`,
`security-analyst`, etc.) exist in `code-env` but are not referenced by anvil's
documented workflow. They may still be invoked ad-hoc; this inventory does not
enumerate optional add-ons.

## Commands

### Repo-local commands (`.claude/commands/`)

| Name            | Purpose                                                                                                                                             |
| --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `/council`      | Risk-tiered Council review (`quick \| mini \| full`). Canonical for anvil — see [`.claude/commands/council.md`](../../.claude/commands/council.md). |
| `/plan`         | Start or continue APS planning                                                                                                                      |
| `/plan-status`  | Inspect APS planning state                                                                                                                          |
| `/review`       | Targeted pre-PR review routed by changed paths and risk                                                                                             |
| `/test`         | Run tests and fix failures                                                                                                                          |
| `/debug`        | Systematic debugging                                                                                                                                |
| `/commit`       | Stage and write commit                                                                                                                              |
| `/delegate`     | Delegate to a specialist via Codex MCP                                                                                                              |
| `/autonomous`   | Long-running autonomous task wrapper                                                                                                                |
| `/think-harder` | Deep analytical thinking                                                                                                                            |

### Global commands

Many additional commands exist in `code-env/.claude/commands/` (`council-full`,
`review-brief`, `review-peer`, `weekly`, etc.). Repo-local commands above
override the global one if the names collide.

## Drift Detection

Until automation lands, run the manual cross-check before assuming the inventory
is current:

1. **List repo-local surfaces:**

   ```bash
   ls .claude/skills/ .claude/agents/ .claude/commands/ .opencode/skills/ .codex/skills/
   ```

   Compare against the **Repo-local** tables above. Any name in the filesystem
   but not in the inventory (or vice-versa) is drift.

2. **Check global references resolve.** For each global entry in the tables
   above, verify the canonical source path exists:

   ```bash
   ls ~/Projects/src/code-env/.claude/skills/<name>/SKILL.md
   ls ~/Projects/src/code-env/.claude/agents/<name>.md
   ```

   A missing file means the global entry has been removed upstream and needs
   follow-up.

3. **Cross-reference with `dev-workflow`.** Skills, agents, and commands named
   in the Stage Map of `.claude/skills/dev-workflow/SKILL.md` and
   `.opencode/skills/dev-workflow/SKILL.md` must appear in one of the tables
   above.

4. **Cross-reference with `commands/council.md` Role Map.** Every agent in the
   Role Map must appear under
   [Repo-local agents](#repo-local-agents-claudeagents) or
   [Global agents](#global-agents-the-anvil-workflow-references).

If drift is found and the fix is small, fix the inventory in the same PR that
introduced the drift. If the drift is structural (new skill class, new role),
file a CIB item and resolve in a focused change.

## Update Protocol

The inventory is authoritative for _anvil's expectations_. Update it when:

- A new repo-local skill, agent, or command is added to `.claude/` or
  `.opencode/`. Add a row in the relevant Repo-local table.
- A repo-local entry is renamed, removed, or moved.
- The `dev-workflow` Stage Map starts referencing a new global skill or agent.
  Add a row to the relevant Global table with the canonical source.
- A previously-global entry is vendored repo-local. Move it from the Global
  table to the Repo-local table.

Do not update the inventory speculatively for globals anvil does not yet use —
the inventory's value comes from naming dependencies, not the universe of
available tools.

## References

- `dev-workflow` (Claude):
  [`.claude/skills/dev-workflow/SKILL.md`](../../.claude/skills/dev-workflow/SKILL.md)
- `dev-workflow` (OpenCode):
  [`.opencode/skills/dev-workflow/SKILL.md`](../../.opencode/skills/dev-workflow/SKILL.md)
- Council command + Role Map:
  [`.claude/commands/council.md`](../../.claude/commands/council.md)
- Agent conventions: [`AGENTS.md`](../../AGENTS.md)
- CIB-002 work item:
  [`plans/modules/continuous-improvement-backlog.aps.md`](../../plans/modules/continuous-improvement-backlog.aps.md)
- OpenCode skill schema: <https://opencode.ai/docs/skills/>
