# Distribution and Self-Update

<!-- Executable only if tasks exist and status is Ready or In Progress. -->

| ID      | Owner  | Status      | Progress |
| ------- | ------ | ----------- | -------- |
| DISTRIB | @aneki | Complete | 6/6 |

**Last reviewed:** 2026-06-08 (release-tag reconciliation sweep — DISTRIB-005
advanced to Released/Shipped via v0.7.3-beta (merge `8ae65b10` confirmed in
tag) and DISTRIB-006 to Released/Shipped via v0.7.4-beta (merge `c5ee305b`
confirmed in tag); module advanced **In Progress → Complete** per the
v0.7.4-beta release-record post-tag note. Prior review 2026-05-31: DISTRIB-006
filed — promoted from GitHub issue
[#1726](https://github.com/eddacraft/anvil-001/issues/1726): `ANVIL_HOME` /
`--anvil-home` install-root override for side-by-side candidate installs, so a
pre-release Anvil can be tested without releasing it. Filed **Proposed**;
**Merged 2026-05-31 via PR #2185** once its ADR-060 project-state design gate was
**satisfied** (ADR-060 **Accepted** 2026-05-31) — implemented on
`feat/distrib-006-anvil-home-override`; done-count advances 5 → 6. DISTRIB-006
merged **after** the v0.7.3-beta tag commit (`8bfd48c4d`) and rode
`v0.7.4-beta` as freight; that tag shipped 2026-06-01, closing the module.
Prior: 2026-05-17 — DISTRIB-003 **Merged** via PR #1652 — Homebrew
formula auto-bump extracted from the inline `release.yml` step into a tested
`scripts/release/bump-homebrew.sh`, plus a `workflow_dispatch` recovery
workflow, smoke install on macOS arm64/x64, and publish runbook; operator
follow-up tracked in
[`plans/reviews/post-merge/feat-distrib-003-homebrew-formula.md`](../../reviews/post-merge/feat-distrib-003-homebrew-formula.md).
DISTRIB-004 **Done** — release cadence and beta
support-window policy now lives at `docs/policies/release-cadence.md` and is
cross-linked from README + CONTRIBUTING. DISTRIB-002 **Merged** via PR #1569 —
`anvil version --check` + advisory surface + watch/status hint; remaining operator
follow-up tracked in
[`plans/reviews/post-merge/feat-distrib-002-version-check.md`](../../reviews/post-merge/feat-distrib-002-version-check.md).
DISTRIB-001 **Merged** via PR #1562; operator follow-up tracked in
[`plans/reviews/post-merge/feat-distrib-001-signature-verification.md`](../../reviews/post-merge/feat-distrib-001-signature-verification.md).
Promoted **Proposed → Ready** alongside acceptance of
[`plans/specs/2026-05-14-release-plan-v0.7.0-sit-on.md`](../../specs/2026-05-14-release-plan-v0.7.0-sit-on.md).
Current state: every item is Released/Shipped — DISTRIB-001..-004 via
`v0.7.0-beta`, DISTRIB-005 **Released/Shipped via v0.7.3-beta** (PR #1984,
`anvil migrate schema` cross-version config reconciliation, subcommand-split
design), and DISTRIB-006 **Released/Shipped via v0.7.4-beta** (PR #2185;
GitHub #1726; ADR-060 design gate **satisfied**, Accepted 2026-05-31).
ADR-044 §9 makes DISTRIB-001 and DISTRIB-002 load-bearing for the
`v0.7.0-beta` MCP-backend swap to actually reach existing users.)

## Purpose

`anvil update` already exists (per
[`plans/execution/2026-04-13-anvil-update-command.md`](../../execution/2026-04-13-anvil-update-command.md))
with a Homebrew-detect → sidecar → axoupdater resolution chain. The hotfix
iteration plan in
[`plans/specs/2026-05-14-release-plan-v0.7.0-sit-on.md`](../../specs/2026-05-14-release-plan-v0.7.0-sit-on.md)
assumes that when a senior user hits a bug on Tuesday and we publish a fix on
Wednesday, they receive the fix without effort.

That assumption is currently load-bearing and lightly tested. The risk model:
if a single user gets stuck on a buggy patch because the update path is
non-obvious, the hotfix iteration premise breaks. If a security-relevant fix
goes unnoticed, the trust premise breaks. This module hardens the surrounding
ecosystem so the update path is **trustworthy, signed, and visible**.

## In Scope

- `anvil update` resolution-chain robustness (Homebrew detection, sidecar
  shell-out, axoupdater library fallback) with signature verification
- `anvil version --check` UX surfacing newer versions and security advisories
- Homebrew formula automation: auto-bump on release, signed artefacts, tested
  on macOS arm64 and x64
- Release cadence and EOL policy (`docs/policies/release-cadence.md`)
- `anvil migrate` for config reconciliation across minor versions
- Install-root override (`ANVIL_HOME` / `--anvil-home`) so a pre-release
  candidate can run side-by-side with the production install for testing

## Out of Scope

- Curl-installer rewrite (already covered under WATCHUX-001 and broader
  install polish)
- Package manager support beyond Homebrew (npm, cargo install, scoop, winget,
  apt, etc.) — deferred to a post-v0.7.0 distribution module
- Hosted release server / cloud delivery — Horizon 2
- Auto-update without user consent — explicit non-goal; always opt-in

## Interfaces

- **Depends on:**
  - `crates/anvil-cli/src/commands/update.rs` (existing)
  - `crates/anvil-cli/src/commands/version.rs` (existing)
  - `crates/anvil-cli/Cargo.toml` (axoupdater dependency)
  - `install.sh` (curl installer; coordinate with WATCHUX-001)
  - GitHub Releases (binary artefacts + signatures)
- **Exposes:**
  - Hardened `anvil update` and `anvil version --check`
  - `anvil migrate` config reconciliation command
  - Documented release cadence and EOL policy
  - Automated Homebrew formula bump on tag

## Work Items

### DISTRIB-001: Harden `anvil update` Resolution Chain And Signature Verification

- **Intent:** Ensure the existing `anvil update` resolution chain is correct
  on every supported install path and that downloaded artefacts are
  signature-verified before installation.
- **Expected Outcome:** Resolution chain is tested end-to-end on
  (a) Homebrew install, (b) curl-installer sidecar install, (c) library
  fallback install. Each path verifies the downloaded artefact against a
  published signature (cosign or minisign — choose one and pin it in an
  ADR) before replacing the running binary. Refusal on signature mismatch
  is loud and actionable.
- **Files:**
  - `crates/anvil-cli/src/commands/update.rs`
  - `crates/anvil-cli/src/commands/update/signature.rs` (NEW)
  - `plans/decisions/045-update-signing-scheme.md` (NEW ADR)
  - `crates/anvil-cli/tests/update_resolution_chain.rs` (NEW)
  - `.github/workflows/release-sign-artefacts.yml` (NEW)
- **Validation:**
  - `cargo test -p eddacraft-anvil --test update_resolution_chain`
  - Integration: fixture install for each path; tampered-artefact refusal
    test; CI runs both on macOS and Linux runners
- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
- **Merged:** 2026-05-14 via PR #1562 (commits `ceadb1c5`, `44c5df3e`,
  `ae36e615`). Cleanup agent will advance to Released/Shipped once
  v0.7.0-beta ships with the production minisign key embedded; see the
  post-merge plan for operator setup steps.
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor
- **releaseNote:**
  - audience: user
  - type: changed
  - text: "`anvil update` now signature-verifies downloads on every install
    path before replacing the running binary."

### DISTRIB-002: `anvil version --check` And Security Advisory Surface

- **Intent:** Surface newer versions and security advisories without
  performing an automatic update, so users can decide when to upgrade and
  cannot miss a security-relevant fix.
- **Expected Outcome:** `anvil version --check` queries the releases feed
  (with offline fallback per the air-gapped guarantee), reports newer
  available versions, and explicitly names any advisory tag attached to
  the running version (e.g. `security-advisory: GHSA-xxxx-...`). The
  watch TUI and `anvil status` show a one-line "update available" hint
  when applicable, rate-limited to once per 24h.
- **Files:**
  - `crates/anvil-cli/src/commands/version.rs`
  - `crates/anvil-cli/src/activation/render.rs`
  - `crates/anvil-tui/src/surfaces/watch/render.rs`
- **Validation:**
  - `cargo test -p eddacraft-anvil commands::version::tests::check_surfaces_advisory`
  - `cargo test -p eddacraft-anvil-tui watch::tests::update_hint_rate_limited`
  - Integration: fixture releases feed with advisory metadata
- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
- **Merged:** 2026-05-15 via PR #1569 (commits `194fd4a7`, `33103c39`,
  `aa896d19`, `35d43a04`, `b2879f76`). Cleanup agent will advance to
  Released/Shipped once v0.7.0-beta ships with a real advisory in the
  release body; the post-merge plan tracks the downstream smoke-test.
- **Dependencies:** DISTRIB-001 (Merged via PR #1562)
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor
- **releaseNote:**
  - audience: user
  - type: added
  - text: "`anvil version --check` reports new releases and security
    advisories; watch and status surface a non-intrusive hint."

### DISTRIB-003: Homebrew Formula Automation

- **Intent:** Auto-bump the `eddacraft/tap/anvil` Homebrew formula on
  release so Homebrew users receive hotfixes without manual maintainer
  action.
- **Expected Outcome:** GitHub Actions workflow on release publishes the
  updated formula to `eddacraft/homebrew-tap` with the new version,
  artefact SHAs, and bottle URLs. Workflow is tested by the release
  runbook. Formula publishes are signed by the release identity. macOS
  arm64 and x64 install paths both produce a working `anvil` binary.
- **Files:**
  - `.github/workflows/homebrew-bump.yml` (NEW)
  - `scripts/release/bump-homebrew.sh` (NEW)
  - `docs/runbooks/homebrew-publish.md` (NEW)
- **Validation:**
  - CI dry-run on candidate SHA produces a valid formula file
  - Integration: install from the tap on macOS arm64 and x64 runners
- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
- **Picked up:** 2026-05-17 on `feat/distrib-003-homebrew-formula`.
- **Merged:** 2026-05-17 via PR #1652 (commits `657ca39e`, `53a022eb`,
  `b36988a2`). Cleanup agent will advance to Released/Shipped once the
  next release tag ships and the macOS arm64/x64 smoke matrix in
  `Homebrew — bump and smoke` is green; operator follow-up tracked in
  [`plans/reviews/post-merge/feat-distrib-003-homebrew-formula.md`](../../reviews/post-merge/feat-distrib-003-homebrew-formula.md).
- **changeType:** internal
- **releaseIntent:** candidate
- **releaseScope:** minor
- **releaseNote:**
  - audience: operator
  - type: added
  - text: "Homebrew formula now auto-bumps on every release tag."

### DISTRIB-004: Release Cadence And EOL Policy

- **Intent:** Document what users can expect about release cadence,
  patch/minor/major semantics, and version support windows.
- **Expected Outcome:** `docs/policies/release-cadence.md` documents:
  the hotfix iteration plan (weekly during active signal, 48h for P0),
  patch/minor/major scope per `plans/aps-rules.md`, the "sit on a release"
  cadence (no major release within 6 weeks unless triggered by Boring-
  Week regressions), and the support window for `-beta` releases
  (latest minor + previous minor get security fixes). Cross-linked from
  README and CONTRIBUTING.
- **Files:**
  - `docs/policies/release-cadence.md` (NEW)
  - `README.md` (cross-link)
  - `CONTRIBUTING.md` (cross-link)
- **Validation:**
  - `pnpm format:check`
  - Manual link/source/status reconciliation against `plans/index.aps.md`,
    `RELEASE-PLAN.md`, README, CONTRIBUTING, and documentation governance.
  - Quick Council pass.
- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
- **Picked up:** 2026-05-16 on `docs/distrib-release-cadence`.
- **Done:** 2026-05-16 (policy doc + README/CONTRIBUTING cross-links).
- **changeType:** docs
- **releaseIntent:** candidate
- **releaseScope:** minor
- **releaseNote:**
  - audience: user
  - type: added
  - text: "Release cadence and version support window are now documented in
    `docs/policies/release-cadence.md`."

### DISTRIB-005: `anvil migrate schema` For Cross-Version Config Reconciliation

- **Intent:** When a minor version changes a config schema, give users a
  one-command path to migrate without hand-editing files.
- **Spec reconciliation (2026-05-26):** APS truth validation before
  implementation found three stale assumptions plus a command-name
  collision. The corrected contract:
  - `commands/migrate.rs` is **not new** — MLP2-040 shipped it in
    `v0.7.0-beta` as the `.anvilrc` → `.anvil.<ext>` *filename/format*
    migration. `anvil migrate` is therefore restructured into
    subcommands: `anvil migrate format` (the existing MLP2-040
    behaviour, unchanged) and `anvil migrate schema` (this item). Bare
    `anvil migrate` keeps routing to `format` with a deprecation notice
    (operator-accepted design, 2026-05-26).
  - Config location is the root-level `.anvil.<ext>` dotfile discovered
    via `anvil_config::discover(root, ".anvil")` — **not**
    `.anvil/config.{ext}`, which never existed.
  - The project's origin anvil version is read from
    `ProjectIdentity.created_by_version` (the `anvil/project-id`
    surface, via `activation::identity::read_project_id`). `baseline.json`
    also carries a `created_by_version` (baseline-_adoption_ time), but
    `anvil/project-id` is the universal project-origin anchor — present
    after `anvil start` even without a baseline — so it is the chosen
    source. (The original spec's "anvil-version in baseline metadata" was
    directionally right that a version is recorded, but named the wrong
    surface for the project-origin anchor.) The installed version is
    `env!("CARGO_PKG_VERSION")`.
  - Scope reality: there are **zero registered schema migrations** and
    no config-schema-version concept in the tree today. This item ships
    the registry + version-delta detection + dry-run/apply plumbing;
    every current config resolves to "no migration needed" until a
    future minor version registers a real transform.
- **Expected Outcome:** `anvil migrate schema` discovers the project's
  `.anvil.<ext>` config and reads `created_by_version` from
  `anvil/project-id`; it computes the delta against the running
  `CARGO_PKG_VERSION` and, if a migration is registered for that delta,
  previews the change (dry-run default) or writes it under `--apply`
  (atomic write, original format preserved). With no registered
  migration it prints "no migration needed for X → Y"; when the origin
  version cannot be determined it prints clear manual-review guidance.
  The registry lives in `crates/anvil-config/src/migrations.rs` and is
  empty in production today.
- **Files:**
  - `crates/anvil-cli/src/commands/migrate.rs` (MODIFY — split into
    `format` / `schema` subcommands; preserve MLP2-040 behaviour under
    `format`)
  - `crates/anvil-cli/src/main.rs` (MODIFY — `Migrate` command help)
  - `crates/anvil-config/src/migrations.rs` (NEW — migration registry +
    version-delta resolver)
  - `crates/anvil-config/src/lib.rs` (MODIFY — module decl + re-exports)
  - `docs/runbooks/anvil-migrate.md` (NEW)
- **Validation:**
  - `cargo test -p eddacraft-anvil-config migrations::tests`
  - `cargo test -p eddacraft-anvil commands::migrate::tests`
  - Subcommand back-compat: the existing MLP2-040 `migrate::tests` stay
    green under the `format` subcommand path.
- **Status:** Released/Shipped via v0.7.3-beta (2026-05-31; merge commit
  `8ae65b10` confirmed in tag). Merged 2026-05-26 via PR
  [#1984](https://github.com/eddacraft/anvil-001/pull/1984) — `anvil migrate`
  split into `format` (MLP2-040) + new `schema` subcommand; cross-version
  config-schema reconciliation with an empty-by-design migration registry in
  `anvil-config::migrations`.
- **Dependencies:** DISTRIB-002
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor
- **releaseNote:**
  - audience: user
  - type: added
  - text: "`anvil migrate schema` reconciles config across minor versions
    in one step."

### DISTRIB-006: `ANVIL_HOME` / `--anvil-home` Install-Root Override For Side-By-Side Candidate Installs

- **Intent:** Let an internal developer run a pre-release Anvil candidate
  alongside the production install — testing a new version without releasing it
  — by re-rooting all install-owned state (user state, daemon socket, kernel
  cache/logs) under a single override prefix, so the candidate never collides
  with the production install's `~/.anvil/`, its daemon socket, or project
  state.
- **Source:** GitHub issue
  [#1726](https://github.com/eddacraft/anvil-001/issues/1726) (filed 2026-05-19
  by @joshuaboys during `v0.7.0-beta` pre-cut testing). Today no env var or flag
  re-roots the install: the surveyed overrides (`ANVIL_ADMIN_KEY`,
  `ANVIL_API_URL`, `ANVIL_LICENSE`, `ANVIL_TEMPLATES_DIR`, `XDG_CONFIG_HOME`, …)
  leave three concrete collisions — user-state (`~/.anvil/`), daemon-socket, and
  per-project `.anvil/`. The current workaround (stop prod daemon, symlink
  `anvil-beta`, test only under `/tmp`, accept the user-state leak) is tolerable
  one-off but painful for sustained candidate iteration. The daemon
  single-instance constraint is
  [`ADR-036`](../../decisions/036-daemon-scope-discovery-and-boundaries.md)
  (`one daemon per (uid, os)`, PID-file exclusive create), so a distinct socket
  prefix per `ANVIL_HOME` is precisely what lets two daemons coexist.
- **Design gate (SATISFIED) — ADR-060:** Per-project state resolution, the open
  design call that gated this item, is settled in
  [`ADR-060`](../../decisions/060-anvil-home-install-root-override.md) (**Accepted**
  2026-05-31). The accepted answer is **Option (a) + a write-guard**: keep
  per-project `.anvil/` (baseline/cache/witness) + `anvil/project-id` resolving
  to the project root so candidate tests run against the real repo with witness
  continuity, but gate durable project-state mutations behind
  `--touch-project-state` (read-only / dry-run by default under a non-default
  `ANVIL_HOME`). The rejected alternative was **Option (b)** — re-root project
  discovery under `<ANVIL_HOME>/projects/` — which isolates fully but defeats the
  side-by-side purpose. With the gate satisfied, this item was promoted to Ready,
  then **Merged 2026-05-31 via PR #2185**. Cross-version chain *format*
  compatibility (a candidate writing a chain a different anvil version reads) is
  out of scope — that is an `anvil migrate` problem, see DISTRIB-005.
- **Expected Outcome:** A uniform install-root override that every install-owned
  state location honours:
  - `ANVIL_HOME=<path>` re-roots user state (`<path>/user/`), the daemon socket
    (`<path>/daemon.sock` or the platform equivalent), and kernel cache/logs
    (`<path>/cache/`); `~/.anvil/` is never touched when it is set.
  - `--anvil-home <path>` takes precedence over the env var.
  - Two daemons under different `ANVIL_HOME` prefixes run concurrently with no
    socket clash.
  - `anvil status --json` reports the resolved root in a new `installRoot`
    field so an operator can see which install they are talking to.
  - Unsetting `ANVIL_HOME` returns to platform-default behaviour
    byte-for-byte — no regression for users who never set it.
  - Per-project `.anvil/` (baseline/cache/witness) stays rooted at the project
    root per ADR-060 **Option (a)**; durable project-state mutations are gated
    behind `--touch-project-state` (read-only / dry-run by default under a
    non-default `ANVIL_HOME`).
- **Files:**
  - `crates/anvil-cli/src/main.rs` (MODIFY — global `--anvil-home` flag + env
    resolution; flag precedence over env)
  - install-root resolver (NEW — single source of truth the current
    `dirs::home_dir()` / `~/.anvil/` call sites route through)
  - daemon socket-path derivation (MODIFY — honour the resolved root)
  - `crates/anvil-cli/src/commands/status.rs` (MODIFY — `installRoot` JSON field)
  - `plans/decisions/060-anvil-home-install-root-override.md` (NEW ADR)
  - `plans/decisions/DECISION-LOG.md` (MODIFY — index ADR-060)
  - `docs/runbooks/anvil-home-side-by-side.md` (NEW — candidate-testing +
    Boring Week tester instructions, dropping the "stop prod daemon" step)
  - `crates/anvil-cli/tests/anvil_home.rs` (NEW — env resolution, flag
    precedence, two concurrent daemons under different prefixes, missing-path
    fallback)
- **Validation:**
  - `cargo test -p eddacraft-anvil --test anvil_home`
  - Integration: `ANVIL_HOME=$(mktemp -d) anvil start` writes only under the
    prefix and never `~/.anvil/`; two prefixes yield two concurrent daemons;
    unset returns the default surface byte-for-byte.
  - `pnpm adr:check` green with ADR-060 indexed.
- **Status:** Released/Shipped via v0.7.4-beta (2026-06-01; merge commit
  `c5ee305b` confirmed in tag). Merged 2026-05-31 via PR
  [#2185](https://github.com/eddacraft/anvil-001/pull/2185)
  (ADR-060 design gate satisfied — Accepted 2026-05-31 via PRs #2164/#2171;
  implemented on `feat/distrib-006-anvil-home-override`: daemon socket/PID +
  user-state re-root under `ANVIL_HOME`, durable project-state writes gated behind
  `--touch-project-state`, `status --json` reports `install_root` /
  `project_writes_gated`). Merged after the v0.7.3-beta tag commit
  (`8bfd48c4d`, 2026-05-30T19:34Z) and rode `v0.7.4-beta` as freight.
- **Dependencies:** ADR-060 (Accepted 2026-05-31) was the Ready gate and is now
  satisfied. Coordinates with ADR-036 (daemon single-instance / socket
  derivation).
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor
- **releaseNote:**
  - audience: operator
  - type: added
  - text: "`ANVIL_HOME` / `--anvil-home` re-roots a candidate install so it runs
    side-by-side with the production install for pre-release testing."

## Sequencing

1. **DISTRIB-001** is first; everything else assumes signature verification
   is in place.
2. **DISTRIB-002** layers the version-check UX onto -001.
3. **DISTRIB-003** is parallel with -001 / -002 — the Homebrew formula
   automation does not depend on the update command itself.
4. **DISTRIB-004** is the policy doc and is independent.
5. **DISTRIB-005** depends on -002 (it reuses the releases-feed contract).
6. **DISTRIB-006** is independent of the update/version chain; its ADR-060
   per-project-state gate is **satisfied** (Accepted 2026-05-31), and it does
   not block -001..-005.

## Release Notes

The DISTRIB items collectively justify a "Anvil now publishes signed
releases with a documented support window and clean upgrade path" line in
`v0.7.0-beta`.

## Cross-References

- Coordinates with: [`WATCHUX-001`](./watch-ux-advisory-rules.aps.md)
  (Homebrew detection on install), [`ADTRUST-001`](./adoption-trust-surface.aps.md)
  (status surface where update hint renders).
- Blocks on: none at module level.
- DISTRIB-001 signing scheme decision: [`ADR-045`](../../decisions/045-update-signing-scheme.md)
  chose minisign for update artefact verification.
- DISTRIB-006 promoted from GitHub issue
  [#1726](https://github.com/eddacraft/anvil-001/issues/1726); coordinates with
  [`ADR-036`](../../decisions/036-daemon-scope-discovery-and-boundaries.md) (daemon
  single-instance / socket derivation) and was gated on **ADR-060**
  (per-project state resolution under `ANVIL_HOME`), **Accepted** 2026-05-31.
