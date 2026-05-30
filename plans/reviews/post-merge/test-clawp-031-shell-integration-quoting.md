# Post-merge: test-clawp-031-shell-integration-quoting

PR: #NNN
Branch: `test/clawp-031-shell-integration-quoting`
APS: CLAWP-031
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Confirm `cargo test -p eddacraft-anvil-run --test shell_integration`
      passes on `main` post-merge — all 5 tests green, including the two new
      tests `sh_single_quote_neutralises_metacharacters` and
      `dispatcher_sources_from_a_path_containing_a_space` (agent: yes)
- [ ] Reconcile CLAWP-031 status in `plans/index.aps.md` and the clawpatch
      module to `Merged YYYY-MM-DD via PR #NNN` — deliberately omitted from this
      PR because the CLAWP-031 count cell is shared with sibling branches in
      this batch and editing it here causes index conflicts (agent: no — parent
      reconciles batch status)
- [ ] Close GitHub issue #1642 once the reconcile lands (agent: yes)

## Notes

This is a pure-code test-hardening fix for CLAWP-031 / issue #1642. The shell
integration tests in `crates/anvil-run/tests/shell_integration.rs` embedded the
wrapper script path unquoted into a `bash -c` script (`. {script}`), so a
checkout path containing a space or shell metacharacter would word-split or glob
and fail to source `anvil-run.sh`.

Fix: a `sh_single_quote` helper single-quotes the path (escaping embedded single
quotes via the canonical `'\''` sequence) and a `source_line` helper applies it
consistently at all three `. {script}` call sites.

Regression coverage:
- `sh_single_quote_neutralises_metacharacters` — unit-asserts spaces, globs,
  command substitution, and embedded single quotes are neutralised.
- `dispatcher_sources_from_a_path_containing_a_space` — copies the shipped
  wrapper into a temp dir named `anvil source dir`, sources the copy through the
  same code path, and asserts the dispatcher still produces the documented argv.

Red was proven before the fix: with `source_line` reverted to an unquoted
`path.display()`, `dispatcher_sources_from_a_path_containing_a_space` fails
because the spaced path never sources and the stub log is never written.

No production code changed — the wrapper script (`shell/anvil-run.sh`) and the
dispatcher behaviour are unchanged; only the test harness was hardened.

APS note: this PR intentionally does NOT touch `plans/index.aps.md` or the
clawpatch module per the batch policy (shared count cell). Only this per-branch
post-merge doc was added under `plans/reviews/post-merge/`.
