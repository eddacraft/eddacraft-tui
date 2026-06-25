# Activation — MCP-Optional Golden Path

| ID    | Owner | Status | Progress |
| ----- | ----- | ------ | -------- |
| ACTMO | Josh  | Ready  | 0/10     |

**Last reviewed:** 2026-06-26 — module created **Ready** from v0.8.2-beta Windows
smoke evidence (GH [#2937](https://github.com/eddacraft/anvil-001/issues/2937)):
daemon ensure succeeds (CIB-072) but activation parks at
`ready_restart_required` with `worktree_unenforced` despite MCP handshake
verified. Strategic design issue:
[#2939](https://github.com/eddacraft/anvil-001/issues/2939) (filed with module).
ADR-092 Proposed pins the MCP-optional spine decision (ACTMO-001 accepts it).

## Purpose

Make `anvil start` reach honest, daemon-backed protection **without requiring
MCP**. MCP remains an optional L0 upgrade when the editor allows it; the spine
is daemon ensure → worktree registration → git hooks → save-time armed.

Closes the adoption gap exposed when CIB-072 fixed `daemon_unreachable` but smoke
still failed on `worktree_unenforced` ([#2937](https://github.com/eddacraft/anvil-001/issues/2937),
recurrence of [#1831](https://github.com/eddacraft/anvil-001/issues/1831) /
[#2583](https://github.com/eddacraft/anvil-001/issues/2583)). Tracks under
[#2874](https://github.com/eddacraft/anvil-001/issues/2874) (v0.8.2-beta).

## In Scope

- MCP-independent worktree registration during `anvil start`
- Activation state machine: do not stall on MCP when spine is live
- `--no-mcp` / `ANVIL_NO_MCP` corporate opt-out
- Git hook installation in the start orchestrator (ADR-038 discipline)
- Default save-time armed posture after start (not requiring manual `anvil watch`)
- Claude Code MCP tool allow-list on install (`mcp__anvil__*`)
- Windows `intercept stop` + doctor daemon visibility improvements
- Corporate no-MCP runbook and public golden-path docs
- E2E regression matrix (Windows/Linux, MCP on/off)

## Out of Scope

- Replacing MCP with a new editor protocol (LSP/RTAI-005 remains separate)
- Graph product delivery (ADR-075 / v0.9 window)
- Changing save-time verdict semantics or policy classes
- System-wide service installation
- Cross-uid daemon sharing

## Interfaces

**Depends on:**

- [ADR-015](../decisions/015-intercept-loop-enforcement.md) — session registration
- [ADR-038](../decisions/038-hook-surface-and-noise-discipline.md) — hook install
- [ADR-044](../decisions/044-mcp-entry-activation-owned.md) — MCP entry ownership
- [ADR-061](../decisions/061-save-time-daemon-delta-validation.md) — save-time path
- [ADR-082](../decisions/082-daemon-lifecycle-user-startup.md) — daemon ensure
- [ADR-092](../decisions/092-mcp-optional-activation-spine.md) — spine decision
- [daemon-lifecycle](daemon-lifecycle.aps.md) — DLIFE (merged; ensure primitive)
- [multilayer-protection-v2](multilayer-protection-v2.aps.md) — MLP2-051f promotion
- GH [#2937](https://github.com/eddacraft/anvil-001/issues/2937) — smoke evidence

**Exposes:**

- Honest activation states without MCP as a hard gate
- Documented corporate `--no-mcp` path
- Terminating diagnostics when worktree registration fails (not restart loops)

## Constraints

- UK English spelling in plan and docs
- `--verify` and `--json` remain non-mutating
- No `Protecting` claim before daemon attests worktree enforcement
- MCP install must remain skippable without blocking spine success
- Preserve ADR-044 `UnsafeDrift` never-overwrite rule

## Ready Checklist

- [x] Smoke evidence captured and filed ([#2937](https://github.com/eddacraft/anvil-001/issues/2937))
- [x] Strategic ADR drafted (ADR-092 Proposed)
- [x] Dependencies identified (DLIFE, MLP2-051f, ADR-015/038/044)
- [x] Work items scoped with validation commands
- [x] Cross-links to release tracking ([#2874](https://github.com/eddacraft/anvil-001/issues/2874))

## Work Items

### ACTMO-001: Accept MCP-optional activation spine (ADR-092)

- **Status:** Ready
- **Intent:** Pin the product contract that daemon + worktree registration + hooks
  + save-time is the required spine; MCP is optional L0.
- **Expected Outcome:** ADR-092 status **Accepted**; DECISION-LOG row added;
  ADR-092 cross-linked from activation as-built and wow-start docs.
- **Validation:** `pnpm adr:check`; `pnpm aps:index:check`
- **Files:** `plans/decisions/092-mcp-optional-activation-spine.md`,
  `plans/decisions/DECISION-LOG.md`, `plans/modules/activation-mcp-optional.aps.md`
- **Dependencies:** None
- **Confidence:** high
- **Risks:** Operator may request copy tweaks before acceptance.
- **changeType:** internal
- **releaseIntent:** hold
- **releaseScope:** none

### ACTMO-002: Register worktree from activation (MCP-independent)

- **Status:** Ready
- **Intent:** Close [#2937](https://github.com/eddacraft/anvil-001/issues/2937) core
  gap: `anvil start` registers the worktree with the intercept daemon without
  requiring MCP `validate_write` or `anvil-run` wrapper.
- **Expected Outcome:** After `anvil start` with live daemon, `anvil intercept status`
  lists the worktree; `anvil start --verify` can promote past
  `worktree_unenforced` when other MLP2-051f predicates hold. Windows path
  canonicalisation matches daemon register-time bytes.
- **Validation:** `cargo test -p eddacraft-anvil activation::daemon_evidence`;
  `cargo test -p eddacraft-anvil-intercept`; Windows smoke checklist in #2937
- **Files:** `crates/anvil-cli/src/activation/orchestrator/**`,
  `crates/anvil-cli/src/activation/daemon_evidence.rs`,
  `crates/anvil-intercept/src/**`, related tests
- **Dependencies:** ACTMO-001
- **Confidence:** medium
- **Risks:** Windows path canonicalisation drift vs daemon registry.
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** patch

### ACTMO-003: Activation state machine — honest fall-through without MCP

- **Status:** Ready
- **Intent:** Stop indefinite `ready_restart_required` when MCP handshake is
  verified but worktree enforcement is the remaining gap — or when MCP is absent
  but spine is live.
- **Expected Outcome:** New honest states/copy: daemon-backed `watching` or
  save-time-armed success without implying another editor restart. `Protecting`
  still requires `LiveValidation` + daemon promotion per MLP2-051f.
- **Validation:** `cargo test -p eddacraft-anvil activation::render`;
  `cargo test -p eddacraft-anvil activation::diagnostic`
- **Files:** `crates/anvil-cli/src/activation/diagnostic.rs`,
  `crates/anvil-cli/src/activation/state.rs`,
  `crates/anvil-cli/src/activation/render.rs`,
  `crates/anvil-cli/src/activation/daemon_evidence.rs`
- **Dependencies:** ACTMO-002
- **Confidence:** medium
- **Risks:** Over-claiming protection before daemon attests.
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** patch

### ACTMO-004: `--no-mcp` / `ANVIL_NO_MCP` opt-out

- **Status:** Ready
- **Intent:** Corporate and MCP-skeptical users can run the spine without MCP
  install or MCP-gated promotion.
- **Expected Outcome:** `anvil start --no-mcp` and `ANVIL_NO_MCP=1` skip MCP
  orchestrator steps; activation reports spine-only success honestly; `--help`
  documents the flag.
- **Validation:** `cargo test -p eddacraft-anvil commands::start`;
  `anvil start --help` lists `--no-mcp`
- **Files:** `crates/anvil-cli/src/commands/start.rs`,
  `crates/anvil-cli/src/activation/orchestrator/**`
- **Dependencies:** ACTMO-001
- **Confidence:** high
- **Risks:** Users may confuse with `--no-daemon` (distinct semantics).
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** patch

### ACTMO-005: Git hooks in `anvil start` orchestrator

- **Status:** Ready
- **Intent:** Install ADR-038 hooks during start so session registration and
  witness chain work for editor-native agents without MCP.
- **Expected Outcome:** `anvil start` offers/installs hooks per existing
  coexistence policy (ADOPT-001); hook install failure is non-fatal with honest
  WARN; `register-session` fires on PreToolUse where configured.
- **Validation:** `cargo test -p eddacraft-anvil activation::orchestrator`;
  `docs/runbooks/anvil-hook-coexistence.md` cross-link unchanged or updated
- **Files:** `crates/anvil-cli/src/activation/orchestrator/**`,
  `crates/anvil-hook/**`, hook install tests
- **Dependencies:** ACTMO-002
- **Confidence:** medium
- **Risks:** Hook noise / coexistence conflicts on Windows.
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

### ACTMO-006: Default save-time armed after start

- **Status:** Ready
- **Intent:** Users should not need a separate `anvil watch` step for L1/L2
  save-time to be the default outcome of `anvil start`.
- **Expected Outcome:** Successful start with live daemon reports save-time
  validation armed; watch remains available for scoped/TUI flows but is not the
  only path to armed posture.
- **Validation:** `cargo test -p eddacraft-anvil commands::start`;
  activation integration tests for armed reporting
- **Files:** `crates/anvil-cli/src/commands/start.rs`,
  `crates/anvil-cli/src/activation/**`
- **Dependencies:** ACTMO-002, DLIFE-003 (Merged)
- **Confidence:** medium
- **Risks:** Interaction with `--no-daemon` and scoped fallback copy.
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

### ACTMO-007: Claude MCP allow-list on install

- **Status:** Ready
- **Intent:** When user approves MCP install during `anvil start`, also add
  `mcp__anvil__*` (or tool-specific rules) to Claude Code `permissions.allow` so
  `anvil_validate_write` does not prompt every write.
- **Expected Outcome:** After Claude MCP install path, settings.json contains
  allow rules; existing user rules preserved; non-Claude editors unchanged.
- **Validation:** `cargo test -p eddacraft-anvil activation::mcp_client::claude_code`;
  fixture tests for settings merge
- **Files:** `crates/anvil-cli/src/activation/mcp_client/claude_code.rs`,
  related tests
- **Dependencies:** ACTMO-001
- **Confidence:** medium
- **Risks:** Claude settings schema drift; must not broaden allow beyond Anvil tools.
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** patch

### ACTMO-008: Windows intercept stop + doctor daemon visibility

- **Status:** Ready
- **Intent:** Operators can see and stop the headless daemon (CREATE_NO_WINDOW)
  without PID-lock confusion ([#2937](https://github.com/eddacraft/anvil-001/issues/2937)
  Matt scenario).
- **Expected Outcome:** `anvil intercept stop` releases lock on Windows;
  `anvil doctor` (or status) surfaces daemon PID/command-line hint; foreground
  start error copy names `intercept stop` recovery.
- **Validation:** `cargo test -p eddacraft-anvil commands::intercept`;
  Windows manual smoke per #2937
- **Files:** `crates/anvil-cli/src/commands/intercept.rs`,
  `crates/anvil-intercept/src/lib.rs`, doctor/status surfaces
- **Dependencies:** None
- **Confidence:** medium
- **Risks:** V060F-002 `intercept stop` may partially overlap — reconcile in PR.
- **changeType:** fix
- **releaseIntent:** candidate
- **releaseScope:** patch

### ACTMO-009: Corporate no-MCP runbook and golden-path docs

- **Status:** Ready
- **Intent:** Document the MCP-optional spine for enterprise users and update wow
  start demo to stop implying MCP-only protection.
- **Expected Outcome:** Runbook for `--no-mcp`; public guide states spine vs MCP
  layers; wow-start-demo lists hooks + daemon, not MCP-only loop.
- **Validation:** `pnpm run docs:check`; `pnpm run lint:md`
- **Files:** `docs/public/anvil/guides/wow-start-demo.md`,
  `docs/runbooks/**`, activation as-built cross-links
- **Dependencies:** ACTMO-003, ACTMO-004
- **Confidence:** high
- **Risks:** Docs drift until code lands — ship with implementation PRs.
- **changeType:** docs
- **releaseIntent:** candidate
- **releaseScope:** patch

### ACTMO-010: E2E regression matrix (MCP on/off, Windows/Linux)

- **Status:** Ready
- **Intent:** Prevent recurrence of #2937-class failures across platforms and MCP
  configurations.
- **Expected Outcome:** E2E fixtures cover: MCP on + daemon ensure → enforcing;
  `--no-mcp` → spine success; `worktree_unenforced` → terminating diagnostic.
  CI runs Linux; Windows cases documented or gated.
- **Validation:** `pnpm --filter @eddacraft/anvil-e2e test`; targeted e2e file
  for activation golden path
- **Files:** `apps/e2e/src/**`, activation fixture helpers
- **Dependencies:** ACTMO-002, ACTMO-003, ACTMO-004
- **Confidence:** medium
- **Risks:** Windows E2E may remain manual until harness supports it.
- **changeType:** test
- **releaseIntent:** candidate
- **releaseScope:** patch