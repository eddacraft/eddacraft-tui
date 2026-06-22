# Post-merge: feat-poleng-engine-core

PR: #1931
Branch: `feat/poleng-engine-core`
APS: POLENG
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Advance POLENG-002..007 from `In Progress` to `Merged <date> via PR #NNN`
      in `plans/archive/modules/policy-engine.aps.md` (agent: yes)
- [ ] Update the POLENG row in `plans/index.aps.md` (done count + the
      POLENG-001 skeleton note, which still reads "In Progress (2026-05-12)")
      to reflect 002..007 merged and POLENG-008 as the only open task
      (agent: yes)
- [ ] Confirm CI green on `main` after merge: `cargo test --workspace`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo fmt --all --check`, and the Docs Lint / oxfmt job (agent: yes)
- [x] POLENG-008 (bench parity gate vs Go OPA) — **Merged 2026-05-25 via PR #1942**.
      POLENG module is Complete per `plans/archive/modules/policy-engine.aps.md`.

## Notes

This PR implements POLENG-002 through POLENG-007: the `crates/anvil-policy-engine`
facade over `regorus` (input schema, determinism contract + Builtin trait,
first-party builtins, ADR-002/003 post-processing, coverage/trace) plus the
`anvil policy eval` CLI surface.

Known limitation carried forward (recorded on POLENG-006): regorus 0.10.0 does
not expose a structured rule-firing-order trace, so `EvalResult::trace()` and
`--why` surface the available query bindings only. Full trace is gated on an
upstream regorus capability.

`anvil policy eval` is a subcommand of the licence-gated `policy` group, so it
inherits that gate; it uses the existing `crate::output` JSON envelope (no
AIGUARD envelope type exists in the CLI yet).
