# Post-merge: feat-distrib-001-signature-verification

PR: #NNN
Branch: `feat/distrib-001-signature-verification`
APS: DISTRIB-001 (in module DISTRIB)
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Step 1 — Generate the production minisign keypair offline per
      `docs/runbooks/release-signing.md` §"One-time setup" (human required;
      the key must never leave a secure store). Owner: release maintainer.
- [ ] Step 2 — Add the base64-encoded private key as
      `secrets.ANVIL_MINISIGN_PRIVATE_KEY` in the `eddacraft/anvil` repo
      Settings → Secrets → Actions (human required).
- [ ] Step 3 — Add the public key base64 (just the second line of the
      `.pub` file, no comment header) as `vars.ANVIL_MINISIGN_PUBLIC_KEY`
      in repo Settings → Variables → Actions (human required).
- [ ] Step 4 — Smoke-test the `release-sign-artefacts.yml` workflow on a
      pre-existing release tag via `workflow_dispatch` and confirm
      `.minisig` files are uploaded (agent: yes — `gh workflow run
      release-sign-artefacts.yml -f tag=v0.6.2-beta` then `gh release view
      v0.6.2-beta --json assets` and assert at least one `*.minisig` is
      present).
- [ ] Step 5 — Update the v0.7.0-beta release notes to direct legacy-
      binary users to re-install via curl-installer or Homebrew before
      relying on `anvil update` for future hotfixes (bootstrap step, see
      `docs/runbooks/release-signing.md` §"Initial bootstrap"). Owner:
      release maintainer.
- [ ] Step 6 — Move DISTRIB module status from `In Progress` to `In
      Progress 1/5` (mark DISTRIB-001 Merged) once PR merges. The cleanup
      agent advances the module status; this entry is the trigger
      record. (agent: yes — read `plans/modules/distribution-and-update.aps.md`
      and update the progress count + DISTRIB-001 Status to Merged.)
- [ ] Step 7 — When `v0.7.0-beta` ships, confirm a downstream `anvil
      update` against the signed release works end-to-end on a clean
      macOS arm64 + Linux x86_64 machine (manual; this is the real-user
      validation the spec calls for). Owner: release maintainer or QA
      participant from N9 Boring-Week gate.

## Notes

- ADR-045 is the trust model; the runbook captures the operational steps.
- The `--insecure-skip-verify` flag is intentionally hidden from
  `--help`. The integration test asserts it stays hidden and that clap
  still parses it.
- The committed dev keypair under
  `crates/anvil-cli/tests/fixtures/minisign/` is test-only; the
  release-sign workflow refuses to sign when the production var still
  matches the committed dev public key (preflight in both
  `release-sign-artefacts.yml` and `release.yml`'s build-local job).
- This PR does not change Homebrew or sidecar install-path behaviour —
  those paths delegate to their respective package managers. Signature
  verification is enforced on the library-fallback path only, which is
  the documented scope per ADR-045 §"Out of scope for this ADR".
- DISTRIB-002 (`anvil version --check` advisory surface) will consume the
  `VerifiedArtefact::trusted_comment` parser; the `tag=` field is
  already exposed on the struct, no further plumbing needed.
