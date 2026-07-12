<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Agent Instruction Contract

| ID    | Owner | Status      | Progress |
| ----- | ----- | ----------- | -------- |
| AICON | —     | Complete        | 5/5      |

2026-07-13: all Merged items confirmed in the v0.9.0-beta tag (record:
plans/releases/v0.9.0-beta.md) and advanced to Released/Shipped; module
ready to archive per the archive cascade.

## Purpose

Shrink the root `AGENTS.md` surface into a lean shared agent contract while
routing operational detail to the documents that already own it. The root file
owns universal behaviour; guides own procedures; APS owns work state; ADRs own
durable decisions; tool-specific files own runtime adapters.

## Problem

`AGENTS.md` had the right role, but it duplicated APS lifecycle detail,
documentation closeout, repository operations, agent-surface inventory, and test
infrastructure guidance that have stronger homes. That increased startup context
and made authority drift more likely.

## In Scope

- Reduce root `AGENTS.md` to universal behaviour, routing, invariants, and
  validation pointers.
- Preserve the `AGENTS.md` and `CONTEXT.md` split: behaviour contract versus
  repository orientation map.
- Keep Claude, OpenCode, and Codex-specific behaviour in their own config or
  skill surfaces.
- Move or link operational detail to existing authoritative guides when a guide
  exists.
- Create a small repository-operations guide if no current guide owns `gx` and
  local setup expectations.

## Out of Scope

- Changing APS lifecycle semantics or Worktrunk branch policy.
- Rewriting `CLAUDE.md`, `.opencode/`, or `.codex/` beyond cross-link fixes that
  keep them aligned with the lean root contract.
- Changing validation commands, test runners, Council tiers, or release policy.
- Automating agent-surface drift detection.

## Interfaces

**Depends on:**

- `AGENTS.md` — current shared agent behaviour contract.
- `CONTEXT.md` — repository orientation map.
- `plans/project-context.md` — anvil-specific APS, workflow, release, feature
  flag, validation, and repository operations context.
- `docs/guides/documentation-governance.md` — documentation authority and
  closeout protocol.
- `docs/guides/agent-surface-inventory.md` — authoritative skill, agent, and
  command inventory.
- `docs/guides/testing.md` — testing conventions and command surface.
- Tool-specific config surfaces: `CLAUDE.md`, `.opencode/`, `.codex/`.

**Exposes:**

- A shorter root agent contract that all agent runtimes can load cheaply.
- Clear authority routing for procedures that should not live in `AGENTS.md`.
- Focused follow-up work items for maintaining guide surfaces that absorb moved
  detail.

## Work Items

### AICON-001: Lean root agent contract

- **Status:** Done 2026-07-07 on `docs/aicon-001-agent-contract`.
- **Intent:** Make `AGENTS.md` a compact universal contract instead of a detailed
  procedure manual.
- **Expected Outcome:** `AGENTS.md` retains must-read files, operating rules, APS
  workflow routing, architecture/scope guards, documentation-change routing,
  validation pointers, and links to agent-surface inventory. Detailed APS,
  documentation, test, Council, release, and repository-operation procedures are
  removed or replaced with links to their authoritative homes.
- **Scope:** `AGENTS.md`.
- **Non-scope:** changing workflow semantics or weakening mandatory gates.
- **Files:**
  - `AGENTS.md`
- **Validation:**
  - `pnpm run format:check`
  - `pnpm run docs:check`
  - `pnpm run aps:active-lint`
  - `pnpm run aps:index:check`
- **Dependencies:** none
- **Confidence:** high

### AICON-002: Rehome testing detail

- **Status:** Done 2026-07-07 on `docs/aicon-001-agent-contract`.
- **Intent:** Ensure detailed test infrastructure guidance has one guide-owned
  home outside the root agent contract.
- **Expected Outcome:** `docs/guides/testing.md` owns the stack table, local test
  commands, E2E conventions, coverage notes, and OPA/Regal testing notes that
  agents previously got from `AGENTS.md`. `AGENTS.md` links to the guide instead
  of carrying the full catalogue.
- **Scope:** testing documentation and cross-links.
- **Non-scope:** changing tests, CI jobs, coverage thresholds, or runner config.
- **Files:**
  - `docs/guides/testing.md`
  - `docs/guides/README.md`
  - `AGENTS.md`
- **Validation:**
  - `pnpm run docs:check`
  - `pnpm run format:check`
- **Dependencies:** none
- **Confidence:** high
- **Closeout:** `docs/guides/testing.md` now owns the stack table, command
  selection, E2E harness, coverage, and OPA/Regal notes; `AGENTS.md` and
  `docs/guides/README.md` route to the guide instead of duplicating the
  catalogue.

### AICON-003: Create repository-operations guide

- **Status:** Done 2026-07-07 on `docs/aicon-001-agent-contract`.
- **Intent:** Move `gx` and local repository-management expectations out of the
  always-loaded agent contract.
- **Expected Outcome:** A guide owns repository setup and management conventions,
  including `gx` usage and clone-location expectations. `AGENTS.md` and any
  relevant guide index link to it without duplicating command tables.
- **Scope:** local repository-operation guidance.
- **Non-scope:** changing `gx`, Worktrunk, branch policy, or developer shell
  setup.
- **Files:**
  - `docs/guides/repository-operations.md`
  - `docs/guides/README.md`
  - `AGENTS.md`
- **Validation:**
  - `pnpm run docs:check`
  - `pnpm run format:check`
- **Dependencies:** none
- **Confidence:** high
- **Closeout:** Added `docs/guides/repository-operations.md` as the authoritative
  home for `gx`, local setup expectations, and Worktrunk boundary links; root
  and guide-index routing now point to it.

### AICON-004: Reconcile authority links

- **Status:** Done 2026-07-07 on `docs/aicon-001-agent-contract`.
- **Intent:** Make the destination documents explicitly own the detail that root
  `AGENTS.md` routes away from.
- **Expected Outcome:** `plans/project-context.md`,
  `docs/guides/documentation-governance.md`, and
  `docs/guides/agent-surface-inventory.md` have current upstream/downstream
  metadata and prose that match the lean root contract. Any stale ownership
  references to archived `DOCGOV` are updated or explained by active follow-up
  ownership.
- **Scope:** authority metadata and cross-links for agent instruction surfaces.
- **Non-scope:** broad doc taxonomy rewrites or new ADRs.
- **Files:**
  - `plans/project-context.md`
  - `docs/guides/documentation-governance.md`
  - `docs/guides/agent-surface-inventory.md`
  - `AGENTS.md`
- **Validation:**
  - `pnpm run docs:check`
  - `pnpm run aps:active-lint`
  - `pnpm run aps:index:check`
- **Dependencies:** AICON-002, AICON-003
- **Confidence:** medium
- **Closeout:** Refreshed authority metadata and prose in
  `plans/project-context.md`, `docs/guides/documentation-governance.md`, and
  `docs/guides/agent-surface-inventory.md` so each destination document names
  its current upstreams/downstreams and explains archived `DOCGOV` inheritance
  where relevant.

### AICON-005: Verify tool-specific adapters stay thin

- **Status:** Done 2026-07-07 on `docs/aicon-001-agent-contract`.
- **Intent:** Confirm runtime-specific instruction surfaces remain adapters over
  the shared root contract instead of duplicating generic procedure text.
- **Expected Outcome:** `CLAUDE.md`, `.opencode/skills/dev-workflow/SKILL.md`,
  and `.codex/skills/dev-workflow/SKILL.md` still route to the shared contract
  and keep only runtime-specific hooks, commands, permissions, or workflow
  details. Required cross-link fixes are made where drift is found.
- **Scope:** tool-specific adapter cross-link review.
- **Non-scope:** changing skill semantics, command implementations, or runtime
  permissions.
- **Files:**
  - `CLAUDE.md`
  - `.claude/skills/dev-workflow/SKILL.md`
  - `.opencode/skills/dev-workflow/SKILL.md`
  - `.codex/skills/dev-workflow/SKILL.md`
  - `docs/guides/agent-surface-inventory.md`
- **Validation:**
  - `pnpm run docs:check`
  - `pnpm run format:check`
- **Dependencies:** AICON-001
- **Confidence:** medium
- **Closeout:** Reviewed `CLAUDE.md`, `.claude/skills/dev-workflow/SKILL.md`,
  `.opencode/skills/dev-workflow/SKILL.md`, and
  `.codex/skills/dev-workflow/SKILL.md`; added explicit routing back to
  `AGENTS.md` and `docs/guides/agent-surface-inventory.md` where adapter
  thinness was implicit.

## Success Criteria

- Root `AGENTS.md` is concise enough to serve as an always-loaded contract.
- No procedure table is duplicated between `AGENTS.md` and its authoritative
  guide.
- Documentation governance closeout points to one authority.
- Agent-surface inventory remains the single inventory for skills, agents, and
  commands.
- APS validation and docs validation pass for the affected files.

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Root contract becomes too thin and drops mandatory gates | High | Keep invariants in `AGENTS.md`; move only procedure detail |
| Destination guides are stale | Medium | AICON-002..004 refresh guides after the root rewrite |
| Tool-specific adapters duplicate old root content | Medium | AICON-005 verifies adapter thinness after the root rewrite |
| Index or metadata drift after adding the module | Low | Run APS and docs validation during implementation |
