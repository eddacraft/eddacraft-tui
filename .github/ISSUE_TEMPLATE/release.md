---
name: Release
about: Track a release from preflight through post-release verification
title: 'release/'
labels: release, priority:P1, readiness:ready
assignees: ''
---

## Authority

- **Release plan:** <!-- RELEASE-PLAN.md section or release record -->
- **APS/CIB links:** <!-- release blocker or follow-up IDs -->
- **Priority:** P1

## Release Identity

- **Version:**
- **Tag:**
- **Release type:** <!-- beta / production -->
- **Branch strategy:** <!-- direct / stabilisation -->
- **Release branch:** <!-- release/x.y.z or N/A -->

## 1. Preflight

- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes
- [ ] `cargo build --release -p eddacraft-anvil` succeeds
- [ ] `anvil --version` matches expected version
- [ ] `pnpm install --frozen-lockfile && pnpm build` succeeds
- [ ] `pnpm nx run-many -t test --skip-nx-cache` passes
- [ ] `crates/anvil-cli/Cargo.toml` version is correct
- [ ] `CHANGELOG.md` has release notes

## 2. Branch & Tag

- [ ] Branch strategy chosen and executed
- [ ] Release PR opened (if applicable)
- [ ] Version bumped in `crates/anvil-cli/Cargo.toml`
- [ ] `CHANGELOG.md` updated
- [ ] Tag pushed: <!-- vX.Y.Z -->
- [ ] Workflow triggered: <!-- run URL -->

## 3. Workflow Monitoring

- [ ] `plan` job succeeded
- [ ] `build-local-artifacts` jobs succeeded (5 targets)
- [ ] `build-global-artifacts` job succeeded
- [ ] `host` job created GitHub Release on `EddaCraft/anvil`
- [ ] `announce` job posted release notes

## 4. Post-Release Verification

- [ ] Install from public release works: `anvil --version`
- [ ] `anvil doctor` passes
- [ ] `anvil auth login` works
- [ ] `anvil gate` runs successfully
- [ ] All 7 expected artefacts present in GitHub Release
- [ ] Public release not stuck in prerelease (if production)

## 5. Documentation Review

<!-- Skill triages which items apply based on diff summary -->

- [ ] Changelog reviewed for completeness
- [ ] Public docs reviewed for accuracy
- [ ] Upgrade notes added (if applicable)
- [ ] Beta testing guide version current

## 6. Communications

- [ ] Release comms drafted and sent

## 7. Post-Release Cleanup

- [ ] Release branch merged to `main` or marked N/A
- [ ] Release branch deleted (if used)
- [ ] `install.eddacraft.ai` serves correct version
- [ ] Issue closed
