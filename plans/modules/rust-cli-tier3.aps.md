<!--
APS Module: Rust CLI Tier 3
=========================
Port Tier 3 (subsystem & specialised) commands to crates/anvil-cli/.
Depends on RCLI (Tier 1) and RCLI2 (Tier 2) foundation.

Scopes: RCLI3 (main)
-->

# Rust CLI — Tier 3

| ID    | Owner | Status   |
| ----- | ----- | -------- |
| RCLI3 | —     | Proposed |

## Purpose

Port Tier 3 subsystem and specialised commands from the Node.js CLI
(`apps/anvil-cli/`) to the Rust binary (`crates/anvil-cli/`). These commands
expose the Edda Stack (memory/proposals), APS planning, agent governance,
and operational utilities. Completing Tier 3 enables full Node.js CLI archival
(RCLI-023) and single-binary distribution (RCLI-024).

**Why:** Tier 3 contains the domain subsystem commands (Edda, Ember, Plan,
Agent) that power day-to-day governance workflows. Without them the Rust CLI
cannot replace the Node.js CLI for users who interact with canonical memories,
proposals, or APS plans. The cutover milestone (RCLI-023) is blocked until
all three tiers reach parity.

**ADR:** [012-rust-cli-replacement](../decisions/012-rust-cli-replacement.md)
**Spec:** [2026-03-18-rust-cli-design](../specs/2026-03-18-rust-cli-design.md) §6 Tier 3

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
  confidence levels, provenance schema). These TypeScript contracts define
  the storage format; the Rust port must read/write the same structures
- `packages/aps/` — APS plan loading, filtering, task locking interfaces

**Exposes:**

- 10 additional subcommands (with ~25 total sub-subcommands) on the `anvil`
  binary
- Once all three tiers are complete, unblocks RCLI-023 (archival) and
  RCLI-024 (distribution)

## Constraints

- Storage format parity: Rust commands must read `.anvil/edda/` YAML,
  `.anvil/ember.db` SQLite, `.anvil/agents/`, and `.anvil/executions/`
  identically to Node.js
- Edda YAML files include provenance metadata — Rust YAML serialiser must
  preserve field ordering and comments where possible
- Ember SQLite access via `rusqlite`; schema must match the Node.js
  `better-sqlite3` schema exactly
- Agent state files use JSON; no special handling needed
- Same error handling conventions as RCLI: `anyhow` + `thiserror`

## Ready Checklist

Change status to **Ready** when:

- [ ] RCLI Tier 1 Phases 1–6 complete (all 24 items)
- [ ] RCLI2 Tier 2 Phases 1–2 complete (non-OPAE items)
- [ ] Edda YAML schema documented (or inferable from edda-stack contracts)
- [ ] Ember SQLite schema documented (or inferable from ProposalStore)

---

## Tasks

#### Phase 1 — Edda & Ember (Core Domain)

The highest-priority commands. These are the most-used subsystem commands and
the primary blocker for replacing the Node.js CLI in daily workflows.

### RCLI3-001: edda list command

- **Status:** Proposed
- **Intent:** Port `anvil edda list`. Query active, superseded, and retired
  memories with multi-criteria filtering: `--type` (observation, decision,
  convention, constraint), `--status` (active, superseded, retired),
  `--min-confidence` (high, medium, low), `--since <duration>` (e.g., 7d, 2w),
  `--limit`, `--json`. Reads from `.anvil/edda/` YAML store
- **Expected Outcome:** `anvil edda list --status active --min-confidence
  medium` shows filtered memories with ID, type, confidence, age, and
  truncated statement. JSON mode returns full memory objects
- **Validation:** Row count and content match Node.js CLI for same store
  state; time-ago formatting consistent
- **Files:** `crates/anvil-cli/src/commands/edda.rs`
- **Confidence:** medium (263 LOC in Node.js; YAML parsing + multi-filter
  logic)
- **Priority:** High
- **Dependencies:** RCLI (foundation)

---

### RCLI3-002: edda show command

- **Status:** Proposed
- **Intent:** Port `anvil edda show <id>`. Display full memory details:
  statement, context, type, confidence, provenance (ember source, kindling
  lineage, session), attribution, tags, timestamps, evolution chain
  (supersedes/superseded-by links)
- **Expected Outcome:** `anvil edda show edda-abc123` renders formatted memory
  with all metadata sections
- **Validation:** Output includes all fields present in Node.js CLI; provenance
  chain is correctly resolved
- **Files:** `crates/anvil-cli/src/commands/edda.rs`
- **Confidence:** high (121 LOC in Node.js; display-only)
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
- **Confidence:** medium (168 LOC in Node.js; cross-store write:
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

- **Status:** Proposed
- **Intent:** Port `anvil ember list`. Query proposals by type (observation,
  pattern, anomaly, suggestion) and status (active, promoted, expired,
  rejected) with filtering and expiry display. Reads from `.anvil/ember.db`
  SQLite database
- **Expected Outcome:** `anvil ember list --status active` shows proposals
  with ID, type, summary, confidence, expiry, observation count. JSON mode
  returns full proposal objects
- **Validation:** Row count and content match Node.js CLI; expiry time
  formatting consistent
- **Files:** `crates/anvil-cli/src/commands/ember.rs`
- **Confidence:** medium (198 LOC in Node.js; SQLite via rusqlite)
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
- **Validation:** Output fields match Node.js CLI
- **Files:** `crates/anvil-cli/src/commands/ember.rs`
- **Confidence:** high (116 LOC in Node.js; display-only)
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
- **Confidence:** high (104 LOC in Node.js; single UPDATE)
- **Priority:** Medium
- **Dependencies:** RCLI3-005

---

#### Phase 2 — Plan & Stack (Workflow)

### RCLI3-008: plan validate command

- **Status:** Proposed
- **Intent:** Port `anvil plan validate <path>`. Validate APS markdown
  structure and rules; output issues by severity with line numbers and
  context. Delegates to APS validation logic
- **Expected Outcome:** `anvil plan validate plans/modules/foo.aps.md`
  reports structural issues (missing fields, invalid status transitions,
  broken references)
- **Validation:** Issue list matches Node.js CLI for same plan file
- **Files:** `crates/anvil-cli/src/commands/plan.rs`
- **Confidence:** medium (105 LOC in Node.js; needs APS parser in Rust)
- **Priority:** High
- **Dependencies:** RCLI (foundation)

---

### RCLI3-009: plan load and status commands

- **Status:** Proposed
- **Intent:** Port `anvil plan load` (filter plans by scope, module, task,
  owner, tag, priority, confidence; three output modes: JSON, text,
  files-only) and `anvil plan status` (show task states: open, locked, completed,
  cancelled with grouping and filtering)
- **Expected Outcome:** `anvil plan load --scope RCLI --priority high` filters
  and displays matching work items. `anvil plan status --task RCLI-001` shows
  single task detail
- **Validation:** Filter results and task state counts match Node.js CLI
- **Files:** `crates/anvil-cli/src/commands/plan.rs`
- **Confidence:** medium (465 LOC combined in Node.js; complex filter logic)
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
- **Confidence:** high (95 LOC in Node.js; filesystem operations)
- **Priority:** Medium
- **Dependencies:** RCLI3-008

---

### RCLI3-011: stack command

- **Status:** Proposed
- **Intent:** Port `anvil stack status` and `anvil stack validate`. Stack
  status shows health of the three-layer architecture (Kindling → Ember →
  Edda). Stack validate checks configuration consistency across layers
- **Expected Outcome:** `anvil stack status` shows layer health, store sizes,
  and last-activity timestamps. `anvil stack validate` reports configuration
  issues
- **Validation:** Status output matches Node.js CLI for same store state
- **Files:** `crates/anvil-cli/src/commands/stack.rs`
- **Confidence:** high (33 LOC in Node.js; delegates to services)
- **Priority:** Low
- **Dependencies:** RCLI3-001 (Edda store), RCLI3-005 (Ember store)

---

#### Phase 3 — Agent & Authorship (Operational)

### RCLI3-012: agent status and list commands

- **Status:** Proposed
- **Intent:** Port `anvil agent status` (current agent identity, registration
  state, heartbeat) and `anvil agent list` (all registered agents with state
  filtering: active, idle, stale, terminated). Includes agent type icons
  (claude, cursor, copilot, aider, continue, codeium, human, ci) and
  session ID masking
- **Expected Outcome:** `anvil agent list --state active` shows running agents
  with time-since-heartbeat. `anvil agent status` shows current agent
- **Validation:** Agent list matches Node.js CLI; time-ago colour coding
  (green <5m, yellow <1h, red ≥1d) preserved
- **Files:** `crates/anvil-cli/src/commands/agent.rs`
- **Confidence:** medium (296 LOC combined in Node.js; state file parsing)
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
- **Validation:** Cleanup results match Node.js CLI; dry-run produces no
  side effects
- **Files:** `crates/anvil-cli/src/commands/agent.rs`
- **Confidence:** high (140 LOC in Node.js; JSON file operations)
- **Priority:** Low
- **Dependencies:** RCLI3-012

---

### RCLI3-014: authorship command

- **Status:** Proposed
- **Intent:** Port `anvil authorship` with three subcommands: `show [commit]`
  (display AI authorship for a commit from Git Notes refs/notes/ai), `list`
  (recent N commits with AI authorship), `stats [range]` (coverage %, line
  changes, tools used for revision range). Reads Git AI Standard v3.0.0
  notes format
- **Expected Outcome:** `anvil authorship stats HEAD~10..HEAD` shows AI
  contribution percentage, tools breakdown, and line statistics
- **Validation:** Coverage % and tool counts match Node.js CLI for same
  git history
- **Files:** `crates/anvil-cli/src/commands/authorship.rs`
- **Confidence:** medium (234 LOC in Node.js; git notes parsing via
  `git2` or shell)
- **Priority:** Low
- **Dependencies:** RCLI (foundation)

---

#### Phase 4 — Utility Commands

### RCLI3-015: explain command

- **Status:** Proposed
- **Intent:** Port `anvil explain <warning-id>`. Bidirectional lookup: parse
  warning ID (e.g., AP-001-file.ts:42) to find explanation, or `--rules` to
  list all explainable rules, or `--list` to show recent warnings from last
  check run. Renders explanation sections: title, whyItMatters, howToAddress,
  whenToSuppress, related links
- **Expected Outcome:** `anvil explain AP-001` shows formatted explanation.
  `anvil explain --rules` lists all rules grouped by prefix
- **Validation:** Explanation content matches Node.js CLI; rule grouping
  (AP/ARCH/BOUND prefixes) is identical
- **Files:** `crates/anvil-cli/src/commands/explain.rs`
- **Confidence:** medium (230 LOC in Node.js; requires explanation catalogue
  ported or embedded)
- **Priority:** Medium
- **Dependencies:** RCLI (foundation)

---

### RCLI3-016: mcp-config command

- **Status:** Proposed
- **Intent:** Port `anvil mcp-config`. Generate MCP server configuration for
  AI editors (claude-code, cursor, windsurf, vscode). Supports `--target`,
  `--transport` (stdio/http), `--port`, `--write`. Handles symlink-safe path
  resolution and escape detection (confirms if writing outside workspace)
- **Expected Outcome:** `anvil mcp-config --target claude-code --write`
  creates `.claude/mcp.json` with correct server entry. Each target generates
  its editor-specific config format
- **Validation:** Generated configs are valid for each editor; VSCode format
  (type field) differs from others (command/args) — both correct
- **Files:** `crates/anvil-cli/src/commands/mcp_config.rs`
- **Confidence:** high (176 LOC in Node.js; template generation with path
  safety)
- **Priority:** Low
- **Dependencies:** RCLI (foundation)

---

### RCLI3-017: release command

- **Status:** Proposed
- **Intent:** Port `anvil release`. Interactive release workflow orchestration
  with profile selection (beta, stable, hotfix). Supports `--target <version>`
  (skip prompt), `--execute` (non-dry-run), `--resume` (from saved state),
  `--skip-preflight`
- **Expected Outcome:** `anvil release --profile beta --execute` runs the
  beta release pipeline (version bump, changelog, tag, publish)
- **Validation:** Release artefacts match Node.js CLI workflow; state
  resumption works after interruption
- **Files:** `crates/anvil-cli/src/commands/release.rs`
- **Confidence:** medium (41 LOC wrapper + ~400 LOC service; release runner
  logic needs porting)
- **Priority:** Low
- **Dependencies:** RCLI (foundation)

---

### RCLI3-018: beta command

- **Status:** Proposed
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
- **Confidence:** high (80 LOC in Node.js; HTTP calls via reqwest)
- **Priority:** Low
- **Dependencies:** RCLI-015 (auth/HTTP infrastructure)

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Edda YAML format divergence (Rust serde_yaml vs Node.js yaml) | Medium | High | Write format round-trip tests; validate field ordering preservation |
| Ember SQLite schema mismatch (rusqlite vs better-sqlite3) | Low | High | Dump Node.js schema with `.schema`; create migration test fixture |
| APS parser complexity (markdown → structured plan) | Medium | Medium | Consider reusing @eddacraft/anvil-aps via napi-rs, or port the core parser |
| Git notes parsing for authorship | Low | Low | Use `git2` crate or shell out to `git notes`; well-defined format |
| Release runner service size (~400 LOC) | Medium | Low | Lowest priority; defer if cutover can proceed without it |
| Total scope (18 items) delays cutover | Medium | High | Ship phases incrementally; cutover after Phase 2 if Phase 3–4 can be deferred |

## Sequencing Note

RCLI-023 (Node.js archival) and RCLI-024 (distribution) are blocked on all
three tiers reaching parity. The recommended sequencing:

1. RCLI Tier 1 Phase 5–6 (complete policy/architecture stubs, output formatters)
2. RCLI2 Tier 2 Phases 1–2 (check, validate, drift, gate-config)
3. RCLI3 Tier 3 Phase 1 (Edda & Ember — highest user impact)
4. RCLI3 Tier 3 Phase 2 (Plan & Stack — workflow)
5. RCLI-023 cutover checkpoint — evaluate whether Phase 3–4 commands can be
   deferred post-archival (users rarely use agent/authorship/release via CLI)
6. RCLI3 Tier 3 Phases 3–4 (Agent, Authorship, Utilities)
7. RCLI2 Tier 2 Phases 3–4 (OPAE-dependent commands, when OPAE lands)
8. RCLI-024 distribution pipeline

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| 1 — Edda & Ember | 7 | Proposed |
| 2 — Plan & Stack | 4 | Proposed |
| 3 — Agent & Authorship | 3 | Proposed |
| 4 — Utility Commands | 4 | Proposed |
| **Total** | **18** | — |
