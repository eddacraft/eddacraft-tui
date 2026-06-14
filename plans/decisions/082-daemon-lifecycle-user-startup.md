# ADR-082: User-facing daemon lifecycle startup

## Status

Proposed — 2026-06-14, shaped from operator feedback that `anvil start` and
`anvil watch` should make daemon-backed protection the normal path while keeping
an explicit opt-out.

If accepted, this ADR supersedes
[ADR-079](079-watch-daemon-guidance-only.md). Until then, ADR-079 remains the
live decision and `anvil watch` stays guidance-only when no daemon answers.

## Date

2026-06-14

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

If accepted, Anvil will add a user-facing daemon lifecycle layer:

1. `anvil start` becomes the canonical daily protection command. It configures
   protection, ensures or offers to ensure the per-user daemon is running, and
   reports the resulting protection state.
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

The concrete startup mode remains the main product choice this ADR must settle
before acceptance:

- **Prompt in TTY, no prompt in headless** — strongest consent posture, more
  ceremony in watch.
- **Auto-start by default, explicit opt-out** — best daily-user path, larger
  consent and operations surface.
- **`anvil start` auto-starts; `anvil watch` prompts in TTY and falls back in
  headless** — recommended compromise unless beta evidence demands a stronger
  default.

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
- APS module: [Daemon Lifecycle](../modules/daemon-lifecycle.aps.md)
