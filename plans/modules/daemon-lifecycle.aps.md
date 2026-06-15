# Daemon Lifecycle

| ID    | Owner | Status      | Progress |
| ----- | ----- | ----------- | -------- |
| DLIFE | Josh  | In Progress | 2/6      |

**Last reviewed:** 2026-06-15 (DLIFE-001 Done — ADR-082 Accepted by operator with
the **tiered** startup mode: `anvil start` auto-starts the daemon; `anvil watch`
prompts in TTY and falls back in headless. ADR-079 superseded. DLIFE-002/-003/-004
unblocked to Proposed; **DLIFE-002 now flipped to Ready** — ensure-primitive design
pinned (probe → same-user lock → re-probe → detached spawn → bound-wait), cross-platform
risk split Unix-first (Windows background-launch follows DSV-010/011), and module
validation commands agreed, closing the last two Ready Checklist boxes. **DLIFE-006
Merged via #2639** — the terminating `--verify` diagnostic for the daemon-unreachable
case ships, module 2/6. Module created
2026-06-14 from operator direction that `anvil start` and `anvil watch` should make
daemon-backed protection the normal path, with an explicit opt-out.)

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

- [x] ADR-082 accepted or replaced by an accepted lifecycle decision (Accepted 2026-06-15)
- [x] Startup posture selected for TTY, headless, JSON, and CI-like contexts (tiered: `start` auto-starts; `watch` prompts in TTY, falls back headless/JSON/CI/MCP/hook)
- [x] Opt-out surface named and documented (`--no-daemon` + `ANVIL_WATCH_DAEMON=0`)
- [x] Cross-platform lifecycle risks accepted or split (Unix-first: Linux + macOS background launch in v0.8; Windows background-launch split to follow the existing save-time Windows gap DSV-010/011 — on Windows the ensure primitive returns a deterministic no-start and the scoped fallback is preserved until that path lands)
- [x] Validation commands agreed for CLI, docs, and APS checks (`cargo test -p eddacraft-anvil-intercept` for the ensure primitive; `cargo test -p eddacraft-anvil` for CLI wiring; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo fmt --all --check`; `pnpm docs:check` + `pnpm docs:index:check` for DLIFE-005; `pnpm aps:index:check` for APS bookkeeping)

## Work Items

### DLIFE-001: Accept daemon lifecycle startup decision

- **Status:** Done
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

- **Status:** In Progress
- **Intent:** Provide a safe internal `ensure_daemon` primitive that user-facing CLI commands (`start`, `watch`) call to bring up the per-user daemon, reusing a live one and never double-starting under concurrency.
- **Expected Outcome:** A typed ensure primitive returns one of `Reused` (a live daemon already answers the per-user endpoint), `Started` (exactly one daemon was launched and now answers), `NoStart{reason}` where `reason` is a **typed enum** with at least `OptOut`, `NonInteractive`, and `PlatformUnsupported` arms (so `start`/`watch` can render a platform-specific advisory distinct from a deliberate opt-out — a Windows user must not see the opt-out hint), or `Failed{recovery}` (launch or bind failed, with an actionable recovery hint). Repeated and concurrent calls across `start` and `watch` converge on exactly one daemon; stale sockets/PIDs are detected (endpoint present but no status answer) and recovered from **without unlinking a live-but-slow daemon's endpoint**; every non-`Started` path preserves the existing ADR-061 scoped fallback for watch and the activation honesty contract for start (no `Protecting` claim before the daemon attests).
- **Design:** The primitive lives in the `anvil-intercept` library (testable without the CLI) with a thin entry point in `intercept.rs`; `start.rs`/`watch.rs` consume the typed outcome only. Flow: **probe** the per-user save-time endpoint for a live status answer → if live, `Reused`; else acquire a same-user advisory lock around the spawn critical section so concurrent callers serialise (reuse the existing cross-platform `fs2` file-lock pattern at `lib.rs:620` — `flock` on Unix, `LockFileEx` on Windows — and scope the lock path **per-`ANVIL_HOME`**, not per-uid, so ADR-060 re-rooted instances of the same user do not share a lock) → re-probe under the lock (a racing caller may have started one → `Reused`) → otherwise spawn a detached background child running the existing `run_foreground` loop **with stdout/stderr redirected to a log file in the runtime/PID directory before exec** (the parent surface owns the terminal; `main.rs:46-53` flags DLIFE as owner of this capture story), then bound-wait — to a **named timeout constant**, not an implicit duration — for it to bind and answer the status verb → `Started`; a spawn that never binds within the timeout → `Failed{recovery}` naming the log path. Stale detection must distinguish a **dead** endpoint (connect fails fast) from a **live-but-slow** one (connect succeeds but no status answer within the probe timeout): only the former is unlinked and re-spawned, so a daemon under graph/GC load is never torn out from under its own listener. Startup is gated by an explicit caller capability flag, so headless/JSON/CI/MCP/hook/`--verify` callers get `NoStart` deterministically and never spawn or prompt. This retires the current `anvil intercept start` foreground-only bail (`intercept.rs` ~L974) for internal callers; the pinned `run_start_without_foreground_bails_with_actionable_message` test is updated to reflect that backgrounded launch now flows through `ensure_daemon` while the operator `--foreground` surface stays available.
- **Validation:** `cargo test -p eddacraft-anvil-intercept` (primitive: live reuse, absent→start, concurrent ensure converges on one daemon, dead-endpoint recovery, **live-but-slow endpoint is NOT unlinked** — inject a socket that accepts connections but never answers, no-start contexts, bind-timeout→`Failed`, **`#[cfg(windows)]` returns `NoStart{PlatformUnsupported}` deterministically rather than panicking**, daemon log file reachable after detach), then `cargo test -p eddacraft-anvil` (CLI entry wiring + updated bail/foreground contract); `cargo clippy --workspace --all-targets -- -D warnings`; `cargo fmt --all --check`.
- **Files:** `crates/anvil-intercept/src/**` (the `ensure_daemon` primitive, outcome type, same-user lock, stale-endpoint recovery, background spawn), `crates/anvil-cli/src/commands/intercept.rs` (thin CLI entry point; update the foreground-only bail + pinned test), `crates/anvil-cli/src/commands/start.rs` and `crates/anvil-cli/src/commands/watch.rs` (call sites consuming the typed outcome — full wiring lands in DLIFE-003/-004), related Rust tests
- **Scopes:** The internal ensure/launch primitive and its same-user lock, stale-endpoint recovery, and outcome type; the CLI entry point that exposes it.
- **Non-scope:** `anvil start`/`anvil watch` UX wiring and copy (DLIFE-003/-004); the `--no-daemon` CLI qualifier (DLIFE-004); Windows background launch (split — see Risks); any change to verdict semantics, graph backing, or the save-time wire.
- **Dependencies:** DLIFE-001 (Done)
- **Confidence:** medium
- **Risks:** First background daemon lifecycle in the codebase — the approach is now pinned (probe → same-user lock → re-probe → detached spawn → bound-wait), Unix-first. The sharpest correctness edge is stale detection: a live-but-slow daemon must not be mistaken for a dead endpoint and unlinked (pinned by the slow-endpoint test). Orphaned detached children (parent Ctrl-C'd mid-bound-wait) must not hold the lock/PID file open in a way that blocks the next `ensure_daemon` — verify the inherited descriptor is closed in the child. Windows background-launch is **split** to follow the existing save-time Windows gap (DSV-010/011): on Windows the primitive returns `NoStart{PlatformUnsupported}` and the scoped fallback is preserved until that path lands; council confirmed the existing `#[cfg]` free-function platform split (transports already split at `watch.rs:594/619`) means no abstraction seam is needed now. Non-interactive contexts must never spawn or hang — pinned by the no-start tests.
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

### DLIFE-003: Make `anvil start` manage daemon lifecycle

- **Status:** Proposed
- **Intent:** Make `anvil start` the canonical command that brings a normal user to daemon-backed protection when the accepted lifecycle posture allows it.
- **Expected Outcome:** `anvil start` configures protection, ensures the per-user daemon by default (tiered auto-start with explicit `--no-daemon` opt-out per the accepted ADR-082 posture), reports the resulting daemon-backed posture, and keeps `--verify` / `--json` non-mutating and non-interactive.
- **Validation:** Activation tests cover daemon absent, daemon live, daemon ensure failure, `--verify`, `--json`, and repair-hint rendering.
- **Files:** `crates/anvil-cli/src/commands/start.rs`, `crates/anvil-cli/src/activation/**`, `crates/anvil-cli/src/commands/status*.rs`, related start/status tests
- **Dependencies:** DLIFE-001, DLIFE-002
- **Confidence:** medium
- **Risks:** Activation already has careful honesty contracts; daemon lifecycle copy must not over-claim protection before the daemon attests the worktree.
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

### DLIFE-004: Make `anvil watch` start or offer daemon startup by default

- **Status:** Proposed
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

- **Status:** Merged 2026-06-15 via PR #2639
- **Intent:** When daemon attestation is `Unreachable`, `anvil start --verify` and `anvil status --verify` must give a terminating, actionable reason rather than silently parking at `ready_restart_required` as if another editor restart would help (recurring symptom: #2609, #2583, #1831).
- **Expected Outcome:** On `DaemonAttestation::Unreachable`, the rendered repair hint names *why* protection cannot graduate (no daemon answering the worktree) and the concrete next step to obtain a live daemon, and reads as an end state rather than a transient "restart again" loop. The `ReadyRestartRequired` headline is also made attestation-aware (via a shared `headline_for` selector routed through by both the human and `--json` surfaces) so the prominent first line no longer tells the user to "restart your editor" when no daemon is answering; the existing `state_explanation()` meaning line (#2590) is reused unchanged. `--verify` stays read-only and starts no daemon; `--json` keeps its stable key set (the `headline` value varies by attestation, as the schema already permits).
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
