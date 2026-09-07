# MCP Live-Heal (agent-ready without harness restart)

| ID    | Owner  | Priority | Status | Progress |
| ----- | ------ | -------- | ------ | -------- |
| MCPLH | @aneki | high     | Ready  | 7/8      |

**Last reviewed:** 2026-08-17 — Ready wave MCPLH-001..006 and MCPLH-008
**Released/Shipped** via `v0.9.5-beta` (`5c4b61a7`). MCPLH-007 remains
Draft until soak evidence and is not that claim.

**Contract amendment (2026-09-07):** [ADR-143](../decisions/143-established-mcp-session-continuity.md)
supersedes the post-read re-exec parts of the original v1 design. Established
sessions preserve requests and require a targeted MCP reconnect to change image.

[`plans/specs/2026-08-09-mcp-live-heal-without-harness-restart.md`](../specs/2026-08-09-mcp-live-heal-without-harness-restart.md).
Exclusive module (feature PRs may flip item `Status:` only; do not bump header
`N/M` — ADR-053).

## Purpose

Make multi-client MCP attach reliable after anvil upgrades without losing
accepted requests or requiring a full parent-harness restart.

Live-heal uses startup re-exec before the first stdin read, bulk **config
rewrite** to PATH-stable `anvil`, and **daemon recycle** when CLI and daemon
diverge. An established MCP child remains usable on its current image and emits
targeted reconnect guidance.

## Design authority

| Document | Role |
| -------- | ---- |
| [`2026-08-09-mcp-live-heal-without-harness-restart.md`](../specs/2026-08-09-mcp-live-heal-without-harness-restart.md) | Accepted design contract for this module (re-exec, refresh cascade, process policy, non-goals) |
| [ADR-143](../decisions/143-established-mcp-session-continuity.md) | Current established-session continuity contract; supersedes post-read re-exec |

Open questions OQ-1..OQ-6 in the original spec remain historical unless
ADR-143 resolves them.

## In scope

- PATH-stable managed MCP install (never default to versioned Cellar/absolute)
- Startup self-heal re-exec before the first stdin read
- Established-session skew checks with request-preserving reconnect guidance
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
- [ ] `mcp serve` may re-exec before its first stdin read; after that boundary,
      it preserves request/reply identity, buffered input, protocol state, and
      exactly-once handling while reporting targeted MCP reconnect guidance
- [ ] One operator command rewrites owned configs, may recycle daemon, signals
      live children, and reports residual skew by parent
- [ ] status/verify distinguishes config vs daemon vs MCP process vs graph
- [ ] No default path kills children of live parents

## Work Items

### MCPLH-001: PATH-stable MCP install command

- **Status:** Released/Shipped via v0.9.5-beta (5c4b61a7 · 2026-08-16)
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

- **Status:** Released/Shipped via v0.9.5-beta (5c4b61a7 · 2026-08-16)
- **Historical Outcome:** v0.9.5 attempted to recycle long-lived MCP children to
  the preferred binary under a live harness stdio pipe.
- **Current Correction (ADR-143 / JREL-001):** `execve` is safe only at
  startup, before the first stdin read. On `initialize`, `tools/list`, and
  `tools/call`, an established session detects skew but handles the consumed
  request on its current image, preserves pipelined input and negotiated
  protocol state, and emits one targeted reconnect instruction. The
  `ANVIL_MCP_NO_REEXEC=1` kill-switch still governs startup replacement.
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

- **Status:** Released/Shipped via v0.9.5-beta (5c4b61a7 · 2026-08-16)
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

- **Status:** Released/Shipped via v0.9.5-beta (5c4b61a7 · 2026-08-16)
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

- **Status:** Released/Shipped via v0.9.5-beta (5c4b61a7 · 2026-08-16)
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

- **Status:** Released/Shipped via v0.9.5-beta (5c4b61a7 · 2026-08-16)
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

- **Status:** Released/Shipped via v0.9.5-beta (5c4b61a7 · 2026-08-16)
- **Intent:** MCP updates happen on the daily paths (`anvil`, `anvil start`,
  `anvil doctor`) without operators memorising `mcp refresh`. Refresh stays
  the emergency verb. People who hate auto-updates can pin easily.
- **Expected Outcome:** When configs, the CLI, or the daemon are stale, daily
  paths rewrite owned MCP entries and signal live children to re-check. `anvil
  mcp pin` / `ANVIL_MCP_PIN` freezes daily heal and startup re-exec; `anvil mcp
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
| Post-read replacement loses accepted input | Established sessions never re-exec; MCPLH-007 owns any future supervisor |
| Re-exec loop | Anti-loop env; version identity |
| Mass-kill temptation | Policy ladder; CIB-242; tests forbid default kill |
| Conflating graph not_ready with MCP skew | Split claims in MCPLH-005 |
