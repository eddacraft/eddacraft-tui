# Agent Surface Inventory

| Type  | Authority     | Owner | Status | Freshness                                                                                              |
| ----- | ------------- | ----- | ------ | ------------------------------------------------------------------------------------------------------ |
| Guide | Authoritative | AICON | Live   | Last reviewed 2026-07-21 against tracked `AGENTS.md`, `.claude/`, `.opencode/`, and `.codex/` surfaces |

| Upstream                                                                                                           | Downstream                                                               |
| ------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------ |
| `AGENTS.md`, `.claude/agents/`, `.claude/commands/`, `.opencode/agents/`, `.codex/config.toml`, runtime catalogues | `docs/guides/documentation-governance.md`, `AGENTS.md`, runtime adapters |

Authoritative inventory of the skills, agents, and commands the anvil workflow
depends on. Each entry names a canonical source so drift between the inventory,
the tracked runtime adapters, and external skill repositories is detectable.

`AGENTS.md` points here for inventory questions. Runtime-specific adapters may
name their own hooks and commands, but this guide remains the single inventory
for shared agent-surface discovery.

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
| **Repo-local Claude Code**         | Anvil-specific agents and commands                                                  | `.claude/agents/`, `.claude/commands/`                                                                                    |
| **Repo-local OpenCode**            | Anvil-specific agent adapter                                                        | `.opencode/agents/anvil-plan-spec.md`                                                                                     |
| **Repo-local Codex**               | Codex-facing project configuration                                                  | `.codex/config.toml`                                                                                                      |
| **Global (`joshuaboys/code-env`)** | Cross-project skills, agents, commands the user maintains as their personal toolkit | `https://github.com/joshuaboys/code-env` — `.claude/skills/`, `.claude/agents/`, `.claude/commands/`; `.opencode/skills/` |
| **Runtime-provided surfaces**      | Skills and built-ins installed or supplied by the active agent runtime              | The runtime's discovered skill catalogue; no source file in this repository                                               |

When a name exists in both repo-local and global, the repo-local entry
**overrides** the global. The override pattern is intentional: anvil tunes the
surface for its `quick|mini|full` Council tiers, main-first branching, and
Rust + TypeScript polyglot conventions.

## Skills

### Repo-local skills

No skills are vendored in the tracked repository. Directories such as
`.claude/skills/` may exist in an individual working copy as ignored runtime
material, but they are not an anvil source of truth and must not be linked from
repository documentation. Workflow skills are supplied by the active runtime or
an external canonical source listed below.

### Global skills the anvil workflow references

These are not vendored here. They are expected to be available via the agent
runtime (for example, Claude Code globals from `~/.claude/skills/` or
`joshuaboys/code-env`, or skills loaded on demand by another runtime).

| Name                             | Canonical source                                          | Where anvil references it                                                       |
| -------------------------------- | --------------------------------------------------------- | ------------------------------------------------------------------------------- |
| `brainstorming`                  | `code-env/.claude/skills/brainstorming/`                  | `dev-workflow` Stage Map (Idea / spec)                                          |
| `writing-plans`                  | `code-env/.claude/skills/writing-plans/`                  | `dev-workflow` Stage Map (Plan)                                                 |
| `using-git-worktrees`            | `code-env/.claude/skills/using-git-worktrees/`            | `dev-workflow` Stage Map (Branch)                                               |
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

**Cross-repo review fallback (CIB-027):** When implementation work occurs in a
downstream/sibling repository that does not have Anvil's `/council` command
available, use a focused code review (e.g. any available `code-reviewer` agent
or equivalent in the target environment, augmented by the target repository's
own CI and automated review checks). Record the evidence (review notes + target
CI results) before publishing the PR. Do not invoke Anvil-specific `/council` or
assume Anvil Council surfaces exist in the target. See `dev-workflow` for the
full review stage and when full Anvil Council is not applicable.

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
   git ls-tree -r --name-only HEAD -- \
     .claude/agents .claude/commands .opencode/agents .codex/config.toml
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

3. **Cross-reference runtime workflows.** Skills, agents, and commands named by
   `AGENTS.md` or the tracked command files must appear in one of the tables
   above and resolve through the active runtime catalogue or canonical external
   source. Do not treat ignored local skill directories as repository-owned.

4. **Cross-reference with `commands/council.md` Role Map.** Every agent in the
   Role Map must appear under
   [Repo-local agents](#repo-local-agents-claudeagents) or
   [Global agents](#global-agents-the-anvil-workflow-references).

If drift is found and the fix is small, fix the inventory in the same PR that
introduced the drift. If the drift is structural (new skill class, new role),
file a CIB item and resolve in a focused change.

## Update Protocol

The inventory is authoritative for _anvil's expectations_. Update it when:

- A new tracked repo-local agent or command is added to `.claude/`,
  `.opencode/`, or `.codex/`. Add a row in the relevant Repo-local table.
- A repo-local entry is renamed, removed, or moved.
- A tracked workflow starts referencing a new global skill or agent. Add a row
  to the relevant Global table with the canonical source.
- A previously-global entry is intentionally vendored and tracked repo-local.
  Move it from the Global table to the Repo-local table.

Do not update the inventory speculatively for globals anvil does not yet use —
the inventory's value comes from naming dependencies, not the universe of
available tools.

### Continuous improvement

- Guide:
  [`docs/guides/continuous-improvement-log.md`](./continuous-improvement-log.md)
- Tracked log: `plans/reviews/continuous-improvement-log.md`
- Commands: `pnpm ci-log:append|harvest|status|since|set-watermark`,
  `pnpm test:ci-log`
- Workflow: `.claude/workflows/triage-ci-log.js` (pair with
  `complete-cib-items.js`)

## References

- Council command + Role Map:
  [`.claude/commands/council.md`](../../.claude/commands/council.md)
- Agent conventions: [`AGENTS.md`](../../AGENTS.md)
- CIB-002 work item:
  [`plans/modules/continuous-improvement-backlog.aps.md`](../../plans/modules/continuous-improvement-backlog.aps.md)
- OpenCode skill schema: <https://opencode.ai/docs/skills/>
