# Daemon Lifecycle

| ID    | Owner | Status   | Progress |
| ----- | ----- | -------- | -------- |
| DLIFE | Josh  | Proposed | 0/6      |

**Last reviewed:** 2026-06-15 (DLIFE-006 added from #2609 triage — a terminating
`--verify` diagnostic for the daemon-unreachable case; the one item independent of
ADR-082. Module created 2026-06-14 from operator direction that `anvil start` and
`anvil watch` should make daemon-backed protection the normal path, with an
explicit opt-out for users who do not want daemon startup.)

## Purpose

Make daemon-backed protection a normal user workflow rather than an operator-only
foreground daemon ceremony. `anvil start`, `anvil watch`, and `anvil status` are
the product surface; `anvil intercept start --foreground` remains the low-level
operator/debugging surface.

## In Scope

- The durable decision that supersedes ADR-079's guidance-only watch posture
- A race-safe, same-user daemon ensure/startup primitive for CLI surfaces
- `anvil start` daemon lifecycle integration
- `anvil watch` daemon lifecycle integration and explicit opt-out
- An honest, terminating `--verify` diagnostic when no daemon is reachable
- User-facing docs, help text, release notes, and runbook alignment

## Out of Scope

- Changing the ADR-061 `validate_paths` wire or assurance vocabulary
- Changing daemon verdict semantics, graph backing, or hot-path certifiability
- Replacing the scoped daemon-absent fallback
- System service installation for GA platforms unless required by the accepted ADR
- Cross-uid daemon sharing

## Interfaces

**Depends on:**

- [ADR-061](../decisions/061-save-time-daemon-delta-validation.md) — daemon-mediated save-time validation
- [ADR-075](../decisions/075-v080-graph-product-scope.md) — v0.8 rollout controls and default-on routing context
- [ADR-079](../decisions/079-watch-daemon-guidance-only.md) — current live posture to supersede if ADR-082 is accepted
- [ADR-082](../decisions/082-daemon-lifecycle-user-startup.md) — proposed lifecycle decision
- [daemon-save-time-validation](daemon-save-time-validation.aps.md) — DSV-021 routing semantics and fallback path

**Exposes:**

- A user-facing daemon lifecycle posture for `anvil start` and `anvil watch`
- A discoverable daemon opt-out qualifier
- Updated docs that stop teaching raw foreground daemon startup as the daily path

## Constraints

- UK English spelling in plan and docs
- `--verify` remains read-only and must not start a daemon
- `--json` remains machine-readable and must not prompt
- Non-interactive contexts must never hang waiting for consent
- Concurrent `start` / `watch` invocations must not create duplicate daemons
- Fallback remains scoped and honest when daemon startup is disabled or fails

## Ready Checklist

- [ ] ADR-082 accepted or replaced by an accepted lifecycle decision
- [ ] Startup posture selected for TTY, headless, JSON, and CI-like contexts
- [ ] Opt-out surface named and documented
- [ ] Cross-platform lifecycle risks accepted or split
- [ ] Validation commands agreed for CLI, docs, and APS checks

## Work Items

### DLIFE-001: Accept daemon lifecycle startup decision

- **Status:** Proposed
- **Intent:** Supersede ADR-079 with an accepted user-facing daemon lifecycle posture.
- **Expected Outcome:** ADR-082 is accepted or replaced by an accepted ADR; ADR-079 is marked superseded; the decision log states the new `start`/`watch` posture and the opt-out semantics.
- **Validation:** `pnpm adr:check`; `pnpm aps:index:check`; review confirms ADR-079 status and decision-log row are consistent.
- **Files:** `plans/decisions/082-daemon-lifecycle-user-startup.md`, `plans/decisions/079-watch-daemon-guidance-only.md`, `plans/decisions/DECISION-LOG.md`, `plans/modules/daemon-lifecycle.aps.md`, `plans/index.aps.md`
- **Dependencies:** None
- **Confidence:** medium
- **Risks:** Product consent posture may need operator choice before implementation can proceed.
- **changeType:** internal
- **releaseIntent:** hold
- **holdCondition:** Hold until ADR-082 is accepted.
- **releaseScope:** none

### DLIFE-002: Add idempotent daemon ensure primitive

- **Status:** Blocked
- **Intent:** Provide a safe internal way for user-facing CLI commands to ensure the per-user daemon is running.
- **Expected Outcome:** Repeated and concurrent ensure calls reuse the live daemon or start one daemon; stale sockets/PIDs produce actionable recovery; failures preserve the existing scoped fallback path for watch.
- **Validation:** Targeted Rust tests cover live reuse, absent start, concurrent ensure, stale endpoint recovery, and no-start contexts.
- **Files:** `crates/anvil-cli/src/commands/intercept.rs`, `crates/anvil-cli/src/commands/start.rs`, `crates/anvil-cli/src/commands/watch.rs`, daemon lifecycle helper modules as introduced by implementation, related CLI tests
- **Dependencies:** DLIFE-001
- **Confidence:** medium
- **Risks:** Foreground-only daemon launch is the current validated low-level mode; background lifecycle semantics need explicit design and tests.
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

### DLIFE-003: Make `anvil start` manage daemon lifecycle

- **Status:** Blocked
- **Intent:** Make `anvil start` the canonical command that brings a normal user to daemon-backed protection when the accepted lifecycle posture allows it.
- **Expected Outcome:** `anvil start` configures protection, ensures or offers daemon startup, reports daemon-backed posture, and keeps `--verify` / `--json` non-mutating and non-interactive.
- **Validation:** Activation tests cover daemon absent, daemon live, daemon ensure failure, `--verify`, `--json`, and repair-hint rendering.
- **Files:** `crates/anvil-cli/src/commands/start.rs`, `crates/anvil-cli/src/activation/**`, `crates/anvil-cli/src/commands/status*.rs`, related start/status tests
- **Dependencies:** DLIFE-001, DLIFE-002
- **Confidence:** medium
- **Risks:** Activation already has careful honesty contracts; daemon lifecycle copy must not over-claim protection before the daemon attests the worktree.
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

### DLIFE-004: Make `anvil watch` start or offer daemon startup by default

- **Status:** Blocked
- **Intent:** Close the fallback-only gap for bare `anvil watch` while preserving explicit opt-out and machine-readable behaviour.
- **Expected Outcome:** Bare `anvil watch` follows the accepted lifecycle posture when no daemon answers; `--no-daemon` and `ANVIL_WATCH_DAEMON=0` never start or prompt; `--json` and headless modes are deterministic and parse-safe.
- **Validation:** Watch routing tests cover live daemon, absent daemon with startup allowed, absent daemon with startup disabled, daemon startup failure, JSON mode, non-TTY mode, and forced/disabled environment values.
- **Files:** `crates/anvil-cli/src/commands/watch.rs`, `crates/anvil-cli/src/commands/watch_save_time.rs`, watch TUI/plain/JSON tests, public watch-output fixtures if help/output changes affect them
- **Dependencies:** DLIFE-001, DLIFE-002
- **Confidence:** medium
- **Risks:** Watch has stdout/stderr and NDJSON contracts; startup prompts or advisories must not pollute JSON stdout.
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

### DLIFE-005: Align docs, help text, and runbooks with daemon lifecycle

- **Status:** Blocked
- **Intent:** Replace guidance-only daemon startup instructions with the accepted user-facing lifecycle model.
- **Expected Outcome:** Public docs, CLI long help, beta testing guide, troubleshooting, and release notes all describe the same `start`/`watch`/opt-out behaviour and reserve `anvil intercept start --foreground` for operator/debugging use.
- **Validation:** `pnpm docs:check`; `pnpm docs:index:check`; `pnpm aps:index:check`; targeted help-text tests pass.
- **Files:** `docs/public/anvil/guides/save-time-validation.md`, `docs/public/anvil/quickstart.md`, `docs/public/anvil/beta-testing-guide.md`, `docs/public/anvil/operations/config.md`, `docs/public/anvil/operations/troubleshooting.md`, `docs/public/anvil/integrations/watch-output.md`, `docs/public/anvil/integrations/mcp.md`, `docs/public/anvil/guides/agent-harness.md`, `docs/public/anvil/releases/changelog.md`, `docs/public/anvil/releases/upgrade-notes.md`, CLI help text tests
- **Dependencies:** DLIFE-001, DLIFE-003, DLIFE-004
- **Confidence:** high
- **Risks:** Several docs currently state no auto-start; partial updates would create user-facing drift.
- **changeType:** docs
- **releaseIntent:** candidate
- **releaseScope:** minor

### DLIFE-006: Make `--verify` give a terminating reason when the daemon is unreachable

- **Status:** Proposed
- **Intent:** When daemon attestation is `Unreachable`, `anvil start --verify` and `anvil status --verify` must give a terminating, actionable reason rather than silently parking at `ready_restart_required` as if another editor restart would help (recurring symptom: #2609, #2583, #1831).
- **Expected Outcome:** On `DaemonAttestation::Unreachable`, the rendered repair hint names *why* protection cannot graduate (no daemon answering the worktree) and the concrete next step to obtain a live daemon, and reads as an end state rather than a transient "restart again" loop. `--verify` stays read-only and starts no daemon; `--json` stays machine-readable; copy reuses the existing `state_explanation()` surface (#2590) rather than adding a parallel path.
- **Validation:** Activation render tests cover the `Unreachable` repair-hint wording for `ReadyRestartRequired`, the `--verify` read-only contract (no daemon spawned), and `--json` shape stability; UK spelling check.
- **Files:** `crates/anvil-cli/src/activation/daemon_evidence.rs`, `crates/anvil-cli/src/activation/render.rs`, `crates/anvil-cli/src/activation/diagnostic.rs`, related activation tests
- **Dependencies:** None
- **Confidence:** high
- **Risks:** Activation honesty contracts are strict — the message must not imply protection is active, nor over-promise that starting a daemon is automatic. While DLIFE-003 is unshipped the actionable step is the operator surface (`anvil intercept start --foreground`); once DLIFE-003 lands the copy should re-point at `anvil start`. Independent of ADR-082 — this is honest diagnostics, not a posture change, so it can land ahead of the rest of the module.
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

## Action Plan

Detailed execution checkpoints live in
[`plans/execution/DLIFE.actions.md`](../execution/DLIFE.actions.md).
