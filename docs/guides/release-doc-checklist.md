# Release Documentation Checklist

Companion to the [release runbook](./release-runbook.md). Use this checklist to
ensure all documentation is updated before and after every release.

Copy the relevant section into a release PR description or tracking issue and
tick items as they are completed.

---

## Node.js CLI Release (`@eddacraft/anvil-cli`)

### Pre-release: changelog and version files

- [ ] `CHANGELOG.md` -- add release entry following keep-a-changelog format
- [ ] `apps/anvil-cli/package.json` -- bump `version` field
- [ ] `docs/public/anvil/releases/changelog.md` -- mirror notable changes for
      public site
- [ ] `docs/public/anvil/releases/upgrade-notes.md` -- add migration section if
      there are breaking or behavioural changes

### Pre-release: public docs sync

Review each page for accuracy against the new release. Only check files that are
affected by the changes shipping in this release.

- [ ] `docs/public/anvil/overview.md` -- feature descriptions still accurate
- [ ] `docs/public/anvil/quickstart.md` -- install commands, prerequisites,
      version references
- [ ] `docs/public/anvil/first-project.md` -- walkthrough still works with new
      CLI version
- [ ] `docs/public/anvil/when-to-use.md` -- use-case list reflects current
      capabilities
- [ ] `docs/public/anvil/concepts/gates.md` -- gate behaviour changes
- [ ] `docs/public/anvil/concepts/audit-trail.md` -- audit format changes
- [ ] `docs/public/anvil/concepts/plans.md` -- APS plan format changes
- [ ] `docs/public/anvil/concepts/sessions.md` -- session model changes
- [ ] `docs/public/anvil/tutorials/` -- all tutorials still runnable
  - [ ] `architecture.md`
  - [ ] `ci.md`
  - [ ] `drift.md`
  - [ ] `policies.md`
  - [ ] `suppressions.md`
- [ ] `docs/public/anvil/integrations/github.md` -- CI workflow examples
- [ ] `docs/public/anvil/integrations/mcp.md` -- MCP server integration
- [ ] `docs/public/anvil/integrations/vscode.md` -- extension compatibility
- [ ] `docs/public/anvil/operations/troubleshooting.md` -- new error codes or
      known issues
- [ ] `docs/public/anvil/operations/config.md` -- config schema changes
- [ ] `docs/public/anvil/operations/security.md` -- security model changes
- [ ] `docs/public/anvil/guides/team-flow.md` -- team workflow changes
- [ ] `docs/public/anvil/guides/solo-dev-flow.md` -- solo workflow changes
- [ ] `docs/public/anvil/guides/agent-harness.md` -- agent harness changes

### Pre-release: internal docs sync

- [ ] `docs/architecture/anvil-full-architecture.md` -- architecture changes
- [ ] `docs/architecture/system-spec.md` -- system spec updates
- [ ] `docs/architecture/auth-as-built.md` -- auth changes
- [ ] `docs/guides/command-safety.md` -- new command safety rules
- [ ] `docs/guides/command-safety-configuration.md` -- config changes
- [ ] `docs/guides/custom-architecture-policies.md` -- OPA policy changes
- [ ] `docs/guides/release-runbook.md` -- process changes

### Pre-release: package READMEs

Only update packages that changed in this release.

- [ ] `packages/anvil/core/README.md` (if it exists) -- API changes
- [ ] `packages/anvil/runtime/README.md` (if it exists) -- runtime changes
- [ ] `packages/edda-stack/README.md` -- Edda/Ember contract changes
- [ ] `packages/aps/README.md` (if it exists) -- APS format changes
- [ ] `packages/mcp-server/README.md` (if it exists) -- MCP tool changes
- [ ] `packages/platform/README.md` -- platform config changes
- [ ] `apps/anvil-cli/README.md` (if it exists) -- CLI usage changes
- [ ] `apps/anvil-api/README.md` -- API route changes

### Pre-release: CI and deployment

- [ ] `.github/workflows/publish.yml` -- publish pipeline changes
- [ ] `.github/workflows/ci.yml` -- CI gate changes
- [ ] `.github/workflows/security.yml` -- security pipeline changes
- [ ] `.github/workflows/README.md` -- workflow documentation

### Post-release

- [ ] Verify `npm view @eddacraft/anvil-cli@<version>` returns correct version
- [ ] Human comms sent (see runbook section 8)
- [ ] GitHub release notes reviewed and published
- [ ] Close related documentation issues (label: `docs`)

---

## Rust Crate Release (`crates/`)

### Pre-release: changelog and version files

- [ ] `CHANGELOG.md` -- add Rust-specific entries
- [ ] `Cargo.toml` (workspace root) -- workspace version bump if applicable
- [ ] `crates/anvil-checks/Cargo.toml` -- crate version
- [ ] `crates/anvil-kernel-types/Cargo.toml` -- crate version
- [ ] `crates/eddacraft-tui/Cargo.toml` -- crate version
- [ ] `crates/spike/Cargo.toml` -- crate version (if publishing)

### Pre-release: public docs sync

- [ ] `docs/public/anvil/releases/changelog.md` -- Rust performance or feature
      highlights for end users
- [ ] `docs/public/anvil/quickstart.md` -- if Rust CLI replaces or supplements
      Node CLI, update install commands
- [ ] `docs/public/anvil/operations/troubleshooting.md` -- Rust-specific errors

### Pre-release: internal docs sync

- [ ] `docs/architecture/rust-architecture-overview.md` -- architecture changes
- [ ] `docs/architecture/rust-architecture-endstate.md` -- endstate alignment
- [ ] `docs/architecture/rust-kernel-spec.md` -- kernel spec changes
- [ ] `docs/guides/cli-output-streams.md` -- output format changes
- [ ] `docs/guides/eddacraft-autonomy-constitution.md` -- autonomy model changes

### Pre-release: crate READMEs

- [ ] `crates/anvil-checks/README.md` (if it exists)
- [ ] `crates/anvil-kernel-types/README.md` (if it exists)
- [ ] `crates/eddacraft-tui/README.md` (if it exists)

### Pre-release: CI and deployment

- [ ] `.github/workflows/rust.yml` -- Rust CI changes
- [ ] `.github/workflows/ci-nightly.yml` -- nightly build changes

### Post-release

- [ ] Verify crate published to crates.io (when applicable)
- [ ] GitHub release notes include Rust-specific changes
- [ ] Performance benchmarks documented if relevant

---

## Combined Release (Node.js + Rust)

When shipping both surfaces in the same release, complete both sections above
plus:

- [ ] `docs/architecture/anvil-architecture-evolution.md` -- cross-surface
      architecture alignment
- [ ] `docs/architecture/edda-stack.md` -- Edda stack integration points
- [ ] `docs/guides/stack-migration.md` -- cross-layer schema compatibility
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
