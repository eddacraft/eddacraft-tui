# `eddacraft-tui` Crate Release — Operator Runbook

| Type    | Authority     | Owner  | Status | Freshness                                 |
| ------- | ------------- | ------ | ------ | ----------------------------------------- |
| Runbook | Authoritative | @aneki | Live   | First filed 2026-05-25 alongside TUIR-005 |

| Upstream                                                                                                                                                                                                                                                                                                    | Downstream                                                                                                                                                                                                                                                                                                                                                       |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [TUIR module](../../plans/modules/tui-reintegration.aps.md), [ADR-047](../../plans/decisions/047-eddacraft-tui-canonical-source-mirror.md), [`docs/policies/eddacraft-tui-mirror.md`](../policies/eddacraft-tui-mirror.md), [TUIR-001 baseline](../../plans/specs/2026-05-22-tui-reintegration-baseline.md) | [`.github/workflows/publish-eddacraft-tui.yml`](../../.github/workflows/publish-eddacraft-tui.yml), [`.github/workflows/mirror-eddacraft-tui.yml`](../../.github/workflows/mirror-eddacraft-tui.yml), [`crates/eddacraft-tui/CHANGELOG.md`](../../crates/eddacraft-tui/CHANGELOG.md), [`crates/eddacraft-tui/Cargo.toml`](../../crates/eddacraft-tui/Cargo.toml) |

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

The App + mirror-repo setup has three config traps that each silently fail in a
distinct way on the first attempt, plus a UI save gotcha that manifests on top
of the third. Walk through them in order; the API verification commands surface
state the GitHub UI hides.

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
   `cargo doc --no-deps -p eddacraft-tui` (with `RUSTDOCFLAGS=-D warnings`),
   `cargo deny check`,
   `cargo publish --dry-run -p eddacraft-tui --all-features`,
   `cargo package --list -p eddacraft-tui` byte-diff against the TUIR-001
   baseline.
6. `cargo publish -p eddacraft-tui --all-features` to crates.io.
7. Tag propagation to mirror (`refs/tags/<tag>:refs/tags/<tag>`, no `--force`).
8. `gh release create` on `eddacraft/anvil-001`.

If any gate fails before step 6, no state has been mutated outside anvil-001. If
step 6 succeeds but a later step fails, see Rollback below.

### 4. Verify

```bash
# crates.io — the new version is the latest advertised.
cargo search eddacraft-tui

# Mirror — the new tag exists on the public repo.
gh api repos/eddacraft/eddacraft-tui/git/refs/tags --jq '.[] | .ref' | grep "refs/tags/eddacraft-tui-vX.Y.Z"

# anvil-001 — the GitHub Release exists.
gh release view "eddacraft-tui-vX.Y.Z"

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

- **Tag propagation to mirror failed:** the simplest recovery is to re-dispatch
  the publish workflow via the Actions UI — the `cargo publish` step is
  idempotent on the same version (cargo rejects double-publish), so re-running
  only repeats the mirror push and the GitHub Release create. If you need to
  push the tag manually (e.g. the publish workflow itself is broken), use a
  short-lived App token minted locally rather than reaching for a PAT:
  ```bash
  # One-shot installation token via gh + jwt + the App private key.
  # See docs.github.com authentication-as-a-github-app for the JWT
  # signing helper; in a pinch a fresh workflow_dispatch is faster.
  #
  # `base64 -w0` is GNU-only and fails on macOS/BSD base64 (no -w
  # flag). The `tr -d '\n'` form strips the line-wrap on either
  # implementation, so this snippet is portable across Linux + macOS.
  basic=$(printf '%s' "x-access-token:${INSTALL_TOKEN}" | base64 | tr -d '\n')
  git -c "http.https://github.com/.extraheader=Authorization: Basic ${basic}" \
      push \
      https://github.com/eddacraft/eddacraft-tui.git \
      "refs/tags/eddacraft-tui-vX.Y.Z:refs/tags/eddacraft-tui-vX.Y.Z"
  unset INSTALL_TOKEN basic
  ```
  Do NOT use `--force` — if the tag already exists on the mirror, investigate
  before pushing (someone may have pushed a different SHA).
- **`gh release create` failed:** create the release manually.
  ```bash
  gh release create "eddacraft-tui-vX.Y.Z" \
    --title "eddacraft-tui-vX.Y.Z" \
    --notes "Published from canonical source at crates/eddacraft-tui/. See CHANGELOG."
  ```

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
shim noted in `plans/modules/tui-reintegration.aps.md` (D-TUIR-004 runner
constraint).

To dry-run the publish-side gates locally without publishing:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p eddacraft-tui --all-features
cargo test -p eddacraft-tui --no-default-features
RUSTDOCFLAGS='-D warnings' cargo doc --no-deps -p eddacraft-tui
cargo deny check
cargo publish --dry-run -p eddacraft-tui --all-features --allow-dirty
diff \
  plans/specs/2026-05-22-tui-reintegration-baseline/package-list.txt \
  <(cargo package --list -p eddacraft-tui --allow-dirty)
```
