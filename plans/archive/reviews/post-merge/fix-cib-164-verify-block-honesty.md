# Post-merge: fix-cib-164-verify-block-honesty

PR: #3126
Branch: `fix/cib-164-verify-block-honesty`
APS: CIB-164 (module `continuous-improvement-backlog`)
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Step 1 — Reconcile CIB-164 status to `Merged YYYY-MM-DD via PR #3126` in
      `plans/modules/continuous-improvement-backlog.aps.md` (agent: yes). Left
      out of this PR deliberately — the CIB status cells are shared with
      sibling branches in the same wave and flipping here invites a rebase
      conflict (ADR-053 advisory counts: do not bump the module header or
      index `N/M` count in feature PRs).
- [ ] Step 2 — Manual transcript in a non-hook-installable location (agent:
      no, auth-gated in the agent shell): run `anvil start` in a directory
      without `.git` and confirm the `verify:` block prints **no** "L3/L4
      commit + push hooks" line. The unit test pins the orchestrator threading
      (`hooks_active=false` outside a repo), but the live transcript was not
      reproduced pre-merge because `anvil start` requires authentication in
      this environment.
- [ ] Step 3 — Manual transcript with a wired-but-not-restarted MCP client
      (agent: no): confirm `L0 mcp pre-write (pending — restart required)`
      renders at `RestartRequired`, and flips to active after the client
      restart.
- [ ] Step 4 — Manual transcript in an all-languages-unsupported repo (agent:
      no): confirm the `.ts` smoke recipe is replaced by `recipe: none …` and
      the closing next-step line no longer recommends `anvil watch`.

## Notes

Honesty-predicate shape worth keeping: read back the durable artefact (hook
marker on disk via `is_anvil_managed`) rather than trusting the install-action
enum — `skipped` conflates "already ours" with "someone else's, left alone",
and only the former is coverage. Follow-up CIB-166 (single next-step arbiter)
still needs the diagnostic `next:` and closing `Next:` lines reconciled; this
change only removed the unsupported-repo `anvil watch` contradiction from the
closing line.

Files touched by the code PR:

- `crates/anvil-cli/src/commands/hooks.rs` —
  `install_activation_hooks_silent` returns `Result<bool>` (both hooks
  anvil-managed on disk); 3 new tests.
- `crates/anvil-cli/src/activation/orchestrator/mod.rs` +
  `install.rs` — `InstallReport.hooks_active` threading.
- `crates/anvil-cli/src/commands/start.rs` — recipe reads
  `hooks_active`; pending-L0 label; unsupported-repo recipe/next-step
  suppression; 3 new tests.

Local gates run green on this branch:

- `cargo fmt --all --check`
- `cargo clippy -p eddacraft-anvil --all-targets -- -D warnings`
- `cargo test -p eddacraft-anvil first_run` / `hooks` / `start`
- `pnpm run format:check`
