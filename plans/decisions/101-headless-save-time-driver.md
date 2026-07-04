# ADR-101: Headless Background Save-Time Driver

## Status

Accepted 2026-07-04 (operator)

## Date

2026-07-04

## Context

[ADR-092](092-mcp-optional-activation-spine.md) and ACTMO-006 require `anvil start`
to arm daemon-backed save-time validation without MCP or a visible `anvil watch`
terminal. [ADR-094](094-worktree-registration-ux.md) (ACTMO-013..020, merged)
delivered durable worktree registration and a registry `membership_hook` seam, but
the daemon still receives `validate_paths` only when a **foreground** `anvil watch`
client feeds changed paths.

The operator usefulness review (2026-06-29) and [RELEASE-PLAN](../../RELEASE-PLAN.md)
name this as the remaining `v0.9.0-beta` cut-line gap: honest registration without
unattended validation does not feel useful.

Constraints:

- [ADR-061](061-save-time-daemon-delta-validation.md): frozen `validate_paths` wire;
  watch is a thin daemon client with scoped fallback.
- [ADR-064](064-intercept-graph-cache-crate-boundary.md): resident daemon links
  `anvil-graph-cache` only — no `tree-sitter`, `notify`, or parser in the daemon
  hot path; `SymbolParser` is injected from the CLI at daemon startup.
- [ADR-082](082-daemon-lifecycle-user-startup.md): detached background processes
  use the DLIFE launcher pattern (`CREATE_NO_WINDOW` on Windows, log redirection).
- [ADR-094](094-worktree-registration-ux.md) decision 7: registry owns
  membership-change events; DSV-046 owns the driver consumer.

## Decision

Adopt **daemon-supervised detached CLI driver sidecars**:

1. A `SaveTimeDriverSupervisor` inside `anvil-intercept` subscribes to
   `SessionRegistry`'s `membership_hook` and manages **one detached
   `anvil watch --save-time-driver` child per durable registered worktree**.
2. The child reuses the existing `watch` → `watch_save_time` → `validate_paths`
   path (DSV-007). It runs headless/plain only — no TUI, no daemon offer/spawn.
3. Supervisor spawn/stop uses the same detached-launcher discipline as DLIFE-002
   (log file under `{ANVIL_HOME}/runtime/save-time-drivers/`, PID registry with
   `pid` + `pid_starttime`, startup reconciliation after persisted registration
   reload).
4. Wire addition: `WorktreeStatusV1.save_time_driver: attached | absent | failed`.
   ACTMO-017 assurance derivation: `registered ∧ driver_attached ∧ ¬mcp_live` ⇒
   user-facing `watching` with save-time **active** copy; `registered ∧ driver_attached
   ∧ mcp_live` ⇒ `protecting`.
5. Opt-out: non-empty `ANVIL_NO_SAVE_TIME_DRIVER` disables supervisor spawns;
   registration and daemon ensure are unaffected.
6. Findings without a terminal: per-worktree driver log (cut-line); Kindling
   `gate.evaluated` emission deferred to DPO-001.

## Rationale

In-daemon `notify` would violate ADR-064 or produce parser-less `Partial` verdicts.
Reusing the proven watch client preserves DSV-009 parity and minimises new hot-path
code in the daemon. Daemon-spawned sidecars (rather than only `anvil start`
spawning) cover `workspace register`, `register_on_start`, and post-restart
reconciliation.

### Alternatives considered

| Option | Pros | Cons |
| ------ | ---- | ---- |
| **Detached CLI sidecars (chosen)** | Reuses DSV-007; ADR-064 safe; matches DLIFE spawn pattern | Extra process per worktree; inotify budget |
| In-daemon `notify` | Single process | ADR-064 breach or permanent `Partial` verdicts |
| Copy downgrade | No implementation | Fails usefulness cut-line |
| `anvil start` foreground watch | Simple | Visible terminal; not headless |

## Consequences

### Positive

- `anvil start --no-mcp` can honestly mean unattended save-time validation.
- ACTMO-017 `watching` label becomes evidence-backed when driver is attached.
- Registry seam (ADR-094) has a concrete consumer.

### Negative

- One `notify` consumer per registered worktree — inotify pressure on large
  multi-worktree setups.
- Daemon → CLI re-exec spawn (same pattern as DLIFE; requires resolved anvil
  binary path).

### Risks

- Orphan driver processes after supervisor crash → mitigated by PID registry +
  startup reconciliation (DSV-047).
- ADR-031 latency regression → mitigated by reusing existing watch hot path; gate
  unchanged.

## References

- Design: [`plans/specs/2026-07-04-headless-save-time-driver-design.md`](../specs/2026-07-04-headless-save-time-driver-design.md)
- APS: DSV-046 (design Done), DSV-047..051 (Ready)
- Related ADRs: ADR-061, ADR-064, ADR-082, ADR-092, ADR-094