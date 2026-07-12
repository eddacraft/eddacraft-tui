# Daemon Lifecycle Implementation Plan

**Goal:** Make daemon-backed protection the normal `anvil start` and `anvil watch`
path while preserving explicit opt-out and non-interactive safety.
**Architecture:** Add a user-facing lifecycle layer above the existing intercept
daemon and ADR-061 save-time validation wire. The layer ensures or offers daemon
startup for product surfaces without changing verdict semantics, graph backing, or
the scoped fallback.
**Tech Stack:** Rust CLI (`crates/anvil-cli`), intercept daemon transport
(`crates/anvil-intercept*`), APS/ADR docs, public Markdown docs.

---

## File Map

- `plans/decisions/082-daemon-lifecycle-user-startup.md` — lifecycle decision and ADR-079 supersession path
- `plans/decisions/079-watch-daemon-guidance-only.md` — status update once superseded
- `plans/decisions/DECISION-LOG.md` — ADR index row for ADR-082 and ADR-079 status update
- `plans/archive/modules/daemon-lifecycle.aps.md` — APS authority for lifecycle work
- `plans/index.aps.md` — canonical module index row
- `crates/anvil-cli/src/commands/intercept.rs` — low-level daemon launch/status integration and shared ensure primitive entry point
- `crates/anvil-cli/src/commands/start.rs` — activation command lifecycle integration
- `crates/anvil-cli/src/commands/watch.rs` — watch command lifecycle integration and opt-out CLI surface
- `crates/anvil-cli/src/commands/watch_save_time.rs` — daemon routing semantics and assurance fallback helpers
- `crates/anvil-cli/src/activation/**` — protection-state rendering and daemon evidence flow if needed
- `docs/public/anvil/guides/save-time-validation.md` — primary user-facing save-time lifecycle guide
- `docs/public/anvil/quickstart.md` — first-run path
- `docs/public/anvil/beta-testing-guide.md` — beta scenario instructions
- `docs/public/anvil/operations/config.md` — daemon routing configuration
- `docs/public/anvil/operations/troubleshooting.md` — recovery and diagnostics
- `docs/public/anvil/integrations/watch-output.md` — stdout/stderr and watch contract notes
- `docs/public/anvil/integrations/mcp.md` — MCP daemon posture
- `docs/public/anvil/guides/agent-harness.md` — background watch/daemon guidance
- `docs/public/anvil/releases/changelog.md` — release note entry when implementation lands
- `docs/public/anvil/releases/upgrade-notes.md` — upgrade behaviour note when implementation lands

## Waves

| Wave | Actions | Gate |
| ---- | ------- | ---- |
| 1 | 1 | ADR accepted or implementation remains blocked |
| 2 | 2 | Idempotent daemon ensure tests pass |
| 3 | 3, 4 | `start` and `watch` lifecycle tests pass independently |
| 4 | 5 | Docs and APS validation pass |

### 1. Accept lifecycle decision

- **Work Item:** DLIFE-001
- **Checkpoint:** ADR-082 accepted and ADR-079 superseded
- **Validate:** `pnpm adr:check && pnpm aps:index:check`

### 2. Add daemon ensure primitive

- **Work Item:** DLIFE-002
- **Depends on:** 1
- **Checkpoint:** Concurrent ensure converges on one daemon; no-start contexts never spawn
- **Validate:** `cargo test -p eddacraft-anvil-intercept && cargo test -p eddacraft-anvil`

### 3. Wire `anvil start`

- **Work Item:** DLIFE-003
- **Depends on:** 2
- **Checkpoint:** `anvil start` reaches daemon-backed posture when allowed
- **Validate:** targeted activation/start/status tests

### 4. Wire `anvil watch`

- **Work Item:** DLIFE-004
- **Depends on:** 2
- **Checkpoint:** `watch` starts or offers daemon startup with opt-out preserved
- **Validate:** targeted watch daemon-routing tests

### 5. Align docs and help text

- **Work Item:** DLIFE-005
- **Depends on:** 1, 3, 4
- **Checkpoint:** User docs describe one lifecycle model
- **Validate:** `pnpm docs:check && pnpm docs:index:check && pnpm aps:index:check`

## Final Validation

Before PR, run:

```bash
pnpm format:check && pnpm lint:check && pnpm typecheck && pnpm test
cargo test --workspace
```
