# Homebrew Formula Publish — Operator Runbook

| Type    | Authority     | Owner  | Status | Freshness                                    |
| ------- | ------------- | ------ | ------ | -------------------------------------------- |
| Runbook | Authoritative | @aneki | Live   | First filed 2026-05-17 alongside DISTRIB-003 |

| Upstream                                                                                                                                                                                                                                                                                                                                                                                                        | Downstream                                                                                                                                                                                                                               |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [DISTRIB-003 in `distribution-and-update.aps.md`](../../plans/archive/modules/distribution-and-update.aps.md), [`release-token-scope.md`](release-token-scope.md), [`v0.6.0-beta-release-runbook.md`](../archive/runbooks/v0.6.0-beta-release-runbook.md) `docs/runbooks/release-token-scope.md`, `docs/archive/runbooks/v0.6.0-beta-release-runbook.md` `plans/archive/modules/distribution-and-update.aps.md` | [`scripts/release/bump-homebrew.sh`](../../scripts/release/bump-homebrew.sh), [`.github/workflows/homebrew-bump.yml`](../../.github/workflows/homebrew-bump.yml), [`.github/workflows/release.yml`](../../.github/workflows/release.yml) |

## TL;DR

On every release tag, `release.yml` calls `scripts/release/bump-homebrew.sh` to
patch the cargo-dist `eddacraft-anvil.rb` (class rename) and push it to
`eddacraft/homebrew-tap` as `Formula/anvil.rb`. If that step fails, rerun the
`Homebrew — bump and smoke` workflow with the tag as input. If both fail, run
the same script locally.

## What the auto-publish actually does

1. cargo-dist (inside the `host` job in `release.yml`) generates
   `eddacraft-anvil.rb` and uploads it to both the private
   (`eddacraft/anvil-001`) and public (`eddacraft/anvil`) GitHub Releases.
2. The "Publish Homebrew formula" step in `release.yml` shells out to
   `bash scripts/release/bump-homebrew.sh --release-tag <tag> --formula-source artifacts/eddacraft-anvil.rb --out $RUNNER_TEMP/anvil.rb --publish --tap-repo eddacraft/homebrew-tap`.
3. The script renames `class EddacraftAnvil < Formula` → `class Anvil < Formula`
   (Homebrew dispatches `brew install eddacraft/tap/anvil` to a class named
   `Anvil`) and PUTs the file to `Formula/anvil.rb` on the tap via the GitHub
   Contents API.
4. The `Homebrew — bump and smoke` workflow then runs
   `brew install eddacraft/tap/anvil` on macOS arm64 (macos-14) and x64
   (macos-13) and confirms `anvil --version` reports the tag.

## When the smoke install fails

If only the smoke job is red but the publish succeeded, the formula is already
live. Investigate, then either:

- Patch the formula (see "Manual publish from a workstation" below) and rerun
  the smoke job, or
- Roll the tap back to the previous commit (see "Rollback" below) and open a
  hotfix on `main` to fix whatever broke the build.

## Recovery — workflow_dispatch republish

Use when the inline publish step in `release.yml` failed (network 5xx, expired
token, transient `gh api` issue).

1. Open `Actions → Homebrew — bump and smoke → Run workflow`.
2. Inputs:
   - `tag`: the release tag, e.g. `v0.7.0-beta`.
   - `tap-repo`: leave default `eddacraft/homebrew-tap` unless retargeting.
   - `skip-publish`: leave `false`.
3. The job will:
   - Run `scripts/release/_test/bump-homebrew.test.sh` (contract dry-run).
   - Download `eddacraft-anvil.rb` from the public release.
   - Patch + publish via `scripts/release/bump-homebrew.sh --publish`.
   - Run the macOS arm64 + x64 smoke install.

`workflow_dispatch` requires the `ANVIL_RELEASES_TOKEN` secret to have
`contents:write` on `eddacraft/homebrew-tap`. If it 403s, follow
[`release-token-scope.md`](release-token-scope.md) — the fix is usually
edit-in-place on the existing PAT, no rotation.

## Recovery — manual publish from a workstation

Use when GitHub Actions itself is down or the recovery workflow cannot reach the
tap. Requires `gh` authenticated as a user with `contents:write` on
`eddacraft/homebrew-tap`.

```sh
TAG=v0.7.0-beta

mkdir -p /tmp/anvil-hb && cd /tmp/anvil-hb
gh release download "$TAG" \
  --repo eddacraft/anvil \
  --pattern 'eddacraft-anvil.rb'

bash /path/to/anvil/scripts/release/bump-homebrew.sh \
  --release-tag "$TAG" \
  --formula-source eddacraft-anvil.rb \
  --out anvil.rb \
  --publish \
  --tap-repo eddacraft/homebrew-tap
```

Verify with:

```sh
brew untap eddacraft/tap 2>/dev/null || true
brew tap eddacraft/tap https://github.com/eddacraft/homebrew-tap
brew install eddacraft/tap/anvil
anvil --version   # should print ${TAG#v}
```

If the macOS install fails on `arm64` or `x86_64` specifically, the most likely
cause is a missing bottle URL for that arch in the cargo-dist formula — not a
`bump-homebrew.sh` bug. Check `artifacts/eddacraft-anvil.rb` for both
`:arm64_sonoma` (or current macOS codename) and `:sonoma` bottle stanzas; if one
is missing, the underlying problem is in the cargo-dist build matrix, not here.

## Dry-run before tagging

When changing `bump-homebrew.sh` or its workflow, dry-run the contract locally —
no secrets, no network:

```sh
cd /path/to/anvil
bash scripts/release/_test/bump-homebrew.test.sh
```

The same test runs on every PR via the `dry-run` job in
`.github/workflows/homebrew-bump.yml`.

## Rollback — pin the tap to a previous commit

If a published formula is broken (e.g. wrong SHA256, points at a release that
was yanked), rolling back is faster than fixing forward:

```sh
gh api repos/eddacraft/homebrew-tap/contents/Formula/anvil.rb \
  --jq '.sha'                                 # current sha (note it)

# Find the previous good blob via the commits API. Filter by path so
# unrelated commits to other formulae on the tap do not pollute the list
# — the tap repo accumulates other packages over time.
gh api "repos/eddacraft/homebrew-tap/commits?path=Formula/anvil.rb" \
  --jq '.[] | {sha, message: .commit.message}' | head

# Restore the previous file using the contents API:
PREV_COMMIT=<sha from above>
gh api repos/eddacraft/homebrew-tap/contents/Formula/anvil.rb?ref=$PREV_COMMIT \
  --jq '.content' | base64 -d > anvil.rb.prev

CURRENT_SHA=$(gh api repos/eddacraft/homebrew-tap/contents/Formula/anvil.rb --jq '.sha')
gh api repos/eddacraft/homebrew-tap/contents/Formula/anvil.rb -X PUT \
  -f message="rollback: revert anvil formula to $PREV_COMMIT" \
  -f content="$(base64 -w 0 < anvil.rb.prev)" \
  -f sha="$CURRENT_SHA"
```

Then
`gh release edit <bad-tag> --repo eddacraft/anvil --prerelease=true --latest=false`
to demote the broken release so the installer script resolves to the previous
good tag.

## Trust model

- The tap repo `eddacraft/homebrew-tap` is the source of truth Homebrew reads.
  Anyone with `contents:write` on that repo can ship code to every Homebrew user
  on first `brew upgrade anvil`.
- The release pipeline pushes to the tap with `ANVIL_RELEASES_TOKEN`, a
  fine-grained PAT scoped to `eddacraft/homebrew-tap` (and
  `eddacraft/scoop-bucket`, `eddacraft/anvil`) with `Contents: Read and write`.
  See [`release-token-scope.md`](release-token-scope.md).
- The formula does **not** carry an inline signature. Instead, the downloaded
  binary tarballs are minisign-signed per
  [ADR-045](../../plans/decisions/045-update-signing-scheme.md), and the
  `anvil update` resolution chain verifies that signature before replacing the
  running binary. Compromising the tap can serve a bad formula, but the
  installed binary will refuse to self-update to anything not signed by the
  production minisign key.

## Validation matrix

| Surface                                           | What is tested                                              | Where                                         |
| ------------------------------------------------- | ----------------------------------------------------------- | --------------------------------------------- |
| `bump-homebrew.sh` arg validation, patch, dry-run | exit codes, class rename, atomic write, idempotency         | `scripts/release/_test/bump-homebrew.test.sh` |
| Workflow dry-run on every PR                      | Synthetic formula → publish path with `--dry-run`           | `dry-run` job in `homebrew-bump.yml`          |
| Auto-publish on release                           | cargo-dist artefact → patched formula → PUT to tap          | `release.yml` host job                        |
| macOS arm64 + x64 install                         | `brew install eddacraft/tap/anvil` resolves and runs binary | `smoke` matrix in `homebrew-bump.yml`         |

## Known gaps

- The publish call is unsigned at the commit level — the PUT goes via the GitHub
  Contents API, which lets the commit be authored by the PAT user without a
  GPG/SSH signature on the tap commit. ADR-045 covers the binary signature;
  commit-level signing on the tap is a follow-up separate from DISTRIB-003 once
  Anvil has a release-bot identity with a managed signing key.
- The smoke matrix exercises `brew install`, not `brew upgrade` from a prior
  version. Upgrades are covered indirectly by the user-facing hotfix loop and
  the `anvil update` resolution chain
  (`crates/anvil-cli/src/commands/update.rs`).
