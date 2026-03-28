# Anvil CLI Release Runbook

Purpose: ship the Rust `anvil` binary safely and consistently via cargo-dist.

## Release policy (current)

- **Distribution:** pre-built binaries via GitHub Releases on
  `EddaCraft/anvil-releases` (public).
- **Install method:** shell installer script
  (`curl ... | sh`).
- **Targets:** x86_64 + aarch64 for Linux and macOS.
- **Workflow source of truth:** `.github/workflows/release.yml` (auto-generated
  by cargo-dist).
- **Configuration:** `dist-workspace.toml`.

---

## 1) Preflight checklist (required)

From repo root:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release -p anvil-cli
./target/release/anvil --help
./target/release/anvil --version
```

Verify TS workspace still builds (non-CLI packages):

```bash
pnpm install --frozen-lockfile
pnpm build
pnpm nx run-many -t test --skip-nx-cache
```

Sanity assertions before release:

- `crates/anvil-cli/Cargo.toml` version is correct.
- `CHANGELOG.md` has release notes.
- `docs/public/anvil/beta-testing-guide.md` version is current.
- `docs/public/anvil/releases/upgrade-notes.md` has a section for this version.

---

## 2) Promote dev to main

All day-to-day work lands on `dev`. Releases are cut from `main` after
promotion. See `docs/guides/branching-strategy.md` for the full model.

1. Ensure `dev` is green (CI passing, no known blockers).
2. Open a PR from `dev` to `main`.
   - Title convention: `release: vX.Y.Z`.
3. Once the release gate passes, merge the PR.

```bash
gh pr create --base main --head dev --title "release: vX.Y.Z" \
  --body "Promote dev to main for release vX.Y.Z"
```

---

## 3) Version, tag + GitHub Release

1. Switch to `main` and pull the merge:

```bash
git switch main && git pull
```

2. Bump version in `crates/anvil-cli/Cargo.toml`.
3. Update `CHANGELOG.md`.
4. Update `docs/public/anvil/beta-testing-guide.md` -- bump "Current version" and
   add any new feature areas to "What to Test".
5. Update `docs/public/anvil/releases/upgrade-notes.md` -- add a new section.
6. Commit and tag:

```bash
git add crates/anvil-cli/Cargo.toml CHANGELOG.md \
  docs/public/anvil/beta-testing-guide.md \
  docs/public/anvil/releases/upgrade-notes.md
git commit -m "chore(release): vX.Y.Z"
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin main
git push origin vX.Y.Z
```

Pushing the tag triggers `release.yml` (cargo-dist) which builds binaries for
all 4 targets and creates a GitHub Release automatically (pre-release for
beta/alpha/rc tags).

For beta releases, either format works:

```bash
vX.Y.Z-beta      # e.g. v0.3.0-beta
vX.Y.Z-beta.N    # e.g. v0.3.0-beta.0
```

After tagging, merge the version bump back to `dev` via PR:

```bash
gh pr create --base dev --head main \
  --title "chore: merge release vX.Y.Z back to dev" \
  --body "Sync version bump and changelog from release vX.Y.Z"
```

---

## 4) Monitor release workflow

Watch run in real time:

```bash
gh run list --repo EddaCraft/anvil-001 --limit 5
gh run watch <run-id> --repo EddaCraft/anvil-001
```

Or inspect a completed run:

```bash
gh run view <run-id> --repo EddaCraft/anvil-001 --log-failed
```

Expected behaviour:

- `plan` job succeeds and identifies the release.
- `build-local-artifacts` jobs compile for all 4 targets.
- `build-global-artifacts` job produces shell installer.
- `host` job creates the GitHub Release with all artefacts.
- `announce` job posts release notes.

---

## 5) Post-release verification (required)

Install on a clean machine (or container):

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/EddaCraft/anvil-releases/releases/latest/download/anvil-cli-installer.sh | sh

anvil --version
anvil doctor
anvil auth login
anvil gate
```

Verify all 4 platform binaries are present in the GitHub Release:

```bash
gh release view vX.Y.Z --repo EddaCraft/anvil-releases
```

Expected artefacts:

- `anvil-cli-aarch64-apple-darwin.tar.xz`
- `anvil-cli-x86_64-apple-darwin.tar.xz`
- `anvil-cli-aarch64-unknown-linux-gnu.tar.xz`
- `anvil-cli-x86_64-unknown-linux-gnu.tar.xz`
- `anvil-cli-installer.sh`

---

## 6) Fast incident playbook

### If login fails for testers

- Verify API health + auth endpoint.
- If needed as immediate fallback:

```bash
export ANVIL_API_URL=https://eddacraft-api.vercel.app
```

### If a binary is broken on one platform

1. Check the build log for that target in the release workflow.
2. Fix and cut a patch release (vX.Y.Z+1).

### If a bad version needs to be retracted

1. Delete the git tag locally and remotely:

```bash
git tag -d vX.Y.Z
git push origin :refs/tags/vX.Y.Z
```

2. Delete the GitHub Release:

```bash
gh release delete vX.Y.Z --repo EddaCraft/anvil-releases --yes
```

3. Fix the issue, bump to a new version, and re-release.

---

## 7) Known gotchas

- **cargo-dist PR mode:** PRs only run the `plan` job (no builds). Full builds
  only fire on version tags. This is configured in `dist-workspace.toml` as
  `pr-run-mode = "plan"`.
- **allow-dirty CI:** `dist-workspace.toml` has `allow-dirty = ["ci"]` so manual
  edits to `release.yml` (e.g. path filters) are preserved across
  `cargo dist init` re-runs.
- **Cross-compilation:** aarch64-linux uses cross-compilation in CI. If it
  fails, check the cross toolchain setup in the workflow.

---

## 8) Human comms template

After successful release, send:

- version + install command
- one-line auth command
- known temporary workarounds (if any)

Example:

```text
Anvil CLI vX.Y.Z is live.
Install: curl --proto '=https' --tlsv1.2 -LsSf https://github.com/EddaCraft/anvil-releases/releases/latest/download/anvil-cli-installer.sh | sh
Login: anvil auth login
```
