# DOCRB-007 public Draw.io-to-SVG pipeline evidence — 2026-08-20

| Type   | Authority | Owner | Status |
| ------ | --------- | ----- | ------ |
| Review | Advisory  | DOCRB | Open   |

## Scope and source revision

DOCRB-007 started from exact `origin/main`
`e23a60200093dab330b5d61c92a0ae0fdc2a9d85` in the fresh Worktrunk
`/home/aneki/Projects/src/anvil-001.docs-docrb-007-public-svg-pipeline` on
`docs/docrb-007-public-svg-pipeline`. The operator explicitly authorised one
capacity exception; all pre-existing worktrees were preserved. The local
degraded DOCRB-007 claim was acquired and renewed with compare-and-swap but was
not published because this run forbids pushes.

The original implementation commit is
`f5a8d131539beb4dc74dfeeb6241e19dc25703e6`. Council FIX ALL repair commit
`bbed33824e237d32bc37df664cfc960f0b44da44` hardens the existing pipeline
contract without broadening it. At that repair head the exact base-to-head range
changes 15 documentation-tooling, fixture, guidance, report, and APS paths. It
adds no production diagram, public content priority, `docs/public/start-here`
mount, legacy architecture Draw.io handling, component-Mermaid enforcement, CI
workflow, deployment, release claim, package dependency, or lockfile change.

The sibling Worktrunk is outside the MCP server's directly trusted root. Every
repository write was therefore validated through the trusted primary
`/home/aneki/Projects/src/anvil-001` using the equivalent
`.worktrees/docs-docrb-007-public-svg-pipeline/**` content or patch path. No
anvil pre-write decision returned `block`.

## Ready evidence and mounted boundary

The action plan records the ReadyItem before implementation and the module/index
transition from Draft to In Progress. The exact-base source reconciliation found
these production mounts:

| Governed family root     | Production renderer source                     | Result  |
| ------------------------ | ---------------------------------------------- | ------- |
| `docs/public/anvil`      | `apps/anvil-docs-private/docusaurus.config.ts` | Mounted |
| `docs/public/beta`       | `apps/anvil-docs-private/docusaurus.config.ts` | Mounted |
| `docs/public/aps`        | `apps/docs-public/docusaurus.config.ts`        | Mounted |
| `docs/public/kindling`   | `apps/docs-public/docusaurus.config.ts`        | Mounted |
| `docs/public/edda-stack` | `apps/docs-public/docusaurus.config.ts`        | Mounted |

The contract intentionally does not derive scope from rollback-only
`apps/docs-site`. Its disabled `docs/public/start-here` plugin remains unmounted
and outside the manifest. The checker parses both production TypeScript
configuration ASTs, ignores comments, and requires exact root-set and
root-to-renderer equality with the manifest. Undeclared active mounts, missing
declared mounts, and mapping drift therefore fail closed. Both excluded surfaces
are explicit in the contract.

The exporter pin is Draw.io Desktop
[31.1.8](https://github.com/jgraph/drawio-desktop/releases/tag/v31.1.8),
published by the official `jgraph/drawio-desktop` repository on 2026-08-07. The
exact machine contract is
`--export --format svg --embed-diagram --crop --border 0`. The wrapper verifies
output from the selected executable; authentic selection of that local Desktop
binary remains an explicit operator-trusted boundary.

## Pipeline contract

`scripts/docs/public-diagrams.json` is the single machine-readable version,
flag, family-root, and renderer map. The export wrapper:

1. accepts one lower-kebab `.drawio` canonically confined below a governed root
   and rejects symlinked sources, ancestors, family roots, or outputs;
2. accepts only exact stdout `31.1.8` with empty stderr from the
   operator-selected Desktop binary;
3. invokes only the pinned flags and writes through an exclusive same-directory
   temporary file;
4. requires one page and an embedded `<mxfile>` payload canonically equal to the
   sibling source;
5. derives `role="img"`, `aria-labelledby`, `<title>`, and `<desc>` from
   non-empty source `anvil-title` and `anvil-description` attributes;
6. records source filename/raw hash, canonical embedded-source hash,
   export-content hash, exact observed version output, Desktop version, and
   flags; and
7. atomically renames to the same-name sibling `.svg` without following an
   existing destination.

The checker walks only the five governed families. It rejects:

- non-lower-kebab names, missing `.drawio`/`.svg` siblings, symlinks, and
  governed raster candidates without an exact ADR-123-reviewed exception;
- a missing, malformed, or canonically mismatched embedded source payload,
  multi-page sources, stale hashes, changed SVG bytes, missing provenance, or
  the wrong version, exact version output, or flags;
- malformed XML, declarations, processing instructions, custom entities, non-SVG
  namespaces, unapproved elements or attributes, scripts, `foreignObject`, event
  attributes, decoded dangerous schemes, CSS imports, or non-fragment
  references;
- missing or mismatched SVG role/name/description metadata;
- declared roots no longer mounted by their production renderer; and
- an SVG that is not referenced from same-family Markdown/MDX with meaningful
  alt text or an explicit, target-bound adjacent description association;
- renderer comments mistaken for mounts, undeclared active mounts, excluded
  `start-here`/`docs-site` surfaces entering scope, or any manifest/config set
  or mapping difference.

The checker is the non-baselineable `public-diagrams` surface in `docs:check`.
The existing surface contract test now proves all 12 summary labels and a live
12/12 pass. The authoritative architecture-diagram guide documents the same
contract and exclusions without adding a second manifest.

## Fixture and replacement RED/GREEN evidence

The focused fixture is not production content. Its minimal Draw.io source and
Markdown reference live below `scripts/docs/fixtures/public-diagrams/`; each
test copies them to a temporary repository.

Original replacement evidence was captured vertically. The adjacent-prose row is
retained as historical trace and superseded by the Council repair table:

| Slice                     | RED                                                           | GREEN                                                                               |
| ------------------------- | ------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| Pair/provenance validator | Node test exited 1 because the shared validator did not exist | Initial 10/10 behaviours passed                                                     |
| Pinned export wrapper     | Export integration lacked a usable root seam                  | Fake Desktop proof passed with the exact version/flag prefix and sibling provenance |
| Adjacent equivalent prose | Empty-alt reference produced `unreferenced-svg`               | Equivalent adjacent prose passed                                                    |
| Single-page determinism   | Added second `<diagram>` produced no finding                  | `multi-page-source` failed the fixture as intended                                  |
| Docs surface              | Orchestrator emitted no `public-diagrams` row                 | Live summary emitted `pass public-diagrams` and 12/12                               |

Council repair replacement evidence was captured one failing behaviour group at
a time:

| Repair slice                            | RED                                                                                                       | GREEN                                                                                                         |
| --------------------------------------- | --------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| XML/SVG and embedded parity             | Declaration, entity, namespace, CSS/reference, and sibling-mismatch adversaries were accepted or untested | Fail-closed DOM and canonical-source checks passed                                                            |
| Canonical confinement and atomic output | External source, intermediate, and output symlinks were not rejected                                      | All three link boundaries failed closed and the target stayed unchanged                                       |
| Exact structural mounts                 | Comment text counted as a mount and undeclared active mounts escaped the manifest                         | Both production ASTs matched exact manifest roots and mappings                                                |
| Markdown/MDX references                 | Arbitrary nearby prose authorised empty alt text                                                          | Meaningful alt or an exact target-bound association passed; code/comments, weak alt, and wrong targets failed |
| Raster scope and exceptions             | Every public raster was rejected and no reviewed exception existed                                        | Ordinary screenshots stayed out of scope; diagram candidates required exact ADR-123 review                    |
| Version and provenance                  | Near, prefixed, and ambiguous version output passed substring matching                                    | Exact semver/product output and raw, canonical-embedded, and export hashes passed                             |
| XML parser closeout                     | A spoofed default namespace and malformed XML were accepted                                               | Namespace URI and parser-error checks failed both closed                                                      |

The repaired focused suite passes 44/44 behaviours, including the valid fixture,
all adversarial cases, export integration, symlink boundaries, mount set/mapping
equality, accessible references, raster scope, and exact version output.

## Build and boundary evidence

Both production Docusaurus renderers build at the original implementation head
and at the report-inclusive repair candidate:

- `@eddacraft/anvil-docs-private`: exit 0. It retains its known cross-app
  `/anvil/` warnings because the docs shell resolves those routes at runtime;
  the Docusaurus configuration explicitly logs broken links.
- `@eddacraft/docs-public`: exit 0 with generated static files.

The live checker reports 0 errors, 0 warnings, and 0 governed production diagram
files. That zero is intentional: DOCRB-007 establishes the pipeline; DOCRB-008
owns production diagram selection and authoring. A base-to-head path check
confirms `docs/public/**` is unchanged.

## Original implementation-head gates

The implementation head produced these fresh results before this evidence-only
report was added:

| Gate                                                      | Exit/result                                                     |
| --------------------------------------------------------- | --------------------------------------------------------------- |
| `node --test scripts/docs/check-public-diagrams.test.mjs` | 0; 13/13 passed                                                 |
| `pnpm docs:public:diagrams`                               | 0; 0 errors, 0 warnings, 0 files checked                        |
| `pnpm test:docs-check`                                    | 0; focused suite plus every shell contract case passed          |
| `pnpm docs:check`                                         | 0; 12/12 surfaces passed, 0 failed                              |
| `pnpm --filter @eddacraft/anvil-docs-private build`       | 0; static build generated with known shell-routing warnings     |
| `pnpm --filter @eddacraft/docs-public build`              | 0; static build generated                                       |
| `pnpm format:check`                                       | 0; all 1,695 matched files formatted                            |
| `pnpm docs:index:check`                                   | 0; 0 errors, 0 warnings, 6 files checked                        |
| `pnpm docs:owed --since e23a602000`                       | 0; 0 owed, 0 gating, 0 advisory, 3 documents checked            |
| `pnpm aps:active-lint`                                    | 0; 140 files checked, all clean                                 |
| `pnpm aps:index:check`                                    | 0; inherited DOCDEF stored `0/6` versus computed `4/6` advisory |
| `pnpm aps:drift --json`                                   | 0; one inherited DOCDEF `aps-progress-mismatch` warning         |
| `git diff --check`                                        | 0                                                               |

The first restricted pnpm attempt exited 226 with `EROFS` before the product
command ran because pnpm tried to create dependency-status state in the sibling
Worktrunk. The identical authorised rerun with Worktrunk write access reached
the product commands and passed. This is classified as tooling-environment
evidence, not a product failure.

## Council repair core gates

These fresh results were captured with repair commit
`bbed33824e237d32bc37df664cfc960f0b44da44` as the exact code head:

| Gate                                                      | Exit/result                                                       |
| --------------------------------------------------------- | ----------------------------------------------------------------- |
| `node --test scripts/docs/check-public-diagrams.test.mjs` | 0; 44/44 passed                                                   |
| `pnpm docs:public:diagrams`                               | 0; 0 errors, 0 warnings, 0 files checked                          |
| `pnpm test:docs-check`                                    | 0; focused suite plus all shell contract cases passed             |
| `pnpm docs:check`                                         | 0; 12/12 surfaces passed, 0 failed                                |
| `pnpm --filter @eddacraft/anvil-docs-private build`       | 0; static build generated with known shell-routing warnings       |
| `pnpm --filter @eddacraft/docs-public build`              | 0; static build generated                                         |
| `pnpm format:check`                                       | 0; all 1,695 matched files formatted                              |
| `pnpm docs:index`; `pnpm docs:index:check`                | 0; idempotent refresh, then 0 errors, 0 warnings, 6 files checked |
| `pnpm docs:owed --since e23a602000`                       | 0; 0 owed, 0 gating, 0 advisory, 3 documents checked              |
| `pnpm aps:active-lint`                                    | 0; 140 files checked, all clean                                   |
| `pnpm aps:index:check`                                    | 0; inherited DOCDEF stored `0/6` versus computed `4/6` advisory   |
| `pnpm aps:drift --json`                                   | 0; one inherited DOCDEF `aps-progress-mismatch` warning           |
| `git diff --check`                                        | 0                                                                 |

Restricted first attempts at the two Docusaurus builds failed before product
validation with `EROFS` on generated `.docusaurus` files. A restricted
docs-check harness attempt likewise reached case D before fixture `chmod` failed
with `EROFS`. The identical authorised Worktrunk-write reruns passed. These are
tooling-environment observations, not content or product failures.

The DOCDEF count warning is present at the base, belongs to
`plans/modules/docs-definition-layer.aps.md`, and is not absorbed into
DOCRB-007.

## Report-inclusive closeout

The final report-inclusive commands were rerun with the substantive report
postimage present immediately before the evidence commit; this table transcribes
those results:

| Gate                                                      | Exit/result                                                                                                  |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `pnpm docs:index`; `pnpm docs:index:check`                | 0; idempotent refresh, then 0 errors, 0 warnings, 6 files checked                                            |
| `node --test scripts/docs/check-public-diagrams.test.mjs` | 0; 44/44 passed                                                                                              |
| `pnpm docs:public:diagrams`                               | 0; 0 errors, 0 warnings, 0 files checked                                                                     |
| `pnpm test:docs-check`                                    | 0; all focused and shell contract cases passed                                                               |
| `pnpm docs:check`                                         | 0; 12/12 surfaces passed, 0 failed                                                                           |
| both production Docusaurus builds                         | 0; both generated static files; private renderer retained known `/anvil/` shell-routing warnings             |
| `pnpm format:check`                                       | 0; all 1,695 matched files formatted                                                                         |
| `pnpm docs:owed --since e23a602000`                       | 0; 0 owed, 0 gating, 0 advisory, 3 documents checked                                                         |
| `pnpm aps:active-lint`                                    | 0; 140 files checked, all clean                                                                              |
| `pnpm aps:index:check`; `pnpm aps:drift --json`           | 0; only inherited DOCDEF `0/6` versus `4/6` advisory                                                         |
| `git diff --check`                                        | 0                                                                                                            |
| exact-range scope                                         | 15 paths; no `docs/public/**`, repair-time package/lockfile, production asset, workflow, or deployment delta |

No hosted PR, push, merge, release, or production diagram publication is part of
this work item.

## Rollback

Revert the DOCRB-007 commits as one documentation-tooling unit. No production
diagram or runtime/deployment state must be recovered. Removing the new
`public-diagrams` surface, manifest, wrapper, library, tests, fixtures,
guidance, report, and APS bookkeeping restores the exact-base behaviour.
