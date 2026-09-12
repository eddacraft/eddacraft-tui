# Agent Surface Inventory

| Type  | Authority     | Owner | Status | Freshness                                                                                                                                                                                                          |
| ----- | ------------- | ----- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Guide | Authoritative | AICON | Live   | Last reviewed 2026-09-13 for the AGENTS.md rolling-main dogfood entry; inventory content is unaffected. Prior review 2026-09-10 against `eddaskills.toml` / `eddaskills.lock.json` and tracked harness skill trees |

| Upstream                                                                                                                                               | Downstream                                                               |
| ------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------ |
| `AGENTS.md`, `.claude/agents/`, `.claude/commands/`, `.opencode/agents/`, `.codex/config.toml`, `.agents/skills/`, `.grok/skills/`, `eddacraft/skills` | `docs/guides/documentation-governance.md`, `AGENTS.md`, runtime adapters |

Authoritative inventory of the skills, agents, and commands the anvil workflow
depends on. Each entry names a canonical source so drift between the inventory,
the tracked runtime adapters, and external skill repositories is detectable.

`AGENTS.md` points here for inventory questions. Runtime-specific adapters may
name their own hooks and commands, but this guide remains the single inventory
for shared agent-surface discovery.

This guide is **Live**. CIB-002 automated drift validation is still pending;
until that lands, the [Drift Detection](#drift-detection) section below
describes the manual cross-check.

## Purpose

- Make the set of skills and agents anvil expects to be available explicit.
- Distinguish repo-local definitions from globally-available ones.
- Identify the canonical source for each global entry so syncing has a target.
- Provide a single doc that contributors and agents can read to answer "is `X`
  something this repo defines, or is it expected to come from somewhere else?".

## Out of Scope

- Listing every skill in `eddacraft/skills`. `eddaskills.toml` is the full vend
  declaration; this inventory is the workflow routing index.
- Defining what each skill/agent _does_ — that's the skill or agent file's own
  description. This inventory is a routing index, not a re-description.
- Automating drift detection. That's a follow-up once the manual check
  stabilises.

## Canonical Sources

| Source                             | Role                                                                      | Path                                                                            |
| ---------------------------------- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| **Repo-local Claude Code**         | Anvil-specific agents and commands                                        | `.claude/agents/`, `.claude/commands/`                                          |
| **Repo-local OpenCode**            | Anvil-specific agent adapter                                              | `.opencode/agents/`                                                             |
| **Repo-local Codex**               | Codex-facing project configuration                                        | `.codex/config.toml`, `.codex/agents/`                                          |
| **Repo-local Grok Build**          | Native adapter only; shared skills are discovered from Claude/Codex roots | `.grok/skills/dev-loop-adapter-grok/`                                           |
| **Catalogue (`eddacraft/skills`)** | Locked project vend via `eddaskills sync`                                 | `eddaskills.toml`, `eddaskills.lock.json`, `.claude/skills/`, `.agents/skills/` |
| **anvil-managed Claude skill**     | `anvil-developer-functions` follows the anvil binary                      | `anvil skill install --client claude-code`                                      |
| **Runtime-provided surfaces**      | Skills and built-ins installed or supplied by the active agent runtime    | The runtime's discovered skill catalogue; no source file in this repository     |

When a name exists in both repo-local and global, the repo-local entry
**overrides** the global. The override pattern is intentional: anvil tunes the
surface for its `quick|mini|full` Council tiers, main-first branching, and
Rust + TypeScript polyglot conventions.

## Skills

### Vendored skills (`eddaskills.toml`)

A fresh clone is enough. Skills are locked projections from private
`eddacraft/skills`. Refresh with `eddaskills sync --update-lock --yes` and
commit the lock plus emitted files together. `eddaskills sync --check` verifies
a clone. Do not hand-edit emitted skill bodies.

Not vendored: `dev-loop-executor` (deprecated shim), `dev-loop-differential`
(optional overlay, default off), `skill-librarian` (catalogue ops), `loop-retro`
(optional drain audit), `anvil-developer-functions` (anvil-managed via
`anvil skill install`).

Shared skills vend only into `.claude/skills` and `.agents/skills`. Grok and
OpenCode already read those roots (CAP-006), so a third copy under
`.grok/skills` or `.opencode/skills` shows up as a duplicate name in the picker
and is unsafe once any copy drifts. Native adapters stay in their own root:
`dev-loop-adapter-claude` under `.claude/skills`, `dev-loop-adapter-codex` under
`.agents/skills`, `dev-loop-adapter-grok` under `.grok/skills`,
`dev-loop-adapter-opencode` under `.opencode/skills`. The Grok and OpenCode
adapters are repo-local unmanaged files until the catalogue emitter can skip
overlapping discovery roots.

| Workflow job        | Skill                                                             |
| ------------------- | ----------------------------------------------------------------- |
| Orchestrate         | `/dev-loop` → `agentic-loop` + `dev-loop-router` + native adapter |
| Design Q&A          | `grill-design`                                                    |
| Plan / ReadyItem    | `planning-workflow`, `plan-ready`                                 |
| APS truth / doctor  | `aps-planning`, `plan-doctor`                                     |
| Isolate             | `isolate-workspace`                                               |
| Implement           | `loop-build-tdd`                                                  |
| Debug               | `loop-debug`                                                      |
| Evidence            | `evidence-gate`                                                   |
| Verify              | `verify-loop`                                                     |
| Land                | `land-branch`                                                     |
| PR CI / threads     | `address-reviews`                                                 |
| Council             | `council`, `agent-messaging`                                      |
| Docs                | `docs-workflow`                                                   |
| Flags               | `openfeature`                                                     |
| anvil CLI / gates   | `using-anvil`                                                     |
| anvil opportunity   | `anvil-opportunity-assessment`                                    |
| Explain a subsystem | `how`                                                             |

Remaining vendored leaves: `dev-loop`, `dev-loop-module`, APS drain skills,
`commit`, `code-review`, `security-and-quality`, `agent-vault`. Full ids:
`eddaskills.toml`.

### Global skills

Do not rely on `joshuaboys/code-env` for the development loop. The old names
(`brainstorming`, `writing-plans`, `using-git-worktrees`,
`systematic-debugging`, `verification-before-completion`, `finishing-a-branch`,
`parallel-agents`) map to the vendored skills in the table above.

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
| `tdd-coach`            | Test-first guidance (Anvil-specific; catalogue copy not applied)                                                 | `dev-workflow` Stage Map (Code)          |
| `debugger`             | Systematic debugging / root-cause analysis (vendored)                                                            | `dev-loop` debug stage                   |
| `dev-loop-verifier`    | Read-only independent acceptance check for one bounded `dev-loop` change                                         | `verify-loop`                            |
| `council-supervisor`   | Quality-gates Council reviewer output                                                                            | `council` skill                          |
| `council-debate`       | Resolves material reviewer contradictions                                                                        | `council` skill                          |
| `council-judge`        | Synthesises Council outputs into PASS/REPAIR/REWORK/REPLAN/BLOCK                                                 | `council` skill                          |
| `security-analyst`     | Security advisory and threat modelling (not the Anvil `adversarial-reviewer`)                                    | `council` reviewer pool                  |

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
| `autonomous` | `code-env/.claude/agents/autonomous.md` | `dev-workflow` Stage Map (Parallelise), repo-local `/autonomous` command |

`debugger` and the extra Council roles are vendored from `eddacraft/skills`.
Anvil-tuned agents (`tdd-coach`, Council reviewers, APS roles,
`anvil-plan-spec`) are **not** overwritten by `eddaskills sync`. Catalogue
agents are projected only into `.claude/agents/`, `.codex/agents/`, and
`.opencode/agents/`.

Several global agents (`code-reviewer`, `architect`, `librarian`, `planner`, and
others) exist in `code-env` but are not referenced by anvil's documented
workflow. They may still be invoked ad-hoc; this inventory does not enumerate
optional add-ons.

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
     .claude/agents .claude/commands .claude/skills \
     .agents/skills .opencode/agents .opencode/skills \
     .codex/config.toml .codex/agents .grok/skills
   ```

   Compare against the **Repo-local** tables above. Any name in the filesystem
   but not in the inventory (or vice-versa) is drift.

2. **Check the vend** (needs `eddaskills` on `PATH`):

   ```bash
   eddaskills sync --check
   ```

3. **Cross-reference runtime workflows.** Skills, agents, and commands named by
   `AGENTS.md` or the tracked command files must appear in `eddaskills.toml` or
   the repo-local tables above.

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
