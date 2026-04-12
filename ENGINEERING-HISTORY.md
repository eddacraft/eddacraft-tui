# Engineering History

Internal engineering changes, refactors, and maintenance work. For
customer-relevant changes, see the [Changelog](./CHANGELOG.md).

## [0.3.1-beta]

### Infrastructure

- **Feature flags module** — shared feature flagging system across TypeScript
  and Rust surfaces (`FLAGS-001`–`FLAGS-009`)
  - Contract schema with JSON Schema validation
  - Runtime resolver with environment-aware flag evaluation
  - Snapshot system for point-in-time flag state capture
  - Telemetry hooks for flag evaluation tracking
  - Exemplar test fixtures
  - Kernel-side feature flag types, resolver, and snapshot mirroring TS surface
  - Feature flag governance, inventory, and reference guides
  - ADR-019: flags–observability alignment decision
- **CI composite actions** — `setup-workspace` action extracted to deduplicate
  Node/pnpm/Nx setup across workflows; `detect-changes` action for path-based
  job filtering
- **CI workflow fixes** — 8 issues resolved: checkout ordering, clippy/rustfmt
  failures, formatting in setup-workspace action
- **Docs-shell app** — Next.js shell application for docs domain proxy with auth
  callback, login, logout routes, JWT/cookie/state libraries, and unit tests
- **Docs upstream scaffolding** — Docusaurus apps for private and public docs
  with middleware, sidebar configs, and Vercel deployment configuration
- **Vercel build skip** — `vercel-ignore-build.sh` script for skipping preview
  deploys on non-release branches

### Planning & Documentation

- **APS planning** — FLAGS module plan, welcome screen restore plan, CI
  improvements execution plan, docs-shell landing page design spec and plan, SPA
  gap remediation spec and plan
- **PR template** — durable link requirement for manual testing; rationale moved
  to section comment
- **README** — lowercase brand usage, Windows aarch64 scope clarification,
  'Anvil Check' action name restored
- **Release doc checklist** — public distribution repo section added; broken
  inline code span fixed; oxfmt formatting applied
- **AGENTS.md** — updated with current conventions

### Dependencies

- pnpm dependency upgrades (vitest, vite, @types/node, globals, @nx/eslint,
  @nx/vite, @vitest/coverage-v8, @github/copilot)
- Cargo dependency upgrades via Cargo.lock refresh
- lint-staged configuration updated

### Tooling

- `aps-cleanup.service` — systemd service for APS status lifecycle automation
- nx.json configuration updates

## [0.3.0-beta]

See commit history for full engineering details. Key internal items:

- TypeScript upgraded to 6.0 across all workspace packages (`MAINT-011`)
- Node engine floor raised to >= 22
- Rust toolchain bumped to 1.94.0 with Windows and macOS cross-compilation
- oxlint adopted as first-pass linter, oxfmt replaces prettier
- Criterion benchmarks added for kernel critical paths and wired into CI
- Stress test harness for kernel benchmarking (`BENCH`)
- 59 unit tests for under-covered anvil-cli modules
- Integration test suite for checks crate
- GitHub Actions bumped: checkout v6, setup-node v6, download-artifact v8,
  nx-set-shas v5, labeler v6, azure/login v3, pnpm/action-setup v5
- Unused CI jobs removed (Playwright, e2e-harness, tui-tests)
- Benchmarks restricted to main pushes and manual dispatch
- CodeQL workflow added with paths-ignore
- Docusaurus upgraded to 3.10
- Dependency bumps: criterion 0.8, reqwest 0.13, dirs 6, Vite 8
- ADR-015 (shared packages restructure) and ADR-016 (unified config format)
  published

## [0.2.1-beta]

- Edda/Ember/Stack integration contracts and service layer
- Security hardening across parsers, adapters, and plan loader
- Subprocess execution hardening
- Dependency patches: minimatch, axios, svgo, tar

## [0.1.3]

- CLI stderr/stdout stream policy standardised
- Git hook scripts consolidated to a single source of truth
- Default API URL changed to `eddacraft-api.vercel.app`
- ADR-011: Rust core engine architecture decision published
