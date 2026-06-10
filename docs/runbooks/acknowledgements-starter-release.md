# Acknowledgements Starter Kit Release — Operator Runbook

| Type    | Authority     | Owner  | Status | Freshness                                   |
| ------- | ------------- | ------ | ------ | ------------------------------------------- |
| Runbook | Authoritative | @aneki | Live   | First filed 2026-06-08 alongside ATTRIB-017 |

| Upstream                                                                                                                                                                                                                                                                                                                    | Downstream                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [acknowledgements-starter-releases module](../../plans/archive/modules/acknowledgements-starter-releases.aps.md), [release spec](../../plans/specs/2026-06-08-acknowledgements-starter-releases.md), [ATTRIB-017 actions](../../plans/execution/ATTRIB-017.actions.md), [eddacraft-tui release runbook](./eddacraft-tui-release.md) | [`.github/workflows/release-acknowledgements-starter.yml`](../../.github/workflows/release-acknowledgements-starter.yml), [`.github/workflows/mirror-acknowledgements-starter.yml`](../../.github/workflows/mirror-acknowledgements-starter.yml), [`tools/starters/acknowledgements/VERSION`](../../tools/starters/acknowledgements/VERSION), [`tools/starters/acknowledgements/CHANGELOG.md`](../../tools/starters/acknowledgements/CHANGELOG.md) |

## TL;DR

The kit has two publish surfaces. The rolling mirror
([`.github/workflows/mirror-acknowledgements-starter.yml`](../../.github/workflows/mirror-acknowledgements-starter.yml))
force-pushes `tools/starters/acknowledgements/` to
`eddacraft/acknowledgements-starter:main` on every change — that is "latest" and
needs no operator action. This runbook covers the **deliberate release**
surface: cutting an immutable `vX.Y.Z` tag + GitHub Release so consumers can pin
and be notified.

To cut a release:

1. Land a version-bump PR on `anvil-001` that updates
   [`tools/starters/acknowledgements/VERSION`](../../tools/starters/acknowledgements/VERSION)
   and adds a `## [X.Y.Z]` entry to
   [`tools/starters/acknowledgements/CHANGELOG.md`](../../tools/starters/acknowledgements/CHANGELOG.md).
2. After merge, tag the merge commit on `main` as
   `acknowledgements-starter-vX.Y.Z` (prefixed) and push the tag.
3. [`.github/workflows/release-acknowledgements-starter.yml`](../../.github/workflows/release-acknowledgements-starter.yml)
   asserts the version triple, subtree-splits the kit, pushes a bare `vX.Y.Z`
   tag (append-only) to the mirror, and creates the GitHub Release.
4. Verify the tag and Release on the mirror.

Release cadence is **deliberate**: trivial changes (doc tweaks, formatting) flow
to the rolling mirror only. Cut a release when there is a consumer-meaningful
change — a new ecosystem driver (minor), a contract break (major), or a fix
(patch).

## Prerequisites

- **`MIRROR_PUSH_TOKEN`** on `eddacraft/anvil-001` repo secrets — the same
  fine-grained PAT the rolling mirror already uses, scoped to
  `eddacraft/acknowledgements-starter` with `Contents: Read and write`.
  Fine-grained `Contents: write` covers both the tag push and GitHub Release
  creation, so no additional secret is needed. The release workflow's sanity
  check fails loud if it is empty. Confirm the PAT is not near its expiry before
  a cut.
- **No mirror tag-protection rule blocking the PAT** for the `v*` pattern (see
  the pre-flight below).

## Pre-flight: mirror tag protection

`Contents: write` lets the PAT push content, but a **tag protection rule** on
the mirror (classic Settings → Tags, or a tag ruleset) can still deny tag
creation — the tag analog of the eddacraft-tui release gotcha #5. The push step
would fail with a `denied`/`protected tag` error after the content split
succeeds. Probe before the first cut (both reads need mirror-admin scope):

```bash
gh api repos/eddacraft/acknowledgements-starter/rulesets --jq '.[] | select(.target=="tag") | {name, enforcement}'
gh api repos/eddacraft/acknowledgements-starter/tags/protection --jq '.[].pattern'
```

If either matches `v*`, fix one of:

- Add the `MIRROR_PUSH_TOKEN`'s identity to the ruleset's bypass list for the
  `v*` pattern, or
- Scope/relax tag protection so the PAT can create `v*` tags.

## Cut procedure

### 1. Version-bump PR

Bump
[`tools/starters/acknowledgements/VERSION`](../../tools/starters/acknowledgements/VERSION)
to the new `X.Y.Z` and add a matching `## [X.Y.Z] - <date>` entry at the top of
[`tools/starters/acknowledgements/CHANGELOG.md`](../../tools/starters/acknowledgements/CHANGELOG.md),
written for external consumers (no internal work-item ids). Run the consistency
check locally before opening the PR:

```bash
bash tools/starters/acknowledgements/check-version.sh
```

The kit-self-test CI workflow
([`.github/workflows/acknowledgements-kit.yml`](../../.github/workflows/acknowledgements-kit.yml))
also gates this on the PR. Merge to `main`.

### 2. Tag the merge commit

```bash
git fetch origin main
git tag acknowledgements-starter-vX.Y.Z origin/main
git push origin acknowledgements-starter-vX.Y.Z
```

The tag MUST point at a commit on `main` — the workflow refuses a tag whose
commit is not reachable from `main`.

### 3. Watch the release workflow

The push triggers
[`.github/workflows/release-acknowledgements-starter.yml`](../../.github/workflows/release-acknowledgements-starter.yml).
It asserts the version triple (tag `X.Y.Z` == `VERSION` == newest `CHANGELOG.md`
heading), re-checks licence drift, subtree-splits the kit (swapping the public
mirror README banner onto the kit README), pushes the bare `vX.Y.Z` tag
append-only, and creates the GitHub Release with notes from the changelog
section.

```bash
gh run watch "$(gh run list --workflow=release-acknowledgements-starter.yml -L1 --json databaseId --jq '.[0].databaseId')"
```

### 4. Verify

```bash
gh release view vX.Y.Z --repo eddacraft/acknowledgements-starter
gh api repos/eddacraft/acknowledgements-starter/git/refs/tags/vX.Y.Z --jq .ref
```

Optional round-trip proof that a consumer can pin the release:

```bash
git -C "$(mktemp -d)" init -q && \
  git -C "$_" subtree add --prefix kit \
    https://github.com/eddacraft/acknowledgements-starter.git vX.Y.Z --squash
```

## Rollback

Releases are **append-only** — a pushed tag and its Release cannot be silently
overwritten. To correct a bad cut:

- **Cut a corrected `vX.Y.Z+1`** with a fixed `VERSION` + `CHANGELOG.md`. This
  is the normal path.
- Optionally mark the bad Release as not-latest and add a note pointing at the
  fix:

  ```bash
  gh release edit vX.Y.Z --repo eddacraft/acknowledgements-starter --latest=false
  ```

- **Partial failure (tag pushed, Release not created):** re-run the workflow via
  `workflow_dispatch` with the same tag. The workflow is idempotent — it detects
  the already-present mirror tag and skips the push (leaving the immutable tag
  untouched), then creates the missing Release. Do **not** delete and re-push
  the tag.
- **Re-dispatching a fully-completed release** is safe and a no-op: the tag-push
  step skips (tag exists) and the Release step skips (Release exists); the job
  succeeds without changing anything.
- If the tag push fails with a `denied`/`protected tag` error, the mirror has a
  tag-protection rule — see the [Pre-flight](#pre-flight-mirror-tag-protection)
  above; fix the rule, then re-dispatch.

## Token rotation

Regenerate the fine-grained PAT on GitHub (same scope:
`eddacraft/acknowledgements-starter`, `Contents: Read and write`) and update the
secret:

```bash
gh secret set MIRROR_PUSH_TOKEN --repo eddacraft/anvil-001
```

The same secret backs the rolling mirror, so rotation covers both workflows.
