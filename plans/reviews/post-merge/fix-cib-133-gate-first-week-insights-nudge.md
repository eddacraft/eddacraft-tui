# Post-merge: fix-cib-133-gate-first-week-insights-nudge

PR: #3185
Branch: `fix/cib-133-gate-first-week-insights-nudge`
APS: CIB-133 (module `continuous-improvement-backlog`)
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Step 1 — Reconcile CIB-133 status from `In Progress` to
      `Merged YYYY-MM-DD via PR #NNNN` in
      `plans/modules/continuous-improvement-backlog.aps.md` (agent: yes). This
      was intentionally NOT done in this PR — CIB status flips collide on the
      shared module/index count cells when sibling CIB branches merge in the
      same window (ADR-053 advisory counts; reconcile with `pnpm aps:index`
      only if a refresh is needed).
- [ ] Step 2 — Confirm the new tests run in CI's Rust test job (agent: yes):
      `cargo test -p eddacraft-anvil first_week_hint` shows
      `insights::first_week_hint::tests::suppressed_when_project_writes_gated`,
      `commands::status::tests::first_week_hint_suppressed_when_project_writes_gated`,
      and `commands::watch::tests::first_week_hint_suppressed_when_project_writes_gated`
      passing alongside the carried-over INSIGHTS-004/-005 in-window tests.
- [ ] Step 3 — Live-run sanity check (agent: yes, needs a scratch repo and a
      side-by-side/candidate `ANVIL_HOME`): in a real first-week project,
      confirm `.anvil/insights-hint.json` under the real project root is
      untouched (mtime unchanged, once-per-week marker unconsumed) after
      running `anvil status` and `anvil watch` once under a gated
      `ANVIL_HOME` (e.g. `ANVIL_HOME=<candidate-root>`), then confirm the
      nudge still appears normally under the real, ungated `ANVIL_HOME` on
      the next run. This is the exact DISTRIB-006 / ADR-060 regression the
      fix targets and is not exercisable by the in-process unit tests, which
      construct `project_writes_gated` directly rather than going through
      `install_root::project_writes_gated()`'s real environment detection.

## Notes

Canonical-function gating: `first_week_insights_hint` now takes a
`project_writes_gated: bool` and returns `None` — with no read and no write —
at the top when gated, so `status`, `watch`, and `welcome` all inherit the
guard from one place instead of three separate call-site checks. Dropped
INSIGHTS-005's `welcome`-only `welcome_insights_hint` wrapper; `welcome.rs`
now calls the canonical function directly, same as the other two surfaces.

Files touched by the code PR:

- `crates/anvil-cli/src/insights/first_week_hint.rs` — new `project_writes_gated`
  parameter + gated-root early return + test.
- `crates/anvil-cli/src/commands/status.rs` — pass
  `install_root::project_writes_gated()`; gated-root test.
- `crates/anvil-cli/src/commands/watch.rs` — pass
  `install_root::project_writes_gated()`; gated-root test.
- `crates/anvil-cli/src/commands/welcome.rs` — drop the `welcome_insights_hint`
  wrapper; call the canonical function directly.
- `plans/modules/continuous-improvement-backlog.aps.md` — CIB-133 → In
  Progress.
- `plans/reviews/continuous-improvement-log.md` — CI log entry.

Local gates run green on this branch:

- `cargo fmt -p eddacraft-anvil --check`
- `cargo clippy -p eddacraft-anvil --all-targets -- -D warnings`
- `cargo test -p eddacraft-anvil first_week_hint` (8 passed, 0 failed)
