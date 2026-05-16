# Post-merge: feat-mlp2-061-063-witness-policy

PR: <!-- filled on push -->
Branch: `feat/mlp2-061-063-witness-policy`
APS: MLP2-061, MLP2-062, MLP2-063
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Advance MLP2-061 to `Merged` in
      `plans/modules/multilayer-protection-v2.aps.md` (agent: yes)
- [ ] Advance MLP2-062 to `Merged` in
      `plans/modules/multilayer-protection-v2.aps.md` (agent: yes)
- [ ] Advance MLP2-063 to `Merged` in
      `plans/modules/multilayer-protection-v2.aps.md` (agent: yes)
- [ ] Bump Group M progress counter from `0/6` to `3/6` in the module's
      `## Stats` table and the `index.aps.md` mirror (agent: yes)
- [ ] Confirm no template-render path still calls
      `anvil hook pre-push`-style policy load without the bounded loader —
      `grep -rn "fs::read_to_string.*policy" crates/` should match nothing
      after this PR (agent: yes)

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
