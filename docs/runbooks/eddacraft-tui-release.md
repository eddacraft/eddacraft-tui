# `eddacraft-tui` Crate Release — Operator Runbook

| Type    | Authority     | Owner  | Status | Freshness                                 |
| ------- | ------------- | ------ | ------ | ----------------------------------------- |
| Runbook | Authoritative | @aneki | Live   | First filed 2026-05-25 alongside TUIR-005 |

| Upstream                                                                                                                                                                                                                                                                                                            | Downstream                                                                                                                                                                                                                                                                                                                                                       |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [TUIR module](../../plans/archive/modules/tui-reintegration.aps.md), [ADR-047](../../plans/decisions/047-eddacraft-tui-canonical-source-mirror.md), [`docs/policies/eddacraft-tui-mirror.md`](../policies/eddacraft-tui-mirror.md), [TUIR-001 baseline](../../plans/specs/2026-05-22-tui-reintegration-baseline.md) | [`.github/workflows/publish-eddacraft-tui.yml`](../../.github/workflows/publish-eddacraft-tui.yml), [`.github/workflows/mirror-eddacraft-tui.yml`](../../.github/workflows/mirror-eddacraft-tui.yml), [`crates/eddacraft-tui/CHANGELOG.md`](../../crates/eddacraft-tui/CHANGELOG.md), [`crates/eddacraft-tui/Cargo.toml`](../../crates/eddacraft-tui/Cargo.toml) |

## TL;DR

To cut a release of `eddacraft-tui`:

1. Land a version-bump PR on `anvil-001` that updates
   `crates/eddacraft-tui/Cargo.toml` and `crates/eddacraft-tui/CHANGELOG.md`.
2. After merge, tag the merge commit on `main` as `eddacraft-tui-vX.Y.Z` and
   push the tag.
3. `.github/workflows/publish-eddacraft-tui.yml` runs the publish-side gate
   matrix, `cargo publish`es to crates.io, pushes the tag (append-only) to the
   public mirror, and creates a GitHub Release on anvil-001.
4. Verify on crates.io, on the mirror, and on the anvil-001 Release.

The crate's semver is independent of Anvil product releases (D-TUIR-006). A
crate cut MUST NOT imply an Anvil release and vice versa.

## Prerequisites

Wire these once per environment; the publish workflow's sanity check fails loud
if either secret is empty.

- **`CRATES_IO_EDDACRAFT_TUI_TOKEN`** on `eddacraft/anvil-001` repo secrets.
  Least-privilege token from crates.io, scope `publish-update` on the
  `eddacraft-tui` crate only. Crate ownership on crates.io is unchanged from the
  pre-migration state; only the publish surface moves. The previous
  `CARGO_REGISTRY_TOKEN` on `eddacraft/eddacraft-tui` is slated for revocation
  after the first successful publish from canonical source (TUIR-008).
- **`EDDACRAFT_MIRROR_BOT_APP_ID`** and **`EDDACRAFT_MIRROR_BOT_PRIVATE_KEY`**
  on `eddacraft/anvil-001` repo secrets. Back the `eddacraft-mirror-bot` GitHub
  App (org-owned by `eddacraft`, installed on `eddacraft/eddacraft-tui` only,
  permissions `Contents: Read and write` + `Metadata: Read` +
  `Workflows: Read and write`). The `Workflows` permission is required because
  the mirrored crate tree contains a PR-redirect workflow file under
  `crates/eddacraft-tui/.github/workflows/` (added by TUIR-007 on the mirror
  side to auto-close drive-by source PRs against the mirror); without
  `Workflows: Read and write`, GitHub rejects the first push with
  `refusing to allow a GitHub App to create or update workflow … without workflows permission`.
  Same App is used by the mirror content workflow
  (`.github/workflows/mirror-eddacraft-tui.yml`). The workflow mints a
  short-lived installation token at runtime via
  `actions/create-github-app-token` and uses it to push the release tag to the
  mirror.

## Bootstrap gotchas

The App + mirror-repo setup has four config traps that each silently fail in a
distinct way on the first attempt, plus a UI save gotcha that manifests on top
of the third. Branch protection (#3) blocks the content force-push; tag
protection (#5) blocks the release-tag push — they fail independently, so a
green content sync does not prove the tag push will work. Walk through them in
order; the API verification commands surface state the GitHub UI hides.

### 1. App registration ≠ App installation

Creating the App in `eddacraft` org settings (Developer settings → GitHub Apps →
New) **registers** the App. That step alone does not install it on any repo.
Installation is a separate click sequence:

- On the App's settings page, click **Install App** in the left nav.
- On the row for the `eddacraft` org, click **Install**.
- Choose **Only select repositories** and tick `eddacraft/eddacraft-tui`.
- Confirm.

Without the install step, `actions/create-github-app-token` fails on the first
dispatch with:

```
RequestError [HttpError]: Not Found
    at getTokenFromRepository
```

Verify with:

```bash
gh api orgs/eddacraft/installations --jq '.installations[] | {app_slug, target: .account.login, repository_selection}'
```

If `eddacraft-mirror-bot` is absent from the list, the install step is the gap.

### 2. Permission set must include `Workflows: Read and write`

See the `EDDACRAFT_MIRROR_BOT_*` bullet above. Setting the permission on the
App's Permissions & events page sends a permission-update notification to the
installation — the existing install on `eddacraft-tui` will display a yellow
banner asking for approval of the new permission. Click through and approve, or
the workflow keeps running under the old (insufficient) permission set even
though the App page shows the new one.

### 3. Mirror repo's classic branch protection blocks force-push

GitHub repos that started life as a regular project usually have classic branch
protection on `main`. The mirror workflow **must** be able to force-push to
`main` (D-TUIR-004) — protection on a mirrored branch is contradictory by
design.

Required state (confirm via API, do NOT just check the UI):

```bash
# Null-guarded so it works on any protection shape (a repo with no
# PR-required rule, or no force-push field, returns sane defaults
# instead of an "Cannot index null" jq error).
gh api repos/eddacraft/eddacraft-tui/branches/main/protection \
  --jq '{
    allow_force_pushes: (try .allow_force_pushes.enabled catch false),
    pr_bypass_apps: (try (.required_pull_request_reviews.bypass_pull_request_allowances.apps | map(.slug)) catch []),
    required_status_checks: (.required_status_checks // "none")
  }'
```

Required values:

- `allow_force_pushes: true` (or the App on the force-push actor allowlist if
  the repo uses the newer ruleset variant — classic protection is
  all-or-nothing).
- Either the PR-required rule is disabled entirely (`pr_bypass_apps: null` AND
  no `required_pull_request_reviews` field), OR `eddacraft-mirror-bot` is
  present in `pr_bypass_apps`. Disabling the rule entirely is fine for a mirror
  — protection on a mirrored branch is contradictory by design — but if you keep
  the rule for human-direct-push defence, the App MUST be on the bypass list.
- `required_status_checks: "none"` (the mirror push wouldn't have time to let CI
  run).

Failure signature in the workflow log if force-push is blocked:

```
remote: error: GH006: Protected branch update failed for refs/heads/main.
- Cannot force-push to this branch
```

or, with the newer ruleset path:

```
remote: error: GH013: Repository rule violations found for refs/heads/main.
- Cannot force-push to this branch
```

### 4. "Save changes" button is at the very bottom of the protection page

The classic branch protection edit page is long enough that the green **Save
changes** button sits well below the fold. Toggling **Allow force pushes** at
the top of the page and navigating away does NOT save. Always scroll to the
bottom and click Save, then re-poll the API to confirm:

```bash
gh api repos/eddacraft/eddacraft-tui/branches/main/protection \
  --jq '.allow_force_pushes.enabled'
```

Expected: `true`. If the API still shows `false` after a UI save, the save
didn't take.

### 5. Tag protection blocks the App from creating release tags

`Contents: Read and write` lets the App force-push `main` (content sync) but
does **not** let it create tags when the mirror has a **tag protection rule**
(classic Settings → Tags, or a tag ruleset) matching the release pattern. The
App is denied with:

```
remote: Permission to eddacraft/eddacraft-tui.git denied to eddacraft-mirror-bot[bot].
```

This is the tag analog of gotcha #3 and bit the first `eddacraft-tui-v0.2.3`
publish (run `26570817546`, step "Propagate tag to mirror"): `cargo publish`
succeeded (0.2.3 went live on crates.io) but the tag-propagation step failed,
leaving the mirror without the release tag. **Diagnostic:** if the content
workflow (`.github/workflows/mirror-eddacraft-tui.yml`) is succeeding while only
the publish workflow's tag step fails, the denial is tag-specific (not a general
permission loss). Confirm:

```bash
# Either can block. Both reads need mirror-admin scope.
gh api repos/eddacraft/eddacraft-tui/rulesets --jq '.[] | select(.target=="tag") | {name, enforcement}'
gh api repos/eddacraft/eddacraft-tui/tags/protection --jq '.[].pattern'
```

Fix (one of):

- Add `eddacraft-mirror-bot` to the tag ruleset's bypass list for the
  `eddacraft-tui-v*` pattern, or
- Scope/relax tag protection so the App can create `eddacraft-tui-v*` tags. The
  old unprefixed `v0.x.y` tags stay protected — the App never needs to touch
  them (D-TUIR-011).

After fixing, **re-propagate the tag manually** (Rollback → "Tag propagation to
mirror failed") — do NOT re-run the whole publish once `cargo publish` has
already succeeded.

## First cutover (one-shot, TUIR-008)

This section is the **one-time** migration cutover. It is distinct from the
recurring "Cut procedure" below and runs exactly once, when canonical source
first takes over the public mirror. Re-read it only for audit or rollback.

### What "cutover" means here

The mirror workflow (`.github/workflows/mirror-eddacraft-tui.yml`) force-pushes
the `crates/eddacraft-tui/` subtree-split onto `eddacraft/eddacraft-tui:main`.
The **first** such push replaces the public repo's entire pre-existing `main`
history with a fresh, disconnected subtree-split root. This is **irreversible
and one-shot** (Risk: "Cutover force-push destroys existing public history") and
is distinct from the ongoing per-change history rewrites (D-TUIR-020), which are
expected and continuous.

### Required preservation BEFORE the first force-push (D-TUIR-010)

Before the first canonical force-push, the pre-cutover public `main` tip MUST be
captured as a durable, never-updated archive branch `pre-canonical-archive`:

```bash
# Capture the CURRENT (pre-cutover) mirror main SHA, then archive it.
PRE=$(gh api repos/eddacraft/eddacraft-tui/git/refs/heads/main --jq .object.sha)
gh api -X POST repos/eddacraft/eddacraft-tui/git/refs \
  -f ref='refs/heads/pre-canonical-archive' \
  -f "sha=$PRE"
gh api repos/eddacraft/eddacraft-tui/branches/pre-canonical-archive --jq .name
# Expected: pre-canonical-archive
unset PRE
```

The old unprefixed `v0.x.y` tags are left untouched (D-TUIR-011) and keep
pointing at pre-cutover commits, so already-published `0.x.y` artifacts retain
their source.

### Current-state reconciliation (read before cutting v0.2.3)

⚠️ The **content** force-push has already run during TUIR-004 / TUIR-005
validation — mirror `main` HEAD is the banner-swap commit
`chore(mirror): prepend public README header` (2026-05-25) — and
`pre-canonical-archive` was **not** created beforehand. The D-TUIR-010
preservation step above was therefore skipped on the content cutover. Confirm
the current state before proceeding:

```bash
# main now has a disconnected subtree-split root: no common ancestor with the
# pre-cutover release tags, which confirms the content cutover already ran.
gh api repos/eddacraft/eddacraft-tui/compare/v0.2.2...main --jq .status
#   → 404 "No common ancestor between v0.2.2 and main" (expected post-cutover)

# pre-canonical-archive is still missing — this is the gap to close:
gh api repos/eddacraft/eddacraft-tui/branches/pre-canonical-archive --jq .name
#   → 404 "Branch not found"
```

**Corrective step (do this before cutting v0.2.3).** Create the archive branch
retroactively from the most complete recoverable pre-cutover commit. The last
pre-cutover release tag `v0.2.2` (`5078007b961bcb648ac0d5a190af94d797e01daf`) is
the durable anchor; if the pre-cutover `main` advanced past `v0.2.2`, a fuller
tip may be recoverable from the leftover `fix/relocate-acknowledgements-subtree`
branch's ancestry, the publishing host's reflog, or a standalone clone. Prefer
the fullest recoverable tip; fall back to the tag:

```bash
gh api -X POST repos/eddacraft/eddacraft-tui/git/refs \
  -f ref='refs/heads/pre-canonical-archive' \
  -f sha='5078007b961bcb648ac0d5a190af94d797e01daf'
```

Record the chosen SHA on the TUIR-008 PR. TUIR-008's validation
(`gh api …/branches/pre-canonical-archive --jq .name` → `pre-canonical-archive`)
passes once this branch exists.

### Then cut the first publish

Once `pre-canonical-archive` exists and the `v0.x.y` tags are confirmed intact,
proceed to "Cut procedure" to cut `eddacraft-tui-v0.2.3` — the first publish
from canonical source. After the first successful publish, revoke the legacy
`CARGO_REGISTRY_TOKEN` on `eddacraft/eddacraft-tui` (see Token rotation →
"Retiring the legacy" notes) and confirm the public repo presents as read-only:
the MIRROR-README banner and the PR-redirect workflow (both from TUIR-007)
provide the "read-only with a notice" posture; no human-writable path should
remain other than the mirror bot's force-push.

## Cut procedure

### 1. Version-bump PR

Land a PR on `anvil-001` `main` that:

- Bumps the `version = "…"` field in `crates/eddacraft-tui/Cargo.toml` by the
  appropriate semver bucket. The crate's MSRV is independent of Anvil's
  toolchain pin (D-TUIR-015) — do not change `rust-version` unless the bump is
  intentional and has its own CHANGELOG entry.
- Adds a CHANGELOG entry at the top of `crates/eddacraft-tui/CHANGELOG.md` under
  a new `## [X.Y.Z] - YYYY-MM-DD` header. Convention: keep the format consistent
  with prior entries in that file.
- Does NOT touch `releaseIntent` / `releaseScope` on any APS work item; the
  crate release is not an Anvil product release.

#### Breaking-change checklist (stability markers — D-TUIN-005 / TUIN-006)

Before choosing the semver bucket, audit the diff against the `# Stability`
grades in the rustdoc:

- **A `stable` item changed incompatibly** (signature / trait-method / removal —
  e.g. the `Theme` override hook or the `Surface` trait) → this is a **breaking
  change**: take the breaking bucket (for 0.x that is a **minor** bump per the
  CHANGELOG's SemVer note) **and** file an ADR. Never ship it as a patch.
- **An `unstable` or `experimental` item changed** (e.g. `test_utils`, the
  `mode` probes) → no breaking bump required; just note it in the CHANGELOG.
  These carry no compatibility guarantee.
- **New public items** must declare a `# Stability` grade (default `unstable`),
  or the warn-only `scripts/check-stability-markers.mjs` check flags them. As
  you grade baselined items, prune them from
  `crates/eddacraft-tui/stability-baseline.txt`.

Watch CI on the PR. The full D-TUIR-007 gate matrix runs there; the publish
workflow re-runs the same gates on the tagged commit, but catching a regression
at PR review is cheaper.

### 2. Tag the merge commit

Once the PR is merged, locally fast-forward `main` and tag the merge commit:

```bash
git fetch origin main
git checkout main
git pull --ff-only
git tag -a "eddacraft-tui-vX.Y.Z" -m "eddacraft-tui X.Y.Z"
git push origin "eddacraft-tui-vX.Y.Z"
```

The tag name MUST be `eddacraft-tui-vX.Y.Z` (prefixed semver per D-TUIR-002).
The publish workflow's `on.push.tags` filter is constrained to that pattern;
bare `vX.Y.Z` will not trigger it.

### 3. Watch the publish workflow

`gh run watch` or open the workflow run from the Actions tab.

The workflow runs (in order):

1. Ref-reachability guard — refuses tags pointing at commits not on `main`.
2. Secret sanity check.
3. Checkout (full history) + Rust toolchain + cargo cache.
4. Tag-version match against `crates/eddacraft-tui/Cargo.toml`.
5. D-TUIR-007 publish-side gates: `cargo fmt --all --check`,
   `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test -p eddacraft-tui --all-features`,
   `cargo test -p eddacraft-tui --no-default-features`,
   `cargo doc --no-deps -p eddacraft-tui --all-features` (with
   `RUSTDOCFLAGS=-D warnings`),
   `cargo deny --all-features check --config attribution/deny.toml`,
   `cargo publish --dry-run -p eddacraft-tui --all-features`,
   `cargo package --list -p eddacraft-tui` byte-diff against the TUIR-001
   baseline.
6. `cargo publish -p eddacraft-tui --all-features` to crates.io.
7. Tag propagation to mirror (`refs/tags/<tag>:refs/tags/<tag>`, no `--force`).
8. `gh release create --prerelease` on `eddacraft/anvil-001`. The `--prerelease`
   flag is load-bearing: anvil-001 tracks Anvil product releases
   (`v0.x.y-beta`), not crate sub-product releases. Without it, every crate cut
   would pin as the anvil-001 `latest` and shadow the Anvil product release list
   — see D-TUIR-021 (ratified by TUIR-009, 2026-06-07) and the "Mirror Release
   backfill" subsection below.

If any gate fails before step 6, no state has been mutated outside anvil-001. If
step 6 succeeds but a later step fails, see Rollback below.

### 4. Verify

```bash
# crates.io — the new version is the latest advertised.
cargo search eddacraft-tui

# Mirror — the new tag exists on the public repo.
gh api repos/eddacraft/eddacraft-tui/git/refs/tags --jq '.[] | .ref' | grep "refs/tags/eddacraft-tui-vX.Y.Z"

# anvil-001 — the GitHub Release exists, marked as pre-release so it
# doesn't pin as the anvil-001 `latest` (anvil-001 tracks Anvil product
# releases, not crate sub-product releases; the canonical user-facing
# release is on the public mirror, see "Mirror Release backfill" below).
gh release view "eddacraft-tui-vX.Y.Z"
gh api repos/eddacraft/anvil-001/releases/tags/eddacraft-tui-vX.Y.Z --jq .prerelease \
  | grep -Fxq true \
  && echo "anvil-001 release is correctly marked as pre-release" \
  || echo "::error::anvil-001 release is NOT marked as pre-release — re-pin with: gh api -X PATCH repos/eddacraft/anvil-001/releases/<id> -f prerelease=true"

# Mirror — the GitHub Release exists and is the latest (publish workflow
# only creates the anvil-001 release; the mirror release is a backfill
# — see "Mirror Release backfill" under Rollback). If this command
# returns the prior legacy `vX.Y.Z` instead of the new prefixed tag,
# run the backfill before declaring the cut done.
gh api repos/eddacraft/eddacraft-tui/releases/latest --jq .tag_name
gh release view "eddacraft-tui-vX.Y.Z" -R eddacraft/eddacraft-tui

# Mirror — the new release's `target_commitish` equals the tag's commit,
# NOT the mirror's `main` HEAD. The default `gh release create` target
# is `main`; if you don't pin it, the release gets re-anchored to
# whatever `main` HEAD is at the next mirror force-push.
TAG_COMMIT=$(gh api "repos/eddacraft/eddacraft-tui/git/refs/tags/eddacraft-tui-vX.Y.Z" --jq .object.sha \
  | xargs -I{} gh api "repos/eddacraft/eddacraft-tui/git/tags/{}" --jq .object.sha)
gh api repos/eddacraft/eddacraft-tui/releases/tags/eddacraft-tui-vX.Y.Z --jq .target_commitish \
  | grep -Fxq "${TAG_COMMIT}" \
  && echo "mirror release target OK: ${TAG_COMMIT}" \
  || echo "::error::mirror release target_commitish does not match the vX.Y.Z tag's commit — re-pin with: gh api -X PATCH repos/eddacraft/eddacraft-tui/releases/<id> -f target_commitish=${TAG_COMMIT}"

# Mirror — CHANGELOG and README body are in sync with canonical source
# (mirror is a subtree split; this confirms no drift was introduced by
# the mirror workflow between the cut and the backfill).
diff -u crates/eddacraft-tui/CHANGELOG.md \
  <(gh api -H 'Accept: application/vnd.github.raw' \
      repos/eddacraft/eddacraft-tui/contents/CHANGELOG.md)
# README: mirror README is `MIRROR-README.md` (banner) + canonical
# `README.md` (body), concatenated by `mirror-eddacraft-tui.yml` step
# "Swap MIRROR-README onto README". The banner's line count is
# derived dynamically so the offset stays correct if the banner ever
# grows (e.g. a new "How to depend" paragraph is added to
# MIRROR-README.md between releases).
MIRROR_README_BANNER_LINES=$(wc -l < crates/eddacraft-tui/MIRROR-README.md)
diff -u crates/eddacraft-tui/README.md \
  <(gh api -H 'Accept: application/vnd.github.raw' \
      repos/eddacraft/eddacraft-tui/contents/README.md \
    | tail -n +$((MIRROR_README_BANNER_LINES + 1)))

# Downstream Anvil consumers still build green (sanity).
cargo check -p eddacraft-anvil-tui
cargo check -p eddacraft-anvil
```

## Rollback

The crates.io publish is the irreversible step. Plan around it.

### `cargo publish` failed before completing

No state mutated outside anvil-001. Fix the underlying issue, push a new commit
on `main` if a source fix is needed, then push a fresh tag
(`eddacraft-tui-vX.Y.Z+1` or a re-cut after deleting the old tag). Do not re-use
the same tag name unless the previous tag was deleted locally AND on origin —
git tag-pushes are not idempotent across ref-deletion.

### `cargo publish` succeeded but a later step failed

The crate is on crates.io and cannot be un-published, only yanked. Decide:

- **Tag propagation to mirror failed:** first check whether the cause is tag
  protection denying the App (Bootstrap gotcha #5) — if so, fix that before
  retrying, or the retry is denied the same way. **Do NOT re-dispatch the whole
  publish workflow once `cargo publish` has already succeeded:** the publish
  step hard-fails on the already-published version (`cargo` rejects the
  duplicate with a non-zero exit, and `set -e` stops the job before it reaches
  the mirror push), so a re-dispatch fails earlier than the step you are trying
  to retry. Instead, push **only the tag** manually with a short-lived App token
  (not a PAT):
  ```bash
  # INSTALL_TOKEN is a short-lived eddacraft-mirror-bot installation token —
  # the same credential CI mints via actions/create-github-app-token. Minting
  # one by hand needs the App's numeric ID (the EDDACRAFT_MIRROR_BOT_APP_ID
  # repo secret) and a PEM private key for the App (generate one per the
  # "eddacraft-mirror-bot App private key" section below). In a pinch, a fresh
  # workflow_dispatch of the publish workflow is faster than minting locally.
  app_id=<EDDACRAFT_MIRROR_BOT_APP_ID>      # numeric App ID
  pem=/path/to/eddacraft-mirror-bot.pem     # PEM private key for the App
  b64url() { openssl base64 -A | tr '+/' '-_' | tr -d '='; }
  now=$(date +%s)
  jwt_h=$(printf '{"alg":"RS256","typ":"JWT"}' | b64url)
  jwt_p=$(printf '{"iat":%d,"exp":%d,"iss":"%s"}' \
      "$((now - 60))" "$((now + 540))" "$app_id" | b64url)
  jwt="${jwt_h}.${jwt_p}"
  jwt="${jwt}.$(printf '%s' "$jwt" | openssl dgst -sha256 -sign "$pem" -binary | b64url)"
  # Resolve the App's eddacraft installation, then mint the token (≤1h TTL):
  install_id=$(curl -fsS -H "Authorization: Bearer ${jwt}" \
      -H 'Accept: application/vnd.github+json' \
      https://api.github.com/app/installations \
      | jq -r '.[] | select(.account.login == "eddacraft") | .id')
  INSTALL_TOKEN=$(curl -fsS --request POST \
      -H "Authorization: Bearer ${jwt}" \
      -H 'Accept: application/vnd.github+json' \
      "https://api.github.com/app/installations/${install_id}/access_tokens" \
      | jq -r '.token')
  #
  # `base64 -w0` is GNU-only and fails on macOS/BSD base64 (no -w
  # flag). The `tr -d '\n'` form strips the line-wrap on either
  # implementation, so this snippet is portable across Linux + macOS.
  basic=$(printf '%s' "x-access-token:${INSTALL_TOKEN}" | base64 | tr -d '\n')
  git -c "http.https://github.com/.extraheader=Authorization: Basic ${basic}" \
      push \
      https://github.com/eddacraft/eddacraft-tui.git \
      "refs/tags/eddacraft-tui-vX.Y.Z:refs/tags/eddacraft-tui-vX.Y.Z"
  unset INSTALL_TOKEN basic app_id pem jwt jwt_h jwt_p install_id
  ```
  Do NOT use `--force` — if the tag already exists on the mirror, investigate
  before pushing (someone may have pushed a different SHA).
- **`gh release create` failed:** create the release manually.
  ```bash
  gh release create "eddacraft-tui-vX.Y.Z" \
    --title "eddacraft-tui-vX.Y.Z" \
    --notes "Published from canonical source at crates/eddacraft-tui/. See CHANGELOG."
  ```

### Mirror Release backfill (publish workflow does NOT create this)

The publish workflow's `gh release create` step targets `anvil-001` only. The
public mirror `eddacraft/eddacraft-tui` never gets a GitHub Release object
created automatically — its `…/releases/latest` would otherwise stay pinned at
the most recent legacy `v0.x.y` release indefinitely, and external consumers
landing on the mirror would see an outdated "Latest" badge even though the
prefixed `eddacraft-tui-vX.Y.Z` tag is present. This is a structural,
intentional choice (D-TUIR-021): scoping the Release to `anvil-001` keeps the
publish workflow free of any second GitHub App or PAT and preserves D-TUIR-009 /
D-TUIR-011 tag-protection guarantees. The mirror Release is therefore an
**operator backfill**, not an automation step, and must be run as part of every
cut.

If the verify block above reports the mirror `…/releases/latest` is the prior
legacy tag, run:

```bash
# 1. Build the release body from the canonical CHANGELOG entry.
notes=$(mktemp --suffix=.md)
{
  echo '# eddacraft-tui X.Y.Z — YYYY-MM-DD'
  echo
  echo '> **Note:** This repository is a read-only mirror. The canonical source for'
  echo '> `eddacraft-tui` lives in the Anvil monorepo; releases are published from'
  echo '> `crates/eddacraft-tui/` and mirrored here. To depend on this crate, use the'
  echo '> crates.io release (`eddacraft-tui = "X.Y"`), not git `main`.'
  echo
  echo '## Changes'
  echo
  awk '/^## \[X\.Y\.Z\]/{flag=1; next} /^## \[<prev>\]/{flag=0} flag' \
    crates/eddacraft-tui/CHANGELOG.md
  echo
  echo '## Links'
  echo
  echo '- **Source:** [crates/eddacraft-tui/](https://github.com/eddacraft/anvil-001/tree/main/crates/eddacraft-tui) in the Anvil monorepo'
  echo '- **Changelog:** [crates/eddacraft-tui/CHANGELOG.md](https://github.com/eddacraft/anvil-001/blob/main/crates/eddacraft-tui/CHANGELOG.md)'
  echo '- **crates.io:** [eddacraft-tui](https://crates.io/crates/eddacraft-tui)'
  echo '- **API docs:** [docs.rs/eddacraft-tui/X.Y.Z](https://docs.rs/eddacraft-tui/X.Y.Z)'
  echo '- **Runbook:** [docs/runbooks/eddacraft-tui-release.md](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/eddacraft-tui-release.md)'
} > "${notes}"

# 2. Resolve the tag's commit (annotated tag → peel to commit).
TAG_COMMIT=$(gh api "repos/eddacraft/eddacraft-tui/git/refs/tags/eddacraft-tui-vX.Y.Z" --jq .object.sha \
  | xargs -I{} gh api "repos/eddacraft/eddacraft-tui/git/tags/{}" --jq .object.sha)

# 3. Create the mirror release pinned to the tag's commit (NOT main HEAD).
gh release create eddacraft-tui-vX.Y.Z \
  -R eddacraft/eddacraft-tui \
  --title "eddacraft-tui X.Y.Z" \
  --notes-file "${notes}" \
  --target "${TAG_COMMIT}"

# 4. If the release was created against the wrong target (default = main),
# patch the existing release in place — do NOT delete and re-create, that
# loses the release URL.
REL_ID=$(gh api repos/eddacraft/eddacraft-tui/releases/tags/eddacraft-tui-vX.Y.Z --jq .id)
gh api -X PATCH "repos/eddacraft/eddacraft-tui/releases/${REL_ID}" \
  -f target_commitish="${TAG_COMMIT}"

# 5. Re-run the Verify block's mirror Release checks to confirm.
unset notes TAG_COMMIT REL_ID
```

Why `--target` matters: `gh release create` defaults to `main`, so without
`--target` the release's `target_commitish` resolves to whatever `main` HEAD is
at backfill time — not the commit the `eddacraft-tui-vX.Y.Z` tag actually points
to. The next mirror force-push advances `main` and silently re-anchors the
release. The `--target "${TAG_COMMIT}"` form pins the release to the immutable
tag commit. `target_commitish` is patchable after the fact (step 4) for the same
reason — fixing the wrong target must NOT delete the release (and its URL), it
must rewrite the field.

### Bad version got onto crates.io

Yank the published version. Yanking does NOT delete the artifact — existing
`Cargo.lock` pins still resolve, but new resolutions and fresh installs error
out. Use yank only for genuinely broken versions, not for cosmetic fixes.

```bash
cargo yank --version X.Y.Z eddacraft-tui
```

To un-yank later (e.g. if the yank was hasty):

```bash
cargo yank --version X.Y.Z --undo eddacraft-tui
```

The mirror tag and the GitHub Release stay in place after yank — they are
append-only by policy (D-TUIR-009 / D-TUIR-011).

## Migration rollback (two layers)

The "Rollback" section above covers a single failed publish. This section covers
backing out the **canonical-source migration itself** if the model proves
unworkable. Two layers, smallest blast radius first.

### Layer (a): dependency rollback — revert Anvil consumers only

Use when Anvil-side consumption of the path crate is the problem but the
migration model is otherwise sound. Reverts Anvil consumers to the crates.io
release without losing in-flight crate work:

1. In the workspace dep table (root `Cargo.toml`), repoint `eddacraft-tui` from
   the `path = "crates/eddacraft-tui"` form back to the published version
   requirement: `eddacraft-tui = "0.2.2"` (or the latest published version).
2. Drop any per-crate `path` overrides on first-party consumers
   (`crates/anvil-tui`, `crates/anvil-cli`) so they inherit the workspace
   crates.io dep.
3. Regenerate workspace-hack: `cargo hakari generate` (the registry dep
   re-enters hakari aggregation). Run this AFTER the dep edit — hakari ordering
   is load-bearing (see TUIR-003): regenerating before the rewrite leaves the
   split-resolution graph in place.
4. Validate: `cargo tree -p eddacraft-anvil-tui -i eddacraft-tui` shows the
   registry crate; `cargo hakari verify`; `cargo test --workspace`.

The canonical source at `crates/eddacraft-tui/` stays in place — this only
changes how Anvil _consumes_ it. In-flight crate edits remain in the workspace
tree and can be published later.

### Layer (b): full-migration rollback — abandon canonical source

Use only if the canonical-source + mirror model must be abandoned wholesale.

1. Apply Layer (a) to revert consumers to `eddacraft-tui = "0.2.2"`.
2. Un-freeze `eddacraft/eddacraft-tui` as the canonical standalone source again:
   re-enable human PRs (remove/relax the PR-redirect workflow and the branch
   protection that allow only the mirror bot to push), and restore the
   standalone README (drop the prepended mirror banner).
3. Resume standalone publishing from `eddacraft/eddacraft-tui` — re-instate the
   repo's own `CARGO_REGISTRY_TOKEN`. **Do NOT revoke the legacy
   `CARGO_REGISTRY_TOKEN` while rollback is still a live option** (TUIR-008
   schedules its revocation only after the first publish from canonical source
   commits to the new model).
4. Stand down the Anvil-side `.github/workflows/mirror-eddacraft-tui.yml` and
   `.github/workflows/publish-eddacraft-tui.yml` workflows — disable them, do
   not delete, for audit.

**Irreversibility caveat.** The public mirror's `main` history rewrite is
**not** reversible — the pre-cutover graph cannot be restored as `main`.
Forensics and re-basing of any standalone work rely on `pre-canonical-archive`
(D-TUIR-010) and the preserved `v0.x.y` tags (D-TUIR-011). This is exactly why
the preservation step is a hard precondition of cutover, not an afterthought.

## Token rotation

### `CRATES_IO_EDDACRAFT_TUI_TOKEN`

1. Log into crates.io with the crate owner account → Account Settings → API
   Tokens → New Token.
2. Scope: `publish-update` only (NOT `publish-new` — the crate already exists).
   Crate-scope filter: `eddacraft-tui`. Pick an expiry.
3. Copy the value once (it's only shown at creation).
4. Stash to both sinks in one paste (matches the
   `agent-vault --reveal needs a TTY` workaround — value never lands in shell
   history):
   ```bash
   read -rs TOKEN && echo
   agent-vault set crates-io-eddacraft-tui-token \
     --from-env TOKEN \
     --desc 'crates.io publish-update token, eddacraft-tui crate, TUIR-005'
   printf '%s' "$TOKEN" \
     | gh secret set CRATES_IO_EDDACRAFT_TUI_TOKEN \
         -R eddacraft/anvil-001 --body -
   unset TOKEN
   ```
5. After the first successful publish with the new token, revoke the old one on
   crates.io (same Account Settings page).

### `eddacraft-mirror-bot` App private key

GitHub App private keys are long-lived but rotatable. The App itself does not
expire — only the per-run installation tokens do (those are minted fresh each
workflow run with a 1-hour lifetime, no rotation required).

Rotate the App private key when:

- A team member with access to the stored private key leaves.
- The PEM file may have been exposed (committed accidentally, sent through an
  insecure channel, stored on a compromised host).
- Routine: every 12 months as a hygiene baseline.

Steps:

1. GitHub → `eddacraft` org Settings → Developer settings → GitHub Apps →
   `eddacraft-mirror-bot` → Private keys → Generate a private key. Downloads a
   new `.pem` file. The App ID is unchanged.
2. Stash the new PEM to both sinks:
   ```bash
   PEM=/path/to/eddacraft-mirror-bot.YYYY-MM-DD.private-key.pem
   agent-vault set eddacraft-mirror-bot-private-key --stdin \
     --desc 'GitHub App private key for eddacraft-mirror-bot (PEM, rotated YYYY-MM-DD)' \
     < "$PEM"
   gh secret set EDDACRAFT_MIRROR_BOT_PRIVATE_KEY \
     -R eddacraft/anvil-001 < "$PEM"
   shred -u "$PEM" 2>/dev/null || rm -P "$PEM" 2>/dev/null || rm "$PEM"
   unset PEM
   ```
3. Run any App-authenticated workflow once to confirm the new key works — e.g.
   `gh workflow run .github/workflows/mirror-eddacraft-tui.yml -R eddacraft/anvil-001 --ref main -f reason='post-rotation smoke'`.
4. Once you confirm a successful run, delete the OLD private key on the GitHub
   App's Private keys page. Keys not deleted there remain valid even after a new
   one is generated.

The `EDDACRAFT_MIRROR_BOT_APP_ID` secret never needs rotation — the App ID is
permanent for the App's lifetime.

### Retiring the legacy `EDDACRAFT_TUI_MIRROR_PUSH_TOKEN` PAT

The pre-App-migration PAT (`EDDACRAFT_TUI_MIRROR_PUSH_TOKEN` on `anvil-001` repo
secrets, plus its `eddacraft-tui-mirror-push-token` entry in agent-vault) is no
longer referenced by any workflow after TUIR-005 lands. After the first
successful App-authenticated mirror sync or tag propagation:

```bash
gh secret delete EDDACRAFT_TUI_MIRROR_PUSH_TOKEN -R eddacraft/anvil-001
agent-vault rm eddacraft-tui-mirror-push-token
# Then revoke the underlying PAT on the GitHub fine-grained token page.
```

## Local reproduction

The publish workflow runs on GitHub Actions `ubuntu-latest`, which ships GNU
coreutils. The mirror sibling workflow uses `git subtree split` with a
multi-segment prefix and is broken under uutils coreutils 0.2.2's `dirname`
(returns `a/b` for `a/b/c/.`). The publish workflow does NOT call
`git subtree split` — only the mirror content workflow does — so local
reproduction of the publish flow is unaffected by uutils. If you need to repro
the mirror workflow locally on a uutils box, see the `/tmp/gnu-shim/dirname`
shim noted in `plans/archive/modules/tui-reintegration.aps.md` (D-TUIR-004
runner constraint).

To dry-run the publish-side gates locally without publishing:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p eddacraft-tui --all-features
cargo test -p eddacraft-tui --no-default-features
RUSTDOCFLAGS='-D warnings' cargo doc --no-deps -p eddacraft-tui --all-features
cargo deny --all-features check --config attribution/deny.toml
cargo publish --dry-run -p eddacraft-tui --all-features --allow-dirty
diff \
  plans/specs/2026-05-22-tui-reintegration-baseline/package-list.txt \
  <(cargo package --list -p eddacraft-tui --allow-dirty)
```
