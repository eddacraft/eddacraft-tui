# MCP Live-Heal (agent-ready without harness restart)

| ID    | Owner  | Priority | Status | Progress |
| ----- | ------ | -------- | ------ | -------- |
| MCPLH | @aneki | high     | Ready  | 7/8      |

**Last reviewed:** 2026-08-16 — Ready wave MCPLH-001..006 and MCPLH-008
Merged on `main` and **primary claim of `v0.9.5-beta`**. MCPLH-007 remains
Draft until soak evidence and is not this claim.
[`plans/specs/2026-08-09-mcp-live-heal-without-harness-restart.md`](../specs/2026-08-09-mcp-live-heal-without-harness-restart.md).
Exclusive module (feature PRs may flip item `Status:` only; do not bump header
`N/M` — ADR-053).

## Purpose

Make multi-client MCP attach reliable after Anvil upgrades **without** requiring
operators to restart long-lived agent sessions (Grok, Claude Code, Codex,
Cursor, …).

Live-heal targets the **MCP child** under the harness stdio pipe (re-exec into
the preferred binary), bulk **config rewrite** to PATH-stable `anvil`, and
**daemon recycle** when CLI and daemon diverge. Session restart remains a
residual failure mode only.

## Design authority

| Document | Role |
| -------- | ---- |
| [`2026-08-09-mcp-live-heal-without-harness-restart.md`](../specs/2026-08-09-mcp-live-heal-without-harness-restart.md) | Accepted design contract for this module (re-exec, refresh cascade, process policy, non-goals) |

Open questions OQ-1..OQ-6 in that spec may be resolved in-item or via a thin ADR
if re-exec becomes cross-cutting beyond CLI MCP.

## In scope

- PATH-stable managed MCP install (never default to versioned Cellar/absolute)
- Self-heal re-exec of `anvil mcp serve --stdio` between JSON-RPC messages
- `anvil mcp refresh` (or equivalent): bulk config rewrite, generation poke,
  inventory report
- Daemon auto-recycle on version skew inside refresh/ensure paths
- status/verify fields: MCP process inventory, skew, split readiness claims
- Opt-in orphan reap (parent PID gone only)

## Out of scope

- Silent kill of MCP children belonging to live harness parents (CIB-242)
- Large-repo graph progressive warm / scan-timeout (separate design)
- LSP productisation / LSPNAV
- Supervisor/proxy unless re-exec fails soak (Draft stretch)
- Multi-host fleet orchestration

## Interfaces

**Depends on:** MCPX (Done — client registry), RMCP/RMCPF MCP serve surface,
CIB-242 visibility posture (no auto-kill of foreign sessions), intercept daemon
lifecycle (DLIFE shipped).

**Coordinates with:** bare ensure ([spec](../specs/2026-08-01-bare-anvil-ensure.md)),
MCP26 protocol (orthogonal), GCTX readiness claims (split from MCP binary heal).

**Exposes:** Preferred-binary resolution shared by install + re-exec; operator
refresh verb; honest `mcp_skew` / process inventory on status surfaces.

## Acceptance criteria (module)

- [ ] Managed MCP installs write PATH `anvil` by default
- [ ] Long-lived `mcp serve` re-execs to preferred binary without harness restart
      (Unix v1; Windows demotes honestly if needed)
- [ ] One operator command rewrites owned configs, may recycle daemon, signals
      live children, and reports residual skew by parent
- [ ] status/verify distinguishes config vs daemon vs MCP process vs graph
- [ ] No default path kills children of live parents

## Work Items

### MCPLH-001: PATH-stable MCP install command

- **Status:** Merged 2026-08-14 via PR #3900
- **Intent:** Stop managed installers from pinning versioned absolute paths
  (e.g. Homebrew Cellar) so upgrades do not strand new and existing configs.
- **Expected Outcome:** Default managed entries use `command: anvil` with args
  `mcp serve --stdio` (plus client type discriminators where required).
  Absolute/versioned paths are treated as drift and rewritten on install/refresh.
  `--command` remains for explicit side-by-side overrides.
- **Files:** `crates/anvil-cli/src/activation/mcp_client.rs`,
  `crates/anvil-cli/src/activation/orchestrator/install.rs`,
  `crates/anvil-cli/src/commands/mcp.rs`,
  `crates/anvil-cli/src/commands/mcp_config.rs`,
  `crates/anvil-cli/src/commands/ensure.rs`,
  `crates/anvil-cli/tests/mcp_config.rs`,
  `docs/architecture/activation-as-built.md`
- **Validation:** `cargo test -p eddacraft-anvil -- mcp_install` (or the package
  test filter that covers install rewrite); fixture asserts default command is
  bare `anvil`, not a Cellar path
- **Confidence:** high
- **Priority:** High
- **Dependencies:** none
- **Design ref:** spec §6, §10, slice A

---

### MCPLH-002: Self-heal re-exec in `mcp serve`

- **Status:** Merged 2026-08-14 via PR #3901
- **Intent:** Long-lived MCP children recycle themselves to the preferred binary
  under a live harness stdio pipe so agents need not restart sessions after
  upgrade.
- **Expected Outcome:** On `initialize`, `tools/list`, and `tools/call` entry
  (between frames), detect skew vs preferred binary; `execve` preferred `anvil
  mcp serve --stdio` at most once per process (anti-loop env); kill-switch
  `ANVIL_MCP_NO_REEXEC=1`. Never re-exec mid-frame or mid-response. Unix first;
  Windows demotes to honest skew reporting if re-exec is unsafe.
- **Files:** `crates/anvil-cli/src/mcp/reexec.rs`,
  `crates/anvil-cli/src/commands/mcp.rs`,
  `crates/anvil-cli/tests/mcp_reexec.rs`,
  `crates/anvil-cli/tests/mcp_serve_stdio.rs`
- **Validation:** `cargo test -p eddacraft-anvil -- mcp_reexec` (or equivalent);
  unit tests for anti-loop and between-message gate; process test proves
  preferred version after forced skew when platform allows
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MCPLH-001 (preferred resolution shared)
- **Design ref:** spec §7, slice B

---

### MCPLH-003: `anvil mcp refresh` bulk cascade

- **Status:** Merged 2026-08-15 via PR #3910
- **Intent:** Give operators one verb that rewrites owned configs, pokes live
  MCP heal, and reports residual skew without walking every session UI.
- **Expected Outcome:** `anvil mcp refresh [--dry-run] [--json]` implements the
  cascade: (1) rewrite Anvil-owned client entries to preferred command,
  (2) optional/auto daemon recycle when skewed (see MCPLH-004), (3) bump
  install-scoped refresh generation so live serves re-check,
  (4) report config actions + process inventory grouped by parent.
  Default process mode is report-only; no kill of live parents' children.
- **Files:** `crates/anvil-cli/src/commands/mcp.rs`,
  `crates/anvil-cli/src/commands/mcp_refresh.rs`,
  `crates/anvil-cli/src/commands/mcp_generation.rs`,
  `crates/anvil-cli/src/commands/mcp_inventory.rs`,
  `crates/anvil-cli/src/mcp/reexec.rs`,
  `crates/anvil-cli/tests/mcp_refresh.rs`,
  `docs/runbooks/cli-surface.md`
- **Validation:** `cargo test -p eddacraft-anvil -- mcp_refresh`; dry-run does
  not mutate; real run rewrites a fixture drifted entry and bumps generation
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MCPLH-001
- **Coordinates with:** MCPLH-002 (generation consumed by serve), MCPLH-004
- **Design ref:** spec §9, §10, slice C

---

### MCPLH-004: Daemon auto-recycle on CLI/daemon version skew

- **Status:** Merged 2026-08-14 via PR #3899
- **Intent:** Recycle the Anvil-owned intercept daemon when its version differs
  from the CLI without requiring harness restart.
- **Expected Outcome:** Refresh (and/or ensure) path stops the skewed daemon,
  waits for PID exit, starts the current binary, and reports before/after
  versions. Matches existing stop → wait → start guidance but automated under
  refresh `--daemon auto` (default when skew detected).
- **Files:** `crates/anvil-cli/src/commands/daemon_recycle.rs`,
  `crates/anvil-cli/src/commands/ensure.rs`,
  `crates/anvil-cli/src/commands/intercept.rs`,
  `crates/anvil-cli/src/commands/start.rs`,
  `crates/anvil-intercept/src/lib.rs`
- **Validation:** integration or unit test with mocked/status-double versions
  asserts stop+start sequence is invoked on skew and skipped when matched
- **Confidence:** high
- **Priority:** High
- **Dependencies:** none (can land with MCPLH-003 or slightly before)
- **Design ref:** spec §5, §9.2 step 2, slice D

---

### MCPLH-005: status/verify MCP inventory and split readiness claims

- **Status:** Merged 2026-08-15 via PR #3911
- **Intent:** Make config, daemon, live MCP binary, and graph readiness
  independently visible so operators and agents do not conflate protecting with
  current tools or graph ready.
- **Expected Outcome:** Human and `--json` status/verify expose CLI vs MCP
  process versions (best-effort inventory), `mcp_skew` aggregates, parent
  grouping when available, and split claims (`protecting` / pre-write attach vs
  `graph_ready` or equivalent blocker list). Extends CIB-242 visibility; does
  not auto-kill.
- **Files:** `crates/anvil-cli/src/commands/status.rs`,
  `crates/anvil-cli/src/commands/status_mcp.rs`,
  `crates/anvil-cli/src/commands/mod.rs`,
  `schemas/anvil-status.v1.json`
- **Validation:** `cargo test -p eddacraft-anvil -- status` (or targeted
  status_render / verify tests); fixture with mismatched versions prints skew
  guidance without claiming false agent-ready
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** MCPLH-002 useful for “current after heal”; can start after
  inventory-only slice
- **Design ref:** spec §7.4, §9.5, §12, slice E; coordinates with CIB-242

---

### MCPLH-006: Opt-in orphan MCP process reap

- **Status:** Merged 2026-08-15 via PR #3912
- **Intent:** Clean same-user `anvil mcp serve` processes whose parent PID is
  gone without touching children of live harnesses.
- **Expected Outcome:** `anvil mcp refresh --processes orphan-reap` (or
  equivalent) SIGTERMs only shape-checked orphans; default remains report.
  Documented; tested with fake parent-dead PIDs where the platform allows.
- **Files:** `crates/anvil-cli/src/commands/mcp_refresh.rs`,
  `crates/anvil-cli/src/commands/mcp_inventory.rs`,
  `crates/anvil-cli/tests/mcp_refresh.rs`,
  `docs/runbooks/cli-surface.md`
- **Validation:** unit tests for parent-alive vs parent-dead classification;
  dry-run lists orphans without signalling
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** MCPLH-003 (refresh surface)
- **Design ref:** spec §9.4, slice F

---

### MCPLH-008: Daily self-heal with easy pin

- **Status:** Merged 2026-08-16 via PR #3932
- **Intent:** MCP updates happen on the daily paths (`anvil`, `anvil start`,
  `anvil doctor`) without operators memorising `mcp refresh`. Refresh stays
  the emergency verb. People who hate auto-updates can pin easily.
- **Expected Outcome:** When configs, the CLI, or the daemon are stale, daily
  paths rewrite owned MCP entries and poke live children. `anvil mcp pin`
  / `ANVIL_MCP_PIN` freezes daily heal and in-process re-exec; `anvil mcp
  unpin` or `ANVIL_MCP_PIN=0` resumes. First-time `NotPresent` install on
  `anvil start` still works while pinned. Emergency `mcp refresh` still
  runs when pinned and says so.
- **Files:** `crates/anvil-cli/src/commands/mcp_heal.rs`,
  `crates/anvil-cli/src/commands/mcp_generation.rs`,
  `crates/anvil-cli/src/commands/mcp.rs`,
  `crates/anvil-cli/src/commands/ensure.rs`,
  `crates/anvil-cli/src/commands/start.rs`,
  `crates/anvil-cli/src/commands/doctor.rs`,
  `crates/anvil-cli/src/mcp/reexec.rs`,
  `crates/anvil-cli/src/activation/orchestrator/install.rs`,
  `crates/anvil-cli/tests/mcp_heal.rs`,
  `docs/runbooks/cli-surface.md`
- **Validation:** `cargo test -p eddacraft-anvil -- mcp_heal`; doctor check
  is registered; pin blocks daily poke; emergency refresh still bumps
- **Confidence:** high
- **Priority:** High
- **Dependencies:** MCPLH-003 (generation + refresh), MCPLH-002 (re-exec)
- **Design ref:** spec §9 / OQ-3 (daily poke); pin is the auto-update opt-out

---

## Stretch (not Ready)

### MCPLH-007: Supervisor/proxy if re-exec fails soak (Draft)

- **Status:** Draft
- **Intent:** Hold harness stdio in a stable supervisor that restarts a worker
  serve when re-exec proves unsafe on a promoted client.
- **Expected Outcome:** Only authorised after soak evidence that parents tear
  down on re-exec; config argv stays `anvil mcp serve --stdio`.
- **Validation:** Not executable until soak evidence promotes this item to
  Ready; then process tests prove worker restart under a stable parent pipe
  without harness session restart
- **Dependencies:** MCPLH-002 failed soak evidence
- **Design ref:** spec §8, slice G

Large-repo graph progressive ready remains **out of module** (spec §12 / slice H
— separate design).

## Risks

| Risk | Mitigation |
| ---- | ---------- |
| Client drops pipe on re-exec | Between-message only; soak; supervisor fallback |
| Re-exec loop | Anti-loop env; version identity |
| Mass-kill temptation | Policy ladder; CIB-242; tests forbid default kill |
| Conflating graph not_ready with MCP skew | Split claims in MCPLH-005 |
