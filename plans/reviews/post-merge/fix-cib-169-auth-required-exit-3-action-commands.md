# Post-merge: fix-cib-169-auth-required-exit-3-action-commands

PR: <!-- filled at PR creation -->
Branch: `fix/cib-169-auth-required-exit-3-action-commands`
APS: CIB-169 (module `continuous-improvement-backlog`)
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Step 1 — Reconcile CIB-169 status from `In Progress` to
      `Merged YYYY-MM-DD via PR #NNN` in
      `plans/modules/continuous-improvement-backlog.aps.md` (agent: yes). The
      flip is deferred here per the CIB wave convention — the backlog file and
      `plans/index.aps.md` counts are shared with sibling branches and editing
      them in-PR causes rebase conflicts.
- [ ] Step 2 — Confirm the new tests run green in CI's Rust test job (agent:
      yes): `cargo test -p eddacraft-anvil --bin anvil` shows the
      `auth_required_*` unit assertions (action → 3, `status` → 0, probe → 3)
      and `cargo test -p eddacraft-anvil --test start` shows the shell-driven
      `start && echo reached` chain test passing.
- [ ] Step 3 — Real-world smoke on a logged-out install (agent: yes): in a
      fresh repo with no auth session, `anvil start && echo reached` must exit
      `3` and NOT print `reached`; `anvil status` must still exit `0`;
      `anvil whoami` must still exit `3`.
- [ ] Step 4 — Comment on issue #1822 noting its exit-0 mapping is superseded
      on action-command surfaces by CIB-169 (owner decision 2026-07-04), with a
      link to the merged PR (agent: yes).
- [ ] Step 5 — Confirm the CHANGELOG "Breaking (beta)" entry rides the next
      release notes so script authors see the exit-code contract change
      (agent: yes — verify presence at tag time; release owner drafts notes).

## Notes

Breaking-in-beta exit-code change: gated action commands (`start`, `init`,
`watch`, `gate`, `check`, `audit`, and siblings) now exit `3` on the
pre-dispatch auth wall instead of `0`. The `--json` `authRequired` envelope
shape is unchanged — only the exit code moved. Read-only surfaces keep their
contracts: `status` stays exit `0` (informational), `whoami` / `auth whoami`
stay exit `3` (state probes), and the `--verify` local-probe bypass is
untouched. The classifier is encoded as named predicates
(`is_auth_state_probe`, `is_read_only_auth_surface`) in
`crates/anvil-cli/src/main.rs` so future callers cannot silently re-broaden
the exit-0 coercion.
