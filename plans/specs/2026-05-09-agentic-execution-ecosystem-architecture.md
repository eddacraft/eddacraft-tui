# Agentic Execution Ecosystem Architecture Review

Date: 2026-05-09

Status: Proposed

Scope: skills, hooks, agents, prompts, orchestrators, workflows, execution
pipelines, automation layers, policy integration, context management,
delegation, memory, runtime coordination, and APS integration.

## Purpose

Define a coherent target architecture for autonomous and semi-autonomous
execution across this repository and organisation.

The goal is an agentic operating system for AI-native software engineering: a
small set of composable primitives with explicit authority boundaries,
deterministic enforcement where required, observable execution state, and
low-friction human supervision.

APS remains mandatory and foundational. APS may evolve in integration and
execution semantics, but it remains the authoritative planning and intent layer.

## Alignment With Plan / Build / Release

This architecture implements the execution layer of
`2026-05-09-plan-build-release-operating-model.md`. It does not define a
separate lifecycle.

The shared lifecycle is:

```text
APS Draft -> APS Proposed -> APS Ready -> In Progress -> Merged -> Released/Shipped -> Complete/Archived
```

Agentic execution contributes routing, judgement, sessions, events, leases, and
playbooks around that lifecycle. It must not redefine release authority,
validation authority, or APS shipped-state semantics.

Target-state versus migration-state rule:

- Target-state agent workflows branch from `main`, open PRs to `main`, and use
  release records to update shipped APS state.
- During migration, agents may encounter current-state docs that still route
  through `dev`. Those are compatibility instructions, not a competing target
  architecture.
- If a skill, command, or playbook sees both models in scope, it must state which
  one it is following and stop if the conflict changes a safety or release
  decision.

Authority boundaries inherited from the operating model:

| Concern | Agentic responsibility | Non-responsibility |
| --- | --- | --- |
| APS intent | Load, validate, reconcile, and link evidence | Do not invent untracked work authority from chat. |
| Validation | Run or request deterministic checks and record evidence links | Do not treat agent judgement as pass/fail proof. |
| Release | Route to release playbooks and deterministic commands | Do not manually replace release commands except approved emergency recovery. |
| Shipped state | Consume release records for APS reconciliation | Do not mark shipped from PR merge, changelog prose, or memory. |
| Observability | Emit workflow/session events and evidence links | Do not use spans or chat as source-of-truth. |

Release tracking issues and release records have distinct roles: the issue is the
operator log and recovery narrative; the release record is the canonical
machine-readable proof of what shipped.

## Current-State Assessment

### Surfaces Observed

Repository-local agentic surfaces include:

- `.claude/skills/release/SKILL.md`
- `.claude/commands/{autonomous,commit,council,debug,delegate,plan,plan-status,review,test,think-harder}.md`
- `.claude/agents/{anvil-plan-spec,council-reviewer,adversarial-reviewer,kernel-maintainer,operations-reviewer,pragmatic-lead,plan-synthesizer,tdd-coach}.md`
- `.claude/agents/protocols.md`
- `.claude/hooks/{security-guard,codex-review-post}.sh`
- `.claude/council/{schema,council-session,council-finding,council-evidence,council-publish}.sh`
- `.claude/rules/{aps-index,architecture}.md`
- `tools/local-agent-run.sh`
- `.github/copilot-instructions.md`
- `.github/PULL_REQUEST_TEMPLATE.md`
- GitHub Actions workflows and local scripts that act as deterministic gates
- APS plans, modules, specs, decisions, and release/process documents

The broader environment also exposes global OpenCode/Claude skills, including
planning, council, local review council, release, APS planning, TDD,
debugging, verification, delegation, and worktree skills.

### What Works

- APS is already treated as mandatory and central.
- There are real specialist agents with reasonably clear perspectives.
- Council has a local session schema with findings, evidence, waivers, events,
  publication, and statuses.
- The release skill has already moved conceptually toward a thin agent wrapper
  around deterministic commands.
- Hooks already distinguish at least one deterministic safety concern: dangerous
  shell command blocking.
- `tools/local-agent-run.sh` provides a local non-interactive agent runner and
  logs output under `plans/agent-runs/`.
- PR template already asks for APS execution context and durable validation
  references.

### Current Drift And Inconsistency

- The release skill on `dev` expects deterministic helper commands under
  `scripts/release/*`, but this checkout only exposes `scripts/release.sh`. That
  is a live skill-to-repo authority mismatch.
- Repo-local `/council` describes an always-five-agent batch process; global
  council skills distinguish streaming, quick, standard, and full modes.
- Repo-local `/review` is a generic checklist rather than a targeted pre-PR
  review router.
- Planning Council exists as a global skill, but repo-local `/plan` and
  `anvil-plan-spec` do not enforce planning-council validation before
  substantial Ready work starts.
- Hook behaviour includes post-commit probabilistic review via Codex. That is
  later than the desired quality loop and blurs deterministic hook boundaries.
- Agent role names and capabilities differ across Claude, OpenCode, and local
  command docs.
- Session state exists for council, but not as a unified execution/event model
  for planning, release, agent runs, provenance, and drift.
- Operational logic is split across skills, command prompts, runbooks, shell
  scripts, workflows, and tribal memory.

## Structural Weakness Analysis

### 1. Authority Boundaries Are Too Soft

The ecosystem mixes four different kinds of authority:

- policy and governance
- orchestration state
- agent prompts
- deterministic execution

Several prompts contain operational procedures that should be deterministic
commands, schemas, or state transitions. This makes the system vulnerable to
prompt drift and partial reimplementation by agents.

### 2. Skills, Commands, And Agents Overlap

Skills often describe full workflows. Commands also describe workflows. Agents
sometimes encode workflow steps. Hooks sometimes trigger agent-like behaviour.

This creates parallel execution paths. Two humans or agents can ask for “review”
or “release” and follow different semantics depending on entrypoint.

### 3. Hooks Are Not Classified By Risk

Hooks currently mix deterministic guardrail behaviour with optional agent review
behaviour. Hooks should be classified by whether they are allowed to block,
warn, record, or invoke probabilistic systems.

### 4. Prompt Ownership Is Implicit

Prompts live in command files, agent files, skills, docs, and scripts. There is
no machine-readable manifest identifying owner, lifecycle, authority level,
inputs, outputs, or deprecation state.

### 5. Context Propagation Is Inconsistent

Some flows use APS context. Some use git diff. Some use session JSON. Some use
GitHub issue comments. Some rely on the current chat. There is no standard
context envelope for agent execution.

### 6. Memory Is Fragmented

Memory appears in:

- APS files
- council sessions
- `plans/agent-runs/` logs
- release tracking issues
- PR descriptions
- local chat/session context
- GitHub Actions logs

These are all useful, but their ownership is not explicit.

### 7. Deterministic And Probabilistic Boundaries Are Blurred

LLM agents are appropriate for judgement, synthesis, critique, and ambiguity.
They are not appropriate as the only authority for release readiness, policy
enforcement, or validation evidence.

### 8. Concurrency Assumptions Are Underdeveloped

Multiple agents may run concurrently, but the system lacks a standard lease,
lock, ownership, or event protocol for shared work items, files, branches, and
plans.

### 9. Observability Is Mostly Artefact-Based

Logs and session files exist, but there is no unified event stream or trace model
for agentic execution. It is hard to answer: who decided what, based on which
context, with which tools, producing which artefacts?

### 10. Discoverability Depends On Knowing The Right Incantation

Users and agents must know whether to call `/review`, `/council`, `/plan`,
`/release`, a global skill, a local script, or a workflow. This does not scale.

## Recommended Architectural Model

Use a layered model with explicit authority boundaries.

```text
┌────────────────────────────────────────────────────────────┐
│ Human / Agent Surfaces                                     │
│ CLI, IDE, MCP, web, Claude, OpenCode, GitHub               │
└───────────────────────┬────────────────────────────────────┘
                        │
┌───────────────────────▼────────────────────────────────────┐
│ Workflow Router                                            │
│ maps intent + changed paths + APS state to playbooks       │
└───────────────────────┬────────────────────────────────────┘
                        │
┌───────────────────────▼────────────────────────────────────┐
│ Orchestration Layer                                        │
│ state machines, sessions, events, leases, resumability     │
└───────────────────────┬────────────────────────────────────┘
                        │
┌───────────────────────▼────────────────────────────────────┐
│ Execution Layer                                            │
│ deterministic scripts, CI workflows, tests, build, release │
└───────────────────────┬────────────────────────────────────┘
                        │
┌───────────────────────▼────────────────────────────────────┐
│ Policy + Validation Layer                                  │
│ Anvil checks, APS lint, CI gates, security guards          │
└───────────────────────┬────────────────────────────────────┘
                        │
┌───────────────────────▼────────────────────────────────────┐
│ Memory + Provenance Layer                                  │
│ APS, decisions, sessions, release records, event log       │
└────────────────────────────────────────────────────────────┘
```

### Core Separation

| Concern | Belongs In | Must Not Live Only In |
| --- | --- | --- |
| Intent and execution authority | APS | chat memory |
| Workflow routing | deterministic guidance metadata/script | ad-hoc prompt text |
| Judgement and critique | skills/agents/council | shell scripts |
| Deterministic execution | scripts, CI, CLI commands | agent prose |
| Policy enforcement | Anvil, hooks, CI, schemas | reviewer opinion |
| Memory and provenance | APS, session records, release records | transient LLM context |
| Human approval | explicit approval events | inferred silence |

## Recommended Execution Taxonomy

### Primitive Types

| Primitive | Definition | Example |
| --- | --- | --- |
| Plan | Authoritative intent and acceptance criteria | APS module/work item |
| Playbook | Ordered human/agent workflow for a phase | release-publish.md |
| Command | Deterministic executable unit | scripts/release/preflight.sh |
| Agent | Probabilistic specialist with bounded role | operations-reviewer |
| Skill | Router and context loader for a domain | release, council, planning-council |
| Hook | Deterministic guardrail at a tool boundary | security-guard.sh |
| Policy | Machine-checkable rule | Anvil check, schema, CI gate |
| Session | Resumable orchestration record | council session JSON |
| Event | Append-only execution/provenance observation | review_started, tag_pushed |
| Artefact | Durable output | spec, release record, PR summary |

### Execution Classes

| Class | Description | Allowed To Block? | Probabilistic? |
| --- | --- | --- | --- |
| Mechanical guard | Fast deterministic safety check | yes | no |
| Advisory guidance | Deterministic recommendation of playbooks/reviews | no by default | no |
| Agent judgement | Review, synthesis, critique, trade-off analysis | only through policy gate | yes |
| Deterministic execution | Build, test, release, validation commands | yes | no |
| Orchestration | State transition and resumability | yes, when state invalid | no by default |
| Governance | APS/ADR/policy approval | yes | mixed, but recorded |

## Skills Architecture Recommendations

### Canonical Skill Structure

Every non-trivial skill should use this structure:

```text
.claude/skills/<skill>/
  SKILL.md                 # router, trigger, authority, safety rules
  playbooks/               # phase-specific workflows
  checklists/              # deterministic review lists
  schemas/                 # machine-readable IO contracts
  references/              # stable facts, command lists, artefact lists
  examples/                # optional worked examples
```

`SKILL.md` should not be a 900-line workflow. It should answer:

- when to activate
- what authority it has
- which playbook to read
- what deterministic commands own execution
- what state record to update
- when to stop and ask

### Skill Types

| Skill Type | Purpose | Examples |
| --- | --- | --- |
| Router skill | Select playbook and context | release, council |
| Planning skill | Create/validate APS plans | planning-council, plan |
| Execution skill | Drive a bounded workflow | release-publish, rollback |
| Review skill | Run critique and convergence | local-review-council |
| Diagnostic skill | Debug with evidence discipline | systematic-debugging |
| Governance skill | Apply policy/approval semantics | APS reconciliation |

### Required Skill Metadata

Add machine-readable metadata to each skill:

```yaml
skill:
  id: release
  owner: engineering-platform
  authority: router
  deterministicCommands:
    - scripts/release/preflight.sh
  stateRecords:
    - release-record
    - github-issue
  requiredInputs:
    - repo-state
    - aps-context
  outputs:
    - release-candidate
    - release-record
  allowedTools:
    - git-read
    - gh
    - bash-deterministic
```

## Hooks Architecture Recommendations

### Hook Doctrine

Hooks are guardrails, not orchestrators.

Hooks may:

- block dangerous deterministic actions
- run fast deterministic checks
- emit guidance
- record lightweight events

Hooks must not:

- invoke LLM review as a required step
- perform long-running orchestration
- make product judgement calls
- mutate APS status without explicit command context
- hide failures in background logs that humans never see

### Hook Classes

| Class | Example | Behaviour |
| --- | --- | --- |
| Blocker | dangerous command guard | fail closed |
| Formatter | staged file formatting | deterministic mutation allowed |
| Guidance | changed-path playbook hints | warn by default |
| Evidence | record command/event summary | append-only |
| Policy | APS/schema/path checks | warning first, later required |

### Recommended Hook Changes

- Keep `security-guard.sh` as a deterministic blocker, but move dangerous pattern
  configuration into a data file so it can be reviewed and tested.
- Retire or disable post-commit LLM review by default. Replace with pre-PR
  targeted review guidance.
- Add a deterministic `agent-guidance` hook path that prints required playbooks,
  reviews, and checks based on staged files.
- Ensure hooks output machine-readable JSON plus human-readable summaries.

## Agent Architecture Recommendations

### Agent Role Model

Agents should be defined by capability and authority, not just persona.

Recommended role fields:

```yaml
agent:
  id: operations-reviewer
  role: operations
  authority: advisory
  canModifyFiles: false
  canRunCommands: read-only-or-validate
  requiredContext:
    - diff
    - aps-items
    - validation-output
  outputSchema: council-finding-v1
  escalationTargets:
    - security
    - pragmatic
```

### Agent Categories

| Category | Responsibility | Examples |
| --- | --- | --- |
| Planner | interrogate, negotiate, synthesize plans | plan-synthesizer |
| Reviewer | critique changes | council-reviewer, adversarial-reviewer |
| Operator | run bounded workflows | release operator skill |
| Implementer | modify files under plan authority | general coding agent |
| Verifier | gather evidence and close findings | TDD/verification agent |
| Coordinator | manage sessions, leases, event flow | future orchestrator |

### Agent Lifecycle

Each agent should have:

- owner
- version
- capability schema
- allowed tools
- input contract
- output contract
- escalation protocol
- deprecation path
- test/example prompts

Agents should not own persistent business process. They should execute within a
playbook and produce structured outputs.

## Prompt Governance Recommendations

### Prompt Authority Levels

| Level | Meaning | Example |
| --- | --- | --- |
| Normative | Defines required behaviour | AGENTS.md, APS rules |
| Routing | Selects workflow/playbook | SKILL.md |
| Procedural | Stepwise workflow | playbooks/*.md |
| Persona | Role behaviour | agents/*.md |
| Contextual | Temporary task prompt | user request |

Higher-level prompts override lower-level prompts. When conflict exists,
agents must cite the conflict and stop if it affects safety or authority.

### Prompt Manifests

Every durable prompt should declare:

- purpose
- owner
- authority level
- inputs
- outputs
- allowed mutations
- dependent policies
- last-reviewed date
- deprecation status

Prompt changes should trigger targeted review based on authority level.

## Orchestration Strategy Recommendations

### State Machines Over Prose

Substantial workflows should be state machines with explicit transitions.

Examples:

- plan creation
- pre-execution validation
- pre-PR review
- release candidate
- release publish
- rollback
- APS reconciliation

State records should include:

- id
- kind
- status
- current phase
- APS item IDs
- branch/SHA
- playbook version
- actors
- events
- evidence
- decisions
- waivers

### Event-Driven Coordination

Introduce a local event envelope:

```json
{
  "id": "evt_...",
  "time": "2026-05-09T00:00:00Z",
  "kind": "review.completed",
  "actor": "operations-reviewer",
  "workflow": "pre-pr-review",
  "apsItems": ["MOD-001"],
  "branch": "feat/example",
  "sha": "...",
  "payload": {}
}
```

The first implementation can be local files. The architecture should not assume
local-only forever.

### Leases For Concurrency

Multiple agents need a simple lease protocol:

```json
{
  "resource": "APS:MOD-001",
  "holder": "agent-id",
  "branch": "feat/mod-001",
  "expiresAt": "...",
  "intent": "implementation"
}
```

Leases should warn, not hard-block, until the workflow is proven.

## Delegation Model Recommendations

Delegation should be explicit and typed.

| Delegation Type | Use When | Output |
| --- | --- | --- |
| Research | Need context discovery | findings summary |
| Review | Need critique | structured findings |
| Implementation | Need file changes | branch/diff + validation |
| Verification | Need evidence | command results |
| Planning | Need APS/ADR/spec | plan artefacts |
| Operations | Need release/deploy action | state record + evidence |

Delegated work must include:

- scope
- APS item or exception
- allowed files
- forbidden actions
- validation command
- expected output schema
- timeout/checkpoint expectations

## Context Propagation Recommendations

Define a standard context envelope passed to agents and playbooks:

```yaml
context:
  repo: EddaCraft/anvil-001
  branch: docs/example
  base: main
  sha: abc123
  aps:
    items: [MOD-001]
    module: MOD
  changedFiles: []
  relevantDecisions: []
  playbook: .claude/skills/example/playbooks/foo.md
  authority:
    canEdit: true
    canCommit: false
    canPush: false
  validation:
    required: []
```

Agents should not have to reconstruct this independently for every workflow.
During migration, `base` may be `dev` only when the workflow explicitly declares
it is executing the current compatibility model rather than the target operating
model.

## Memory And State Recommendations

### Memory Classes

| Memory | Authority | Storage |
| --- | --- | --- |
| Intent memory | APS | `plans/**` |
| Decision memory | ADR/spec | `plans/decisions`, `plans/specs` |
| Execution memory | workflow session | `.claude/sessions` or future state dir |
| Review memory | council session + PR summary | `.claude/council`, `plans/reviews` |
| Release memory | release record | GitHub Release asset / plans release record |
| Observability memory | event log | local event log, later service |
| Ephemeral memory | chat/session | not authoritative |

### Source-Of-Truth Rule

No workflow should require reading chat history to know the current state.

## Validation And Enforcement Recommendations

Validation should be layered:

```text
editor/save feedback
  -> hook mechanical guardrails
  -> pre-PR agent review
  -> PR CI and policy checks
  -> post-merge readiness
  -> release candidate checks
  -> release verification
```

Deterministic gates should own:

- formatting
- linting
- tests
- build
- APS schema/count checks
- policy checks
- release artefact verification
- dangerous command blocking

Agent gates should own:

- ambiguity
- design critique
- failure mode exploration
- plan validation
- release judgement recommendations
- human-readable synthesis

## Runtime Observability Recommendations

Minimum observability fields:

- workflow id
- state transition
- actor
- tool invoked
- inputs digest
- outputs digest
- APS item IDs
- branch/SHA
- validation result
- error code/class
- human approval event

Provide these views:

- current active workflows
- active leases
- recent agent runs
- open council findings
- APS drift report
- release readiness state
- failed hooks/checks

## Naming And Folder Structure Recommendations

### Proposed Structure

```text
.claude/
  agents/
    roles/
    schemas/
  skills/
    release/
      SKILL.md
      playbooks/
      schemas/
      references/
    council/
    planning-council/
  hooks/
    policies/
    guidance/
  workflows/
    schemas/
  sessions/
    council/
    planning/
    release/
  rules/

scripts/
  agent/
    guidance.sh
    validate-workflow.sh
  release/
    assess.sh
    readiness.sh
    publish.sh

plans/
  specs/
  decisions/
  modules/
  reviews/
  agent-runs/
```

### Naming Rules

- Skills use domain names: `release`, `planning-council`, `council`.
- Playbooks use verb-noun names: `candidate-build.md`, `publish-release.md`.
- Agents use role names: `operations-reviewer`, `plan-synthesizer`.
- Schemas use versioned names: `workflow-session.v1.schema.json`.
- Events use dotted names: `review.completed`, `release.tagged`.

## Source-Of-Truth Recommendations

| Question | Source Of Truth |
| --- | --- |
| What are we trying to do? | APS |
| Why this architecture? | ADR/spec |
| Who is allowed to execute? | workflow/session authority record |
| What should an agent do next? | guidance script + playbook |
| Did review converge? | council session + PR summary |
| Did validation pass? | CI/local command evidence |
| What shipped? | tag + release record |
| What remains? | APS + open findings/issues |

## APS Integration Recommendations

APS should evolve from planning ledger to orchestration authority without
becoming a general-purpose runtime engine.

Add to APS:

- machine-readable work item metadata
- validation commands
- release note metadata
- allowed/expected file areas
- planning council validation status
- execution lease references
- as-built docs impact marker once docs model is finalised

Do not put in APS:

- raw agent transcripts
- large logs
- CI output blobs
- detailed execution traces

APS should link to those artefacts, not embed them.

## Policy Enforcement Integration

Anvil should be the deterministic policy enforcement layer for agent work where
possible.

Near-term integrations:

- staged-file scope checks against APS `files` metadata
- changed-path review guidance
- policy check summaries attached to council/session evidence
- release readiness checks that include APS and artefact consistency

Longer-term integrations:

- signed review/release attestations
- workflow provenance records
- agent capability policy
- MCP pre-write enforcement using APS scope

## Scalability Under Concurrency

Concurrency risks:

- agents editing same files
- APS status races
- duplicate PRs for same work item
- stale plan execution after repo changes
- release and feature work colliding
- hidden local session state

Controls:

- branch/worktree per work stream
- APS item leases
- changed-file conflict detection
- pre-execution reality validation
- event log and session ids
- PR template requiring APS IDs
- deterministic guidance script warning about conflicts

## Local Vs Distributed Execution

The first implementation can be local-first, but contracts should not assume one
machine.

Local-first now:

- `.claude/council/sessions`
- `plans/agent-runs`
- local hooks
- local scripts

Distributed-compatible later:

- GitHub Actions as canonical CI/readiness record
- release records as assets
- event logs uploadable to CI artefacts
- session ids and schemas stable across surfaces
- no dependence on private chat context

## Surface Interoperability

All surfaces should consume the same workflow metadata:

- CLI
- IDE
- daemon
- web
- MCP
- GitHub Actions
- Claude/OpenCode/Codex agents

The contract should be metadata and events, not a specific UI.

## Migration Strategy

### Phase 1: Inventory And Declare Authority

- Add manifests for skills, agents, hooks, commands, and workflows.
- Mark each surface as router, executor, reviewer, policy, or memory.
- Identify deprecated or duplicate surfaces.
- Fix live drift such as release skill expecting missing `scripts/release/*`.

### Phase 2: Introduce Guidance Layer

- Add deterministic `scripts/agent/guidance.sh` in advisory mode.
- Encode path-to-playbook/reviewer/check rules.
- Hook prints guidance; agents consume JSON; CI reports warnings.

### Phase 3: Standardise Playbooks And Schemas

- Refactor release skill into playbooks.
- Add planning-council playbooks.
- Add workflow session schema.
- Add agent capability schema.

### Phase 4: Integrate APS Execution Semantics

- Add work item metadata.
- Add planning council validation marker.
- Add execution leases.
- Add APS drift checks in warning mode.

### Phase 5: Strengthen Enforcement

- Promote stable guidance and APS checks to required gates.
- Replace post-commit LLM review with pre-PR review workflow.
- Require release readiness records before tags.

### Phase 6: Observability And Provenance

- Add event log.
- Add active workflow status command.
- Add release/review/provenance records.
- Integrate with Anvil policy/attestation where appropriate.

## Failure-Mode Analysis

### Prompt Drift

Risk: durable workflows change through prompt edits without schema/policy review.

Mitigation: prompt manifests, authority levels, review triggers for prompt
changes, deterministic commands for execution.

### Parallel Execution Conflict

Risk: multiple agents edit same files or execute same APS item.

Mitigation: leases, branch-per-stream, changed-file conflict warnings, APS
execution state.

### Hook Overreach

Risk: hooks become slow, probabilistic, or hard to debug.

Mitigation: hook classes, no LLM in blocking hooks, JSON output, strict timeout
budgets.

### Orchestration Duplication

Risk: skills, commands, scripts, and docs implement the same flow differently.

Mitigation: router skills, playbooks, deterministic commands, single guidance
rules engine.

### Stale Plan Execution

Risk: an agent executes a Ready item based on old repo reality.

Mitigation: planning council pre-execution validation, APS drift checks,
decision-log checks.

### Missing Observability

Risk: no one can reconstruct why an agent acted.

Mitigation: event envelope, session records, evidence links, release records.

### Deterministic/Probabilistic Confusion

Risk: agent judgement is treated as validation proof.

Mitigation: evidence model distinguishes review findings from deterministic
checks.

### Human Override Abuse

Risk: humans bypass process without trace.

Mitigation: explicit override events with reason, expiry, and accepted risk.

### Skill-To-Repo Drift

Risk: skills reference commands or files that do not exist.

Mitigation: skill manifest validation and CI check for referenced scripts,
playbooks, schemas, and command paths.

## Minimum Viable Agentic Architecture

If radically simplified, keep only these concepts:

```text
APS item
  -> guidance script
  -> playbook
  -> agent or deterministic command
  -> evidence
  -> event/session record
  -> PR/release artefact
```

Minimum required artefacts:

- APS work item metadata
- `scripts/agent/guidance.sh`
- skill/playbook folder convention
- agent capability schema
- council/pre-PR review session schema
- deterministic hook doctrine
- release record schema
- event envelope schema

Minimum rules:

- APS is execution authority.
- Hooks are deterministic guardrails only.
- Skills route to playbooks.
- Agents produce structured outputs.
- Scripts/CI own deterministic execution.
- Sessions/events own continuity.
- PRs and releases are publication artefacts, not primary memory.

## Recommended Next Artefacts

Create these follow-on artefacts before implementing broad changes:

1. `plans/specs/YYYY-MM-DD-agentic-execution-taxonomy.md`
2. `plans/specs/YYYY-MM-DD-skill-manifest-and-folder-standard.md`
3. `plans/specs/YYYY-MM-DD-hook-semantics-and-guidance.md`
4. `plans/specs/YYYY-MM-DD-agent-capability-schema.md`
5. `plans/specs/YYYY-MM-DD-workflow-session-and-event-schema.md`
6. `plans/specs/YYYY-MM-DD-aps-execution-metadata.md`
7. `plans/specs/YYYY-MM-DD-release-skill-playbook-refactor.md`
8. `plans/specs/YYYY-MM-DD-as-built-docs-agent-integration.md`

The as-built documentation integration should wait for the dedicated docs
session, but the architecture should reserve a hook point now.
