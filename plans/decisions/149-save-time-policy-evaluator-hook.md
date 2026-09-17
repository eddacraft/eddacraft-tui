# ADR-149: Save-time policy evaluator hook

## Status

Accepted

## Date

2026-09-17

## Context

Installed policy packs evaluate on MCP pre-write and at `anvil gate`, not on
the daemon save-time path. ADR-098 AD-4 deferred daemon save-time policy behind
a latency bench, a new ADR, and an ADR-067 injected-trait on-ramp.
`daemon_dep_boundary.rs` forbids linking `regorus` into `anvil-intercept`.

A 2026-09-17 MCP bench showed the extra cost of the shipped control-examples
pack is about 1 ms p50 / 2.6 ms p95 on a tiny file, well under the ADR-031
save-time SLO. OPAE-011 caches compiled engines in the CLI process. The
remaining product gap is editor saves and non-MCP writes.

## Decision

Inject policy evaluation into `validate_paths` the same way ADR-067 injects
symbol parsing.

- `anvil-intercept` defines `save_time::PolicyEvaluator` and never links
  `regorus` or `anvil-policy-engine`.
- `anvil-cli` implements the trait by calling the existing MCP pre-write
  evaluator (OPAE-011 cache included) and injects it via
  `ForegroundOpts::with_policy_evaluator` when starting the daemon.
- When the hook is present, `check_families` is `[antipattern, policy]`.
  Coverage (`certified` / `partial`) stays graph plus antipattern; policy
  findings are extra diagnostics. Save-time does not gain a new interrupt
  action in this slice — the write has already landed.
- `ANVIL_INTERCEPT_DISABLE_POLICY_EVALUATOR=1` withholds the hook
  (antipattern-only, as today).
- ADR-098 AD-4 crate forbid list is unchanged. This ADR answers open question 1
  (on-ramp timing): now, via the reserved hook.

## Rationale

Direct `anvil-intercept → regorus` still fails the ADR-071 feature-unification
lesson. The injected trait keeps the crate graph and puts eval in the process
that already links the engine. Latency is no longer the reason to skip
save-time. Interrupt-before-write stays MCP; save-time is the belt for humans
and bypassy agents.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| Injected trait (chosen) | Crate boundary holds; reuses MCP evaluator and cache | Hook absent in standalone `anvil-intercept` binary |
| Link regorus in the daemon crate | Simpler call path | Violates ADR-064/071/098; pulls Verus/jsonschema into intercept |
| Leave packs MCP-only | No save-time risk | Editor saves never see custom policy |

## Consequences

- **Positive:** Installed packs vote on daemon-backed save-time when the CLI
  starts the daemon.
- **Negative:** Standalone `anvil-intercept` without CLI injection stays
  antipattern-only (same as parser-less Partial).
- **Risks:** Older watch clients that cannot deserialise `CheckFamily::Policy`
  fail closed on the response. Same-binary ship is the supported pairing.
- **Mitigations:** Kill switch; fail-open evaluator; watch honesty names
  `antipattern+policy` when both families ran.

## References

- Related ADRs: ADR-061, ADR-067, ADR-098 AD-4, ADR-031
- APS modules: OPAE-023, OPAE-011
