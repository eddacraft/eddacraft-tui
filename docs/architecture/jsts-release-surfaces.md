# JS/TS Release Surfaces — As-Built Inventory

| Type     | Authority | Owner  | Status | Freshness                                                |
| -------- | --------- | ------ | ------ | -------------------------------------------------------- |
| As-built | Derived   | @aneki | Live   | 2026-05-20 (drift-fix on #1216; see "Drift notes" below) |

| Upstream                                                                                                                                                   | Downstream                                                                                                                                                                                                                       |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [#1216 inventory ask](https://github.com/eddacraft/anvil-001/issues/1216), [ADR-033 archival](../../plans/decisions/033-park-ide-mcp-retire-ts-scanner.md) | [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml), [`.github/workflows/release.yml`](../../.github/workflows/release.yml), [`.github/workflows/release-readiness.yml`](../../.github/workflows/release-readiness.yml) |

## Why this exists

#1216 asked us to "inventory JS/TS build, test, deploy, and preview jobs still
required for 0.5.x" and split release-blocking checks from compatibility-canary
coverage. The intent: keep release CI fast and purposeful, and stop letting
archived or never-to-be-reintegrated surfaces drive deploy-blocking jobs by
accident.

This file is the inventory. The columns below answer, for every JS/TS surface in
the repo:

- **What is it?** — one line.
- **Release tier** — `release-blocking`, `compatibility-canary`, or
  `out-of-band`. Definitions below.
- **CI signal** — the named workflow / job that actually runs it.
- **Rationale** — why it sits in that tier.

If a surface is not in the table below, it is not in the release path (checked
the workflow files at the freshness date above; revisit when adding new
apps/packages).

## Release tier definitions

- **release-blocking** — a job whose failure blocks a release or push-to-`main`
  integration. Belongs to a binary or service the next release tag will ship.
- **compatibility-canary** — a job we want green so future reintegration is
  cheap, but whose failure does not block this release. Runs in CI on PRs that
  touch its sources only.
- **out-of-band** — surfaces that build/test on their own cadence or cron
  (preview deploys, nightly benchmarks, archive holdovers). Failure is observed,
  not gated.

## Apps

| Surface                        | What it is                                                                                        | Release tier         | CI signal                                              | Rationale                                                                                                     |
| ------------------------------ | ------------------------------------------------------------------------------------------------- | -------------------- | ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------- |
| `anvil-archive/admin-cli-node` | Node CLI shipped to operators of the waitlist + admin surface.                                    | release-blocking     | `ci.yml` Lint & Format, Type Check, Unit Tests, Build  | First-party operator tool. Has tests and an Nx `build` target; lives in the integration-readiness aggregator. |
| `apps/anvil-api`               | Hono service powering admin + waitlist + auth endpoints.                                          | release-blocking     | `ci.yml` Lint & Format, Type Check, Unit Tests, Build  | Deployed alongside each release; admin surface is the OPS gating path.                                        |
| `apps/anvil-docs-private`      | Docusaurus site for private/beta docs sitting behind `docs-shell`.                                | compatibility-canary | `ci.yml` Lint & Format, Type Check (docs-only changes) | Documentation, not part of the install bundle. Lint/typecheck only.                                           |
| `apps/docs-public`             | Docusaurus site for public docs (APS, Kindling, edda-stack, blog).                                | compatibility-canary | `ci.yml` Lint & Format, Type Check (docs-only changes) | Same as `anvil-docs-private`; site deploys are out-of-band via Vercel.                                        |
| `apps/docs-shell`              | Next.js auth-gated proxy fronting docs at `docs.eddacraft.ai`.                                    | release-blocking     | `ci.yml` Lint & Format, Type Check, Unit Tests, Build  | Has unit tests for the auth/cookie/jwt path. Failure blocks pushes that touch docs auth.                      |
| `apps/docs-site`               | Legacy Docusaurus site retained pending consolidation under `docs-public` + `anvil-docs-private`. | compatibility-canary | `ci.yml` Type Check                                    | Type-check only; no Nx `test` target. Slate for retirement when the new docs split is fully cut over.         |
| `apps/e2e`                     | Vitest-driven end-to-end harness (CLI / API / core / smoke).                                      | release-blocking     | `ci.yml` E2E Harness                                   | Aggregated by integration-readiness; covers cross-surface smoke.                                              |
| `apps/website`                 | Next.js marketing site at `eddacraft.ai`.                                                         | compatibility-canary | `ci.yml` Lint & Format, Type Check, Build              | No unit tests; build covers structural breakage. Deploy is out-of-band via Vercel.                            |

## Packages

| Surface                           | What it is                                                             | Release tier         | CI signal                    | Rationale                                                                                                                                                                                                                                                                         |
| --------------------------------- | ---------------------------------------------------------------------- | -------------------- | ---------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `packages/anvil/contracts`        | Cross-surface TypeScript contracts (anvil ↔ runtime ↔ adapters).       | release-blocking     | Nx `test`, Type Check        | Imported by `anvil-api`, `admin-cli`, `runtime`.                                                                                                                                                                                                                                  |
| `packages/anvil/core`             | Pure-TS schema/types shared by CLI + API.                              | release-blocking     | Nx `test`, Type Check        | Foundation for the typed surface.                                                                                                                                                                                                                                                 |
| `packages/anvil/policy`           | OPA-backed policy plumbing.                                            | release-blocking     | Nx `test`, Type Check        | Bundled with admin tooling; failure breaks release-gate.                                                                                                                                                                                                                          |
| `packages/anvil/ports`            | Boundary interface adapter ports.                                      | release-blocking     | Nx `test`, Type Check        | Imported by adapters.                                                                                                                                                                                                                                                             |
| `packages/anvil/runtime`          | Runtime orchestration (gate runner, file watch, cache).                | release-blocking     | Nx `test`, Type Check        | Used by `anvil-api`.                                                                                                                                                                                                                                                              |
| `packages/anvil-driver-client`    | JS-side driver-client / TS mirror of the editor-driver protocol.       | release-blocking     | Nx `test`, Type Check        | No in-repo TS consumer; tier driven by byte-parity tests against captured Rust fixtures (MLP2-029 / MLP2-030 / MLP2-051c) and the v0.7.0-beta release runbook gating on its tests. Designed as an external publish target (see `docs/architecture/driver-framework-as-built.md`). |
| `packages/aps`                    | APS planning schema + helpers.                                         | compatibility-canary | Type Check                   | Used by docs/tooling; failure is fixable post-release.                                                                                                                                                                                                                            |
| `packages/docs-meta`              | Metadata generator consumed by Docusaurus sites and `pnpm docs:check`. | release-blocking     | Nx `test`, Type Check        | `pnpm docs:check` is in Docs Lint required path.                                                                                                                                                                                                                                  |
| `packages/edda-stack`             | Shared UI primitives for site/docs apps.                               | compatibility-canary | Nx `test`, Type Check        | Site-facing; not in the install bundle.                                                                                                                                                                                                                                           |
| `packages/eslint-plugin-anvil`    | Custom ESLint rules used by repo `eslint.config.mjs`.                  | release-blocking     | Nx `test`                    | Drives every Lint & Format invocation; its tests are unit tests for those rules.                                                                                                                                                                                                  |
| `packages/kindling-integration`   | Adapter for Kindling embedded in the CLI.                              | release-blocking     | Nx `test`, Type Check        | Imported by `crates/anvil-cli` (via NAPI).                                                                                                                                                                                                                                        |
| `packages/libs/render`            | Rendering primitives for architecture-health dashboard.                | compatibility-canary | Nx `test`, Type Check        | Owns `specs/architecture-health.dashboard.json`; no in-repo renderer found at the 2026-05-20 pass. Retained under [ADR-023](../../plans/decisions/023-shared-packages-restructure.md); revisit for archive if the dashboard work is not picked up.                                |
| `packages/shared/admin-contracts` | Shared admin contracts between API and admin-cli.                      | release-blocking     | Nx `test`, Type Check        | Co-released with `apps/anvil-api` / `anvil-archive/admin-cli-node`.                                                                                                                                                                                                               |
| `packages/shared/storage`         | Storage adapters intended for API/CLI use.                             | compatibility-canary | Nx `test`, Type Check        | No in-repo consumer at the 2026-05-20 pass (`apps/anvil-api` does not import it). Retained under [ADR-023](../../plans/decisions/023-shared-packages-restructure.md) as the ports/adapters wire-up site; revisit for archive if it stays unconsumed.                              |
| `packages/transactional`          | Email templates for waitlist invites + migration messages.             | release-blocking     | Nx `test`, Type Check        | Imported by `apps/anvil-api` email-sending code.                                                                                                                                                                                                                                  |
| `packages/tooling/eslint-config`  | Shared ESLint config consumed by every package.                        | release-blocking     | Lint & Format                | Indirect — any change to it can break every other lint job.                                                                                                                                                                                                                       |
| `packages/tooling/tsconfig`       | Shared TypeScript configs.                                             | release-blocking     | Type Check                   | Indirect — any change can ripple to every other typecheck.                                                                                                                                                                                                                        |
| `crates/anvil-checks-napi`        | Rust→Node NAPI binding shipped as an npm package.                      | release-blocking     | `.github/workflows/napi.yml` | Required by `kindling-integration` and the CLI's TUI; cross-compiled per release.                                                                                                                                                                                                 |

## Drift notes (2026-05-20)

A repo-wide importer scan on 2026-05-20 found three rationale claims from the
2026-05-16 inventory pass that no longer matched ground truth:

- **`packages/anvil-driver-client`** — claimed "Imported by `apps/e2e`".
  Reality: no in-repo TS importer. Tier stays `release-blocking` because the
  package is the TS protocol mirror, exercised by byte-parity tests against
  captured Rust fixtures (MLP2-029 / MLP2-030 / MLP2-051c) and gated by
  `docs/runbooks/v0.7.0-beta-release-runbook.md`. Rationale corrected.
- **`packages/shared/storage`** — claimed `release-blocking` "Imported by
  `apps/anvil-api`". Reality: zero importers anywhere. Demoted to
  `compatibility-canary`. Created under
  [ADR-023](../../plans/decisions/023-shared-packages-restructure.md) as a
  ports/adapters wire-up site; if it stays unconsumed, the next inventory pass
  should consider archive.
- **`packages/libs/render`** — tier stayed `compatibility-canary` but the
  rationale claimed an "internal dashboard" that has no in-repo renderer.
  Rationale corrected. Same archive-or-revive caveat as `shared/storage`.

These corrections were doc-only; no workflow changes accompanied this pass.
Workflow follow-up (demoting the corrected packages out of integration-readiness
aggregation, or archiving them) is a separate decision.

## Archived surfaces (excluded from the workspace)

The following ship from `archive/` and are **excluded** from
`pnpm-workspace.yaml` via `'!archive/**'`. They do **not** run in release CI;
references in the repo are documentation comments only (see `vitest.config.ts`,
`apps/e2e/vitest.config.ts`, `packages/anvil/runtime/src/index.ts`,
`packages/libs/render/specs/architecture-health.dashboard.json`).

- `anvil-archive/anvil-cli-node`
- `anvil-archive/anvil-mcp-server`
- `anvil-archive/anvil-ts-scanner`
- `anvil-archive/anvil-tui-ink`
- `anvil-archive/anvil-vscode-extension`
- `anvil-archive/eddacraft-tui-local`
- `anvil-archive/tools-node`

If a future workflow grep turns up a live reference to any of these, treat it as
a regression of this inventory pass and remove it.

## Release path summary

The release path (`release.yml` + `release-readiness.yml` +
`release-sign-artefacts.yml`) is intentionally **Rust-side**: it runs
`cargo-dist`, signs the installer scripts with minisign, and binds the public
artefacts to the private source commit via the provenance manifest (#1217). The
only JS/TS work in those workflows is in `release-readiness.yml` and consists
of:

- `pnpm format:check` — repo-wide formatter, sub-second on cold cache.
- `pnpm lint:md` — markdown linter, sub-second on cold cache.

Everything else under "release tier: release-blocking" above is enforced via the
integration-readiness aggregator in `ci.yml` (push to `main`) and via the per-PR
check matrix, not via `release.yml`. That separation means the release workflow
itself stays small and predictable; pre-release-tag validation is owned by the
merged-SHA gate on `main`.

## Adding a new JS/TS surface

1. Add the package to `pnpm-workspace.yaml`.
2. Decide the release tier from the definitions above.
3. If **release-blocking**, ensure it has Nx `test` / `build` targets so it
   joins the integration-readiness aggregator automatically.
4. If **compatibility-canary**, ensure its targets are present but do not need
   to be in `pnpm-workspace.yaml`'s required-jobs path. The per-PR matrix in
   `ci.yml` will path-gate execution.
5. If **out-of-band**, no CI wiring; add a one-line entry to this file
   describing the alternate cadence.
6. Add an entry to the table above.
