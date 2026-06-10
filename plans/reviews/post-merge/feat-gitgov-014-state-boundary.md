# Post-merge: feat-gitgov-014-state-boundary

PR: #TBD
Branch: `feat/gitgov-014-state-boundary`
APS: GITGOV-014
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] `anvil init` in a fresh consumer repo seeds a `.gitignore` containing the
      wholesale `.anvil/` line plus `anvil/exceptions/.lock`, and NOT the
      legacy `.anvil/cache/` / `.anvil/gates.json` entries (agent: yes —
      `cargo test -p eddacraft-anvil init_gitignore`)
- [ ] `anvil doctor` in this repo reports the `state-boundary` check as
      `warn` listing exactly the four tracked `.anvil/` paths recorded in
      CIB-053 with a surgical `git rm --cached -- '<path>' …` remediation
      (agent: yes — build and run `anvil doctor --json`, inspect
      `checks[] | select(.name == "state-boundary")`)
- [ ] CIB-053 disposition of the dogfood repo's tracked `.anvil/` paths is
      picked up in a follow-up slice — once it lands, the dogfood
      state-boundary warn should report only recorded deviations
      (human required — scheduling decision)

## Notes

The dogfood warn is expected and honest: ADR-073 records the
`anvil/witness/` + `anvil/kindling/` ignore deviation, and CIB-053 tracks the
four tracked `.anvil/` paths the new check surfaced. Do not treat the warn as
a regression; it disappears only when CIB-053 is dispositioned.
