# GHCLIAUTH loop run (cycles 16–18): sweep for the pattern, not the named file

When a work item retires a *pattern*, grep for the pattern across the
codebase before trusting the item's named files. GHCLIAUTH-007 named only
`/admin/invite`, but `rg "INSERT INTO device_codes"` found `/admin/approve`
carrying the identical vestigial device-code generation + activate email —
same removal, recorded as a drift correction rather than a second item.

Other lessons that compounded this run:

- **Background implementation agents + parent-side verify/review/ship.**
  Dispatching an item to an autonomous agent in its own worktree (no push,
  no PR — local commit only) while the parent handles the previous item's
  merge/reconcile kept three items moving concurrently with zero collisions.
  The parent always ran a fresh-context Expected-Outcome verifier AND a
  code reviewer on the agent's commit before shipping; every cycle they
  caught real issues the implementer missed (per-poll info-log spam on the
  login hot path; latency measured through an unrelated network call;
  vacuous test assertions that sanitisation would mask).
- **Hygiene-test teeth need mutation thinking**: a `not.toContain(secret)`
  assertion is vacuous if a sanitiser would mask the leak to `[REDACTED]`
  first — also assert the redaction marker itself never appears.
- **E2E cross-platform credential isolation**: set a temp `ANVIL_HOME`
  (DISTRIB-006 re-root, takes precedence everywhere) rather than relying on
  XDG vars Windows ignores.
- **PR monitors should emit on state change only** — a poll loop that
  re-prints "MERGED" every iteration spams the event stream.
