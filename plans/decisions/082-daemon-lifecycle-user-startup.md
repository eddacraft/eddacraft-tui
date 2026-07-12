# ADR-082: User-facing daemon lifecycle startup

## Status

**Accepted** — 2026-06-15, Josh (operator decision). Beta testing showed the
guidance-only posture leaves normal users learning the raw foreground daemon
command, which pulls forward the GA-revisit condition ADR-079 itself set. The
tiered startup mode is settled (see Decision). This supersedes
[ADR-079](079-watch-daemon-guidance-only.md) and narrows the ADR-075
no-auto-start rollout invariant for the `anvil start` activation moment.

## Date

2026-06-15

## Context

[ADR-061](061-save-time-daemon-delta-validation.md) made save-time governance a
daemon-mediated validation path. DSV-021 then made `anvil watch` route through a
live daemon by default while preserving a scoped fallback when no daemon answers.
[ADR-079](079-watch-daemon-guidance-only.md) deliberately stopped short of an
offer-to-start prompt or auto-start because the beta guidance surface had just
landed and ADR-075's rollout controls avoided spawning a persistent daemon
without consent.

Beta testing has now exposed the product cost of that posture: a normal user who
wants to leave Anvil running for a few hours still has to understand the raw
daemon command and run a foreground daemon separately. That leaks the
implementation detail (`anvil intercept start --foreground`) into the daily
workflow and makes the best path harder than the fallback path.

The product surface should be `anvil start`, `anvil watch`, and `anvil status`.
The intercept daemon remains the implementation detail.

## Decision

Anvil will add a user-facing daemon lifecycle layer:

1. `anvil start` becomes the canonical daily protection command. It configures
   protection, ensures the per-user daemon is running, and reports the resulting
   protection state.
2. `anvil watch` uses daemon-backed validation by default. When no daemon
   answers, it follows the accepted startup posture from this ADR rather than
   only advising the user to run another command.
3. An explicit opt-out is always available. Existing
   `ANVIL_WATCH_DAEMON=0` remains honoured for watch routing, and a
   user-facing `--no-daemon` qualifier is added where the command surface needs a
   discoverable local override.
4. Headless and machine-readable modes never prompt or hang. `--json`,
   non-interactive stdin/stdout, CI-like contexts, MCP/hook invocations, and
   `--verify` probes use deterministic behaviour: either an idempotent
   non-interactive ensure path if allowed, or the existing scoped fallback with a
   clear advisory.
5. The low-level foreground command remains available for operators and service
   managers, but normal users are not expected to learn it.

### Settled startup mode

The startup mode is the **tiered posture**:

- `anvil start` **auto-starts** the per-user daemon (explicit opt-out via
  `--no-daemon`), because activation is the least-surprising moment to take
  daemon lifecycle responsibility.
- `anvil watch` **prompts in an interactive TTY** when no daemon answers, and
  **falls back to the scoped check in headless / `--json` / CI-like / MCP / hook
  contexts** — never prompting or hanging automation.

Considered and not chosen: *prompt in TTY for both* (most ceremony in watch for
no daily-path gain at the activation moment) and *auto-start both* (largest
consent and operations surface, strongest reversal of ADR-079 — kept in reserve
if beta evidence later shows the TTY prompt is still leaving watch users on the
fallback path).

## Rationale

The recommended tiered posture keeps `anvil start` as the explicit activation
moment where daemon startup is least surprising, while making `anvil watch`
helpful in interactive sessions without risking blocked automation. It preserves
ADR-061's honest scoped fallback and makes the opt-out visible before any
implementation work changes runtime behaviour.

### Alternatives Considered

| Option | Pros | Cons |
| --- | --- | --- |
| Keep ADR-079 guidance-only | No new lifecycle surface; preserves current consent posture | Continues leaking `anvil intercept start --foreground` into beta testing and normal use |
| Prompt from `watch` only | Consent-preserving; smaller than full lifecycle | Leaves `anvil start` unable to guarantee the normal protection path |
| Auto-start from both `start` and `watch` | Best user experience | Strongest reversal of ADR-079; needs the most operational hardening |
| Tiered posture: `start` auto-starts, `watch` prompts in TTY | Balances daily path and consent | More rules to document and test |

## Consequences

- **Positive:** users reach daemon-backed save-time validation through the same
  commands they already use; the raw intercept daemon surface becomes an
  operator/debugging tool rather than onboarding material.
- **Positive:** `anvil start --watch` can become a genuine one-command sustained
  use path after activation.
- **Negative:** Anvil gains daemon lifecycle responsibility: duplicate-start
  avoidance, stale PID/socket recovery, logs, shutdown instructions, and
  cross-platform launch semantics all become product behaviour.
- **Negative:** the old no-auto-start consent posture is reversed or narrowed,
  so docs, tests, and release notes must be updated together.
- **Risk:** headless prompts can hang automation. The implementation must pin
  non-interactive behaviour in tests.
- **Risk:** concurrent `anvil start` and `anvil watch` can race. The daemon
  ensure primitive must be idempotent and same-user scoped.

## References

- Supersedes if accepted: [ADR-079](079-watch-daemon-guidance-only.md)
- Builds on: [ADR-061](061-save-time-daemon-delta-validation.md),
  [ADR-075](075-v080-graph-product-scope.md)
- APS module: [Daemon Lifecycle](../archive/modules/daemon-lifecycle.aps.md)
