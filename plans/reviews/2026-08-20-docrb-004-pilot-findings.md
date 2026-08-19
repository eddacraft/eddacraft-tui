# DOCRB-004 Component Pilot Findings — 2026-08-20

| Type   | Authority | Owner | Status |
| ------ | --------- | ----- | ------ |
| Review | Advisory  | DOCRB | Closed |

## Scope and source revision

This report records the single evidence set for the six DOCRB-002-selected
component roots:

- `crates/anvil-kernel`
- `crates/anvil-intercept`
- `apps/dashboard`
- `crates/anvil-dashboard-server`
- `apps/anvil-api`
- `apps/docs-shell`

The review used source revision
`d6c8b565c375e9e75db44c5d20d2acb066e4471c`, which was both the branch base
and `origin/main` when DOCRB-004 started. No product, configuration, central
as-built, governance, public-diagram, or sibling APS status change is part of
the pilot.

## Result

**Pass.** Each pilot root now has a concise authoritative orientation README and
a derived, source-linked local architecture map. Each architecture map contains
one Mermaid diagram for its bounded component concern. Retained central
authorities remain linked and explicitly unsuperseded.

## Navigation trace

The trace started at each component root, followed local orientation to
internals, opened every linked load-bearing source path, and then followed the
retained central authority.

| Root | Navigation trace | Result |
| ---- | ---------------- | ------ |
| `crates/anvil-kernel` | README → local architecture → watcher/parser/graph/protocol source → retained kernel as-built | Pass; initial baseline and incremental-finding distinction is visible |
| `crates/anvil-intercept` | README → local architecture → IPC/admission/save-time/path/fence source → retained intercept and driver maps | Pass; validation failure is not misrepresented as automatic fencing |
| `apps/dashboard` | README → local architecture → router/query/API client/generated contract → local dashboard guide | Pass; UI ownership stops at the typed same-origin client |
| `crates/anvil-dashboard-server` | README → local architecture → loopback guard/capability/workspace/OpenAPI source → local dashboard guide | Pass; network boundary and deliberate absence of user auth are explicit |
| `apps/anvil-api` | README → local architecture → global middleware/routes/database source → retained API and BAUTH auth maps | Pass; APGOV component ownership and BAUTH auth authority remain separate |
| `apps/docs-shell` | README → local architecture → OAuth/session/proxy source → deployment truth and governance | Pass; live private/public renderers, rollback-only docs-site, and the DOCRB/DSITE gap remain explicit |

Root `CONTEXT.md` discovers the new thin docs-shell `AGENTS.md` spoke. No
other pilot received a local agent file.

## Mermaid render and source trace

Six Mermaid blocks were extracted directly from the six local
`ARCHITECTURE.md` files and rendered with
`@mermaid-js/mermaid-cli 11.16.0`. Chromium cannot start its nested sandbox in
this container, so the successful manual preview used a temporary Puppeteer
configuration with `--no-sandbox`; inputs and SVG outputs were written only
under `/tmp` and are not repository artefacts.

| Diagram concern | Render | Source-edge trace |
| --------------- | ------ | ----------------- |
| Kernel source → parse → graph → finding | Pass, non-empty SVG | `watch.rs`, `parser/`, `anvil-graph-cache`, `protocol/` |
| Intercept save → validate → conditional fence | Pass, non-empty SVG | `ipc.rs`, `auth.rs`, `workspace_admission.rs`, `save_time.rs`, `validate_paths.rs`, `fence.rs` |
| Dashboard UI → generated client → loopback server | Pass, non-empty SVG | router/modules/hooks, API client, generated OpenAPI types, generator |
| Dashboard server capability/access boundary | Pass, non-empty SVG | loopback guard, read-only routes, capability loaders, `WorkspaceAnchor` |
| Hosted API request → middleware → route → persistence/trust | Pass, non-empty SVG | `index.ts`, middleware, representative route boundaries, database client/queries |
| Docs shell auth/routing → private/public renderer | Pass, non-empty SVG | login/callback, BAUTH exchange, licence verification, proxy, Vercel deployment |

Adjacent prose provides the same meaning for readers who cannot consume the
diagram.

## Metadata, links, ownership, and duplication

Manual checks are required because current `pnpm docs:check` does not enforce
component-root README/architecture metadata, cited paths, or links.

- All 123 repository-local Markdown links across the twelve pilot docs and the
  thin docs-shell spoke resolve. This trace caught and repaired three incorrect
  draft targets before closeout: kernel parser/protocol module links and a
  nonexistent API adapter path.
- Metadata source paths and globs were traced against the pinned tree. Each
  README is `Authoritative` for component orientation; each architecture file
  is `Derived` from current source.
- KERN owns kernel, INTD owns interception, DASH owns both dashboard roots, and
  APGOV owns the hosted API component. BAUTH remains authoritative for
  authentication.
- Docs-shell metadata deliberately says `DOCRB/DSITE gap`: DOCRB owns this
  pilot evidence, while DSITE still owns recorded legacy-host work and has not
  adopted the live shell. This is not a joint runtime ownership assignment.
- Local documents link rather than copy central diagrams, public guidance, ADR
  rationale, or operator procedures. Temporary overlap is labelled as a
  DOCRB-004 pilot, and DOCRB-005 retains authority over migration or deliberate
  central retention.

## Validation evidence

Replacement RED assertions first proved the local documents, metadata,
component flows, docs-shell spoke, and report were absent. The corresponding
GREEN assertions now cover all seven DOCRB-004 acceptance behaviours.

Focused component validation:

```text
cargo test -p eddacraft-anvil-kernel
cargo test -p eddacraft-anvil-intercept
cargo test -p eddacraft-anvil-dashboard-server
pnpm --filter @eddacraft/anvil-dashboard test
pnpm --filter @eddacraft/anvil-api test -- --run
pnpm --filter @eddacraft/docs-shell test
```

All commands exited zero. The concise JavaScript summaries were 16 files /
71 tests for dashboard, 46 files / 757 tests for the API, and 6 files / 51 tests
for docs-shell. All Rust target test binaries passed; the kernel's primary unit
suite reported 337 passing tests and interception's primary unit suite reported
1,106.

Repository validation at report time:

```text
pnpm format:check
pnpm docs:index:check
pnpm docs:check
pnpm aps:active-lint
pnpm aps:index:check
pnpm aps:drift --json
git diff --check
```

Formatting and index freshness passed. Documentation checks passed all 11
surfaces with only pre-existing baselined/advisory warnings. APS active lint and
stored counts passed; drift reported `findingCount: 0`; the diff whitespace
check passed.

## Recommendations

### DOCRB-005 migration

1. Compare each retained central as-built with its new local source-linked map
   before moving or deleting content. Preserve genuinely cross-system concerns
   centrally.
2. Treat kernel, intercept, and API central maps as explicit reconciliation
   candidates. Do not delete them merely because a pilot exists.
3. Keep the local dashboard operator guide and any genuine multi-surface TUI
   view central; migrate only component-internal UI/server material.
4. Keep production docs topology and the DOCRB/DSITE ownership gap in ADR-123
   and documentation governance. A component map must not become a second
   deployment authority.
5. Use the pilot's conditional-edge pattern for fallback and degraded paths;
   compact diagrams become inaccurate when they collapse validation, fencing,
   auth, or availability into one unconditional edge.

### DOCRB-009 enforcement

1. Extend documentation checks to component-root README and architecture
   metadata, paths, and Markdown links only after the canonical migration map is
   established.
2. Add Mermaid syntax/render checking through an authorised, pinned toolchain
   with a documented container launch configuration and useful file-level
   diagnostics.
3. Enforce the ADR-123 trigger and exemptions at the changed-component
   boundary, not as a repository-wide requirement for every code change.
4. Keep enforcement advisory until DOCRB-009 is explicitly activated; this
   pilot adds no automated Mermaid tooling or CI rule.
