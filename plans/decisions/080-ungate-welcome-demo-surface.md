# ADR-080: Ungate `anvil welcome` as the beta demo surface

## Status

**Accepted** — 2026-06-10, Josh (operator decision closing UJ-004). Adjusts
the beta `cli.licence-gate` posture for one command; the gate itself and its
"revisit at GA" stance are unchanged.

## Date

2026-06-10

## Context

Under the beta licence gate, `welcome`, `check`, `status`, `init`, `start`,
and `watch` are all in `CLI_GATED_COMMANDS` — so the first command any new
user runs is a login prompt before Anvil has shown anything. The v0.8.0-beta
user-journey review (UJ module) made `anvil welcome` one of the two explicit
golden paths ("discovery wow: see what Anvil finds in your own repo within
minutes"), and the install/upgrade message tells every user to run it. A
licence wall as the literal first interaction undercuts that path; during the
UJ loop run every manual transcript of a gated command hit
`Refresh token is invalid or revoked` before any output.

`anvil dashboard` is already ungated, demonstrating that read-mostly surfaces
can sit in front of the wall.

## Decision

Remove `welcome` from `CLI_GATED_COMMANDS`. `anvil welcome` runs without
authentication: the discovery scan, tutorial, and welcome hub are the demo a
new user can experience before signing in. Durable, ongoing-value surfaces —
`init`, `start`, `watch`, `check`, `status`, `gate`, and the rest of the
gated list — remain behind the licence gate.

## Rationale

The beta gate exists to control access to ongoing value, not to hide the
product. Moving the wall one step later — after the user has seen real
findings in their own repo — preserves access control on everything durable
while letting the discovery path deliver its wow. This is the smoothest
version of the gate-first posture the UJ-004 item contemplated, at a
one-line-of-metadata cost.

### Alternatives Considered

| Option | Pros | Cons |
| --- | --- | --- |
| Ungate `welcome` only (chosen) | Demo before the wall; durable surfaces stay gated; one-line change; mirrors the already-ungated `dashboard` | Welcome's guided onboarding can seed config without auth (see Consequences) |
| Affirm gate-first with smoother login | No access-surface change | The first interaction stays a login prompt; the discovery path's wow is paywalled before any value is shown |
| Ungate the whole read-only set (`check`, `status`, …) | Maximum demo surface | Gives away recurring daily value, not just discovery; effectively moves the beta gate to GA early |

## Consequences

- **Positive:** install → `anvil welcome` shows real findings with zero
  friction; the install/upgrade message ("run `anvil start` or
  `anvil welcome`") has an ungated branch; the welcome ending then points at
  the gated `anvil start`, putting the wall exactly where ongoing value
  begins.
- **Negative / accepted:** welcome's embedded guided-setup flow can write
  `.anvilrc` and the first-run marker without authentication — config
  seeding is not treated as gated value (running the gated `init` command
  directly still requires auth). Likewise, the welcome hub runs gate /
  audit / doctor data collection and a watch session **in-process**, so
  equivalents of gated commands are reachable interactively without auth.
  This is accepted for beta: the hub's one-off interactive runs are the
  demo; the gate is a client-side access funnel, not a security boundary,
  and the **scriptable** CLI commands (`gate`, `watch`, `audit`, `check`,
  …) remain gated for automation and daily use. All scans stay local; no
  licensing-relevant remote capability is exposed.
- **Risks:** if beta access control later needs to cover discovery itself,
  re-adding `welcome` to `CLI_GATED_COMMANDS` is the same one-line change.
- **Mitigations:** the GA revisit of `cli.licence-gate` (already on record)
  re-evaluates the whole list.

## References

- Related ADRs: ADR-080 complements ADR-079 (the other UJ design decision);
  flag host model per FLAGS-008 / FLAGCAT-005.
- APS modules: UJ-004 (`plans/archive/modules/user-journey.aps.md`).
- Shipped path context: UJ-001/-002/-003 (welcome path threading), PR #2500+.
