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
  permissions `Contents: Read and write` + `Metadata: Read`). Same App used by
  the mirror content workflow (`.github/workflows/mirror-eddacraft-tui.yml`).
  The workflow mints a short-lived installation token at runtime via
  `actions/create-github-app-token` and uses it to push the release tag to the
  mirror.

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
  basic=$(printf '%s' "x-access-token:${INSTALL_TOKEN}" | base64 -w0)
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
