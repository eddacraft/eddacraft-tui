# ADR-020: Versioning strategy — lockstep core vs independent peripherals

## Status

Accepted

## Date

2026-04-14

## Context

The monorepo contains multiple categories of crate and package:

- **Anvil core** — the CLI binary and everything that ships inside it or
  directly supports it at runtime (`crates/anvil-*`, `packages/anvil/*`)
- **Separate products** — edda-stack (internal product line), website,
  docs-site
- **External OSS** — anvil-plan-spec (aps), eddacraft-tui, kindling
- **Peripheral tooling** — mcp-server, vscode-extension, transactional
  (emails)

Today Rust crates use a workspace-level version (`version.workspace = true`)
which puts all crates in lockstep. npm packages have Changesets wired up but
no explicit policy on whether they track the release version.

A decision is needed because:

1. The release script and manifest (RELMGMT-009) need to know which
   components to version-bump on each release
2. Users see one version number (`anvil --version`) and shouldn't need to
   reason about internal crate versions
3. Separate products and OSS packages have their own users and release
   cadences — coupling them to Anvil releases creates unnecessary churn

## Decision

**Lockstep versioning for Anvil core.** All crates and packages that are part
of the Anvil product share a single version number — the release tag (e.g.
`v0.4.0-beta`). A release bumps them all, even if an individual crate had no
changes in that cycle.

**Independent versioning for everything else.** Separate products, external
OSS repos, and peripheral tooling version on their own cadence. Their
versions are recorded in the release manifest for traceability but are not
bumped by the Anvil release process.

### Classification

| Category | Versioning | Scope |
|---|---|---|
| Anvil core | Lockstep (release tag) | `crates/anvil-*`, `packages/anvil/*` |
| Separate products | Independent | edda-stack, website, docs-site |
| External OSS | Independent (own repos) | aps (anvil-plan-spec), eddacraft-tui, kindling |
| Peripheral tooling | Independent | mcp-server, vscode-extension, transactional |

### The rule

If the crate or package ships inside the `anvil` binary or directly supports
it at runtime, it is core and follows lockstep versioning. Everything else
versions independently.

## Rationale

Lockstep for core avoids version-matrix complexity. The internal crates are
tightly coupled (anvil-kernel → anvil-checks → anvil-cli) and a user running
`anvil --version` should get a single authoritative number. Independent
versioning for those crates would create coordination overhead with no user
benefit.

Independent versioning for non-core avoids forced releases. A VS Code
extension bug fix, an edda-stack schema change, or an eddacraft-tui widget
improvement shouldn't require a full Anvil release cycle.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| Lockstep core + independent peripherals (chosen) | Simple mental model, single version for users, no coordination overhead for core | Minor version churn on unchanged core crates |
| Fully independent (every crate versions separately) | Semantically precise — version bump means something changed | Coordination nightmare in a tightly-coupled monorepo; users see version mismatches |
| Fully lockstep (everything in the monorepo shares one version) | Simplest possible scheme | Forces releases of unrelated products; blocks independent OSS release cadence |

## Consequences

- **Positive:** Release script can bump one version for all core components.
  Manifest clearly distinguishes what's in the release vs. what's referenced.
- **Positive:** External OSS repos (aps, eddacraft-tui, kindling) can release
  whenever they're ready without waiting for an Anvil release.
- **Negative:** A core crate with no changes still gets a version bump. This
  is cosmetic — the workspace version in `Cargo.toml` already does this.
- **Risks:** Boundary between core and non-core could blur as new packages
  are added.
- **Mitigations:** The rule is simple — "does it ship in the binary or
  support it at runtime?" When adding a new crate or package, apply the rule
  and update this ADR's classification table if needed.

## References

- Related ADRs: ADR-017 (crates.io naming), ADR-018 (product IP architecture)
- APS modules: RELMGMT-003 (semver policy), RELMGMT-009 (release script)
