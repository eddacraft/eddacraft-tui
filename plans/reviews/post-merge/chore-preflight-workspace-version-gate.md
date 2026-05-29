# Post-merge: chore-preflight-workspace-version-gate

PR: #NNN
Branch: `chore-preflight-workspace-version-gate`
APS: CIB (release-tooling hardening; sourced from v0.7.1-beta pre-tag council D3)
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Step 1 — Run the preflight fixture suite on `main` to confirm the
      `cargo-version` gate is wired and deterministic:
      `bash scripts/release/_test/preflight.test.sh` exits `0` and prints
      `preflight.test.sh: ok` (agent: yes)
- [ ] Step 2 — Confirm the live gate passes against the current workspace
      version on `main`:
      `bash scripts/release/preflight.sh --json | node -e 'const j=JSON.parse(require("fs").readFileSync(0));const g=j.data.gates.find(x=>x.id==="cargo-version");console.log(g.status)'`
      reports `pass` (the workspace `Cargo.toml` version differs from the
      latest existing release tag and matches root `package.json`) (agent: yes)
- [ ] Step 3 — At the next release cut, exercise the gate end-to-end: invoke
      `bash scripts/release/preflight.sh --version <vX.Y.Z>` with the intended
      tag and confirm it fails loudly if the workspace `Cargo.toml` bump was
      forgotten, and passes once the bump lands. This is the real-world
      regression the gate exists to catch (human required — needs a live
      release cut)

## Notes

The gate (`require_workspace_version_match` in `scripts/release/preflight.sh`)
checks four conditions: workspace `Cargo.toml` carries a
`[workspace.package].version`; root `package.json` matches it; an optional
`--version vX.Y.Z` equals `v<workspace-version>`; and, even without `--version`,
the workspace version no longer equals the latest existing release tag (the
missing-bump case from issue #1871).

Steps 1-2 are agent-verifiable on `main` immediately after merge. Step 3 can
only be fully exercised during a live release cut, so it is flagged for human
attention — the cleanup agent should surface it rather than mark it verified.
