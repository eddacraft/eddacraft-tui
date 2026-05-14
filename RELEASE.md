# Release Runbook

## Branches

Trunk-based — `main` is the only long-lived branch. Releases are cut
from a semver tag on `main`.

## Pre-release checklist

1. All checks pass locally (`cargo fmt --check`, `clippy`, `test`, `publish --dry-run`).
   CI runs automatically on PRs targeting `main`.
2. Update version in `Cargo.toml`
3. Update snapshot tests if the version string appears in rendered output:
   ```sh
   INSTA_UPDATE=always cargo test
   ```
4. Commit version bump and snapshot updates together

## Dry run (release candidate)

Use a pre-release version to validate the pipeline without publishing:

```sh
# 1. Branch from main, set version in Cargo.toml to e.g. 0.1.0-rc.0
# 2. Open a PR, merge to main once CI is green
# 3. Tag main with the pre-release suffix
git checkout main && git pull
git tag v0.1.0-rc.0
git push origin v0.1.0-rc.0
```

Pre-release tags (`v0.1.0-rc.0`, `v0.1.0-beta.1`, etc.) do **not** trigger
the release workflow. The CI job still runs `cargo publish --dry-run` to
validate packaging.

## Release

```sh
# 1. Open a release PR targeting main with the version bump
git checkout -b release/v0.1.0
# Set final version in Cargo.toml (e.g. 0.1.0), then:
INSTA_UPDATE=always cargo test   # refresh snapshots if affected
git add Cargo.toml src/snapshots/
git commit -m "chore(release): prepare v0.1.0"
git push -u origin release/v0.1.0
gh pr create --base main --title "release: v0.1.0"

# 2. After CI is green and the PR is merged, tag main from the merge commit
git checkout main && git pull
git tag v0.1.0
git push origin v0.1.0
```

## What the release workflow does

Triggered by pushing an exact semver tag (`vX.Y.Z`) to any branch:

1. Checks out the tagged commit
2. Verifies the tag version matches `Cargo.toml` version
3. Runs `cargo publish` (requires `CARGO_REGISTRY_TOKEN` secret)
4. Creates a GitHub Release with auto-generated notes

## Secrets required

| Secret                 | Purpose                                              |
| ---------------------- | ---------------------------------------------------- |
| `CARGO_REGISTRY_TOKEN` | crates.io API token for publishing                   |
| `GITHUB_TOKEN`         | Provided automatically, used for `gh release create` |

## Troubleshooting

- **Tag/manifest mismatch**: The workflow verifies the tag matches `Cargo.toml`.
  If it fails, delete the tag, fix the version, and re-tag.
- **Snapshot test failure**: Run `INSTA_UPDATE=always cargo test` and commit
  the updated snapshot files.
- **Pre-release tag triggered release**: Should not happen with current config.
  The release workflow uses GitHub Actions filter pattern `v[0-9]+.[0-9]+.[0-9]+`
  where `+` means "one or more of the preceding character" (GitHub's extended
  glob syntax, not regex). This matches exact `vX.Y.Z` tags only — suffixes
  like `-rc.0` are excluded because there is no trailing wildcard.
