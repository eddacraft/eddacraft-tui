# Release Token Rotation Runbook

## Purpose

Rotate `ANVIL_RELEASES_TOKEN` — the PAT used by the release workflow to
publish to the WinGet manifests fork and the Scoop bucket
(`eddacraft/scoop-bucket`). Use this when the v0.4.0-beta tag 403'd on
the scoop publisher because the PAT lacked `contents:write` on the
bucket repo.

## When to use

- The release workflow's `scoop` job fails the pre-flight or the PUT
  with `HTTP 403` / "Resource not accessible by personal access
  token". The pre-flight emits a `::error::` annotation pointing here.
- The `winget` job fails its `gh repo fork` or `gh api` call with a
  similar 403.
- The PAT is approaching expiry (GitHub default is 1 year for fine-
  grained tokens; classic PATs may not expire).
- Routine quarterly hygiene rotation.

## Required scope

`ANVIL_RELEASES_TOKEN` is consumed by two release jobs and needs
read + write access to the same set of GitHub resources:

| Resource | Why | Minimum scope |
| --- | --- | --- |
| `eddacraft/anvil-001` | `gh release download` for installer SHAs and icon | `contents:read` |
| `eddacraft/scoop-bucket` | `gh api` PUT for `bucket/anvil.json` | `contents:write` |
| `${FORK_USER}/winget-pkgs` (fork) | `gh repo fork`, branch creation, `gh api` PUT for manifest files | `contents:write` |

For a **fine-grained PAT** (recommended), select these three repositories
and grant `Contents: Read and write` plus `Metadata: Read-only` (mandatory).
Workflow scope is not needed — the token never invokes other workflows.

For a **classic PAT** (legacy), the equivalent is `repo` (full repo
control). Classic PATs cannot scope to specific repos, so this is a
broader grant — prefer fine-grained where possible.

## Procedure

### 1. Mint a new fine-grained PAT

GitHub UI: <https://github.com/settings/personal-access-tokens/new>

- **Token name:** `anvil-releases-YYYY-MM` (so multiple tokens can
  coexist during rotation overlap)
- **Resource owner:** `eddacraft`
- **Expiration:** 1 year (GitHub max for fine-grained)
- **Repository access:** Only select repositories
  - `eddacraft/anvil-001`
  - `eddacraft/scoop-bucket`
  - `${FORK_USER}/winget-pkgs` (where `${FORK_USER}` is the bot/user
    that holds the winget-pkgs fork — currently the `eddacraft` org's
    fork)
- **Repository permissions:**
  - `Contents: Read and write`
  - `Metadata: Read-only` (mandatory for any PAT)

Copy the token immediately — GitHub does not show it again.

### 2. Verify the PAT before installing it

```bash
GH_TOKEN='ghp_...' gh api repos/eddacraft/scoop-bucket --silent && echo "scoop read ok"
GH_TOKEN='ghp_...' gh api repos/eddacraft/anvil-001 --silent && echo "anvil read ok"
```

Both should print `ok`. If either fails with 401/403, the scope is
wrong — fix the PAT before continuing.

For a write probe (optional, requires reverting):

```bash
# Read current sha, write it back unchanged — proves write works
# without modifying state.
SHA=$(GH_TOKEN='ghp_...' gh api repos/eddacraft/scoop-bucket/contents/bucket/anvil.json --jq '.sha')
GH_TOKEN='ghp_...' gh api repos/eddacraft/scoop-bucket/contents/bucket/anvil.json -X PUT \
  -f message='chore: scope verification (no-op)' \
  -f content="$(GH_TOKEN='ghp_...' gh api repos/eddacraft/scoop-bucket/contents/bucket/anvil.json --jq '.content' | tr -d '\n' | base64 -w 0)" \
  -f sha="$SHA"
```

### 3. Install the new token

GitHub UI: <https://github.com/eddacraft/anvil-001/settings/secrets/actions>

- Update the **`ANVIL_RELEASES_TOKEN`** secret with the new value.
- Do not delete the old token from GitHub yet (gives a rollback
  window).

### 4. Smoke-test on the next release-shaped artefact

The cleanest verification is a real release tag. If you do not have
one ready, the lightest dry-run is:

```bash
# Local repro of the scoop pre-flight, using the same gh api call the
# workflow makes.
GH_TOKEN='<new ANVIL_RELEASES_TOKEN>' gh api repos/eddacraft/scoop-bucket --silent \
  && echo "::notice::scoop pre-flight would pass"
```

If this passes, the next real release tag will publish through cleanly.

### 5. Revoke the old PAT

Once the rotation is verified by a successful release run (winget
job + scoop job both `Ready`), revoke the old token:

GitHub UI: <https://github.com/settings/personal-access-tokens> →
old token → **Revoke**.

## Failure modes

### Pre-flight 404 instead of 403

`gh api repos/eddacraft/scoop-bucket` returning 404 means the PAT does
not have access to that repo at all (not a scope issue — a repo-list
issue). Check that `eddacraft/scoop-bucket` is selected in the
fine-grained PAT's "Repository access" list.

### Pre-flight passes but PUT fails

The fine-grained PAT was minted with `Contents: Read-only`. Re-mint
with `Contents: Read and write`.

### Both jobs fail

Likely the token expired or was revoked upstream. Check the GitHub
audit log under `eddacraft` org settings for `personal_access_token`
events.

## Cross-references

- Module: `plans/modules/v041-release-followups.aps.md` §V041F-012
- Release workflow: `.github/workflows/release.yml` (`scoop` and
  `winget` jobs)
- v0.4.0-beta manual recovery commit:
  `eddacraft/scoop-bucket@4f3becf6`
