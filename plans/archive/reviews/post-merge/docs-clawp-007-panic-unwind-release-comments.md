# Post-merge: docs-clawp-007-panic-unwind-release-comments

PR: #NNN
Branch: `docs/clawp-007-panic-unwind-release-comments`
APS: CLAWP-007 (status reconciled by parent — do NOT flip here)
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Context

CLAWP-007 / issue #1648 was a stale-doc correctness fix. The doc comments in
`crates/anvil-intercept/tests/midedit_contract.rs` and the module docs in
`crates/anvil-intercept-rules/src/registry.rs` asserted the workspace release
profile was `panic = "abort"`, so `catch_unwind` rule-panic isolation would be
a no-op in release. That premise is FALSE: root `Cargo.toml` `[profile.release]`
sets `panic = "unwind"` per ADR-051 (Accepted), precisely so panic isolation
holds in release. Comments were corrected; the previously-tracked abort-path
follow-up fixture (`daemon_aborts_on_rule_panic_in_release`) was marked OBSOLETE
because release no longer aborts.

This is a comment/doc-only change. No production behaviour changed; debug and
release are both `unwind`, so the existing unwind-path tests already cover the
release contract. No new test is meaningful.

## Steps

- [ ] No remaining `panic="abort"` / `panic = "abort"` release claims in the two
      intercept crates — `grep -rn 'panic *= *"abort"' crates/anvil-intercept
      crates/anvil-intercept-rules` returns only the corrected line stating
      release does NOT abort (agent: yes)
- [ ] `cargo test -p eddacraft-anvil-intercept --test midedit_contract` passes
      on merged main (agent: yes)
- [ ] `cargo test -p eddacraft-anvil-intercept-rules` passes on merged main
      (agent: yes)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean on merged
      main (agent: yes)
- [ ] Confirm root `Cargo.toml` `[profile.release]` still `panic = "unwind"`; if
      a future change reverts to `abort`, these comments must be revisited and
      the obsolete release abort fixture re-tracked (agent: yes)

## Notes

Issue #1648 recommended adding a multi-process release-profile abort fixture.
That recommendation is OBSOLETE under ADR-051 (release unwinds), and the doc now
says so. If ADR-051 is ever reversed, the abort-path contract becomes relevant
again and the comments in both crates must be reverted alongside it.
