# First Rust Release — Rehearsal Runbook

> **Status:** Draft, never executed **Purpose:** Shakedown plan for the first
> time the Rust cargo-dist release pipeline runs end-to-end. Use as a reference
> while working through the actual rehearsal. **Audience:** eddacraft release
> engineer (you) **Companion docs:**
>
> - [`docs/runbooks/release-runbook.md`](../../runbooks/release-runbook.md) — the operational
>   runbook for _ongoing_ releases (assumes the pipeline works)
> - [`plans/archive/modules/distribution-pipeline.aps.md`](../../../plans/archive/modules/distribution-pipeline.aps.md)
>   — DIST module with item-level scope (archived)
> - [`plans/decisions/018-product-ip-architecture.md`](../../../plans/decisions/018-product-ip-architecture.md)
>   — closed-source IP model

## Why this doc exists

The Anvil cargo-dist release pipeline (`.github/workflows/release.yml`,
`dist-workspace.toml`) has been authored, configured, and committed — but **has
never actually run for a Rust release**. The four tagged releases on the private
repo (`v0.1.0` through `v0.2.1-beta`) are all TypeScript CLI releases from
before the Rust rewrite. The Rust pipeline is theoretical until someone pushes a
tag and watches it execute.

The first time we push a Rust tag, multiple things will be running for the first
time simultaneously:

- `cargo-dist` 0.31.0 build matrix across 6 platform targets
- Cross-compilation for aarch64 Linux, both macOS arches, both Windows arches
- The custom cross-repo push step that copies the release from the private
  monorepo to `eddacraft/anvil` (public)
- The `ANVIL_RELEASES_TOKEN` cross-repo permissions
- `GitHub Pages → install.eddacraft.ai` DNS resolution
- The cargo-dist installer scripts (shell + powershell)

**Anything that breaks will look like a release failure**, and we won't be able
to tell which layer is at fault without isolating them. Hence the
rehearsal-first approach: prove the base pipeline works in isolation before
stacking Homebrew, WinGet, or scoop on top.

## The plan in five phases

| Phase                        | Goal                                                                             | Risk                                                                  | Reversible?                  |
| ---------------------------- | -------------------------------------------------------------------------------- | --------------------------------------------------------------------- | ---------------------------- |
| **A — Pre-flight**           | Verify everything that can be checked statically before pushing a tag            | Zero                                                                  | n/a (no changes)             |
| **B — First rehearsal tag**  | Push a clearly-labeled test tag, watch the workflow run, fix what breaks, repeat | Medium — burns Actions minutes, may leave a mess on `eddacraft/anvil` | Yes (delete release + tag)   |
| **C — Verify install path**  | Confirm the installer artifact actually installs the binary on a clean machine   | Low                                                                   | Yes                          |
| **D — Add layer-2 channels** | Wire up Homebrew, WinGet, scoop on top of the now-known-good pipeline            | Low (each layer is additive)                                          | Yes                          |
| **E — First real release**   | `v0.3.0-beta.0` (or whatever the first real Rust release is)                     | Low — pipeline is shaken down                                         | Hard (it's a public release) |

---

## Phase A — Pre-flight checks

Run all of these _before_ pushing any tag. They're all read-only, all fast, all
zero-risk.

### A1. Confirm the public repo has a `main` branch with content

The custom cross-repo push step in `release.yml` (the `push-to-public-repo` job)
does:

```bash
PUBLIC_HEAD=$(gh api repos/eddacraft/anvil/git/ref/heads/main -q '.object.sha')
```

If `eddacraft/anvil` has no `main` branch (or has `main` but no commits), this
fails immediately and the entire release is dead in the water.

**Check:**

```bash
gh api repos/eddacraft/anvil/branches/main --jq '.commit.sha' 2>&1
gh api repos/eddacraft/anvil/contents/README.md --jq '.name' 2>&1
```

**Expected:** a commit SHA and `README.md`. If either errors, you need to
bootstrap the public repo with at least:

- A `README.md` with project description and install instructions
- A `LICENSE` file (the public repo's license can be permissive — it only covers
  the README, install scripts, and any other files committed to that repo, _not_
  the binary itself, which is governed by the EULA)
- An empty `index.html` or `docs/install.sh` if GitHub Pages is meant to serve
  the install script

This is a one-time bootstrap. Do it before any tag push.

### A2. Confirm `ANVIL_RELEASES_TOKEN` exists with the right scopes

The release workflow needs a fine-grained PAT (or classic PAT with `repo` scope)
that has cross-repo write to:

- `eddacraft/anvil` — `contents:write` (releases, tags) and `metadata:read`
- `eddacraft/homebrew-tap` — `contents:write` (when DIST-009 lands)

**Check:**

```bash
gh secret list 2>&1 | grep -i anvil
```

**Expected:** `ANVIL_RELEASES_TOKEN` listed. If not present, you need to create
the PAT and add it as a repo secret. Fine-grained PATs are the safer option —
they expire and have explicit scopes per repo.

**To verify the token's scopes** (after it's set), the easiest test is to
manually exercise it:

```bash
# In a private gist or scratch repo, with the token exported as GH_TOKEN
GH_TOKEN=ghp_xxx gh api repos/eddacraft/anvil --jq '.permissions'
```

If it returns `{"admin":true,"push":true,"pull":true}` or similar, the token has
write access.

### A3. Confirm `cargo-dist` 0.31.0 is installable

The workflow caches a `dist` binary; on first run it has to install v0.31.0.
Verify the version still resolves on crates.io / GitHub releases:

```bash
curl -fsSL https://github.com/axodotdev/cargo-dist/releases/tag/v0.31.0 -o /dev/null -w "%{http_code}\n"
```

**Expected:** HTTP 200. If 404, cargo-dist may have moved or unpublished — pin a
different version in `dist-workspace.toml` and regenerate.

### A4. Run `dist plan` locally

This is the cheapest and most informative pre-flight. It tells you exactly what
cargo-dist would do for a hypothetical tag without actually building anything.

**Install dist locally** (one-time):

```bash
cargo install cargo-dist --version 0.31.0
# or via the dist installer
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/axodotdev/cargo-dist/releases/download/v0.31.0/cargo-dist-installer.sh | sh
```

**Run plan from the workspace root:**

```bash
cd <repo_root>
dist plan --tag v0.3.0-rc.0
```

**Expected output:**

- A list of "Apps to release" — should include `eddacraft-anvil` (and _only_
  `eddacraft-anvil`, since everything else is `publish=false` or a library)
- A list of "Artifacts to build" — 6 platform binaries + checksums + installer
  scripts
- No errors about missing fields, conflicting versions, or unknown installers

**If `dist plan` errors:** that's the cheapest possible failure mode — fix the
config before doing anything else. Common issues at this stage: missing
`description` / `repository` fields on the binary crate, missing
`[package.metadata.dist] dist = true` on something that needs it, version
mismatch between `cargo-dist` and what the workflow expects.

### A5. Confirm the workspace builds in `--profile dist`

cargo-dist uses the `dist` profile defined in the workspace `Cargo.toml`. If
this profile doesn't compile cleanly, the release build will fail in the same
way during cross-compilation.

```bash
cd <repo_root>
cargo build --profile dist -p eddacraft-anvil
```

**Expected:** clean build, ~5-10 minutes, no errors. Warnings are OK.

This catches `lto = true` / `panic = "abort"` / `strip = "symbols"` issues that
don't show up in `cargo check`.

### A6. Confirm `install.eddacraft.ai` DNS is wired

Independent of the release pipeline, but the install path depends on it:

```bash
dig +short @8.8.8.8 install.eddacraft.ai CNAME
dig +short @1.1.1.1 install.eddacraft.ai CNAME
```

**Expected:** `eddacraft.github.io`. If empty, the Pulumi DNS resource in
`infra/src/dns/eddacraft-ai.ts:57` exists but has not been deployed to Azure
DNS. Run `pulumi up` from the `infra/` directory to apply.

If you don't want to gate the rehearsal on DNS, the alternative install URL is
the direct GitHub Releases URL:

```bash
curl -fsSL https://github.com/eddacraft/anvil/releases/latest/download/eddacraft-anvil-installer.sh | sh
```

This works without any DNS, just uses an uglier URL.

### A7. Sanity-check `dist-workspace.toml` and `release.yml`

Quick read-through. Things to confirm:

```bash
grep -E "version|installers|tap|targets|hosting|ci" <repo_root>/dist-workspace.toml
```

**Expected:**

- `cargo-dist-version = "0.31.0"`
- `installers = ["shell", "powershell", "homebrew"]`
- `tap = "eddacraft/homebrew-tap"`
- `targets = [6 platforms]`
- `hosting = "github"`
- `ci = "github"`

**Note:** `installers` includes `"homebrew"` but `release.yml` does **not** have
a corresponding `publish-homebrew` job. This is a known gap (see Phase D). For
Phase B, the homebrew installer will be built as an artifact but never published
— that's fine for the rehearsal.

### Pre-flight checklist summary

- [ ] A1 — `eddacraft/anvil` has `main` with at least one commit
- [ ] A2 — `ANVIL_RELEASES_TOKEN` secret exists with cross-repo write
- [ ] A3 — `cargo-dist` 0.31.0 is reachable
- [ ] A4 — `dist plan --tag v0.3.0-rc.0` succeeds locally
- [ ] A5 — `cargo build --profile dist -p eddacraft-anvil` succeeds locally
- [ ] A6 — `install.eddacraft.ai` resolves (or accept the GitHub URL fallback)
- [ ] A7 — `dist-workspace.toml` matches expectations

Only proceed to Phase B once **A1, A2, A4, A5 are green**. A3, A6, A7 can be
deferred but they make the rehearsal noisier.

---

## Phase B — First rehearsal tag

The goal here is to push a tag, watch the workflow run end-to-end, and catalogue
what breaks. Expect failures. The tag name should make it clear this is not a
real release.

### B1. Pick a rehearsal version

Use a pre-release suffix that does not collide with anything that might become a
real release:

```
v0.3.0-rc.0
```

`rc.0` (release candidate zero) signals "this is a rehearsal, not the first real
release candidate." If `rc.0` itself feels too release-shaped, you can use
`v0.3.0-test.0` or `v0.0.0-rehearsal.1` — anything that won't be mistaken for a
real release.

Update the workspace version:

```bash
# In Cargo.toml [workspace.package]
version = "0.3.0-rc.0"
```

Then re-run `dist plan --tag v0.3.0-rc.0` to confirm it still parses.

### B2. Push the tag on a throwaway branch

**Do not push the tag on `main` or `dev`** for the rehearsal. Use a disposable
branch so you can rewind cleanly if anything goes sideways:

```bash
git checkout -b rehearsal/first-rust-release
git add Cargo.toml Cargo.lock
git commit -m "chore(release): bump to v0.3.0-rc.0 for rehearsal"
git push -u origin rehearsal/first-rust-release

# Create the tag pointing at the rehearsal commit
git tag v0.3.0-rc.0
git push origin v0.3.0-rc.0
```

### B3. Watch the workflow

The moment the tag lands, the `Release` workflow should fire. Watch it in the
GitHub Actions UI:

```bash
gh run watch --exit-status
# or
gh run list --workflow=release.yml --limit 1
```

**Jobs to watch (in order):**

1. **`plan`** — runs `dist plan` in CI. Should match what you saw locally in A4.
   If it fails, the issue is config; fix and re-tag.
2. **`build-local-artifacts`** — 6 platform builds in parallel matrix.
   Cross-compilation is the most likely failure point.
3. **`build-global-artifacts`** — installer scripts, manifest.
4. **`host`** — uploads artifacts and creates the GitHub Release on the
   **private** repo first via `dist host`.
5. **Custom step "Publish release to eddacraft/anvil (public)"** — the
   hand-written cross-repo push (lines 288–328 of `release.yml`). This is the
   highest-risk step because it's untested.
6. **`announce`** — posts the announcement.

### B4. Expected failure modes (and what to do)

| Symptom                                                                         | Likely cause                                                                                                                           | Fix                                                                                                                                             |
| ------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| `plan` fails with "missing field" or "invalid manifest"                         | Config bug in `dist-workspace.toml` or a `Cargo.toml`                                                                                  | Fix in workspace, push new commit, re-tag with `rc.1`                                                                                           |
| `build-local-artifacts` fails on Linux aarch64                                  | Cross-compilation toolchain not in cache                                                                                               | cargo-dist usually handles this; check matrix logs for which step failed                                                                        |
| `build-local-artifacts` fails on Windows                                        | A dependency in the workspace doesn't compile on Windows (rare but possible — `notify`, `crossterm`, `tree-sitter-*` are usually fine) | Identify the offending crate, decide whether to feature-gate Windows or fix                                                                     |
| `build-local-artifacts` fails because of `lto = true`                           | LTO can OOM cross-compilation; lower opt-level temporarily                                                                             | Edit `[profile.dist]` in `Cargo.toml`, push, re-tag                                                                                             |
| `host` fails uploading artifacts                                                | GitHub API rate limit or transient                                                                                                     | Re-run the failed job from the Actions UI                                                                                                       |
| `host` succeeds but the **release on private repo** is created without binaries | `dist host --steps=upload --steps=release` arg mismatch                                                                                | Check cargo-dist 0.31 docs for the correct `--steps` invocation                                                                                 |
| Custom cross-repo step fails with `gh: command not found`                       | gh CLI not installed in the runner                                                                                                     | Add a setup step (it usually is preinstalled on Ubuntu runners)                                                                                 |
| Custom cross-repo step fails with `Resource not accessible by integration`      | `ANVIL_RELEASES_TOKEN` missing or wrong scopes                                                                                         | Re-issue the PAT, update the secret, re-run the job                                                                                             |
| Custom cross-repo step fails with `Reference not found` for `refs/heads/main`   | Public repo doesn't have a `main` branch                                                                                               | Bootstrap the public repo (see A1)                                                                                                              |
| Custom cross-repo step fails when **creating the tag** on the public repo       | The push above already created the tag from a previous attempt                                                                         | The workflow handles this (see line 314 of `release.yml`); if it doesn't, manually delete the tag from the public repo via `gh api -X DELETE …` |
| Public release is created but binaries are missing                              | Artifact glob `artifacts/*` matched the wrong files                                                                                    | Check the artifact list in the job log; adjust the glob                                                                                         |

**The pattern:** fix → push → re-tag with incrementing `rc.N`. **Do not delete
and re-push the same tag** — it confuses git, GitHub Releases, and people.
Always increment.

### B5. Cleanup after each failed attempt

If a cross-repo push half-succeeds and leaves a broken release on the public
repo:

```bash
# Delete the broken release on the public repo
gh release delete v0.3.0-rc.0 --repo eddacraft/anvil --yes

# Delete the tag on the public repo
gh api -X DELETE repos/eddacraft/anvil/git/refs/tags/v0.3.0-rc.0
```

This is the only situation where deleting a tag is OK — it's a rehearsal release
that no users have ever seen.

### B6. Definition of "rehearsal succeeded"

Phase B is done when, **on a single tag push**:

- All jobs in `release.yml` complete successfully
- A release exists on `eddacraft/anvil` with all 6 platform binaries
  - checksums + installer scripts
- `gh release view v0.3.0-rc.N --repo eddacraft/anvil` shows the expected
  artifact list
- No manual fixup was needed

This may take 3–10 iterations. Budget a focused half-day.

---

## Phase C — Verify install path

Once the rehearsal release exists on the public repo, prove it actually
installs.

### C1. Test the direct GitHub URL on Linux

On a clean Linux container:

```bash
docker run --rm -it ubuntu:24.04 bash
apt update && apt install -y curl
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/eddacraft/anvil/releases/latest/download/eddacraft-anvil-installer.sh | sh

anvil --version
```

**Expected:** `anvil 0.3.0-rc.N` (or whatever rehearsal version).

**Likely failures:**

- Installer downloads but binary is missing → cargo-dist artifact layout
  mismatch
- `anvil: command not found` after install → `install-path` config is wrong, or
  the installer isn't adding `~/.cargo/bin` (or whatever) to `PATH`
- `anvil --version` segfaults → `[profile.dist]` build is broken; fall back to
  `release` profile temporarily

### C2. Test on macOS (if you have access)

Same as C1, but on a real macOS machine or a CI macOS runner. The
aarch64-apple-darwin binary should run on Apple Silicon; the x86_64-apple-darwin
binary should run via Rosetta.

### C3. Test on Windows (PowerShell)

```powershell
irm https://github.com/eddacraft/anvil/releases/latest/download/eddacraft-anvil-installer.ps1 | iex
anvil --version
```

### C4. Test the `install.eddacraft.ai` DNS path (if A6 is green)

```bash
curl -fsSL https://install.eddacraft.ai | sh
```

If A6 is not green, skip this and treat it as a separate work item.

### Phase C checklist

- [ ] Linux x86_64 install via direct GitHub URL works
- [ ] Linux aarch64 install via direct GitHub URL works (or skip if no hardware)
- [ ] macOS x86_64 install via direct GitHub URL works
- [ ] macOS aarch64 install via direct GitHub URL works
- [ ] Windows install via direct GitHub URL works
- [ ] `install.eddacraft.ai` resolves and serves the same installer (A6
      dependent)

---

## Phase D — Add layer-2 channels

Only do this once Phase C is fully green. Each channel is independent and
additive — adding one cannot break the others.

### D1. Homebrew tap (DIST-009)

**The two pieces of work:**

1. **Add `formula = "anvil"` to `crates/anvil-cli/Cargo.toml`** so the generated
   formula is `Formula/anvil.rb` (class `Anvil`) rather than the default
   generated formula name:

   ```toml
   [package.metadata.dist]
   dist = true
   formula = "anvil"
   ```

2. **Hand-write a `publish-homebrew` job in `release.yml`** that runs after the
   `host` job. The current `release.yml` has cargo-dist built it as an artifact
   in the build matrix but no job to publish it. The job needs to:
   - Wait for `host` (so the GitHub Release exists)
   - Use `ANVIL_RELEASES_TOKEN` for cross-repo write
   - Download the formula from the just-published release (or regenerate via
     `dist generate --mode=run -t homebrew`)
   - `git clone https://github.com/eddacraft/homebrew-tap`
   - Commit `Formula/anvil.rb`
   - Push

**Do not regenerate the workflow with `dist init`** — it would lose the custom
cross-repo push step. Hand-write the new job.

**Test:** push `v0.3.0-rc.N+1`, verify a commit lands in
`eddacraft/homebrew-tap`, then on a clean macOS machine:

```bash
brew install eddacraft/tap/anvil
anvil --version
```

**The install command in docs is always the fully-qualified
`eddacraft/tap/anvil`** — not the bare `anvil` — to prevent future collision if
homebrew-core ever adds an unrelated `anvil` package.

### D2. WinGet manifest (DIST-010)

The standard pattern is to use `vedantmgoyal2009/winget-releaser` (a GitHub
Action that submits a WinGet manifest PR to `microsoft/winget-pkgs`
automatically on each release).

**Add a new job to `release.yml`** after `host` succeeds. Roughly:

```yaml
publish-winget:
  needs: host
  runs-on: windows-latest
  steps:
    - uses: vedantmgoyal2009/winget-releaser@v2
      with:
        identifier: eddacraft.Anvil
        installers-regex: '\.exe$|\.zip$'
        token: ${{ secrets.WINGET_TOKEN }}
```

You'll need a `WINGET_TOKEN` PAT with public repo access (because the manifest
gets submitted as a PR to `microsoft/winget-pkgs`).

**The first WinGet submission takes 1–7 days** to be reviewed by Microsoft.
Subsequent updates are auto-merged. Submit early.

### D3. Scoop bucket (DIST-011, optional)

Lower priority. Pattern is the same as Homebrew tap: a public repo
(`eddacraft/scoop-bucket`) with a `bucket/anvil.json` manifest. Can be
hand-written or generated via cargo-dist's `installers = [..., "scoop"]` if you
add it.

### Phase D checklist

- [ ] D1 — Homebrew tap publishes formula on tag push, install works on macOS
- [ ] D2 — WinGet manifest submitted, install works on Windows 11 via
      `winget install eddacraft.anvil`
- [ ] D3 — Scoop bucket published (optional)

---

## Phase E — First real release

Once D is green (or at least D1, with D2/D3 deferred to the second real
release), you can ship `v0.3.0-beta.0` or whatever the first real Rust release
is.

By this point:

- The pipeline has been exercised multiple times
- All install paths have been tested on clean machines
- The blast radius of any remaining bugs is well-understood

**Pre-flight for the real release:**

- [ ] Tag name does not include `rc`, `test`, `rehearsal`
- [ ] `Cargo.toml` workspace version matches the tag
- [ ] Branch is `main` or `dev` (not a rehearsal branch)
- [ ] `docs/runbooks/release-runbook.md` is followed for the actual release-day
      operational steps
- [ ] Announcement post is drafted

The rehearsal runbook stops here. From this point forward, every release follows
`release-runbook.md` and is a routine operation.

---

## Appendix A — Known unknowns

Things this runbook does _not_ cover, and that might bite you during the
rehearsal:

- **Codesigning.** Apple notarisation for macOS, Authenticode signing for
  Windows. cargo-dist supports both but they require extra config and
  certificates. The first rehearsal will produce unsigned binaries that trigger
  SmartScreen warnings on Windows and Gatekeeper warnings on macOS. Acceptable
  for rc, not for the real release.
- **GitHub Pages cert.** GitHub auto-provisions Let's Encrypt for custom
  domains, but propagation can take an hour or two after the CNAME first
  resolves. If `install.eddacraft.ai` errors with a cert mismatch, wait.
- **`announce` job behaviour.** The `announce` job at the bottom of
  `release.yml` is mostly a stub right now. If it tries to post to Slack /
  Discord / a mailing list, those integrations need their own secrets.
- **Updater.** `install-updater = true` in `dist-workspace.toml` means
  cargo-dist installs the `axoupdater` companion. This is a separate binary and
  a separate failure surface. Test it explicitly: after install,
  `anvil --version` should work _and_ the updater should be able to check for
  newer versions.
- **The `eddacraft-tui` git dependency.** The workspace consumes `eddacraft-tui`
  via `git = "..."` rev. This works fine for builds, but when cargo-dist tries
  to package the `anvil-cli` crate _for the homebrew formula generation_, it may
  complain about the git dep depending on cargo-dist version. If this fails, the
  fix is to publish `eddacraft-tui` to crates.io first and pin to a version
  (uses the work already done on `chore/crates-io-publish-prep` in the
  `eddacraft-tui` repo).

## Appendix B — When to abort the rehearsal

Stop and reassess if any of the following happens:

- **Cross-repo push corrupts the public repo state** — e.g. force-push, reset,
  or accidentally overwrites `main`. Stop, restore from git reflog if possible,
  and investigate before trying again.
- **The `eddacraft/anvil` repo gets a real release with broken binaries that has
  been linked publicly** — pull the release, communicate the issue, do not "fix
  forward" silently.
- **Three consecutive rehearsal attempts fail at the same step with the same
  error** — there's a deeper issue. Stop iterating and dig in (e.g. ask in
  cargo-dist's discussions, file an issue, or consider regenerating the workflow
  with `dist init` as a clean-slate approach).
- **You're tempted to skip Phase A or Phase C "to save time"** — don't. The
  whole point of this runbook is to isolate failures by layer.

## Appendix C — Useful one-liners

```bash
# Watch the most recent release.yml run
gh run watch $(gh run list --workflow=release.yml --limit 1 --json databaseId -q '.[0].databaseId')

# List all releases on the public repo
gh release list --repo eddacraft/anvil

# Inspect a specific release
gh release view v0.3.0-rc.0 --repo eddacraft/anvil

# Delete a rehearsal release (only on rehearsal tags!)
gh release delete v0.3.0-rc.0 --repo eddacraft/anvil --yes
gh api -X DELETE repos/eddacraft/anvil/git/refs/tags/v0.3.0-rc.0

# Check what cargo-dist thinks the next release looks like
dist plan --tag v0.3.0-rc.0

# Check what dist would generate for the release config
dist generate --mode=check

# Re-run a failed job from the most recent workflow run
gh run rerun --failed
```

## Appendix D — Files touched by this runbook

When you actually execute the rehearsal, expect to edit some or all of these
files. Track them in commits.

| File                                              | Likely change                                                        |
| ------------------------------------------------- | -------------------------------------------------------------------- |
| `Cargo.toml` (workspace)                          | Bump `version` for each rc tag                                       |
| `crates/anvil-cli/Cargo.toml`                     | Add `formula = "anvil"` (Phase D)                                    |
| `dist-workspace.toml`                             | Adjust if cargo-dist version needs bumping                           |
| `.github/workflows/release.yml`                   | Add `publish-homebrew` job (Phase D), `publish-winget` job (Phase D) |
| `infra/src/dns/eddacraft-ai.ts`                   | Already has the install CNAME; verify deployed                       |
| `eddacraft/anvil/README.md`                       | Bootstrap content if Phase A1 reveals it's empty                     |
| `eddacraft/anvil/LICENSE`                         | Bootstrap content if missing                                         |
| `eddacraft/anvil/index.html` or `docs/install.sh` | Bootstrap if GitHub Pages serves nothing                             |
| `plans/modules/distribution-pipeline.aps.md`      | Mark items complete as they pass each phase                          |
