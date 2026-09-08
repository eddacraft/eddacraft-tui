# Agent Surface Inventory

| Type  | Authority     | Owner | Status | Freshness                                                                                              |
| ----- | ------------- | ----- | ------ | ------------------------------------------------------------------------------------------------------ |
| Guide | Authoritative | AICON | Live   | Last reviewed 2026-09-08 against the interim `development-loop` copy from `eddacraft/skills` (`b26b0964df83d7fd36c3c4e35ff94e1867dcb726`) plus tracked `AGENTS.md`, `.claude/`, `.agents/`, `.opencode/`, `.codex/`, and `.grok/` surfaces |

| Upstream                                                                                                           | Downstream                                                               |
| ------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------ |
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
| **Repo-local OpenCode**            | Anvil-specific agent adapter                                                        | `.opencode/agents/`                                                                                                       |
| **Repo-local Codex**               | Codex-facing project configuration                                                  | `.codex/config.toml`, `.codex/agents/`                                                                                    |
| **Repo-local Grok Build**          | Native skill emission only; no documented `.grok/agents/` contract                  | `.grok/skills/`                                                                                                           |
| **Catalogue (`eddacraft/skills`)** | Canonical development-loop / APS / Council skills (interim manual copy)             | `https://github.com/eddacraft/skills` — `skills/eddacraft/`, `agents/eddacraft/`, `profiles/development-loop.json`         |
| **Global (`joshuaboys/code-env`)** | Cross-project skills, agents, commands the user maintains as their personal toolkit | `https://github.com/joshuaboys/code-env` — `.claude/skills/`, `.claude/agents/`, `.claude/commands/`; `.opencode/skills/` |
| **Runtime-provided surfaces**      | Skills and built-ins installed or supplied by the active agent runtime              | The runtime's discovered skill catalogue; no source file in this repository                                               |

When a name exists in both repo-local and global, the repo-local entry
**overrides** the global. The override pattern is intentional: anvil tunes the
surface for its `quick|mini|full` Council tiers, main-first branching, and
Rust + TypeScript polyglot conventions.

## Skills

### Repo-local skills

This is an **interim manual copy** of the `development-loop` profile from
[`eddacraft/skills`](https://github.com/eddacraft/skills) (commit
`b26b0964df83d7fd36c3c4e35ff94e1867dcb726`) until `eddaskills sync` can vend
the same selection. Catalogue-only `skill.meta.json` and `evals/` are omitted.
Harness loaders read `SKILL.md` plus adjacent references and scripts.

Tracked identically in `.claude/skills/`, `.agents/skills/`, `.opencode/skills/`,
and `.grok/skills/`. Each copied skill directory (except the two APS-managed
skills below) has `.manual-copy.json` stating the interim origin — not an
eddaskills lock digest.

**Already present** (APS CLI-managed under `.claude/skills` and `.agents/skills`;
`.claude` bodies already matched the catalogue. The `.agents` `aps-planning`
copy had a local work-item-claim overlay — that was aligned to the catalogue so
all four roots stay byte-identical. Claim-issue rules remain in
`plans/project-context.md`. Copies of both skills were added to
`.opencode/skills` and `.grok/skills`):

| Name           | Canonical source                                      | Notes                                      |
| -------------- | ----------------------------------------------------- | ------------------------------------------ |
| `aps-planning` | `eddacraft/skills` `skills/eddacraft/aps-planning/`   | Kept `.aps-managed.json` on existing roots |
| `plan-doctor`  | `eddacraft/skills` `skills/eddacraft/plan-doctor/`    | Kept `.aps-managed.json` on existing roots |

**Added from bundles `dev-loop` + `aps` + `council` and profile extras:**

| Name                         | Canonical source |
| ---------------------------- | ---------------- |
| `agentic-loop`               | `skills/eddacraft/agentic-loop/` |
| `dev-loop`                   | `skills/eddacraft/dev-loop/` |
| `dev-loop-module`            | `skills/eddacraft/dev-loop-module/` |
| `planning-workflow`          | `skills/eddacraft/planning-workflow/` |
| `aps-probe`                  | `skills/eddacraft/aps-probe/` |
| `aps-safety-rails`           | `skills/eddacraft/aps-safety-rails/` |
| `aps-landing`                | `skills/eddacraft/aps-landing/` |
| `aps-escalation-queue`       | `skills/eddacraft/aps-escalation-queue/` |
| `aps-resume`                 | `skills/eddacraft/aps-resume/` |
| `plan-ready`                 | `skills/eddacraft/plan-ready/` |
| `grill-design`               | `skills/eddacraft/grill-design/` |
| `isolate-workspace`          | `skills/eddacraft/isolate-workspace/` |
| `loop-build-tdd`             | `skills/eddacraft/loop-build-tdd/` |
| `loop-debug`                 | `skills/eddacraft/loop-debug/` |
| `evidence-gate`              | `skills/eddacraft/evidence-gate/` |
| `land-branch`                | `skills/eddacraft/land-branch/` |
| `address-reviews`            | `skills/eddacraft/address-reviews/` |
| `verify-loop`                | `skills/eddacraft/verify-loop/` |
| `loop-retro`                 | `skills/eddacraft/loop-retro/` |
| `dev-loop-differential`      | `skills/eddacraft/dev-loop-differential/` |
| `dev-loop-router`            | `skills/eddacraft/dev-loop-router/` |
| `dev-loop-adapter-claude`    | `skills/eddacraft/dev-loop-adapter-claude/` |
| `dev-loop-adapter-codex`     | `skills/eddacraft/dev-loop-adapter-codex/` |
| `dev-loop-adapter-opencode`  | `skills/eddacraft/dev-loop-adapter-opencode/` |
| `dev-loop-adapter-grok`      | `skills/eddacraft/dev-loop-adapter-grok/` |
| `dev-loop-executor`          | `skills/eddacraft/dev-loop-executor/` (deprecated shim; keep for older emissions) |
| `council`                    | `skills/eddacraft/council/` |
| `agent-messaging`            | `skills/eddacraft/agent-messaging/` |
| `agent-vault`                | `skills/eddacraft/agent-vault/` |
| `docs-workflow`              | `skills/eddacraft/docs-workflow/` |
| `openfeature`                | `skills/eddacraft/openfeature/` |
| `code-review`                | `skills/eddacraft/code-review/` |
| `commit`                     | `skills/eddacraft/commit/` |
| `skill-librarian`            | `skills/eddacraft/skill-librarian/` |
| `security-and-quality`       | `skills/eddacraft/security-and-quality/` |

Residual: replace this tree with `eddaskills sync` once project vending is
ready. Do not hand-edit the copied skill bodies.

### Global skills the anvil workflow still references

These remain expected from the agent runtime or `joshuaboys/code-env`. `council`
and `commit` moved to the repo-local table above.

| Name                             | Canonical source                                          | Where anvil references it                                                       |
| -------------------------------- | --------------------------------------------------------- | ------------------------------------------------------------------------------- |
| `brainstorming`                  | `code-env/.claude/skills/brainstorming/`                  | `dev-workflow` Stage Map (Idea / spec)                                          |
| `writing-plans`                  | `code-env/.claude/skills/writing-plans/`                  | `dev-workflow` Stage Map (Plan)                                                 |
| `using-git-worktrees`            | `code-env/.claude/skills/using-git-worktrees/`            | `dev-workflow` Stage Map (Branch)                                               |
| `systematic-debugging`           | `code-env/.claude/skills/systematic-debugging/`           | `dev-workflow` Stage Map (Debug)                                                |
| `verification-before-completion` | `code-env/.claude/skills/verification-before-completion/` | `dev-workflow` Stage Map (Verify)                                               |
| `finishing-a-branch`             | `code-env/.claude/skills/finishing-a-branch/`             | `dev-workflow` Stage Map (Finish)                                               |
| `parallel-agents`                | `code-env/.claude/skills/parallel-agents/`                | `dev-workflow` Stage Map (Parallelise)                                          |

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
| `debugger`             | Systematic debugging / root-cause analysis (interim catalogue projection)                                        | `dev-loop` debug stage                   |
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

`debugger` is now a repo-local interim projection from `eddacraft/skills`. Existing
Anvil-tuned agents (`tdd-coach`, Council reviewers, APS roles, `anvil-plan-spec`)
were **not** overwritten. New catalogue agents were projected only into layouts
this repo already uses (`.claude/agents/`, `.codex/agents/`, `.opencode/agents/`).
They were not added to `.github/agents/` (Copilot is not a target for these
assets) or `.grok/agents/` (no documented native Grok agent-file contract).

Several global agents (`code-reviewer`, `architect`, `librarian`, `planner`,
and others) exist in `code-env` but are not referenced by anvil's documented
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
   above and resolve through a tracked harness path or a remaining global
   source. Tracked `.claude/skills/`, `.agents/skills/`, `.opencode/skills/`,
   and `.grok/skills/` copies of `development-loop` are repository-owned until
   `eddaskills sync` replaces them.

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
