# Release Signing — Operator Runbook

| Type    | Authority     | Owner  | Status | Freshness                                                                       |
| ------- | ------------- | ------ | ------ | ------------------------------------------------------------------------------- |
| Runbook | Authoritative | @aneki | Live   | Last verified 2026-07-13 against `.github/workflows/release-sign-artefacts.yml` |

| Upstream                                                                                                                                                                                                                                            | Downstream                                                                                                                                                                                                                                      |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [ADR-045](../../plans/decisions/045-update-signing-scheme.md), [DISTRIB-001 in `distribution-and-update.aps.md`](../../plans/archive/modules/distribution-and-update.aps.md), [ADR-044 §9](../../plans/decisions/044-mcp-entry-activation-owned.md) | [`.github/workflows/release-sign-artefacts.yml`](../../.github/workflows/release-sign-artefacts.yml), [`crates/anvil-cli/src/commands/update/signature.rs`](../../crates/anvil-cli/src/commands/update/signature.rs), v0.7.0-beta release notes |

Background: see [ADR-045](../../plans/decisions/045-update-signing-scheme.md).
This runbook covers the operational mechanics. Read the ADR first for the trust
model and rationale.

## One-time setup (before the first signed release)

Done once by the release maintainer. The private key generated here is
long-lived and must never leave a secure store.

### 1. Generate the keypair offline

On an air-gapped or trusted machine:

```sh
cargo install rsign2
echo "" | rsign generate -W -p anvil-release.pub -s anvil-release.key
```

You now have two files:

- `anvil-release.pub` — the public key (committed to the repo through config,
  never as the dev fallback).
- `anvil-release.key` — the private key. Treat it like the GitHub root signing
  key it effectively is.

Verify the key looks correct:

```sh
cat anvil-release.pub
# expected: two lines, the second starting with RW... (56 chars base64)
```

### 2. Store the private key

- Primary copy: paste the full file content into a 1Password entry under
  `eddacraft/anvil-release` vault. Tag with `signing-key`.
- Backup copy: encrypted USB held by the release maintainer.
- Working copy in GitHub: base64-encode the file and add as the org-scoped
  repository secret `ANVIL_MINISIGN_PRIVATE_KEY` on the `eddacraft/anvil` repo.
  Restrict the secret to `release-sign-artefacts.yml`.

```sh
base64 -w0 anvil-release.key | pbcopy  # macOS
base64 -w0 anvil-release.key | xclip   # Linux
# paste into: Settings → Secrets and variables → Actions → New secret
```

### 3. Publish the public key as a repo variable

The release-sign workflow reads `vars.ANVIL_MINISIGN_PUBLIC_KEY` and refuses to
sign if it is still the committed development fallback.

In Settings → Secrets and variables → Actions → Variables, create:

- Name: `ANVIL_MINISIGN_PUBLIC_KEY`
- Value: the base64 line from `anvil-release.pub` (the second line, no comment
  header)

### 4. Embed the public key in shipped binaries

`release.yml`'s `build-local-artifacts` job already injects
`ANVIL_RELEASE_PUBLIC_KEY: ${{ vars.ANVIL_MINISIGN_PUBLIC_KEY }}` into the build
environment, and a preflight step **fails the release** if the variable is empty
or still equals the committed dev fallback.

Provided steps 1–3 above are done before tagging, no further change is required.
After tag and push, `option_env!("ANVIL_RELEASE_PUBLIC_KEY")` in
`crates/anvil-cli/src/commands/update/signature.rs` resolves to the production
key at compile time, replacing the committed development fallback in every
shipped binary.

> **Note:** `allow-dirty = ["ci"]` in `dist-workspace.toml` preserves these
> manual edits to `release.yml` across `dist generate` runs. If a future
> `dist init` clobbers them, restore the env block and the preflight step
> (search for `DISTRIB-001 preflight` in this file), and also the pinned,
> checksum-verified rustup-init install in the same job (search for `CIB-120` in
> `release.yml`).

### 5. Securely wipe the local copy

After all three of (1Password, encrypted USB, GitHub secret + variable) are
populated:

```sh
shred -uvz anvil-release.key
```

You should never need the local copy again for routine releases. The release
workflow has it via the secret; routine signing happens in CI.

## Routine release

Once setup is complete, no operator action is required per release:

1. Cut the tag as usual (`anvil release …`).
2. `release.yml` runs cargo-dist and publishes the GitHub Release.
3. A successful tag-triggered `Release` workflow emits `workflow_run`, which
   starts `release-sign-artefacts.yml`. Pull-request runs are excluded before
   secrets are consumed. The signing workflow resolves the tag to its exact
   commit, signs every `*-installer.sh` / `*-installer.ps1` asset plus the
   provenance manifest, self-verifies the signatures, and uploads the `.minisig`
   files to both the private source release and the public `eddacraft/anvil`
   release.
4. Users on v0.7.0+ run `anvil update`; the library-fallback path verifies the
   signature against the embedded public key before replacing the running
   binary.

If automatic signing needs a deterministic retry, provide both the tag and the
successful tag-triggered Release run ID. The signer revalidates their SHA and
provenance binding before touching the private key or public release:

```sh
gh workflow run release-sign-artefacts.yml --ref main \
  -f tag=v0.9.0-beta \
  -f run_id=29190475570
```

## Verifying a release locally

```sh
gh release download v0.7.0-beta --pattern '*-installer.sh*'
cargo install rsign2  # if not already installed
cat > anvil-release.pub <<EOF
untrusted comment: anvil release public key
$ANVIL_MINISIGN_PUBLIC_KEY
EOF
rsign verify -p anvil-release.pub -x anvil-installer.sh.minisig anvil-installer.sh
```

## Key rotation

If you suspect the private key has leaked, **rotate immediately**:

1. Generate a fresh keypair offline (step 1 above).
2. Cut a minor version bump (e.g. v0.7.0 → v0.7.1) whose embedded public key is
   the new one. Users updating from N → N+1 cross the rotation boundary using
   N's still-trusted key.
3. Publish a security advisory under the prior release tag (see ADR-044 §9 for
   the advisory surface).
4. Revoke the prior `ANVIL_MINISIGN_PRIVATE_KEY` secret and
   `ANVIL_MINISIGN_PUBLIC_KEY` variable, then add the new pair.
5. Wipe the prior secret-key offline copies.

There is no transparency log; the rotation event is signalled by the
embedded-key change in the source-visible diff to `signature.rs` (when that path
is taken) or by the dist config change (recommended path).

## Failure modes

| Symptom                                               | Cause                                                                 | Action                                                                                                                                                                                                                          |
| ----------------------------------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `anvil update` errors "signature verification failed" | Artefact has been tampered with, or the embedded key is out of sync.  | Verify the release manually (above). If verification fails locally too, the release is compromised — pull it.                                                                                                                   |
| Release workflow logs "refusing to sign — dev key"    | Repo variable still set to the committed dev fallback.                | Run the One-time setup steps 1–3.                                                                                                                                                                                               |
| Signing rejects the decoded private-key structure     | The repository secret is not the complete two-line minisign key file. | Re-encode the full `anvil-release.key` file from the secure store and replace `ANVIL_MINISIGN_PRIVATE_KEY`; do not pad or truncate the value to satisfy a byte count.                                                           |
| Users on legacy binaries cannot verify a new release  | Their embedded key is the dev fallback or an old key.                 | They can update through Homebrew / curl-installer (own verification), then on-board to signed updates from there. The first signed release MUST be reachable through curl-installer / Homebrew — see "Initial bootstrap" below. |

## Initial bootstrap

The first signed release (v0.7.0-beta) is special: every pre-existing binary has
either the committed dev fallback or no key at all. Those binaries will hit the
dev-key branch in `verify_pending_install` and emit the loud
`WARNING: This anvil binary was built without ANVIL_RELEASE_PUBLIC_KEY` line on
stderr without verifying the signature.

To prevent users hitting this silent gap, the v0.7.0-beta announcement must
explicitly direct legacy-binary users to either:

1. Re-install from `install.eddacraft.ai` (curl-installer SHA-verifies
   internally), or
2. `brew upgrade eddacraft/tap/anvil` (Homebrew SHA-verifies via the formula).

Once on v0.7.0-beta, all subsequent `anvil update` calls land on the verified
path. Document the bootstrap step in the release notes.

## Build provenance manifest (#1217)

Every release publishes `anvil-<tag>-provenance.json` alongside the binaries (on
both the private `eddacraft/anvil-001` release and the public `eddacraft/anvil`
release). The manifest binds public artefacts to the private source commit and
tree they were built from, so downstream verifiers do not need to scrape
workflow logs to trust the binding.

### Schema (v1)

```json
{
  "schema_version": "1",
  "schema": "https://eddacraft.github.io/anvil/schemas/build-provenance-v1.json",
  "release_tag": "v0.7.0-beta",
  "built_at": "2026-05-15T11:18:17Z",
  "private_build": {
    "repository": "eddacraft/anvil-001",
    "commit_sha": "…",
    "ref": "refs/tags/v0.7.0-beta",
    "workflow_run_id": "…",
    "workflow_run_attempt": "1",
    "workflow_run_url": "https://github.com/eddacraft/anvil-001/actions/runs/…"
  },
  "public_release": {
    "repository": "eddacraft/anvil",
    "tag": "v0.7.0-beta",
    "ref_at_publish": "…"
  },
  "build_matrix": [{ "runner": "…", "targets": ["…"] }],
  "assets": [{ "name": "…", "sha256": "…", "size_bytes": 0 }]
}
```

### Trust binding

The manifest is signed by the same key chain as the installer scripts:
`release-sign-artefacts.yml` produces an `anvil-<tag>-provenance.json.minisig`
sidecar and uploads it next to the manifest. Verification:

```sh
rsign verify -p anvil-release.pub \
  -x anvil-v0.7.0-beta-provenance.json.minisig \
  anvil-v0.7.0-beta-provenance.json
```

Operators who want a stronger guarantee about which private commit was used can
re-build from that commit and compare the recorded SHA-256s to the locally
produced ones.

## References

- [ADR-045: Update Signing Scheme — Minisign](../../plans/decisions/045-update-signing-scheme.md)
- [DISTRIB-001 — Harden `anvil update` Resolution Chain And Signature Verification](../../plans/archive/modules/distribution-and-update.aps.md)
- [`.github/workflows/release-sign-artefacts.yml`](../../.github/workflows/release-sign-artefacts.yml)
- [`crates/anvil-cli/src/commands/update/signature.rs`](../../crates/anvil-cli/src/commands/update/signature.rs)
