# ADR-079: Watch daemon-absent posture stays guidance-only

## Status

**Superseded** by [ADR-082](082-daemon-lifecycle-user-startup.md) — 2026-06-15.
Beta testing met the GA-revisit condition this ADR set (users persistently on
the fallback path because the daemon-backed path was harder than the fallback),
so the guidance-only posture is replaced by a tiered daemon lifecycle.

Originally **Accepted** — 2026-06-10, Josh (operator decision closing UJ-007).
Amended the [ADR-075](075-v080-graph-product-scope.md) rollout posture for
default-on save-time daemon routing; did not change any Accepted architecture.

## Date

2026-06-10

## Context

DSV-021 made `anvil watch` route save-time validation through a live daemon by
default, with **no auto-start** and a silent scoped-check fallback. UJ-007
asked whether watch should close the last gap between "default-on routing" and
"every user actually daemon-backed" by offering to start the daemon (TTY
prompt), auto-starting it, or staying guidance-only.

Since UJ-007 was filed, the guidance surface shipped:

- every onboarding-path ending names the next step, including `anvil start`
  (UJ-001, PR #2502);
- `anvil watch --help` documents the daemon, the `anvil start` prerequisite,
  and the `ANVIL_WATCH_DAEMON` values, and the daemon-absent fallback advisory
  names `anvil start` (UJ-006, PR #2501);
- `anvil status` always states the save-time posture, including an explicit
  off state naming `anvil start` (UJ-005, PR #2500);
- the install/upgrade message is "run `anvil start` or `anvil welcome`"
  (UJ-001/UJ-003).

## Decision

`anvil watch` stays **guidance-only** when no daemon answers: it warns once
per disconnect, names `anvil start` as the recovery step, and falls back to a
scoped check. No interactive offer-to-start prompt, no auto-start.
`ANVIL_WATCH_DAEMON` semantics are unchanged (unset = default-on-when-live,
`0` = opt out, `1` = force with warned fallback).

UJ-007 closes with this decision; no code changes.

## Rationale

The prompt's job — telling the user how to get daemon-backed validation — is
now done by every surface on the path, exactly once where it matters. An
interactive prompt would add a TTY/headless behavioural split, a new env/flag
surface to control it, and a consent question ADR-075 deliberately avoided
("NO auto-start" was a rollout-control invariant). UJ-007 itself named
guidance-only as an acceptable outcome once the beta messaging landed.

### Alternatives Considered

| Option | Pros | Cons |
| --- | --- | --- |
| Guidance-only (chosen) | Zero new surface; preserves ADR-075's no-auto-start invariant; guidance is already everywhere on the path | A user who ignores the advisory stays on the fallback path |
| One-time TTY offer-to-start prompt | Closes the gap interactively | New TTY/headless split + control surface; prompt fatigue; revisits ADR-075's consent posture for marginal gain over the shipped guidance |
| Auto-start | Maximum coverage | Spawns a persistent daemon without consent; explicitly ruled out by ADR-075 rollout controls |

## Consequences

- **Positive:** the daemon adoption story is carried by visible, testable
  copy on every surface; no new runtime behaviour to gate, test, or revert.
- **Negative:** daemon adoption depends on users following the guidance;
  watch sessions started before `anvil start` remain on the scoped-check
  fallback until restarted.
- **Risks:** if beta telemetry-free feedback shows users persistently stuck
  on the fallback path, this decision should be revisited at GA alongside the
  `cli.licence-gate` posture.
- **Mitigations:** `anvil status` makes the off/fallback posture visible at
  any time, so the gap is observable without telemetry.

## References

- Related ADRs: ADR-075 (rollout controls amended here)
- APS modules: UJ-007 (`plans/archive/modules/user-journey.aps.md`), DSV-021
- Shipped guidance: PRs #2500, #2501, #2502, #2503, #2504
