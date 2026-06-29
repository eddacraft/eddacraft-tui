# Activation — MCP-Optional Golden Path

| ID    | Owner | Status | Progress |
| ----- | ----- | ------ | -------- |
| ACTMO | Josh  | In Progress | 13/21 |

**Last reviewed:** 2026-06-26 — ACTMO-001 through ACTMO-010 completed on
`feat/actmo-spine`: ADR-092 accepted; `anvil start` now performs MCP-independent
worktree registration with the intercept daemon before MCP install; daemon
attestation can now fall through to honest `watching` when MCP is absent or
restart-bound; and `--no-mcp` / `ANVIL_NO_MCP` skips MCP config writes while
leaving the daemon-backed spine active. `anvil intercept status` now exposes
the daemon PID and `anvil intercept stop` recovery command, while Windows stop
uses the PID-file record instead of reporting unsupported. Activation now also
installs Anvil-managed `pre-commit` and `pre-push` hooks when run in a Git repo.
ACTMO-006 reports daemon-backed save-time as armed and directs operators to
`anvil intercept status` instead of a manual `anvil watch`. ACTMO-007 merges the
`mcp__anvil__*` permission allow rule into the sibling `.claude/settings.json`
(treating an empty/whitespace file as absent, and degrading a failed allow-list
write to a non-fatal warning so it never masks a healthy posture), preserving
existing allow/deny rules. ACTMO-009 ships the MCP-optional runbook; ACTMO-010
ships the E2E matrix (default MCP install, `--no-mcp`, terminating daemon-repair
verify). Post-Council fixes folded in: empty-settings normalisation, best-effort
allow-list, structured `session already registered` heartbeat detection,
softened restart hint, and `ANVIL_NO_MCP` documentation.
**ACTMO-011** is **Done** ([#2969](https://github.com/eddacraft/anvil-001/pull/2969)) —
`anvil status` TUI `*`/`o` hook legend + honest Recent Runs empty-state, and
`anvil insights` daemon-uptime now reads "not yet measured" instead of a stub
`0%` (JSON wire value unchanged; most of the original wishlist had already
shipped, reconciliation recorded in the work item). **ACTMO-012** remains
**Ready** (filed from Matt beta smoke UX review: Cursor
`restart_handshake_verified` self-test shown though Matt never used Cursor).
Originally created **Ready** from
v0.8.2-beta Windows smoke ([#2937](https://github.com/eddacraft/anvil-001/issues/2937));
design [#2939](https://github.com/eddacraft/anvil-001/issues/2939). ADR-092
Accepted pins the MCP-optional spine decision. **ACTMO-013** is **Done**
(2026-06-29): the registration-UX design
([spec](../specs/2026-06-29-worktree-registration-ux-design.md) +
[ADR-094](../decisions/094-worktree-registration-ux.md), Accepted 2026-06-29) was
planning-council hardened — the keystone decision is that durable registration is
a daemon-side persisted, TTL-exempt, reload-on-start set (not the 30s session
lease) — and splits into **ACTMO-014..017** (Ready, cut-line), **ACTMO-018**
(Ready, additive), and **ACTMO-019..021** (Proposed). The module moves to
**13/21**; the cut-line usefulness shape also depends on a promoted+split DSV-046.
Counts are advisory (ADR-053) pending `pnpm aps:index` reconcile.

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
- Activation output simplification — one narrative, progressive disclosure
  (ACTMO-011; Matt/Dave smoke screenshots)
- Editor-aware MCP install/probe and honest handshake copy — no fictional
  multi-editor session (ACTMO-012; Matt never used Cursor)
- Subsequent worktree registration and daemon-control UX design (ACTMO-013;
  candidate `v0.9.0-beta` usefulness addendum with DSV-046)

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

- **Status:** Done
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

- **Status:** Done
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

- **Status:** Done
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

- **Status:** Done
- **Intent:** Corporate and MCP-sceptical users can run the spine without MCP
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

- **Status:** Done
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

- **Status:** Done
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

- **Status:** Done
- **Intent:** When user approves MCP install during `anvil start`, also add
  `mcp__anvil__*` (or tool-specific rules) to Claude Code `permissions.allow` so
  `anvil_validate_write` does not prompt every write.
- **Expected Outcome:** After Claude MCP install path, settings.json contains
  allow rules; existing user rules preserved; non-Claude editors unchanged.
- **Validation:** `cargo test -p eddacraft-anvil activation::mcp_client::claude_code`;
  `cargo test -p eddacraft-anvil activation::orchestrator::install`
- **Files:** `crates/anvil-cli/src/activation/mcp_client/claude_code.rs`,
  related tests
- **Dependencies:** ACTMO-001
- **Confidence:** medium
- **Risks:** Claude settings schema drift; must not broaden allow beyond Anvil tools.
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** patch

### ACTMO-008: Windows intercept stop + doctor daemon visibility

- **Status:** Done
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

- **Status:** Done
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

- **Status:** Done
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

### ACTMO-011: Activation output simplification (golden-path UX)

- **Status:** Done
- **Intent:** Fix beta smoke feedback that `anvil start`, `anvil status`, and
  bare `anvil` print too much engineer-facing detail that contradicts itself
  (Matt/Dave logs + screenshots under operator-held `anvil-beta/` artefacts).
- **Expected Outcome:** One coherent user story per command with a single
  actionable `next` line. Default stdout: short summary (setup outcome, protection
  posture in plain language, one next step). Internal detail (language census,
  rule modes, verify recipe, tier labels, JSON tracing) moves behind `--verbose`
  / `--why` or dedicated surfaces. Fixes: never say "run `anvil start`" inside
  `anvil start`; suppress structured WARN JSON on default stderr; align
  headline, `next:`, verify `active layers`, and trailing `Next:`; group/cap
  first-scan findings; `anvil status` TUI legend for `*`/`o` hooks and honest
  empty-state for Recent Runs; bare `anvil` routes to welcome/help not
  zero-filled insights; hide or label stubbed `Daemon uptime: 0%` until real;
  doctor names daemon vs MCP shim processes.
- **Reconciliation (2026-06-27, pre-implementation gap analysis vs current `main`):**
  Most of the wishlist had already landed via the surfaces that postdate the
  beta smoke screenshots, so this item ships the genuinely-open remainder:
  - **Shipped this item:** `anvil status` TUI `*`/`o` hook legend + honest
    "No runs recorded yet" empty-state for Recent Runs; `anvil insights` daemon
    uptime now renders "not yet measured" instead of a stub `0%` that
    contradicts a running daemon. `daemon_uptime_percentage` is a
    schema-locked placeholder (always `0` until instrumented); the JSON
    wire value and `schemas/anvil-insights.v1.json` are unchanged — only
    the human render special-cases the `0` placeholder.
  - **Already satisfied — verified, no change needed:** structured WARN copy
    already goes to `tracing` (invisible at default level) with a single plain
    noise-disciplined `eprintln!`, not JSON, on stderr; bare `anvil` already
    routes to `welcome` (not insights); headline / `next:` / `active layers` /
    trailing `Next:` are already aligned and pinned (ADTRUST-006, UJ-001,
    DLIFE-006); the "run `anvil start`" strings are state/command-correct repair
    hints surfaced by `anvil status`, never self-referential inside `anvil start`.
  - **Owned elsewhere — out of scope here:** first-scan findings grouping/cap is
    the CIB-010 first-scan-vs-steady-state class (Merged); doctor daemon-vs-MCP
    process enumeration is net-new cross-platform process scanning better tracked
    as its own item if beta still needs it (no current mislabel — doctor has no
    process-list line today).
- **Validation:** `cargo test -p eddacraft-anvil-tui status::render`;
  `cargo test -p eddacraft-anvil --bins insights`
- **Files:** `crates/anvil-cli/src/commands/insights.rs`,
  `crates/anvil-cli/src/insights/aggregator.rs`,
  `crates/anvil-tui/src/surfaces/status/render.rs`
- **Dependencies:** ACTMO-003 (state machine truth must precede copy)
- **Confidence:** medium
- **Risks:** `--json` and pinned contract tests (ADTRUST-005) must not regress;
  verbose opt-in must preserve operator/debug surfaces.
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** patch

### ACTMO-012: Editor-aware MCP install, probe, and handshake copy

- **Status:** Done
- **Intent:** Stop narrating a two-editor MCP session when the user only uses one
  tool (Matt: never used Cursor, yet activation shows Cursor
  `restart_handshake_verified` and "AI tools detected: cursor" after Anvil wrote
  `~/.cursor/mcp.json`). `RestartHandshakeVerified` today means Anvil self-spawned
  `anvil mcp serve --stdio` during start — not that the editor restarted.
- **Expected Outcome:** MCP install picker and probe scope only editors with
  pre-existing strong detection (binary on PATH or app data Anvil did not just
  create this run); exclude anvil-written config paths from ADOPT-003 agent
  detection in the same session. User-facing tier copy distinguishes "config
  self-test passed" from "editor connected / live validation". Activation
  headline and MCP block reflect the user's actual editor(s), not the hardcoded
  `all_clients()` pair. Handshake self-test does not imply "restart your editor"
  when no editor session exists for that client.
- **Reconciliation (2026-06-27, as built):** The root cause was the
  install path writing a *fresh* MCP config for every client unconditionally
  (`fresh_repo_auto_installs_to_global_scope`). The fix gates fresh writes by
  editor detection, which collapses all three reported symptoms at once:
  - **Shipped:** `anvil start` only writes a fresh MCP config for editors
    actually detected on the host (binary on PATH / pre-existing editor state,
    via the ADOPT-003 detector); undetected editors are skipped
    (`SkipReason::EditorNotDetected`, suppressed from the install block). An
    existing anvil entry is always managed regardless of detection (no
    orphaning). New `--all-mcp-clients` flag / `ANVIL_ALL_MCP_CLIENTS` env
    (presence-based, like `--no-mcp`) restores wiring both editors. Because an
    undetected editor never gets a config, it never reaches `RestartRequired`,
    so the false `restart_handshake_verified` self-test and the false
    "AI tools detected: cursor" line disappear at the root — anvil no longer
    creates `~/.cursor/mcp.json`, so the ADOPT-003 detector has nothing to
    false-positive on (the in-session anvil-written-path exclusion is therefore
    unnecessary once writes are gated).
  - **Deferred (documented scope boundary):** the read-only diagnostic/probe
    block still lists an undetected editor honestly as "config absent" rather
    than omitting it — gating the probe display would mean threading the enabled
    set through `verify`/`verify_with_home` (≈10 call sites) and making
    PATH-based detection injectable there, a larger refactor with no
    truthfulness gain. The `restart_handshake_verified` *tier label* wording
    (vs "config self-test passed") is unchanged; it only ever surfaces now for a
    genuinely-detected editor, so it is no longer misleading. File a follow-up if
    beta wants the probe block omission or the label re-wording.
- **Validation:** `cargo test -p eddacraft-anvil activation::mcp_client`;
  `cargo test -p eddacraft-anvil activation::detect_agents`;
  `cargo test -p eddacraft-anvil activation::orchestrator`;
  fixture: fresh home with only Claude Code signals → picker omits Cursor unless
  explicitly opted in
- **Files:** `crates/anvil-cli/src/activation/mcp_client.rs`,
  `crates/anvil-cli/src/activation/orchestrator/install.rs`,
  `crates/anvil-cli/src/activation/diagnostic.rs`,
  `crates/anvil-cli/src/activation/detect_agents.rs`,
  `crates/anvil-cli/src/activation/render.rs`
- **Dependencies:** ACTMO-004 (optional), ACTMO-011 (copy alignment)
- **Confidence:** medium
- **Risks:** Under-detection on Windows if Cursor app-data paths stay
  unprobed (ADOPT-003 follow-up); power users who want both editors pre-wired
  need an explicit opt-in (`--all-mcp-clients` or picker toggle).
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** patch

### ACTMO-013: Subsequent worktree registration UX

- **Status:** Done
- **Delivered (2026-06-29):** Design note
  [`worktree-registration UX design`](../specs/2026-06-29-worktree-registration-ux-design.md)
  + [ADR-094](../decisions/094-worktree-registration-ux.md)
  (Accepted 2026-06-29), hardened by a four-persona planning council. The council's load-bearing finding:
  the draft built *durable membership* on the registry's 30s in-memory heartbeat
  lease, so a one-shot CLI registration would evict ~30s after the process exits
  and a daemon restart would drop the whole set. Resolution: durable registration
  is a daemon-side **persisted, TTL-exempt, reload-on-start** set (reaper +
  distinct-worktree cap), reusing the `session.register` RPC but changing daemon
  semantics. Registration surface is `anvil workspace register [PATH]` /
  `unregister` (not `anvil intercept`); `--all` is bounded to exact
  confinement-allowlist entries (never a filesystem scan); the persistent
  `register_on_start` list is a separate additive `workspace.yaml` key (not a
  field on `deny_unknown_fields` `AllowEntry`) deferred behind owner sign-off;
  status splits into membership vs assurance axes; the DSV-046 driver subscribes
  to a registry membership-change signal; new-worktree auto-registration is a
  guided `anvil workspace install-hook`; a local app is a scoped, deferred
  daemon-control surface only. Splits into **ACTMO-014..017** (Ready, cut-line),
  **ACTMO-018** (Ready, additive), **ACTMO-019..021** (Proposed). ADR-094 was
  ratified **Accepted 2026-06-29 (operator)**.
- **Source:** Operator grounding session 2026-06-29 identified a daemon/worktree
  lifecycle gap: the per-user daemon may already be running, `anvil start` may be
  invoked outside a Git worktree, and later-created Worktrunk/Git worktrees need a
  simple way to register without repeating the full activation mental model.
- **Intent:** Design the explicit and automatic paths for registering additional
  worktrees with an already-running per-user daemon.
- **Expected Outcome:** A design note or ADR defines the worktree-registration UX:
  what `anvil start` does outside a worktree, whether a dedicated command such as
  `anvil workspace register` or `anvil intercept register --worktree <path>` is
  added, how Anvil helps a user opt into automatic `anvil start --no-mcp` when a
  new Worktrunk/Git worktree is created, whether Worktrunk hooks can
  auto-register new worktrees, whether a global opt-in mode should discover and
  register all configured in-scope apps/workspaces, what config shape identifies
  those allowed apps without scanning unrelated user directories, whether a small
  local tray/menu-bar app should act as the human-visible vehicle for the daemon,
  how duplicate
  registration/heartbeat behaves, and how status surfaces list registered versus
  unregistered worktrees. The app option must be scoped as a daemon control
  surface only: start/stop, registered worktrees, protection state, recent
  fences, and registration prompts, not a separate product UI. If the design is
  accepted, split implementation into separate Ready work items before treating
  this as `v0.9.0-beta` cut-line scope.
- **Validation:** Planning council review plus a proposed test matrix covering:
  start outside a worktree, register current worktree, register an explicit path,
  guided setup of automatic `anvil start --no-mcp` for newly-created worktrees,
  global opt-in discovery limited to configured in-scope apps/workspaces,
  duplicate registration heartbeat, fenced/cascaded refusal, multiple worktrees on
  one daemon, local-app mediated registration if selected, and a newly-created
  Worktrunk worktree that becomes protected without a visible watch terminal.
- **Files:** `crates/anvil-cli/src/activation/daemon_registration.rs`,
  `crates/anvil-cli/src/commands/{start,workspace,intercept,status}.rs`,
  `crates/anvil-intercept/src/{registry,ipc,status}.rs`, Worktrunk/activation docs
  if the design is accepted.
- **Dependencies:** ACTMO-002 (MCP-independent registration), ACTMO-006 (default
  save-time armed posture), DSV-046 (headless background save-time driver
  contract).
- **Confidence:** medium — the daemon registration primitive exists, but the
  user/agent workflow and outside-worktree semantics need an explicit contract.
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

### ACTMO-014: Durable registration primitive

- **Status:** In Progress
- **Source:** ACTMO-013 design + planning council (2026-06-29). The registry is
  in-memory with a 30s heartbeat TTL (`crates/anvil-intercept/src/registry.rs:69-72`)
  and reloads only fences on startup (`crates/anvil-intercept/src/lib.rs:1307-1324`),
  so a one-shot registration evicts ~30s after the process exits and a daemon
  restart drops the whole set. Keystone for the rest of the split.
- **Intent:** Make activation-tagged worktree registration durable membership,
  not a 30s session lease.
- **Expected Outcome:** Activation-tagged (`claimed_agent_id:"activation-spine"`)
  registrations are persisted to a durable store under `ANVIL_HOME`, exempt from
  the 30s eviction, and reloaded on daemon startup before accepting connections
  (analogous to fence loading), with an INFO "registered N worktrees on startup".
  Live `anvil-run` sessions keep the existing lease. A reaper drops + reports
  registered paths whose directory is gone; distinct registered worktrees are
  capped (default 64, configurable). The `pub(super)` primitive
  (`activation/daemon_registration.rs:26`) moves to a shared `registration`
  module; the client classifies daemon errors (`WorktreeAlreadyOwned` → heartbeat
  the existing owner; `WorktreeFenced`/`WorktreeCascaded` → point at
  `anvil intercept unblock`; `SessionCapExceeded` → cap message) and uses
  `dunce::canonicalize` with server-authoritative identity (verify returned
  worktree matches before treating a result as a heartbeat). A "registerable
  worktree" helper resolves via `git rev-parse` (reject bare + `.git`-internal;
  accept linked + submodule worktrees). The registry emits a membership-change
  signal (register/unregister/reaper) for DSV-046's driver to subscribe to.
- **Validation:** register then wait > 30s (still registered); restart the daemon
  (set reloaded); `git worktree remove` a registered dir (dropped + reported);
  register past the cap (clear error); same path via a different spelling/symlink
  (`WorktreeAlreadyOwned` → heartbeat, not `Rejected`); Windows display paths free
  of `\\?\`; bare/`.git`-internal rejected, submodule/linked accepted.
- **Files:** `crates/anvil-cli/src/activation/daemon_registration.rs`
  (→ shared `registration` module), `crates/anvil-intercept/src/{registry,lib,ipc}.rs`,
  `crates/anvil-intercept-proto/src/status.rs`
- **Dependencies:** ACTMO-002 (MCP-independent registration); coordinates with
  DSV-046 (membership-change signal consumer).
- **Confidence:** medium — the primitive exists; persistence + TTL-exemption +
  reload + reaper + cap are new daemon-side semantics.
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

### ACTMO-015: `anvil workspace register` / `unregister` command

- **Status:** In Progress
- **Source:** ACTMO-013 design D1. `commands/workspace.rs` is config-only today
  (no daemon IPC); there is no command to register a later/other worktree.
- **Intent:** Add the explicit registration surface on the `anvil workspace` noun.
- **Expected Outcome:** `anvil workspace register [PATH]` (PATH defaults to cwd;
  explicit path registers a later/other worktree) and `anvil workspace unregister
  [PATH]` (idempotent) call the durable primitive (ACTMO-014). `anvil workspace
  list` becomes a config↔registry join: degraded behaviour when the daemon is
  unreachable (config half renders; registered half shows "daemon unavailable"),
  and the join canonicalises allowlist paths via `dunce` before comparing to
  registry keys.
- **Validation:** register no-path in a worktree (`Registered`); register an
  explicit path without changing cwd; unregister is idempotent; `list` with the
  daemon down still renders the config half; re-register reports `Refreshed`.
- **Files:** `crates/anvil-cli/src/commands/workspace.rs`,
  shared `registration` module
- **Dependencies:** ACTMO-014
- **Confidence:** high
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

### ACTMO-016: Outside-worktree `anvil start` honest behaviour

- **Status:** In Progress
- **Source:** ACTMO-013 design D2. `anvil start` always registers cwd
  (`daemon_registration.rs:26` called with `"."`); outside a worktree it would
  register a junk session keyed to e.g. `$HOME`.
- **Intent:** Make `anvil start` honest and non-fatal when cwd is not a
  registerable worktree.
- **Expected Outcome:** When cwd is not a registerable worktree, `anvil start`
  ensures the per-user daemon (unless `--no-daemon`/`ANVIL_NO_DAEMON`), does not
  register cwd, reports "daemon ready; no worktree registered (run from inside a
  worktree, or `anvil workspace register <path>`)", and exits 0 — an honest state
  distinct from `protecting`. Uses the shared registerable-worktree helper
  (ACTMO-014).
- **Validation:** `anvil start` in a non-worktree dir (exit 0; daemon ensured;
  guidance shown; cwd not registered); `--no-daemon` skips daemon ensure with a
  message; bare repo / cwd inside `.git` rejected; submodule + linked worktree
  accepted.
- **Files:** `crates/anvil-cli/src/commands/start.rs`,
  `crates/anvil-cli/src/activation/orchestrator/mod.rs`, shared `registration` module
- **Dependencies:** ACTMO-014 (registerable-worktree helper)
- **Confidence:** high
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** patch

### ACTMO-017: Registered-worktree status surfacing

- **Status:** In Progress
- **Source:** ACTMO-013 design D6. `anvil status` plain text omits the worktree
  list though `--json` carries it; `anvil intercept stop` reports nothing about
  worktrees losing protection.
- **Intent:** Surface the registered set and its protection state truthfully.
- **Expected Outcome:** `anvil status` plain text lists registered worktrees on
  two axes — membership (`registered`/`fenced`/`cascaded`/`unregistered`, from
  `WorktreeStatusV1`) and assurance (`clean`/`stale`/`pending`/`running`/`bounded`/
  `unavailable`, parallel query) — and flags whether the current cwd is
  registered. The `protecting` vs `watching` label (ADR-092) is acknowledged as a
  wire/query addition (new `WorktreeStatusV1` field or per-worktree assurance
  query) with a fixed derivation table in this item. `anvil intercept stop`
  best-effort queries `session.list` and prints "stopping daemon; N worktree(s)
  registered" when N > 0; the daemon shutdown path emits one INFO event with the
  count.
- **Validation:** status after registering 2 worktrees lists both with both axes +
  cwd flag; `intercept stop` with N>0 prints the guidance and logs the count;
  evicted/absent sessions render as `unregistered`, never a phantom `stale`
  membership.
- **Files:** `crates/anvil-cli/src/commands/status.rs`,
  `crates/anvil-cli/src/commands/intercept.rs`,
  `crates/anvil-intercept-proto/src/status.rs`,
  `crates/anvil-cli/src/activation/render.rs`
- **Dependencies:** ACTMO-014; soft-dep DSV-046 (the `watching` label needs an
  attached driver)
- **Confidence:** medium — the `watching` label needs a wire/query addition.
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

### ACTMO-018: Bounded `anvil workspace register --all`

- **Status:** In Progress
- **Source:** ACTMO-013 design D5 (manual layer). A bounded "register everything
  in scope" without a filesystem scan.
- **Intent:** Register the operator's curated in-scope worktrees in one command,
  bounded strictly by the confinement allowlist.
- **Expected Outcome:** `anvil workspace register --all` registers the **exact**,
  allowlist-mode `allow` entries of `workspace.yaml` that are live, unfenced Git
  worktrees. Prefix entries are skipped with an explicit warning (walking them
  would be the forbidden scan); all skips (prefix/fenced/gone/not-a-worktree) are
  reported, not silent; `open` mode reports "no allowlist entries (confinement
  mode: open)"; `--no-daemon`/`ANVIL_NO_DAEMON` bypasses with a message;
  per-entry progress is printed so a large allowlist does not read as a hang.
- **Validation:** `--all` over a mixed allowlist (only exact live unfenced
  registered; prefix/fenced/gone reported); `--all` in `open` mode (honest
  message); `--all --no-daemon` (skipped + message); no filesystem walk performed.
- **Files:** `crates/anvil-cli/src/commands/workspace.rs`,
  `crates/anvil-intercept/src/confinement.rs` (read-only enumeration)
- **Dependencies:** ACTMO-014, ACTMO-015
- **Confidence:** high
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

### ACTMO-019: Persistent `register_on_start` config + daemon startup registration

- **Status:** Proposed
- **Source:** ACTMO-013 design D5 (persistent layer). Council Critical-2: adding a
  field to the `deny_unknown_fields` `AllowEntry` makes an older daemon fail
  **closed** and collapse the confinement trust floor. Schema commitment needs
  owner sign-off.
- **Intent:** Let the daemon auto-register a curated set of in-scope worktrees on
  startup, durably, without coupling to the confinement admission schema.
- **Expected Outcome:** A **separate additive top-level key** `register_on_start:
  [paths]` in `workspace.yaml` (NOT a field on `AllowEntry`) with a config
  format-version bump and documented forward/back-compat. The daemon registers
  these entries on startup (atop ACTMO-014's persisted set). Confinement
  admission membership and registration membership remain distinct sets.
- **Validation:** `register_on_start` entries auto-register after daemon restart;
  an older daemon binary reading a `register_on_start`-bearing config does **not**
  fail closed (format-version handling); no filesystem scan.
- **Files:** `crates/anvil-intercept/src/confinement.rs`,
  `crates/anvil-intercept/src/lib.rs`, `crates/anvil-cli/src/commands/workspace.rs`
- **Dependencies:** ACTMO-014
- **Confidence:** medium — depends on the config format-version forward-compat
  decision and owner sign-off on the schema commitment.
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

### ACTMO-020: Guided new-worktree auto-registration

- **Status:** Proposed
- **Source:** ACTMO-013 design D7. Git has no native post-`worktree add` hook;
  no Worktrunk integration exists today.
- **Intent:** Give a guided, portable opt-in so newly-created worktrees register
  without re-running the full activation mental model.
- **Expected Outcome:** `anvil workspace install-hook` (its own subcommand, not a
  flag on `register`) installs a documented Git config alias pinned to a portable
  `sh` form (works under Git-for-Windows MinGW) and prints a PowerShell
  equivalent when Windows lacks `sh`; it never silently shims `git`. A Worktrunk
  post-create hook template is shipped only if Worktrunk exposes such a hook —
  design-gated pending confirmation of that surface.
- **Validation:** `install-hook` then `git wt-add` auto-registers the new worktree
  (Unix `sh` + Windows PowerShell forms); no silent `git` interception.
- **Files:** `crates/anvil-cli/src/commands/workspace.rs`, activation/Worktrunk docs
- **Dependencies:** ACTMO-015
- **Confidence:** low — Worktrunk hook surface unverified.
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

### ACTMO-021: Scoped local daemon-control app

- **Status:** Proposed
- **Source:** ACTMO-013 design D8. Operator review raised a human-visible daemon
  vehicle; the work item constrains it to a control surface only.
- **Intent:** Provide an optional, scoped local app as the human-visible vehicle
  for daemon control, deferred past the `v0.9.0-beta` cut.
- **Expected Outcome:** A small local tray/menu-bar app scoped strictly to daemon
  control: start/stop the daemon, list registered worktrees, show protection
  state and recent fences, prompt to register the current worktree — a thin
  client over the existing `query_status` / `session.list` / `session.register` /
  `session.unregister` / `unblock` IPC verbs. Explicitly **not** a findings/graph
  UI, config editor beyond register/unregister, or a separate product surface.
- **Validation:** app "register current worktree" issues the same
  `session.register` and the worktree appears in `anvil status`; app surfaces only
  control-plane state.
- **Files:** new app crate (TBD if accepted), reusing `crates/anvil-intercept-proto`
- **Dependencies:** ACTMO-014, ACTMO-017
- **Confidence:** low — deferred; needs an explicit owner decision to build.
- **changeType:** feature
- **releaseIntent:** deferred
- **releaseScope:** minor
