# Release Runbook

## Branches

- **dev** — default working branch, all development happens here
- **main** — published branch, only receives merged PRs from dev

## Pre-release checklist

1. All checks pass locally (`cargo fmt --check`, `clippy`, `test`, `publish --dry-run`)
   CI runs automatically on PRs targeting main, not on dev pushes.
2. Update version in `Cargo.toml`
3. Update snapshot tests if the version string appears in rendered output:
   ```sh
   INSTA_UPDATE=always cargo test
   ```
4. Commit version bump and snapshot updates together

## Dry run (release candidate)

Use a pre-release version to validate the pipeline without publishing:

```sh
# 1. Set version in Cargo.toml to e.g. 0.1.0-rc.0
# 2. Commit, push to dev
# 3. Tag with pre-release suffix
git tag v0.1.0-rc.0
git push origin v0.1.0-rc.0
```

Pre-release tags (`v0.1.0-rc.0`, `v0.1.0-beta.1`, etc.) do **not** trigger
the release workflow. The CI job still runs `cargo publish --dry-run` to
validate packaging.

## Release

```sh
# 1. Ensure dev is up to date and CI is green
# 2. Set final version in Cargo.toml (e.g. 0.1.0)
# 3. Update snapshots
INSTA_UPDATE=always cargo test

# 4. Commit and push
git add Cargo.toml src/snapshots/
git commit -m "chore: bump version to 0.1.0"
git push origin dev

# 5. Merge dev into main via PR
gh pr create --base main --head dev --title "release: v0.1.0"

# 6. After PR is merged, tag main
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
