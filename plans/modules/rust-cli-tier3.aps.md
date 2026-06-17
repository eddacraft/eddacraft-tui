<!--
APS Module: Rust CLI Tier 3
=========================
Port Tier 3 (subsystem & specialised) commands to crates/anvil-cli/.
Depends on RCLI (Tier 1) and RCLI2 (Tier 2) foundation.

Scopes: RCLI3 (main)
-->

# Rust CLI — Tier 3

| ID    | Owner | Status      | Progress |
| ----- | ----- | ----------- | -------- |
| RCLI3 | —     | In Progress | 5/20     |

**Last reviewed:** 2026-05-17

> **Post-migration note (2026-04-26):** RCLI Tier 1 is complete (64/64) and
> the Node.js CLI at `apps/anvil-cli/` has been retired (RCLI-023 archival
> already happened for the core surface). References to the Node.js CLI in
> this module describe the historical contract we are reaching parity with,
> not a still-present runtime. The original `edda.aps.md`, `ember.aps.md`,
> `explain-command.aps.md` and `release-management.aps.md` modules are now in
> `plans/archive/modules/` — RCLI3 is the surviving Rust port of those
> command surfaces.

## Purpose

Port Tier 3 subsystem and specialised commands from the historical Node.js
CLI (`apps/anvil-cli/`, retired) to the Rust binary (`crates/anvil-cli/`).
These commands expose the Edda Stack (memory/proposals), APS planning, agent
governance, and operational utilities. Completing Tier 3 closes the remaining
parity gap from the historical Node.js CLI and unblocks single-binary
distribution (RCLI-024).

**Why:** Tier 3 contains the domain subsystem commands (Edda, Ember, Plan,
Agent) that power day-to-day governance workflows. Without them the Rust CLI
does not yet match the historical Node.js CLI for users who interact with
canonical memories, proposals, or APS plans. Distribution (RCLI-024) is
blocked until all three tiers reach parity.

**ADR:** [012-rust-cli-replacement](../decisions/012-rust-cli-replacement.md)
**Spec:** [2026-03-18-rust-cli-design](../specs/2026-03-18-rust-cli-design.md) §6 Tier 3

## Language Guardrails

This module broadens the `anvil` CLI beyond quality commands into governance,
memory, and workflow surfaces. That makes language consistency more important,
not less. Follow the canonical model in
`plans/specs/2026-04-21-anvil-quality-language-design.md`.

- Quality-facing commands should keep the checks -> findings -> gate model
- Governance and plan commands should avoid reusing `gate`, `warning`,
  `violation`, or `finding` unless they truly participate in that model
- `validate` and `status` must be scoped clearly in help and docs so users know
  what is being validated or whose status is being shown
- `graph` and `boundary` should be used deliberately as explanatory terms, not
  casual synonyms for any internal structure

## In Scope

- 10 commands (or command groups) classified as Tier 3 in the design spec
- Rust FFI or native reimplementation for Edda (YAML/Git) and Ember (SQLite)
  storage backends
- JSON and plain-text output modes for all commands
- Compatibility with existing `.anvil/` storage formats (edda YAML, ember.db,
  agent state, plan lock files)

## Out of Scope

- New Edda/Ember features beyond current Node.js CLI parity
- MCP server migration (stays Node.js per RCLI decision)
- Dashboard/website changes
- Edda Stack service layer rewrite (port the CLI surface, call existing
  services via workspace crate or re-implement minimal subset)

## Interfaces

**Depends on:**

- RCLI — Foundation crate structure, clap entry point, output formatters,
  auth middleware
- RCLI2 — Tier 2 must be complete (or in progress) before cutover
- `packages/edda-stack/` — Domain contracts (MemoryType, ProposalStatus,
  confidence levels, provenance schema). These TypeScript contracts still
  exist in-tree as of 2026-04-26 and define the storage format; the Rust
  port must read/write the same structures. If edda-stack is retired before
  RCLI3 lands, the Rust port becomes the canonical contract.
- `packages/aps/` — APS plan loading, filtering, task locking interfaces
  (TypeScript; still in-tree as of 2026-04-26)

**Exposes:**

- 10 additional subcommands (with ~25 total sub-subcommands) on the `anvil`
  binary
- Once all three tiers are complete, unblocks RCLI-024 (distribution).
  RCLI-023 (Node.js archival) already executed for the core surface; Tier 3
  parity is what closes the remaining historical gap.

## Constraints

- Storage format parity: Rust commands must read `.anvil/edda/` YAML,
  `.anvil/ember.db` SQLite, `.anvil/agents/`, and `.anvil/executions/`
  identically to the historical Node.js CLI
- Edda YAML files include provenance metadata — the Rust YAML serialiser must
  preserve field ordering and comments where possible
- Ember SQLite access via `rusqlite`; schema must match the historical
  `better-sqlite3` schema exactly
- Agent state files use JSON; no special handling needed
- Same error handling conventions as RCLI: `anyhow` + `thiserror`
- New Tier 3 command copy should distinguish quality workflow concepts from APS,
  memory, agent, and release workflows so the expanded binary still feels
  coherent to users

## Ready Checklist

Change status to **Ready** when:

- [x] RCLI Tier 1 Phases 1–7 complete (including parity rework items)
      — `rust-cli` archived 64/64
- [x] RCLI2 Tier 2 Phases 1–2 complete (non-OPAE items) — RCLI2-001..-004
      shipped 2026-04-26; RCLI2-009 complete; -005..-008 remain OPAE-gated
      and are not Phase 1–2 blockers
- [x] Edda YAML schema documented (or inferable from edda-stack contracts)
      — see `packages/edda-stack/src/contracts/edda-memory.ts`,
      `memory-types.ts`, `provenance.ts`, `evolution.ts`
- [x] Ember SQLite schema documented (or inferable from ProposalStore)
      — see `packages/edda-stack/src/contracts/ember-proposal.ts` and
      `packages/edda-stack/src/contracts/ports/ember.port.ts`

**Readiness audit:** 2026-05-17 — all four gates pass; Phase 1–4
Proposed items with completed dependencies have been promoted to
Ready (see status lines below).

---

## Work Items

#### Phase 1 — Edda & Ember (Core Domain)

The highest-priority commands. These are the most-used subsystem commands and
the primary blocker for replacing the Node.js CLI in daily workflows.

### RCLI3-001: edda list command

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
  `b47360d0`. Ported the historical Node.js `anvil edda list` to Rust
  in `crates/anvil-cli/src/commands/edda.rs` with `--type`/`--status`/
  `--confidence`/`--since`/`--limit`/`--json` filters, matching the
  legacy JSON envelope shape (`storage_found`, `total`, `limit`,
  `has_more`, `filters`, `memories`). 5 integration tests +
  5 unit tests cover the storage-missing envelope, default-active
  filter, CSV `--type`, pagination, table headers, and parser edges.
  Cleanup agent will advance to Released/Shipped once a tagged
  release records the commit.)
- **Intent:** Port `anvil edda list`. Query active, superseded, and retired
  memories with multi-criteria filtering: `--type` (observation, decision,
  convention, constraint), `--status` (active, superseded, retired),
  `--min-confidence` (high, medium, low), `--since <duration>` (e.g., 7d, 2w),
  `--limit`, `--json`. Reads from `.anvil/edda/` YAML store
- **Expected Outcome:** `anvil edda list --status active --min-confidence
  medium` shows filtered memories with ID, type, confidence, age, and
  truncated statement. JSON mode returns full memory objects
- **Validation:** Row count and content match the historical Node.js CLI contract for same store
  state; time-ago formatting consistent
- **Files:** `crates/anvil-cli/src/commands/edda.rs`
- **Confidence:** medium (263 LOC in historical Node.js; YAML parsing + multi-filter
  logic)
- **Priority:** High
- **Dependencies:** RCLI (foundation)

---

### RCLI3-002: edda show command

- **Status:** Done (2026-05-26 — `anvil edda show <id>` landed in
  `crates/anvil-cli/src/commands/edda.rs`; targeted CLI tests pass)
- **Intent:** Port `anvil edda show <id>`. Display full memory details:
  statement, context, type, confidence, provenance (ember source, kindling
  lineage, session), attribution, tags, timestamps, evolution chain
  (supersedes/superseded-by links)
- **Expected Outcome:** `anvil edda show edda-abc123` renders formatted memory
  with all metadata sections
- **Validation:** Output includes all fields present in Node.js CLI; provenance
  chain is correctly resolved
- **Files:** `crates/anvil-cli/src/commands/edda.rs`
- **Confidence:** high (121 LOC in historical Node.js; display-only)
- **Priority:** High
- **Dependencies:** RCLI3-001 (shared YAML loading)

---

### RCLI3-003: edda promote command

- **Status:** Proposed
- **Intent:** Port `anvil edda promote <ember-id>`. Promote an Ember candidate
  to canonical Edda memory. Requires: `--type`, `--confidence`, `--reason`,
  optional `--context`, `--tags`. Creates new memory YAML file with provenance
  linking back to source proposal
- **Expected Outcome:** `anvil edda promote emb-xyz --type decision
  --confidence high --reason "Validated by team"` creates a new Edda memory
  and marks the Ember proposal as promoted
- **Validation:** Created YAML matches schema; Ember proposal status updated
  to promoted; provenance chain intact
- **Files:** `crates/anvil-cli/src/commands/edda.rs`
- **Confidence:** medium (168 LOC in historical Node.js; cross-store write:
  Edda YAML + Ember SQLite)
- **Priority:** High
- **Dependencies:** RCLI3-001, RCLI3-005 (Ember store access)

---

### RCLI3-004: edda retire and trace commands

- **Status:** Proposed
- **Intent:** Port `anvil edda retire <id> --reason "..."` and `anvil edda
  trace <id>`. Retire marks a memory as retired with rationale and timestamp.
  Trace follows the evolution chain: supersession links, ember source,
  kindling lineage, session provenance
- **Expected Outcome:** `anvil edda retire edda-abc --reason "Superseded by
  ADR-015"` updates memory status. `anvil edda trace edda-abc` shows full
  lineage tree
- **Validation:** Retired memory status persists across reads; trace output
  matches Node.js CLI chain resolution
- **Files:** `crates/anvil-cli/src/commands/edda.rs`
- **Confidence:** medium (241 LOC combined in Node.js)
- **Priority:** Medium
- **Dependencies:** RCLI3-001

---

### RCLI3-005: ember list command

- **Status:** In Progress (2026-06-17 — `feat/rcli3-005-ember-list`; TDD port
  against the real `ProposalStore` SQLite schema)
- **Intent:** Port `anvil ember list`. Query proposals by type (`decision`,
  `pattern`, `warning`, `lesson`, `anomaly`, `constraint`) and status
  (`active`, `promoted`, `expired`, `dismissed`) with filtering and expiry
  display. Reads from `.anvil/ember.db` SQLite database. (Type/status enums
  corrected 2026-06-17 from the historical `ProposalStore` schema in
  `packages/edda-stack/src/ember/proposal-store.ts`; the earlier draft list
  — `observation`/`suggestion` types, `rejected` status — predated the schema.)
- **Expected Outcome:** `anvil ember list --status active` shows proposals
  with ID, type, status, confidence, summary, created, expires. JSON mode
  returns the full proposal objects in the historical envelope
  (`database_found`, `database_path`, `total`, `limit`, `has_more`, `filters`,
  `proposals`)
- **Validation:** Row count and content match the historical Node.js CLI contract; expiry time
  formatting consistent
- **Files:** `crates/anvil-cli/src/commands/ember.rs`
- **Confidence:** medium (198 LOC in historical Node.js; SQLite via rusqlite)
- **Priority:** High
- **Dependencies:** RCLI (foundation)

---

### RCLI3-006: ember show command

- **Status:** Proposed
- **Intent:** Port `anvil ember show <id>`. Display full proposal: summary,
  rationale, observation IDs, session IDs, confidence, resolution status,
  timestamps, metadata
- **Expected Outcome:** `anvil ember show emb-xyz` renders formatted proposal
  with all sections
- **Validation:** Output fields match the historical Node.js CLI contract
- **Files:** `crates/anvil-cli/src/commands/ember.rs`
- **Confidence:** high (116 LOC in historical Node.js; display-only)
- **Priority:** High
- **Dependencies:** RCLI3-005 (shared SQLite access)

---

### RCLI3-007: ember promote command

- **Status:** Proposed
- **Intent:** Port `anvil ember promote <id>`. Mark an active proposal as
  promoted; updates resolution record with actor, reason, and timestamp.
  Only active proposals can be promoted (status guard)
- **Expected Outcome:** `anvil ember promote emb-xyz` updates proposal status
  to promoted; confirmation message shown
- **Validation:** Status transition persists in SQLite; re-promoting a
  non-active proposal returns error
- **Files:** `crates/anvil-cli/src/commands/ember.rs`
- **Confidence:** high (104 LOC in historical Node.js; single UPDATE)
- **Priority:** Medium
- **Dependencies:** RCLI3-005

---

#### Phase 2 — Plan & Stack (Workflow)

### RCLI3-008: plan validate command

- **Status:** Ready (2026-05-17 — readiness audit promoted; depends only on
  RCLI foundation, which is complete)
- **Intent:** Port `anvil plan validate <path>`. Validate APS markdown
  structure and rules; output issues by severity with line numbers and
  context. Delegates to APS validation logic
- **Language Note:** `validate` here is APS document validation, not a quality
  check or gate. Help and docs should make that scope explicit
- **Expected Outcome:** `anvil plan validate plans/modules/foo.aps.md`
  reports structural issues (missing fields, invalid status transitions,
  broken references)
- **Validation:** Issue list matches Node.js CLI for same plan file
- **Files:** `crates/anvil-cli/src/commands/plan.rs`
- **Confidence:** medium (105 LOC in historical Node.js; needs APS parser in Rust)
- **Priority:** High
- **Dependencies:** RCLI (foundation)

---

### RCLI3-009: plan load and status commands

- **Status:** Proposed
- **Intent:** Port `anvil plan load` (filter plans by scope, module, task,
  owner, tag, priority, confidence; three output modes: JSON, text,
  files-only) and `anvil plan status` (show task states: open, locked, completed,
  cancelled with grouping and filtering)
- **Language Note:** `status` here is workflow/task status, not gate status.
  Keep the distinction explicit in command help and docs
- **Expected Outcome:** `anvil plan load --scope RCLI --priority high` filters
  and displays matching work items. `anvil plan status --task RCLI-001` shows
  single task detail
- **Validation:** Filter results and task state counts match the historical Node.js CLI contract
- **Files:** `crates/anvil-cli/src/commands/plan.rs`
- **Confidence:** medium (465 LOC combined in historical Node.js; complex filter logic)
- **Priority:** High
- **Dependencies:** RCLI3-008 (shared APS parser)

---

### RCLI3-010: plan lock and unlock commands

- **Status:** Proposed
- **Intent:** Port `anvil plan lock <task-id>` and `anvil plan unlock
  <task-id>`. Lock creates an execution plan file at `.anvil/executions/`
  with git provenance (branch, commit, author). Unlock cancels a locked task
- **Expected Outcome:** `anvil plan lock RCLI-001` creates lock file and
  prints execution plan path. `anvil plan unlock RCLI-001` removes lock
- **Validation:** Lock file format matches Node.js CLI; double-lock returns
  error; unlock of unlocked task returns error
- **Files:** `crates/anvil-cli/src/commands/plan.rs`
- **Confidence:** high (95 LOC in historical Node.js; filesystem operations)
- **Priority:** Medium
- **Dependencies:** RCLI3-008

---

### RCLI3-011: stack command

- **Status:** Proposed
- **Intent:** Port `anvil stack status` and `anvil stack validate`. Stack
  status shows health of the three-layer architecture (Kindling → Ember →
  Edda). Stack validate checks configuration consistency across layers
- **Language Note:** `status` and `validate` here describe stack health and
  configuration consistency. Avoid borrowing gate/result wording from the
  quality workflow unless the stack surface actually runs checks and emits
  findings
- **Expected Outcome:** `anvil stack status` shows layer health, store sizes,
  and last-activity timestamps. `anvil stack validate` reports configuration
  issues
- **Validation:** Status output matches Node.js CLI for same store state
- **Files:** `crates/anvil-cli/src/commands/stack.rs`
- **Confidence:** high (33 LOC in historical Node.js; delegates to services)
- **Priority:** Low
- **Dependencies:** RCLI3-001 (Edda store), RCLI3-005 (Ember store)

---

#### Phase 3 — Agent & Authorship (Operational)

### RCLI3-012: agent status and list commands

- **Status:** Ready (2026-05-17 — readiness audit promoted; depends only on
  RCLI foundation, which is complete)
- **Intent:** Port `anvil agent status` (current agent identity, registration
  state, heartbeat) and `anvil agent list` (all registered agents with state
  filtering: active, idle, stale, terminated). Includes agent type icons
  (claude, cursor, copilot, aider, continue, codeium, human, ci) and
  session ID masking
- **Language Note:** agent `status` is runtime state, not workflow judgement;
  keep it clearly separate from gate or check terminology
- **Expected Outcome:** `anvil agent list --state active` shows running agents
  with time-since-heartbeat. `anvil agent status` shows current agent
- **Validation:** Agent list matches Node.js CLI; time-ago colour coding
  (green <5m, yellow <1h, red ≥1d) preserved
- **Files:** `crates/anvil-cli/src/commands/agent.rs`
- **Confidence:** medium (296 LOC combined in historical Node.js; state file parsing)
- **Priority:** Medium
- **Dependencies:** RCLI (foundation)

---

### RCLI3-013: agent cleanup command

- **Status:** Proposed
- **Intent:** Port `anvil agent cleanup [--dry-run]`. Mark stale agents
  (>30s since heartbeat), remove expired locks, purge timed-out queue
  entries. Dry-run mode shows what would be cleaned without modifying state
- **Expected Outcome:** `anvil agent cleanup --dry-run` lists stale agents
  and expired locks. Without `--dry-run`, removes them and reports counts
- **Validation:** Cleanup results match the historical Node.js CLI contract; dry-run produces no
  side effects
- **Files:** `crates/anvil-cli/src/commands/agent.rs`
- **Confidence:** high (140 LOC in historical Node.js; JSON file operations)
- **Priority:** Low
- **Dependencies:** RCLI3-012

---

### RCLI3-014: authorship command

- **Status:** Ready (2026-05-17 — readiness audit promoted; depends only on
  RCLI foundation, which is complete)
- **Intent:** Port `anvil authorship` with three subcommands: `show [commit]`
  (display AI authorship for a commit from Git Notes refs/notes/ai), `list`
  (recent N commits with AI authorship), `stats [range]` (coverage %, line
  changes, tools used for revision range). Reads Git AI Standard v3.0.0
  notes format
- **Expected Outcome:** `anvil authorship stats HEAD~10..HEAD` shows AI
  contribution percentage, tools breakdown, and line statistics
- **Validation:** Coverage % and tool counts match the historical Node.js CLI contract for same
  git history
- **Files:** `crates/anvil-cli/src/commands/authorship.rs`
- **Confidence:** medium (234 LOC in historical Node.js; git notes parsing via
  `git2` or shell)
- **Priority:** Low
- **Dependencies:** RCLI (foundation)

---

#### Phase 4 — Utility Commands

### RCLI3-015: explain command

- **Status:** Ready (2026-05-17 — readiness audit promoted; depends only on
  RCLI foundation, which is complete)
- **Intent:** Port `anvil explain <warning-id>`. Bidirectional lookup: parse
  finding ID (for example a warning or violation identifier) to find
  explanation, or `--rules` to list all explainable rules, or `--list` to show recent findings from last
  check run. Renders explanation sections: title, whyItMatters, howToAddress,
  whenToSuppress, related links
- **Language Note:** prefer `finding` as the generic noun in help and docs,
  while still supporting warning/violation-specific identifiers
- **Expected Outcome:** `anvil explain AP-001` shows formatted explanation.
  `anvil explain --rules` lists all rules grouped by prefix
- **Validation:** Explanation content matches Node.js CLI; rule grouping
  (AP/ARCH/BOUND prefixes) is identical
- **Files:** `crates/anvil-cli/src/commands/explain.rs`
- **Confidence:** medium (230 LOC in historical Node.js; requires explanation catalogue
  ported or embedded)
- **Priority:** Medium
- **Dependencies:** RCLI (foundation)

---

### RCLI3-016: mcp-config command 🔒 PULLED FORWARD TO A1 (current release)

- **Status:** Complete (2026-04-26 — landed in
  `crates/anvil-cli/src/commands/mcp_config.rs`; pulled forward because A1
  RTAI Spike Slice runbook needs the install step. See
  [`RELEASE-PLAN.md`](../../RELEASE-PLAN.md) prerequisites.)
- **Intent:** Port `anvil mcp-config`. Generate MCP server configuration for
  AI editors (claude-code, cursor, windsurf, vscode). Supports `--target`,
  `--transport` (stdio/http), `--port`, `--write`. Handles symlink-safe path
  resolution and escape detection (confirms if writing outside workspace)
- **Expected Outcome:** `anvil mcp-config --target claude-code --write`
  creates `.claude/mcp.json` with correct server entry. Each target generates
  its editor-specific config format. `anvil mcp-config --target claude-code
  --verify` confirms current installation state without writing — prints the
  resolved client config path, the entry already present (if any), and whether
  the file parses cleanly; exits non-zero if the entry is missing or malformed
- **Validation:** Generated configs are valid for each editor; VSCode format
  (type field) differs from others (command/args) — both correct
- **Files:** `crates/anvil-cli/src/commands/mcp_config.rs`
- **Confidence:** high (176 LOC in historical Node.js; template generation with path
  safety)
- **Priority:** **High** (was Low — promoted with pull-forward)
- **Dependencies:** RCLI (foundation), RMCP-002 (the generated stdio config
  points at `anvil mcp serve --stdio`; RMCP owns making that command real for
  the A1 launch path)

---

### RCLI3-016b: mcp install wrapper command 🔒 PULLED FORWARD TO A1 (current release)

- **Status:** Complete (shipped 2026-04-28 under RMCP-007 — commit
  `79da411d feat(rmcp): add mcp install wrapper`. The wrapper landed in
  `crates/anvil-cli/src/commands/mcp.rs` with `--client cursor|claude-code`,
  `--verify`, `--command` override, and `--workspace` override; idempotency
  and drift-rewrite are covered by the integration suite at
  `crates/anvil-cli/tests/mcp_config.rs` (`mcp_install_*`). RMCP-007 also
  closed the runbook follow-up gaps #1194 (`--command` override + `--verify`
  semantics) and #1195 (Claude Code client config path).)
- **Intent:** Provide a single-command MCP install / wire-up wrapper around
  `anvil mcp-config`. `anvil mcp install --client <cursor|claude-code>`
  detects the client config path, generates the correct config entry, writes
  it, and prints the next-step "restart your editor" hint. Idempotent on
  re-run. Mirrors the runbook's "one command, then restart" UX promise.
- **Expected Outcome:** `anvil mcp install --client cursor` resolves
  `~/.cursor/mcp.json` (or platform equivalent), writes the `anvil` MCP server
  entry, and prints the operator-visible "Detected client / Installing /
  Restart" lines the demo runbook §1.4 expects. `anvil mcp install --client
  claude-code` does the same against Claude Code's config path. Re-running is
  a no-op if the entry already matches; re-running with a drifted entry
  rewrites and warns. Exits non-zero only on hard failure (no client config
  path resolvable, write refused, etc.)
- **Validation:** Generated config matches RCLI3-016 output for the same
  `--target`; `anvil mcp install --client cursor && anvil mcp-config --target
  cursor --verify` exits zero on a fresh install; idempotent re-run leaves
  the file byte-identical
- **Files:** `crates/anvil-cli/src/commands/mcp.rs` (new),
  `crates/anvil-cli/src/commands/mcp_config.rs` (shared resolver)
- **Confidence:** high (thin wrapper over RCLI3-016 — config resolution and
  write are already implemented there)
- **Priority:** **High** (A1 demo runbook prerequisite)
- **Dependencies:** RCLI3-016 (provides the underlying config writer and
  client-path resolution), RMCP-002 (generated entry must point at a working
  Rust stdio MCP server)

---

### RCLI3-017: release command

- **Status:** Ready (2026-05-17 — readiness audit promoted; depends only on
  RCLI foundation, which is complete. Note: RELORCH is also closed, so the
  underlying deterministic release command surface this CLI wraps is
  stable.)
- **Intent:** Port `anvil release`. Interactive release workflow orchestration
  with profile selection (beta, stable, hotfix). Supports `--target <version>`
  (skip prompt), `--execute` (non-dry-run), `--resume` (from saved state),
  `--skip-preflight`
- **Expected Outcome:** `anvil release --profile beta --execute` runs the
  beta release pipeline (version bump, changelog, tag, publish)
- **Validation:** Release artefacts match the historical Node.js CLI contract workflow; state
  resumption works after interruption
- **Files:** `crates/anvil-cli/src/commands/release.rs`
- **Confidence:** medium (41 LOC wrapper + ~400 LOC service; release runner
  logic needs porting)
- **Priority:** Low
- **Dependencies:** RCLI (foundation)

---

### RCLI3-017b: intercept unblock CLI surface

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
  `cb642908`. Landed the per-fence unblock IPC verb wrapping
  `FenceStore::unblock_worktree`, the `--worktree` / `--all` /
  `--dry-run` CLI surface, and proto `IpcCommand::UnblockWorktree`.
  Cascade clearing via `--acknowledge-cascade` continues through the
  MLP2-026 path. Cleanup agent will advance to Released/Shipped once
  release evidence records this in a tagged build.)
- **Intent:** Port `anvil intercept unblock --worktree <path>`. CLI surface
  for removing a fenced worktree from the daemon's persistence file, so an
  operator can clear demo / test fences without restarting the daemon.
  Wraps the IPC command INTD-007 / INTD-011 expose.
- **Expected Outcome:** `anvil intercept unblock --worktree "$PWD"` sends
  the unblock IPC command for the resolved worktree path, removes the
  matching entry from the disk-persisted fence file, and prints a
  confirmation line. `anvil intercept status` immediately reflects the
  cleared fence (`fences: 0` for that worktree). Unblocking a worktree that
  is not currently fenced is a no-op and exits zero (idempotent), with an
  informational note. Optional `--all` flag clears every fence in one call;
  `--dry-run` lists what would be cleared without modifying state.
- **Validation:** Integration test against a daemon with a seeded fence
  asserts the fence is removed from both in-memory state and disk
  persistence; idempotent re-run leaves the persistence file byte-identical;
  `--dry-run` produces no side effects
- **Files:** `crates/anvil-cli/src/commands/intercept.rs` (extend existing
  surface)
- **Confidence:** high (thin CLI wrapper over the IPC command INTD-007
  already persists; daemon side does the work)
- **Priority:** **Medium** (A1 demo runbook §3.1 prerequisite, but the
  hard-reset path in §3.2 also clears fences so the demo is recoverable
  without this CLI in the worst case)
- **Dependencies:** INTD-007 (fence persistence + unblock primitive),
  INTD-011 (daemon status / IPC surface for fence query)

---

### RCLI3-018: beta command

- **Status:** Ready (2026-05-17 — readiness audit promoted; RCLI-015
  auth/HTTP infrastructure is Complete in archived `rust-cli.aps.md`,
  so no remaining open dependency)
- **Intent:** Port `anvil beta invite` and `anvil beta revoke`. Admin-only
  commands (require `ANVIL_ADMIN_KEY`) for beta access management. Invite
  creates a one-time token with email, TTL, optional name/notes. Revoke
  removes all tokens for a user
- **Expected Outcome:** `anvil beta invite --email user@example.com --ttl 30`
  returns a one-time access token. `anvil beta revoke --email user@example.com`
  removes access
- **Validation:** API calls succeed against staging; email validation via
  regex; token shown only once
- **Files:** `crates/anvil-cli/src/commands/beta.rs`
- **Confidence:** high (80 LOC in historical Node.js; HTTP calls via reqwest)
- **Priority:** Low
- **Dependencies:** RCLI-015 (auth/HTTP infrastructure)

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Edda YAML format divergence (Rust serde_yaml vs historical Node.js yaml) | Medium | High | Write format round-trip tests against historical fixtures; validate field ordering preservation |
| Ember SQLite schema mismatch (rusqlite vs historical better-sqlite3) | Low | High | Use the captured `.schema` dump from the retired Node.js CLI; create migration test fixture |
| APS parser complexity (markdown → structured plan) | Medium | Medium | Consider reusing @eddacraft/anvil-aps via napi-rs, or port the core parser |
| Git notes parsing for authorship | Low | Low | Use `git2` crate or shell out to `git notes`; well-defined format |
| Release runner service size (~400 LOC) | Medium | Low | Lowest priority; defer if cutover can proceed without it |
| Total scope (18 items) delays cutover | Medium | High | Ship phases incrementally; cutover after Phase 2 if Phase 3–4 can be deferred |

## Sequencing Note

RCLI-024 (distribution) is blocked on all three tiers reaching parity.
RCLI-023 (Node.js archival) executed for the core surface ahead of full
Tier 3 parity. The recommended sequencing (updated 2026-03-24 after parity
audit; reviewed 2026-04-26):

1. **RCLI Phase 7 rework** — fix gate checks, watch args, auth migration,
   command registration, hook enforcement, export formatters. These are
   switchover blockers in supposedly-complete phases
2. RCLI Tier 1 Phase 6 remainder (output formatters, archival prep)
3. RCLI2 Tier 2 Phases 1–2 (check, validate, drift, gate-config)
4. RCLI3 Tier 3 Phase 1 (Edda & Ember — highest user impact)
5. RCLI3 Tier 3 Phase 2 (Plan & Stack — workflow)
6. RCLI-023 cutover checkpoint — evaluate whether Phase 3–4 commands can be
   deferred post-archival (users rarely use agent/authorship/release via CLI)
7. RCLI3 Tier 3 Phases 3–4 (Agent, Authorship, Utilities)
8. RCLI2 Tier 2 Phases 3–4 (OPAE-dependent commands, when OPAE lands)
9. RCLI-024 distribution pipeline

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| 1 — Edda & Ember | 7 | 2 Done, 1 In Progress, 4 Proposed |
| 2 — Plan & Stack | 4 | 1 Ready, 3 Proposed |
| 3 — Agent & Authorship | 3 | 2 Ready, 1 Proposed |
| 4 — Utility Commands | 6 | 3 Done, 3 Ready |
| **Total** | **20** | — |
