# Post-merge: feat-mlp2-061-063-witness-policy

PR: #1602
Branch: `feat/mlp2-061-063-witness-policy`
APS: MLP2-061, MLP2-062, MLP2-063
Merged: 2026-05-16 (commit `4d233ee1`)
Verified: 2026-05-16 by cleanup-agent reconciliation

## Steps

- [x] Advance MLP2-061 to `Merged` in
      `plans/modules/multilayer-protection-v2.aps.md` (agent: yes)
- [x] Advance MLP2-062 to `Merged` in
      `plans/modules/multilayer-protection-v2.aps.md` (agent: yes)
- [x] Advance MLP2-063 to `Merged` in
      `plans/modules/multilayer-protection-v2.aps.md` (agent: yes)
- [x] Bump Group M progress counter to `6/6 (Complete)` in the module's
      `## Stats` table (PR #1604 advances the other three; this PR alone
      hit `3/6` but the cleanup sweep ran for both PRs together) and the
      `index.aps.md` mirror — module total `35/66 → 41/66`; index mirror
      `36/66 → 42/66` (agent: yes)
- [x] Confirm no production template-render path still calls
      `anvil hook pre-push`-style policy load without the bounded
      loader — `grep -rn "fs::read_to_string.*policy" crates/` matches
      only test scaffolding under `crates/anvil-cli/src/commands/baseline.rs`
      (tests reading policy files they just wrote, not real loaders).
      Production `load_policy` in `hook.rs` and `l4_validate.rs` go
      through `anvil_config::read_to_string_bounded` (agent: yes)

## Notes

This PR bundles three Council-flagged corrective items from the full-
codebase audit on 2026-05-15:

- **MLP2-061** (Critical) — `chain_head` and `commit_is_witnessed` in the
  pre-push hook now walk archive segments + active, so a witness rollover
  cannot let a fresh genesis be seeded on top of archived history.
- **MLP2-062** (Critical) — `anvil l4-validate` verifies the active +
  archive chain with `verify_chain_dag` before harvesting witnessed SHAs;
  a tampered or forged record now blocks instead of silently admitting
  the commit.
- **MLP2-063** (High) — both `load_policy` sites now go through
  `anvil_config::read_to_string_bounded`, which opens the file once,
  binds the size check to the fd (TOCTOU-resistant), and caps the read
  at `MAX_CONFIG_FILE_BYTES`.

`witness_paths` is now a single source of truth in
`crates/anvil-witness/src/paths.rs`; `hook.rs`, `l4_validate.rs`, and
`audit_chain.rs` all import the same helper so ordering can never drift
across the verifier and the trusted-set harvester.

Council quick review surfaced four findings before push (2 MAJOR + 2
MINOR); all four were addressed in this branch. See PR description for
the post-fix verification.
