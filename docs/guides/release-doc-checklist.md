# Release Documentation Checklist

| Type  | Authority | Owner  | Status | Freshness                                                          |
| ----- | --------- | ------ | ------ | ------------------------------------------------------------------ |
| Guide | Advisory  | DOCGOV | Live   | Last reviewed 2026-05-22 against DOCGOV-006 release-doc sync scope |

| Upstream                                                                            | Downstream                                     |
| ----------------------------------------------------------------------------------- | ---------------------------------------------- |
| `plans/modules/documentation-governance.aps.md`, `docs/runbooks/release-runbook.md` | Release operators, PR authors, `release` skill |

Companion to the [release runbook](../runbooks/release-runbook.md). Use this
checklist to ensure all documentation is updated before and after every release.

Copy the relevant section into a release PR description or tracking issue and
tick items as they are completed.

---

## Node.js CLI Release (`@eddacraft/anvil-cli`) — DEPRECATED

> **Note:** The Node.js CLI has been replaced by the Rust binary distributed via
> cargo-dist. This section is retained for reference only. New releases use the
> Rust Crate Release section below.

### Pre-release: changelog and version files

- [ ] `CHANGELOG.md` — add release entry following keep-a-changelog format
- [ ] `package.json` (workspace root) — bump `version` field
- [ ] `docs/public/anvil/releases/changelog.md` — mirror notable changes for
      public site
- [ ] `docs/public/anvil/releases/upgrade-notes.md` — add migration section if
      there are breaking or behavioural changes

### Pre-release: public docs sync

Review each page for accuracy against the new release. Only check files that are
affected by the changes shipping in this release.

- [ ] `docs/public/anvil/overview.md` — feature descriptions still accurate
- [ ] `docs/public/anvil/quickstart.md` — install commands, prerequisites,
      version references
- [ ] `docs/public/anvil/first-project.md` — walkthrough still works with new
      CLI version
- [ ] `docs/public/anvil/when-to-use.md` — use-case list reflects current
      capabilities
- [ ] `docs/public/anvil/concepts/gates.md` — gate behaviour changes
- [ ] `docs/public/anvil/concepts/audit-trail.md` — audit format changes
- [ ] `docs/public/anvil/concepts/plans.md` — APS plan format changes
- [ ] `docs/public/anvil/concepts/sessions.md` — session model changes
- [ ] `docs/public/anvil/tutorials/` — all tutorials still runnable
  - [ ] `architecture.md`
  - [ ] `ci.md`
  - [ ] `drift.md`
  - [ ] `policies.md`
  - [ ] `suppressions.md`
- [ ] `docs/public/anvil/integrations/github.md` — CI workflow examples
- [ ] `docs/public/anvil/integrations/mcp.md` — MCP server integration
- [ ] `docs/public/anvil/integrations/vscode.md` — extension compatibility
- [ ] `docs/public/anvil/operations/troubleshooting.md` — new error codes or
      known issues
- [ ] `docs/public/anvil/operations/config.md` — config schema changes
- [ ] `docs/public/anvil/operations/security.md` — security model changes
- [ ] `docs/public/anvil/guides/team-flow.md` — team workflow changes
- [ ] `docs/public/anvil/guides/solo-dev-flow.md` — solo workflow changes
- [ ] `docs/public/anvil/guides/agent-harness.md` — agent harness changes

### Pre-release: internal docs sync

- [ ] `docs/architecture/overview.md` — high-level system overview
- [ ] `docs/architecture/README.md` — architecture index classifications
- [ ] relevant `docs/architecture/*-as-built.md` for touched components
- [ ] `docs/architecture/auth-as-built.md` — auth changes
- [ ] `docs/guides/command-safety.md` — new command safety rules
- [ ] `docs/guides/command-safety-configuration.md` — config changes
- [ ] `docs/guides/custom-architecture-policies.md` — OPA policy changes
- [ ] `docs/runbooks/release-runbook.md` — process changes

### Pre-release: package READMEs

Only update packages that changed in this release.

- [ ] `packages/anvil/README.md` — core/runtime API changes
- [ ] `packages/edda-stack/README.md` — Edda/Ember contract changes
- [ ] `packages/aps/README.md` — APS format changes
- [ ] `apps/anvil-api/README.md` — API route changes

> The Node MCP server and VS Code extension were archived per ADR-033
> (`anvil-archive/anvil-mcp-server/`, `anvil-archive/anvil-vscode-extension/`);
> they no longer participate in release cuts. The live MCP path is the Rust shim
> under `crates/anvil-cli/src/commands/mcp.rs`.

### Pre-release: CI and deployment

- [ ] `.github/workflows/release.yml` — publish pipeline changes
- [ ] `.github/workflows/ci.yml` — CI gate changes
- [ ] `.github/workflows/security.yml` — security pipeline changes
- [ ] `.github/workflows/README.md` — workflow documentation

### Post-release

- [ ] Human comms sent (see runbook section 8)
- [ ] GitHub release notes reviewed and published
- [ ] Close related documentation issues (label: `docs`)

---

## Rust Crate Release (`crates/`)

### Pre-release: changelog and version files

- [ ] `CHANGELOG.md` — add Rust-specific entries
- [ ] `Cargo.toml` (workspace root) — workspace version bump if applicable
- [ ] `crates/anvil-kernel/Cargo.toml` — crate version
- [ ] `crates/anvil-kernel-types/Cargo.toml` — crate version
- [ ] `crates/anvil-checks/Cargo.toml` — crate version
- [ ] `crates/anvil-architecture/Cargo.toml` — crate version
- [ ] `crates/anvil-cli/Cargo.toml` — crate version
- [ ] `crates/anvil-policy/Cargo.toml` — crate version
- [ ] `crates/anvil-tui/Cargo.toml` — crate version
- [ ] `crates/spike/Cargo.toml` — crate version (if applicable)

### Pre-release: public docs sync

- [ ] `docs/public/anvil/releases/changelog.md` — Rust performance or feature
      highlights for end users
- [ ] `docs/public/anvil/overview.md` — current capability and install-surface
      claims still match the release
- [ ] `docs/public/anvil/beta-testing-guide.md` — current version, install,
      upgrade, and test-focus text
- [ ] `docs/public/anvil/quickstart.md` — if Rust CLI replaces or supplements
      Node CLI, update install commands
- [ ] `docs/public/anvil/operations/config.md` — current watch flags and config
      behaviour
- [ ] `docs/public/anvil/operations/troubleshooting.md` — Rust-specific errors
- [ ] `docs/public/anvil/integrations/vscode.md` — extension and diagnostics
      behaviour
- [ ] `docs/public/anvil/integrations/mcp.md` — MCP integration changes

### Pre-release: internal docs sync

- [ ] `docs/architecture/rust-architecture-overview.md` — crate layout changes
- [ ] `docs/architecture/kernel-as-built.md` — kernel shipping-state changes
- [ ] `docs/architecture/rust-kernel-spec.md` — only if H1 design-intent notes
      change (historical; prefer as-built for shipped behaviour)
- [ ] `docs/guides/cli-output-streams.md` — output format changes
- [ ] `docs/guides/anvil-rule-authoring.md` — rule-format and authoring changes
- [ ] `docs/public/anvil/integrations/vscode.md` — editor integration changes
- [ ] `docs/public/anvil/integrations/mcp.md` — MCP integration changes
- [ ] `docs/guides/eddacraft-autonomy-constitution.md` — autonomy model changes
- [ ] `docs/architecture/kernel-benchmarking-spec.md` — benchmark methodology
      changes

### Pre-release: crate READMEs

- [ ] `crates/anvil-kernel/README.md`
- [ ] `crates/anvil-kernel-types/README.md`
- [ ] `crates/anvil-checks/README.md`
- [ ] `crates/anvil-architecture/README.md`
- [ ] `crates/anvil-cli/README.md`
- [ ] `crates/anvil-policy/README.md`
- [ ] `crates/anvil-tui/README.md`

### Pre-release: public distribution repo (eddacraft/anvil)

The `eddacraft/anvil` public repo hosts release binaries, the install landing
page, and the top-level README shown on GitHub. Review after any change that
affects install commands, supported targets, or project status.

- [ ] `README.md` — install commands, platform support table, status
- [ ] `docs/index.html` — install.eddacraft.ai landing page (installer URLs,
      copy, branding)

### Pre-release: CI and deployment

- [ ] `.github/workflows/release.yml` — cargo-dist publish pipeline changes
- [ ] `.github/workflows/rust.yml` — Rust CI changes
- [ ] `.github/workflows/ci-nightly.yml` — nightly build changes

### Pre-release: third-party attribution (ATTRIB)

Owned by the
[`attribution-pipeline-v3`](../../plans/archive/modules/attribution-pipeline-v3.aps.md)
module. Both checks below are wired into the `Acknowledgements freshness` job in
CI, so a passing PR pipeline already proves them — but run them locally before
tagging so a stale lockfile surfaces here rather than in the CI fast-fail.

- [ ] `tools/starters/acknowledgements/generate-acknowledgements.sh --check` —
      verifies `ACKNOWLEDGEMENTS.md` is in sync with the runtime dependency
      graph. cargo-about runs with `--fail` (ATTRIB-007), so a workspace crate
      missing the `license` / `license-file` field aborts here rather than
      silently dropping out of attribution.
- [ ] `tools/starters/acknowledgements/expand-licences.sh --check` — verifies
      `about.toml.accepted` and `deny.toml.[licenses].allow` are in sync with
      the canonical `licences.toml` (ATTRIB-006). If you added or removed a
      licence, the source of truth is `licences.toml`; rerun the expander and
      commit all three files together.

### Post-release

- [ ] Verify crate published to crates.io (when applicable)
- [ ] GitHub release notes include Rust-specific changes
- [ ] Performance benchmarks documented if relevant
- [ ] `eddacraft/anvil` release marked `--latest` and `--prerelease=false`
      (cargo-dist auto-marks `-beta` tags as prerelease, which hides them from
      `/releases/latest/download/...`)
- [ ] `eddacraft/anvil-001` GitHub Release created for the tag with CHANGELOG
      excerpt and a pointer to the public binaries, then marked
      `--latest --prerelease=false`:

      ```bash
      gh release create vX.Y.Z --repo eddacraft/anvil-001 --latest --prerelease=false --notes-file <file>
      ```

- [ ] `install.eddacraft.ai` serves `HTTP/2 200` with a valid cert and the
      install commands on the landing page match the published asset names

---

## Combined Release (Node.js + Rust) — DEPRECATED

> **Note:** Combined releases are no longer applicable. The Node.js CLI has been
> deprecated in favour of the Rust binary. This section is retained for
> historical reference only.

- [ ] `docs/architecture/overview.md` — high-level system overview still true
- [ ] `docs/architecture/edda-stack.md` — Edda stack integration points
- [ ] `docs/guides/stack-migration.md` — cross-layer schema compatibility
- [ ] Version strings consistent across `package.json` and `Cargo.toml` files
- [ ] Public changelog entry covers both surfaces clearly

---

## Quarterly Documentation Audit

Run once per quarter to catch drift unrelated to specific releases.

- [ ] All `docs/public/anvil/` pages render correctly on the site
- [ ] All internal links in `docs/guides/` and `docs/architecture/` resolve
- [ ] Code examples in tutorials still compile and run
- [ ] Version references across docs match latest release
- [ ] `README.md` (repo root) reflects current project state
- [ ] APS module plan file map (`plans/modules/`) matches actual file paths
