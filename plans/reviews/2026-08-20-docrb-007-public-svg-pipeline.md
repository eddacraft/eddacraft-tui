# DOCRB-007 public Draw.io-to-SVG pipeline evidence — 2026-08-20

| Type | Authority | Owner | Status |
| ---- | --------- | ----- | ------ |
| Review | Advisory | DOCRB | Open |

## Scope and source revision

DOCRB-007 started from exact `origin/main`
`e23a60200093dab330b5d61c92a0ae0fdc2a9d85` in the fresh Worktrunk
`/home/aneki/Projects/src/anvil-001.docs-docrb-007-public-svg-pipeline` on
`docs/docrb-007-public-svg-pipeline`. The operator explicitly authorised one
capacity exception; all pre-existing worktrees were preserved. The local
degraded DOCRB-007 claim was acquired and renewed with compare-and-swap but was
not published because this run forbids pushes.

The implementation commit is
`f5a8d131539beb4dc74dfeeb6241e19dc25703e6`. It changes exactly 14
documentation-tooling, fixture, guidance, and APS paths. It adds no production
diagram, public content priority, `docs/public/start-here` mount, legacy
architecture Draw.io handling, component-Mermaid enforcement, CI workflow,
deployment, release claim, package dependency, or lockfile change.

The sibling Worktrunk is outside the MCP server's directly trusted root. Every
repository write was therefore validated through the trusted primary
`/home/aneki/Projects/src/anvil-001` using the equivalent
`.worktrees/docs-docrb-007-public-svg-pipeline/**` content or patch path. No
anvil pre-write decision returned `block`.

## Ready evidence and mounted boundary

The action plan records the ReadyItem before implementation and the module/index
transition from Draft to In Progress. The exact-base source reconciliation found
these production mounts:

| Governed family root | Production renderer source | Result |
| -------------------- | -------------------------- | ------ |
| `docs/public/anvil` | `apps/anvil-docs-private/docusaurus.config.ts` | Mounted |
| `docs/public/beta` | `apps/anvil-docs-private/docusaurus.config.ts` | Mounted |
| `docs/public/aps` | `apps/docs-public/docusaurus.config.ts` | Mounted |
| `docs/public/kindling` | `apps/docs-public/docusaurus.config.ts` | Mounted |
| `docs/public/edda-stack` | `apps/docs-public/docusaurus.config.ts` | Mounted |

The contract intentionally does not derive scope from rollback-only
`apps/docs-site`. Its disabled `docs/public/start-here` plugin remains
unmounted and outside the manifest. The checker proves every declared root
still appears in its named production renderer, so manifest drift cannot
silently broaden or detach the governed surface.

The exporter pin is Draw.io Desktop
[31.1.8](https://github.com/jgraph/drawio-desktop/releases/tag/v31.1.8),
published by the official `jgraph/drawio-desktop` repository on 2026-08-07.
The exact machine contract is
`--export --format svg --embed-diagram --crop --border 0`.

## Pipeline contract

`scripts/docs/public-diagrams.json` is the single machine-readable version,
flag, family-root, and renderer map. The export wrapper:

1. accepts one lower-kebab `.drawio` below a governed root;
2. verifies the installed Desktop version;
3. invokes only the pinned flags and writes to a temporary directory;
4. requires one page and an embedded `<mxfile>` payload;
5. derives `role="img"`, `aria-labelledby`, `<title>`, and `<desc>` from
   non-empty source `anvil-title` and `anvil-description` attributes;
6. records source filename/hash, export-content hash, Desktop version, and flags;
   and
7. writes the same-name sibling `.svg`.

The checker walks only the five governed families. It rejects:

- non-lower-kebab names, missing `.drawio`/`.svg` siblings, and raster
  exports;
- a missing or non-Draw.io embedded source payload, multi-page sources, stale
  source hashes, changed SVG bytes, missing provenance, or the wrong
  version/flags;
- scripts, `foreignObject`, event attributes, JavaScript URLs, or external
  HTTP references;
- missing or mismatched SVG role/name/description metadata;
- declared roots no longer mounted by their production renderer; and
- an SVG that is not referenced from same-family Markdown with meaningful alt
  text or deterministic adjacent equivalent prose.

The checker is the non-baselineable `public-diagrams` surface in
`docs:check`. The existing surface contract test now proves all 12 summary
labels and a live 12/12 pass. The authoritative architecture-diagram guide
documents the same contract and exclusions without adding a second manifest.

## Fixture and replacement RED/GREEN evidence

The focused fixture is not production content. Its minimal Draw.io source and
Markdown reference live below `scripts/docs/fixtures/public-diagrams/`; each
test copies them to a temporary repository.

Replacement evidence was captured vertically:

| Slice | RED | GREEN |
| ----- | --- | ----- |
| Pair/provenance validator | Node test exited 1 because the shared validator did not exist | Initial 10/10 behaviours passed |
| Pinned export wrapper | Export integration lacked a usable root seam | Fake Desktop proof passed with the exact version/flag prefix and sibling provenance |
| Adjacent equivalent prose | Empty-alt reference produced `unreferenced-svg` | Equivalent adjacent prose passed |
| Single-page determinism | Added second `<diagram>` produced no finding | `multi-page-source` failed the fixture as intended |
| Docs surface | Orchestrator emitted no `public-diagrams` row | Live summary emitted `pass public-diagrams` and 12/12 |

The final focused suite passes 13/13 cases: valid pair; stale source; multi-page
source; changed SVG; missing embedded source; active SVG; missing accessible
description; missing reference; adjacent prose; invalid name; raster asset;
renderer mount drift; and pinned exporter invocation.

## Build and boundary evidence

Both production Docusaurus renderers build at the implementation commit:

- `@eddacraft/anvil-docs-private`: exit 0. It retains its known
  cross-app `/anvil/` warnings because the docs shell resolves those routes at
  runtime; the Docusaurus configuration explicitly logs broken links.
- `@eddacraft/docs-public`: exit 0 with generated static files.

The live checker reports 0 errors, 0 warnings, and 0 governed production
diagram files. That zero is intentional: DOCRB-007 establishes the pipeline;
DOCRB-008 owns production diagram selection and authoring. A base-to-head path
check confirms `docs/public/**` is unchanged.

## Implementation-head gates

The implementation head produced these fresh results before this evidence-only
report was added:

| Gate | Exit/result |
| ---- | ----------- |
| `node --test scripts/docs/check-public-diagrams.test.mjs` | 0; 13/13 passed |
| `pnpm docs:public:diagrams` | 0; 0 errors, 0 warnings, 0 files checked |
| `pnpm test:docs-check` | 0; focused suite plus every shell contract case passed |
| `pnpm docs:check` | 0; 12/12 surfaces passed, 0 failed |
| `pnpm --filter @eddacraft/anvil-docs-private build` | 0; static build generated with known shell-routing warnings |
| `pnpm --filter @eddacraft/docs-public build` | 0; static build generated |
| `pnpm format:check` | 0; all 1,695 matched files formatted |
| `pnpm docs:index:check` | 0; 0 errors, 0 warnings, 6 files checked |
| `pnpm docs:owed --since e23a602000` | 0; 0 owed, 0 gating, 0 advisory, 3 documents checked |
| `pnpm aps:active-lint` | 0; 140 files checked, all clean |
| `pnpm aps:index:check` | 0; inherited DOCDEF stored `0/6` versus computed `4/6` advisory |
| `pnpm aps:drift --json` | 0; one inherited DOCDEF `aps-progress-mismatch` warning |
| `git diff --check` | 0 |

The first restricted pnpm attempt exited 226 with `EROFS` before the product
command ran because pnpm tried to create dependency-status state in the sibling
Worktrunk. The identical authorised rerun with Worktrunk write access reached
the product commands and passed. This is classified as tooling-environment
evidence, not a product failure.

The DOCDEF count warning is present at the base, belongs to
`plans/modules/docs-definition-layer.aps.md`, and is not absorbed into
DOCRB-007.

## Report-inclusive closeout

The final report-inclusive format, docs-index refresh/check, docs-check,
exact-range docs-owed, APS active/index/drift, diff, focused, and scope gates
are rerun after this report is committed. Their exact post-report result is
recorded in the final DOCRB-007 handoff and commit evidence; no hosted PR, push,
merge, release, or production diagram publication is part of this work item.

## Rollback

Revert the DOCRB-007 commits as one documentation-tooling unit. No production
diagram or runtime/deployment state must be recovered. Removing the new
`public-diagrams` surface, manifest, wrapper, library, tests, fixtures,
guidance, report, and APS bookkeeping restores the exact-base behaviour.
