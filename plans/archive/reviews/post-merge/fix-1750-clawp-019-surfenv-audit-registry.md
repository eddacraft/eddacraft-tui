# Post-merge: fix-1750-clawp-019-surfenv-audit-registry

PR: #2065
Branch: `fix/1750-clawp-019-surfenv-audit-registry`
APS: CLAWP-019
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Close GitHub issue #1750 — `Closes #1750` trailer on the PR should
      auto-close it; confirm it actually closed (agent: yes)
- [ ] Advance CLAWP-019 to `Released/Shipped` once a tagged release includes
      the merge commit (agent: yes)

## Notes

CLAWP-019 was a test-gap finding: `surfenv_suppression_audit.rs` hard-coded the
four SURFENV rule IDs, so a future `SURFENV-005` could be added without a
suppression-audit case and silently bypass the audit. The fix adds a
`SURFENV_RULES` registry constant in `crates/anvil-checks/src/surface/env/mod.rs`,
drives the existing shape check from it, and adds an exhaustiveness trip-wire
(`every_registered_rule_has_a_suppression_case`) that fails when a registered
rule lacks a suppression case. Pure test-hardening plus one public constant; no
runtime behaviour change. Non-vacuity was proven locally by temporarily
injecting `SURFENV-999` and confirming the trip-wire fails.
