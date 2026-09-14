# Journey Simplification — One Continuous First-Run and Daily Experience

| ID | Owner | Priority | Status | Progress |
| -- | ----- | -------- | ------ | -------- |
| JSIMP | Josh | high | Done | 6/6 |

**Packages:** eddacraft-anvil, eddacraft-anvil-tui, @eddacraft/anvil-e2e

## Purpose

Own experience simplification after the reliability gate in [JREL](../archive/modules/journey-reliability.aps.md) and the pinned-build rehearsal in [JOURNEY](./release-user-journeys.aps.md). The operator requested reliability then simplification on 2026-09-05. This module plans execution; no new command semantics or completed implementation are claimed by intake.

## Scope and Boundaries

- Keep familiar welcome/start/bare/status/doctor roles, with a single continuous project journey and shared services.
- JSIMP-001 is the decision gate. Its Ready status authorises developing and accepting the contract through the existing ADR process, not silently changing existing public contracts. Downstream items are Ready only subject to Dependencies and the accepted decision.
- Start after JOURNEY-015 has verified reliability and recorded the release disposition. Publication need not block simplification once that gate passes; publication itself requires normal release authority. Do not invent an internal release channel.
- Separate user learning, project adoption, machine integration choices and ephemeral runtime evidence. Preserve progress, explicit consent, beta gating, optional MCP, pins and security/admission boundaries.
- Reliability fixes stay in JREL and existing owners. This module consumes their evidence and repair operations; it does not create another daemon/MCP state implementation.
- A splash, tutorial rewrite, always-on app and dashboard work are not assumed solutions. JOURNEY-013 owns observation; CIB-353 owns later curriculum work. Existing JOURNEY-009/-010 holds remain unchanged.
- Claim work items only at execution. No execution while a dependency or required ADR decision is unresolved.

## Interfaces

Presentation consumes the same typed project/component result in TUI, plain and machine paths. Action and inspection semantics are explicit and compatibility-tested. The closing receipt teaches daily bare anvil; detailed diagnosis remains accessible without being the normal route to working protection.

**Coordinates with:** [JOURNEY](./release-user-journeys.aps.md), [JREL](../archive/modules/journey-reliability.aps.md), [CIB](./continuous-improvement-backlog.aps.md), [ACTMO](./activation-mcp-optional.aps.md), [MCPLH](./mcp-live-heal.aps.md).

## Work Items

### JSIMP-001: Agree the continuous command journey and migration contract

- **Status:** Released/Shipped via v0.10.0-beta (bd6e4c98 · 2026-09-13)
- **Priority:** P1
- **Intent:** The simplified journey has one accepted public contract grounded in the repaired product.
- **Expected Outcome:** A decision record covers first-use bare routing before gated actions, optional welcome learning, shared setup via start, daily ensure, status/doctor roles and action-versus-output semantics. It accounts for ADR-044/080/082/092/103/114 and script compatibility, and records JOURNEY-013 first-user evidence before any additional splash/tutorial-depth choice. Current entitlement and consent policies are not silently overridden. The contract also records whether public `anvil intercept ensure` / `restart` exist as operator verbs, or whether bare `anvil` and `anvil mcp refresh --daemon restart` remain the only names.
- **Dependencies:** JOURNEY-015, JOURNEY-013
- **Coordinates with:** JOURNEY-013 owns current-build first-user observation; CIB-353 retains editorial ownership if promoted. This item owns contract decisions, not duplicate observation or an assumed splash.
- **Files:** `plans/decisions/145-continuous-command-journey.md`, `plans/specs/2026-09-10-continuous-command-journey.md`, `plans/archive/reviews/2026-09-10-jsimp-001-design-council.md`, `plans/decisions/DECISION-LOG.md`, `plans/decisions/114-bare-anvil-ensure-surface.md`, `plans/decisions/103-tty-default-activation-tui.md`, `docs/public/anvil/quickstart.md`, `docs/runbooks/cli-surface.md`, `docs/architecture/docs-delivery.md`
- **Validation:** `pnpm docs:check`; `pnpm aps:active-lint`; accepted ADR and transition matrix enumerate old/new behaviour and compatibility for TTY, plain, JSON, CI and signed-out entry.
- **Confidence:** medium — contract details are gated by JSIMP-001 and user observation.

### JSIMP-002: Separate actions and consent from output mode

- **Status:** Released/Shipped via v0.10.0-beta (bd6e4c98 · 2026-09-13)
- **Priority:** P1
- **Intent:** Changing presentation cannot silently change intended activation operations.
- **Expected Outcome:** The accepted JSIMP-001 contract governs TUI/plain/JSON consistently. Verification is explicit and non-mutating; unattended installs require explicit accepted intent; piping does not unexpectedly select new integration choices. Existing start --json semantics migrate explicitly with tested compatibility rather than changing silently.
- **Dependencies:** JSIMP-001
- **Coordinates with:** JREL-005 typed outcomes; ADR-103 output posture and ADR-114 daily ensure.
- **Files:** `crates/anvil-cli/src/main.rs`, `crates/anvil-cli/src/commands/start.rs`, `crates/anvil-cli/src/activation/orchestrator/`, `crates/anvil-cli/tests/`
- **Validation:** `cargo test -p eddacraft-anvil --no-fail-fast`; equivalent explicit intent across presentation modes yields equivalent mutations/results, while old scripted contracts follow the accepted migration.
- **Confidence:** medium — contract details are gated by JSIMP-001 and user observation.

### JSIMP-003: Share setup and resume across welcome, start and bare

- **Status:** Released/Shipped via v0.10.0-beta (bd6e4c98 · 2026-09-13)
- **Priority:** P1
- **Intent:** New users reach activation continuously and returning users resume without repeated onboarding.
- **Expected Outcome:** The agreed first-use bare flow offers setup or unsigned discovery without silent installation or premature auth; welcome delegates accepted setup to the same activation service as start. One project context and separate learning/adoption/runtime state persist throughout. Direct start needs no tutorial prerequisite; cancelled or deferred paths resume honestly.
- **Dependencies:** JSIMP-001, JSIMP-002
- **Coordinates with:** JREL-006/-007/-008 already repair progress, root and terminal faults; JOURNEY-012 pointer remains until deliberately migrated.
- **Files:** `crates/anvil-cli/src/main.rs`, `crates/anvil-cli/src/activation/diagnostic.rs`, `crates/anvil-cli/src/activation/orchestrator/mod.rs`, `crates/anvil-cli/src/commands/welcome.rs`, `crates/anvil-cli/src/commands/start.rs`, `crates/anvil-cli/src/commands/ensure.rs`, `crates/anvil-cli/src/services/first_run.rs`, `crates/anvil-cli/tests/bare_invocation.rs`, `crates/anvil-tui/src/surfaces/onboarding/`
- **Validation:** `cargo test -p eddacraft-anvil --no-fail-fast`; fresh welcome/start/bare, interrupted setup, unsigned discovery, subsequent repo and next-session fixtures pass without duplicate setup implementations.
- **Confidence:** medium — contract details are gated by JSIMP-001 and user observation.

### JSIMP-004: Remember integration intent and keep daily recovery quiet

- **Status:** Released/Shipped via v0.10.0-beta (bd6e4c98 · 2026-09-13)
- **Priority:** P1
- **Intent:** Daily use respects selected coverage while reconfiguration remains deliberate.
- **Expected Outcome:** Client, scope, executable and optional protection choices have one durable owner. Healthy bare ensure restores chosen coverage without pickers or needless rewrites; intentional omission/disablement stays distinct from failed installation. Start can deliberately reconsider choices. Ordinary recovery uses shared reliability operations and escalates to doctor only for unresolved faults. If JSIMP-001 accepts operator ensure/restart verbs, `doctor --fix` may invoke those shared operations for unresolved daemon-down faults; it is not the daily on-switch.
- **Dependencies:** JSIMP-001, JSIMP-003
- **Coordinates with:** JREL-009 protects current explicit launch overrides; this item provides unified selection/resume and does not reimplement its repair logic. JREL-005 names the public recovery command until this contract lands.
- **Files:** `crates/anvil-cli/src/activation/`, `crates/anvil-cli/src/activation/intent.rs`, `crates/anvil-cli/src/commands/ensure.rs`, `crates/anvil-cli/src/commands/start.rs`, `crates/anvil-cli/src/commands/doctor.rs`, `crates/anvil-cli/tests/save_time_driver_recovery.rs`, `crates/anvil-cli/tests/start.rs`, `crates/anvil-cli/tests/status_json_contract.rs`
- **Validation:** `cargo test -p eddacraft-anvil --no-fail-fast`; activate, decline/disable, repeat bare, add a client and reconfigure; assert selected intent, safe repair, no unexpected writes and one next action.
- **Confidence:** medium — contract details are gated by JSIMP-001 and user observation.

### JSIMP-005: Prove first value and provide a truthful closing receipt

- **Status:** Released/Shipped via v0.10.0-beta (bd6e4c98 · 2026-09-13)
- **Priority:** P1
- **Intent:** The user finishes setup knowing what works and what to do tomorrow.
- **Expected Outcome:** Selected MCP coverage is demonstrated through an actual supported client validation action; save-time coverage through a real saved fixture. Clean/no-supported-language outcomes stay honest and demos stay isolated. A persistent closing receipt names project, selected coverage, connected/pending client, policy mode, last proof and bare anvil for next use; incomplete setup names one actionable owner. Status and doctor consume the same facts.
- **Dependencies:** JSIMP-003, JSIMP-004
- **Coordinates with:** JREL-002/-003/-005 own evidence; reuse first-win/tutorial/value surfaces. CIB-353 depth cannot substitute for a real proof.
- **Files:** `crates/anvil-cli/src/activation/receipt.rs`, `crates/anvil-cli/src/commands/start.rs`, `crates/anvil-cli/src/commands/welcome.rs`, `crates/anvil-cli/src/commands/status.rs`, `crates/anvil-cli/src/commands/doctor.rs`, `crates/anvil-tui/src/surfaces/activation/`, `crates/anvil-tui/src/surfaces/status/`, `schemas/anvil-status.v1.json`
- **Validation:** `cargo test -p eddacraft-anvil --no-fail-fast`; `cargo test -p eddacraft-anvil-tui --no-fail-fast`; real-client/save-time rehearsal confirms receipt claims and next-session instruction in scrollback.
- **Confidence:** medium — contract details are gated by JSIMP-001 and user observation.

### JSIMP-006: Align public guidance and verify the simplified journey

- **Status:** Released/Shipped via v0.10.0-beta (bd6e4c98 · 2026-09-13)
- **Priority:** P1
- **Intent:** Installation help and everyday command guidance describe the same verified product behaviour.
- **Expected Outcome:** Installer copy, root help, quickstart, activation-state reference and troubleshooting reflect the accepted contract and identify release availability accurately. Platform/client journeys and new-user observation show completion without hidden developer switches. Navigation/accessibility regressions, terminal restoration, output compatibility and reliability fault tests remain covered. Record residual editorial feedback under its existing owner.
- **Dependencies:** JSIMP-002, JSIMP-003, JSIMP-004, JSIMP-005
- **Coordinates with:** JOURNEY-016 owns cross-module acceptance; JOURNEY-013 and CIB-353 retain observation/editorial scopes.
- **Files:** `install.sh`, `scripts/install.test.sh`, `crates/anvil-cli/src/main.rs`, `crates/anvil-cli/tests/air_gapped.rs`, `docs/public/anvil/quickstart.md`, `docs/public/anvil/guides/start-output-contracts.md`, `docs/public/anvil/operations/troubleshooting.md`, `docs/runbooks/anvil-air-gapped.md`, `docs/architecture/docs-delivery.md`, `apps/e2e/`
- **Validation:** `pnpm docs:check`; `pnpm --filter @eddacraft/anvil-e2e test:cli`; `pnpm --filter @eddacraft/anvil-e2e test:smoke`; JREL-012's no-skip command and recorded current-build user observation.
- **Confidence:** medium — contract details are gated by JSIMP-001 and user observation.
