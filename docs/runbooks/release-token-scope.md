# Release Token Scope Runbook

| Type    | Authority     | Owner   | Status | Freshness                                                                                    |
| ------- | ------------- | ------- | ------ | -------------------------------------------------------------------------------------------- |
| Runbook | Authoritative | RELORCH | Live   | Last reviewed 2026-05-24 against v0.4.0-beta 403 failure and `.github/workflows/release.yml` |

| Upstream                                                                                                   | Downstream                         |
| ---------------------------------------------------------------------------------------------------------- | ---------------------------------- |
| `.github/workflows/release.yml`, `.github/workflows/homebrew-bump.yml`, `scripts/release/bump-homebrew.sh` | release council, on-call operators |

## Purpose

`ANVIL_RELEASES_TOKEN` is the GitHub PAT the release workflow uses to publish to
the Scoop bucket (`eddacraft/scoop-bucket`) and the Homebrew tap
(`eddacraft/homebrew-tap`), plus internal release-time steps (asset downloads,
ACKNOWLEDGEMENTS publish). When a release job 403s on it, the cause is almost
always **the existing token is missing a scope on a repo** — not that the token
has expired or been compromised. The fix is usually a one-click in-place edit,
not a rotation.

> **WinGet uses a different token.** The `winget` job in `release.yml`
> authenticates with `WINGET_TOKEN`, not `ANVIL_RELEASES_TOKEN`. A 403 on the
> winget job is a separate scope problem on `WINGET_TOKEN` — adjusting
> `ANVIL_RELEASES_TOKEN` will not fix it.

This runbook leads with the scope-fix path. Full mint + install + revoke is at
the bottom for the cases where you actually do need to rotate (expiry, suspected
compromise, classic → fine-grained conversion).

## When to use

- The release workflow's `scoop` job fails the pre-flight or the PUT with
  `HTTP 403` / "Resource not accessible by personal access token". The
  pre-flight emits a `::error::` annotation pointing here.
- The `Publish Homebrew formula to eddacraft/homebrew-tap` step in `release.yml`
  (or the `republish` job in `homebrew-bump.yml`) fails with a 403 on the
  `gh api ... -X PUT` write to `Formula/anvil.rb`.
- A new repo was added to the org and the existing token needs to reach it (the
  v0.4.0-beta failure mode — `eddacraft/scoop-bucket` was not in the existing
  token's selected-repos list).
- The PAT is approaching expiry (true rotation case — go to §"Mint a fresh
  token" below).

## Required scopes

`ANVIL_RELEASES_TOKEN` is consumed by release-time publish steps and needs the
following access:

| Resource                 | Why                                                                                           | Minimum scope    |
| ------------------------ | --------------------------------------------------------------------------------------------- | ---------------- |
| `eddacraft/anvil-001`    | `gh release download` for installer SHAs and icon; ACKNOWLEDGEMENTS.md publish on release.yml | `contents:write` |
| `eddacraft/scoop-bucket` | `gh api` PUT for `bucket/anvil.json`                                                          | `contents:write` |
| `eddacraft/homebrew-tap` | `gh api` PUT for `Formula/anvil.rb` via `scripts/release/bump-homebrew.sh`                    | `contents:write` |

Plus `metadata:read` on all three (mandatory for any fine-grained PAT).
`workflow` scope is not needed — the token never invokes other workflows.

## Edit the existing token (primary path)

Fine-grained PATs are editable in place — same secret value, no rotation, no
re-paste. This is almost always what you want.

1. Open <https://github.com/settings/personal-access-tokens>, click the row for
   `ANVIL_RELEASES_TOKEN`'s underlying PAT, then **Edit**.
2. **Repository access:** ensure all three repos above are listed under "Only
   select repositories". Add any that are missing.
3. **Repository permissions:** confirm `Contents: Read and write` is selected
   (read-only will pass the pre-flight but fail the PUT).
4. Save. No further action needed — GitHub Secrets stays in sync automatically
   because the token's value did not change.

If the existing token is a **classic PAT** under SSO enforcement, edit-in-place
is more limited — try re-authorising SSO instead:

1. <https://github.com/settings/tokens> → the classic PAT row → **Configure
   SSO** → ensure the `eddacraft` org is authorised.
2. If the bucket is a recently-added repo, click **Re-authorise** so the token's
   SSO grant covers it.

## Verify the fix

After edit-in-place or re-auth, confirm the token can reach the resources before
relying on the next real release.

> **Don't paste the PAT inline on the command line.**
> `GH_TOKEN='<value>' gh ...` ends up in shell history and is visible to other
> processes via `ps` / environment inspection. Read it once into a shell-local
> variable and let it fall out of scope when the verification subshell exits:

```bash
( read -rs -p "ANVIL_RELEASES_TOKEN: " GH_TOKEN && export GH_TOKEN
  gh api repos/eddacraft/scoop-bucket  --silent && echo "scoop read ok"
  gh api repos/eddacraft/anvil-001     --silent && echo "anvil read ok"
  gh api repos/eddacraft/homebrew-tap  --silent && echo "homebrew read ok"
)
```

You should see `scoop read ok`, `anvil read ok`, and `homebrew read ok` on
stdout. If any read 403s, re-check that the missing repo is in the token's repo
list and that `Contents: Read and write` applied — the GitHub UI sometimes
silently keeps an outdated permission set on save.

(If you use a secret manager, e.g. `op run --env-file=.env -- bash -c '...'` or
`gh auth login --with-token < <(secret-cli get …)`, that's preferred over typing
the PAT in.)

For a write probe (produces a real but no-op commit on the bucket — keep it
inside the same subshell so the token doesn't leak into the parent):

```bash
( read -rs -p "ANVIL_RELEASES_TOKEN: " GH_TOKEN && export GH_TOKEN
  # Read current sha + base64 content. The Contents API returns
  # .content already base64-encoded — DO NOT re-encode, that would
  # corrupt the manifest. Write it back verbatim.
  META=$(gh api repos/eddacraft/scoop-bucket/contents/bucket/anvil.json)
  SHA=$(printf '%s' "$META" | jq -r '.sha')
  CONTENT=$(printf '%s' "$META" | jq -r '.content' | tr -d '\n')
  gh api repos/eddacraft/scoop-bucket/contents/bucket/anvil.json -X PUT \
    -f message='chore: scope verification (no-op)' \
    -f content="$CONTENT" \
    -f sha="$SHA"
)
```

If the next real release's `scoop` job runs without a 403, you're done — no
further action needed.

## Mint a fresh token (only when actually rotating)

Use this path **only** when:

- The existing PAT has expired (or is about to). GitHub fine-grained tokens
  default to a 1-year lifetime; classic PATs may not expire.
- You suspect the token has leaked.
- You're converting classic → fine-grained for hygiene.
- You can't see or edit the existing PAT (e.g. it was minted by someone who has
  left the team).

### 1. Mint a new fine-grained PAT

<https://github.com/settings/personal-access-tokens/new>

- **Token name:** `anvil-releases-YYYY-MM` (so multiple tokens can coexist
  during overlap)
- **Resource owner:** `eddacraft`
- **Expiration:** 1 year (GitHub max for fine-grained)
- **Repository access:** Only select repositories
  - `eddacraft/anvil-001`
  - `eddacraft/scoop-bucket`
  - `eddacraft/homebrew-tap`
- **Repository permissions:**
  - `Contents: Read and write`
  - `Metadata: Read-only` (mandatory)

Copy the token immediately — GitHub does not show it again.

### 2. Verify before installing

Run the same `gh api repos/...` checks as the §"Verify the fix" section above
against the new token. Do not paste it into Secrets until all reads pass.

### 3. Install the new token

<https://github.com/eddacraft/anvil-001/settings/secrets/actions> → update
**`ANVIL_RELEASES_TOKEN`** with the new value.

Do not revoke the old token yet — keep a rollback window.

### 4. Smoke-test on the next release tag

A real release tag is the cleanest verification. If you don't have one ready,
the workflow's pre-flight is the same `gh api repos/...` read, so the
verification step above is sufficient.

### 5. Revoke the old PAT

Once the next release's `scoop` and Homebrew publish jobs both end `Ready`,
revoke the old PAT at <https://github.com/settings/personal-access-tokens> → old
token → **Revoke**.

## Failure modes

### Pre-flight 404 instead of 403

`gh api repos/eddacraft/scoop-bucket` returning 404 means the token has no
visibility on that repo at all (not a scope issue — a repo-list issue). Add
`eddacraft/scoop-bucket` to the token's "Repository access" list (edit-in-place
per §"Edit the existing token").

### Pre-flight passes but PUT fails

The token has `Contents: Read-only` on the bucket. Edit it to
`Contents: Read and write` — same edit-in-place flow.

### Both jobs fail and edit-in-place doesn't help

Likely the token expired or was revoked upstream. Check
<https://github.com/organizations/eddacraft/settings/security-analysis> audit
log for `personal_access_token` events. If revoked, fall through to §"Mint a
fresh token".

## Cross-references

- Module: `plans/modules/v050-release-followups.aps.md` §V050F-012
- Release workflow: `.github/workflows/release.yml` (`scoop` job and the
  `Publish Homebrew formula to eddacraft/homebrew-tap` step). The `winget` job
  in the same workflow uses `WINGET_TOKEN` — out of scope for this runbook.
- Homebrew recovery workflow: `.github/workflows/homebrew-bump.yml`
  (`workflow_dispatch` `republish` job)
- v0.4.0-beta manual recovery commit: `eddacraft/scoop-bucket@4f3becf6`
