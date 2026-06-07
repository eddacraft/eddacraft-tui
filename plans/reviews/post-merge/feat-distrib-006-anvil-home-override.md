# Post-merge: feat-distrib-006-anvil-home-override

PR: #2185
Branch: `feat/distrib-006-anvil-home-override`
APS: DISTRIB-006
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Advance DISTRIB-006 In Progress → Merged in
      `plans/archive/modules/distribution-and-update.aps.md` + `plans/index.aps.md`
      (`Merged YYYY-MM-DD via PR #2185`). Module count stays 5/6 until the item
      is Released/Shipped on the `v0.7.4-beta` tag. (agent: yes)
- [ ] Confirm DISTRIB-006 rides the `v0.7.4-beta` cut per
      `RELEASE-PLAN.md` "Save-Time CPU & Daemon Arc" Tier 1 (co-freight with the
      RLB CPU fix). Advance to Released/Shipped only when that tag ships. (agent: no — release-gated)
- [ ] **Operator follow-up (deferred, tracked):** Windows named-pipe re-root so
      two candidate daemons coexist on Windows (the PID file re-roots today; the
      socket is Unix-first). Out of scope for this PR; file a follow-up item if a
      Windows side-by-side need appears. (human required)
- [ ] **Operator follow-up (deferred):** `anvil uninstall --global` does not yet
      clean `<ANVIL_HOME>/user/`; the documented teardown is `rm -rf <prefix>`,
      which covers it. Low priority. (human required)

## Notes

The write-guard is implemented per-command (16 chokepoints) rather than at a
single write primitive (the codebase has no shared project-write helper to gate
centrally). **Invariant for future work:** any new command that performs a
durable per-project write (under `<root>/.anvil/`, `<root>/anvil/`, `.anvilrc`,
`.anvil.<ext>`, `.git/hooks`, `.husky`, `plans/`) MUST route through
`crate::install_root::ensure_project_write_allowed(...)` (refuse) or guard on
`project_writes_gated()` (skip), or it reopens the silent-corruption vector
ADR-060 closes. The `crates/anvil-cli/tests/anvil_home.rs` matrix is the
regression anchor.

ADR-060 is Accepted; this PR is its implementation.
