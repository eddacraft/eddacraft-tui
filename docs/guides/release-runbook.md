# Anvil CLI Release Runbook

Purpose: ship the Rust `anvil` binary safely and consistently via cargo-dist.

## Quick start

The release process is split between a deterministic preflight script and a
`/release` Claude skill that owns every judgment step:

1. **Run the preflight script** — deterministic checks only (cargo fmt / clippy
   / test, pnpm format / lint / typecheck / test):

   ```bash
   ./scripts/release.sh
   ```

   The script runs every check to completion, prints a `PASS`/`FAIL` summary
   table, and exits with the number of failed steps (0 on a clean pass). No
   prompts, no git or GitHub operations.

2. **Invoke `/release` in Claude Code** — the judgment half:

   ```
   /release
   ```

   The skill confirms preflight passed, reads live repository and GitHub state
   to assess the release, proposes a version and branch strategy, opens a
   tracking issue, drives the version bump, PR, tag, and workflow monitoring,
   then handles artefact verification, changelog / docs review, comms, and
   cleanup. The tracking issue is the durable record — there is no local handoff
   artefact and the flow is resumable: if the Claude session dies mid-release,
   re-invoking `/release` picks up from live state.

The sections below are the **reference manual** — the script handles preflight,
the skill handles everything else. Refer to them directly when something goes
wrong or when you need to understand what a step does.

---

## Release policy (current)

- **Distribution:** pre-built binaries via GitHub Releases on `eddacraft/anvil`
  (public).
- **Install method:** shell installer script (`curl ... | sh`).
- **Targets:** x86_64 + aarch64 for Linux, macOS, and Windows.
- **Workflow source of truth:** `.github/workflows/release.yml` (auto-generated
  by cargo-dist).
- **Configuration:** `dist-workspace.toml`.

---

## 1) Preflight checklist (required)

### Automated — run `./scripts/release.sh`

This is the required preflight gate before invoking `/release`. It runs, in
order:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `pnpm format:check`
- `pnpm lint:check`
- `pnpm typecheck`
- `pnpm test`

The `pnpm test` step uses the root
`nx run-many -t test --exclude=@eddacraft/anvil-e2e` command, so bundled package
scope is managed in `nx.json` / workspace config rather than in the script.

The script does not prompt, touch git, or call GitHub. It is re-runnable and
exits with the count of failed steps (0 = clean). You can run it any time to
sanity-check the workspace before release work.

### Additional manual checks (not covered by the script)

Run these from repo root after the script passes — they exercise the release
binary and a full TS build, which the script intentionally skips:

```bash
cargo build --release -p eddacraft-anvil
./target/release/anvil --help
./target/release/anvil --version

pnpm install --frozen-lockfile
pnpm build
```

Sanity assertions before release:

- `Cargo.toml` workspace version is correct.
- `CHANGELOG.md` has release notes.
- `docs/public/anvil/beta-testing-guide.md` version is current.
- `docs/public/anvil/releases/upgrade-notes.md` has a section for this version.

---

## 2) Cut the release branch or promote directly

All day-to-day work lands on `dev`. Releases are promoted from `dev` into
`main`. For small, low-risk releases, a direct `dev -> main` PR is acceptable.
For anything larger, cut a short-lived `release/*` branch from `dev` and do
stabilisation there.

See `docs/guides/branching-strategy.md` for the full policy.

### Option A: direct promotion for small releases

Use this when the change set is small, reviewable, and already stable on `dev`.

1. Ensure `dev` is green.
2. Open a PR from `dev` to `main`.
3. Title convention: `release: vX.Y.Z`.
4. Once the release gate passes, merge the PR.

```bash
gh pr create --base main --head dev --title "release: vX.Y.Z" \
  --body "Promote dev to main for release vX.Y.Z"
```

### Option B: stabilise on `release/*` for non-trivial releases

Use this when you want a short hardening window for packaging, docs, final bug
fixes, or release validation.

1. Ensure `dev` is green.
2. Create `release/x.y.z` from `dev`.
3. Allow only release hardening on the release branch.
4. Open a PR from `release/x.y.z` to `main`.
5. Once the release gate passes, merge the PR.

```bash
git switch dev && git pull --ff-only origin dev
git switch -c release/x.y.z
git push -u origin release/x.y.z

gh pr create --base main --head release/x.y.z --title "release: vX.Y.Z" \
  --body "Promote release/x.y.z to main for release vX.Y.Z"
```

Release branch scope is intentionally narrow:

- version bumps
- changelog and release notes
- docs updates required for release
- packaging and workflow fixes
- bug fixes discovered during final validation

---

## 3) Version, tag + GitHub Release

1. Switch to `main` and pull the merge:

```bash
git switch main && git pull
```

2. On `dev`, bump version in `Cargo.toml` (`[workspace.package].version`).
3. On `dev`, update `CHANGELOG.md`.
4. On `dev`, update `docs/public/anvil/beta-testing-guide.md` -- bump "Current
   version" and add any new feature areas to "What to Test".
5. On `dev`, update `docs/public/anvil/releases/upgrade-notes.md` -- add a new
   section.
6. Commit the release prep on `dev`, promote to `main`, then tag on `main`:

```bash
git switch dev && git pull --ff-only origin dev
git add Cargo.toml CHANGELOG.md \
  docs/public/anvil/beta-testing-guide.md \
  docs/public/anvil/releases/upgrade-notes.md
git commit -m "chore(release): prepare vX.Y.Z"

# promote dev -> main (direct or via release/x.y.z)
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin main
git push origin vX.Y.Z
```

Pushing the tag triggers `release.yml` (cargo-dist) which builds binaries for
all 6 targets and creates a GitHub Release automatically (pre-release for beta
tags).

For beta releases, either format works:

```bash
vX.Y.Z-beta      # e.g. v0.3.0-beta
vX.Y.Z-beta.N    # e.g. v0.3.0-beta.0
```

After tagging, merge the release line back to `dev` immediately.

If the release went direct from `dev`, no extra sync PR is needed.

If the release used `release/x.y.z`, merge that release branch back to `dev`
after tagging so `dev` retains all release-only fixes and versioning changes.

```bash
gh pr create --base dev --head release/x.y.z \
  --title "chore: merge release vX.Y.Z back to dev" \
  --body "Sync release hardening, version bump, and changelog from vX.Y.Z"
```

If the release was cut directly from `dev`, create a sync PR only if an
additional commit landed on `main` outside the original release promotion.

Example:

```bash
gh pr create --base dev --head main \
  --title "chore: merge release vX.Y.Z back to dev" \
  --body "Sync version bump and changelog from release vX.Y.Z"
```

---

## 4) Monitor release workflow

Watch run in real time:

```bash
gh run list --repo eddacraft/anvil-001 --limit 5
gh run watch <run-id> --repo eddacraft/anvil-001
```

Or inspect a completed run:

```bash
gh run view <run-id> --repo eddacraft/anvil-001 --log-failed
```

Expected behaviour:

- `plan` job succeeds and identifies the release.
- `build-local-artifacts` jobs compile for all 6 targets.
- `build-global-artifacts` job produces shell and PowerShell installers.
- `host` job creates or updates GitHub Releases on both `eddacraft/anvil-001`
  (private) and `eddacraft/anvil` (public) with all artefacts.
- `announce` job posts release notes.

---

## 5) Post-release verification (required)

Install on a clean machine (or container):

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/eddacraft/anvil/releases/latest/download/eddacraft-anvil-installer.sh | sh

anvil --version
anvil doctor
anvil auth login
anvil gate
```

Verify all 6 platform binaries are present in both GitHub Releases:

```bash
gh release view vX.Y.Z --repo eddacraft/anvil-001
gh release view vX.Y.Z --repo eddacraft/anvil
```

Expected artefacts:

- `eddacraft-anvil-aarch64-apple-darwin.tar.xz`
- `eddacraft-anvil-x86_64-apple-darwin.tar.xz`
- `eddacraft-anvil-aarch64-unknown-linux-gnu.tar.xz`
- `eddacraft-anvil-x86_64-unknown-linux-gnu.tar.xz`
- `eddacraft-anvil-x86_64-pc-windows-msvc.zip`
- `eddacraft-anvil-aarch64-pc-windows-msvc.zip`
- `eddacraft-anvil-installer.sh`
- `eddacraft-anvil-installer.ps1`

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
2. Fix on a short-lived `hotfix/*` or `release/*` branch.
3. Cut a patch release (vX.Y.Z+1).

### If public release publish fails (partial release)

The workflow creates or updates the private release in `eddacraft/anvil-001`
first, then publishes to `eddacraft/anvil`. If the public step fails:

1. Download artefacts from the private release:

```bash
gh release download vX.Y.Z --repo eddacraft/anvil-001 --dir ./artifacts
```

2. Remove manifests (the automated pipeline does this before publishing):

```bash
rm -f artifacts/*-dist-manifest.json
```

3. Ensure the tag exists on the public repo (mirrors the automated pipeline's
   tag-creation step to prevent tag drift):

```bash
if gh api repos/eddacraft/anvil/git/ref/tags/vX.Y.Z >/dev/null 2>&1; then
  echo "Tag vX.Y.Z already exists on eddacraft/anvil; skipping."
else
  PUBLIC_HEAD=$(gh api repos/eddacraft/anvil/git/ref/heads/main -q '.object.sha')
  gh api repos/eddacraft/anvil/git/refs \
    -f ref="refs/tags/vX.Y.Z" \
    -f sha="$PUBLIC_HEAD"
fi
```

4. Manually publish to the public repo:

```bash
gh release create vX.Y.Z \
  --repo eddacraft/anvil \
  --verify-tag \
  --title "Anvil CLI vX.Y.Z" \
  --notes "See changelog in private repo" \
  artifacts/*
```

5. If the `ANVIL_RELEASES_TOKEN` was the issue, check the secret in repo
   settings and re-run the failed workflow job.

### If a bad version needs to be retracted

1. Delete the git tag locally and remotely:

```bash
git tag -d vX.Y.Z
git push origin :refs/tags/vX.Y.Z
```

2. Delete the GitHub Release from both repos:

```bash
gh release delete vX.Y.Z --repo eddacraft/anvil --yes
gh release delete vX.Y.Z --repo eddacraft/anvil-001 --yes
```

3. Fix the issue, bump to a new version, and re-release.

---

## 7) Known gotchas

- **Release branch lifetime:** `release/*` branches should live for days, not
  weeks. If stabilisation keeps growing, the branch was cut too early or is
  taking too much non-release work.
- **cargo-dist PR mode:** PRs only run the `plan` job (no builds). Full builds
  only fire on version tags. This is configured in `dist-workspace.toml` as
  `pr-run-mode = "plan"`.
- **allow-dirty CI:** `dist-workspace.toml` has `allow-dirty = ["ci"]` so manual
  edits to `release.yml` (e.g. path filters) are preserved across
  `cargo dist init` re-runs.
- **Cross-compilation:** aarch64-linux uses cross-compilation in CI. If it
  fails, check the cross toolchain setup in the workflow.
- **Dual release:** The workflow creates releases on both the private repo
  (`eddacraft/anvil-001`) and the public `eddacraft/anvil`. The private release
  is the internal source-of-truth record; the public one is for distribution.
- **ANVIL_RELEASES_TOKEN:** A PAT/fine-grained token with `contents: write` on
  `eddacraft/anvil` and `eddacraft/homebrew-tap`. Must be set as a repository
  secret on `anvil-001`.

---

## 8) Human comms template

After successful release, send:

- version + install command
- one-line auth command
- known temporary workarounds (if any)

Example:

```text
Anvil CLI vX.Y.Z is live.
Install: curl --proto '=https' --tlsv1.2 -LsSf https://github.com/eddacraft/anvil/releases/latest/download/eddacraft-anvil-installer.sh | sh
Login: anvil auth login
```

---

## 9) Release rules that matter most

1. Day-to-day work lands on `dev`.
2. Small releases may go directly from `dev` to `main`.
3. Larger releases should use a temporary `release/*` branch.
4. Any fix that lands during release stabilisation must be merged back to `dev`
   immediately after release.
5. If `dev -> main` promotion feels too large, promotion waited too long.
