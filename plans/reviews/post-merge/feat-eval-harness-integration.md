# Post-merge: feat-eval-harness-integration

PR: #3013
Branch: `feat/EVAL`
APS: EVAL
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Validate EVAL-001: `cargo test -p eddacraft-anvil-policy -- eval_harness_port` (agent: yes)
- [ ] Validate EVAL-002: `cargo test -p eddacraft-anvil-policy -- eval_harness_adapter` (agent: yes)
- [ ] Validate EVAL-003: `cargo test -p eddacraft-anvil -- eval_regression_command` (agent: yes)
- [ ] Validate EVAL-004: `cargo test -p eddacraft-anvil-policy -- eval_result_persistence` (agent: yes)
- [ ] Validate EVAL-005: `cargo test -p eddacraft-anvil-policy -- eval_policy_guidance` (agent: yes)
- [ ] Flip EVAL-001..005 + module status to `Merged YYYY-MM-DD via PR #NNN` (agent: yes)
- [ ] Decide whether to wire `anvil policy eval-regression` into a CI workflow and
      author the trust-regression suite fixtures it runs (human required — needs a
      quiet box + owner decision on which suites gate, deferred from this PR)

## Notes

- The eval-harness substrate binds to the **frozen** `anvil policy eval --json`
  v1 wire contract (`docs/specs/policy-eval-output-v1.md`), not to
  `anvil-policy-engine` internal `Finding`/`Severity` types. The
  `eval_output_schema_stability_snapshot` test in anvil-cli still pins that
  contract — if it ever changes, the adapter's `SUPPORTED_SCHEMA_MAJOR` gate and
  fixtures must be revisited.
- `anvil policy eval-regression` is **report-only by default** (exit 0, ADR-002);
  `--fail-on-regression` makes it block. Wiring it as a *blocking* CI job is the
  one deferred step above: it needs real suite fixtures and an owner decision, and
  was kept out of this PR to avoid a flaky/empty gate.
