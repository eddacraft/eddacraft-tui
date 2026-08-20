# DOCRB-007 public Draw.io-to-SVG pipeline evidence — 2026-08-20

| Type   | Authority | Owner | Status |
| ------ | --------- | ----- | ------ |
| Review | Advisory  | DOCRB | Open   |

## Scope and source revision

DOCRB-007 started from exact `origin/main`
`e23a60200093dab330b5d61c92a0ae0fdc2a9d85` in the fresh Worktrunk
`/home/aneki/Projects/src/anvil-001.docs-docrb-007-public-svg-pipeline` on
`docs/docrb-007-public-svg-pipeline`. The implementation commits before this
repair are:

- `f5a8d131539beb4dc74dfeeb6241e19dc25703e6` — initial pipeline;
- `bbed33824e237d32bc37df664cfc960f0b44da44` — first Council safety repair;
- `97e3e6db2` — hardening evidence report at the current committed head.

A later uncommitted repair wave added two subsystems that exceeded ADR-123 and
the original DOCRB-007 acceptance boundary. The operator explicitly approved
their removal after the broader-need assessment:

1. checker-time trusted Draw.io re-export and render attestation; and
2. Docusaurus configuration AST analysis, exact mount-set enforcement, and
   renderer/root exclusion-schema governance.

This contraction does not remove essential XML/SVG or file-write safety. It
adds no production diagram, public-content priority, workflow, deployment,
release claim, package dependency, or lockfile change.

The sibling Worktrunk is outside the MCP server's directly trusted root. Every
repository write was validated through the trusted primary
`/home/aneki/Projects/src/anvil-001` with full proposed content. Every
pre-write decision was `allow`.

## Acceptance boundary

ADR-123 requires paired editable source and deterministic accessible SVG,
textual meaning, and parity with the approved export process. DOCRB-007
establishes that asset pipeline. The two production Docusaurus builds prove
renderer integration at the system boundary; the checker does not implement a
second Docusaurus configuration analyser.

| Surface | Decision | Evidence or consequence |
| ------- | -------- | ----------------------- |
| Five explicit `docs/public/*/assets/diagrams` directories | Kept | Manifest requires one directory for each governed family |
| Lower-kebab sibling `.drawio`/`.svg` pairs | Kept | Missing pairs, wrong names, and upper-case extensions fail |
| Well-formed single-page Draw.io XML and embedded-source equality | Kept | Namespace-aware structural parsing and canonical comparison |
| SVG active-content and external-reference rejection | Kept | DOM, namespace, entity, CSS, URL, and fragment checks |
| Path confinement, symlink refusal, exclusive temporary file, atomic rename | Kept | Exporter and checker fixtures cover source, ancestor, and output paths |
| Exact Desktop version/output/arguments | Kept | Export wrapper validates `31.1.8` and the pinned flag sequence |
| Raw source, canonical embedded source, and export freshness hashes | Kept | Checker detects stale source and changed SVG bytes |
| Accessible real Markdown/MDX reference | Kept | Meaningful alt or exact target-bound adjacent description |
| Directory-scoped ADR-123 raster exceptions | Kept | Exact path, consumer reason, review, and accessible reference required |
| `public-diagrams` as the twelfth `docs:check` surface | Kept | Harness asserts all twelve labels and live execution |
| Both production Docusaurus builds | Kept | Proves the five public families remain consumable |
| Checker-time Draw.io re-export | Dropped | Exporter produces provenance; checker verifies committed artefacts without a local Desktop dependency |
| Docusaurus TypeScript AST and exact mount-set analyser | Dropped | Build outputs are the integration proof |
| `productionRenderers`, `excludedRoots`, and `excludedRenderers` schema | Dropped | Not needed to enumerate the five governed asset directories |
| Fake-binary render-attestation and renderer-config golden cases | Dropped | Tests now cover only the retained public-asset contract |

## Size and test contraction

The pre-contraction snapshot was measured before the approved edits. Core LOC is
the sum of the checker, exporter, shared library, and focused test file.

| Measure | Before | After | Delta |
| ------- | -----: | ----: | ----: |
| Core implementation and focused-test LOC | 2,081 | 1,670 | -411 (-19.8%) |
| Focused Node tests | 72 | 64 | -8 net |
| Scope-drift tests removed | 10 | 0 | -10 |
| Contraction-boundary tests added | 0 | 2 | +2 |

The two new tests prove that committed provenance validates without invoking
Draw.io and that the contract contains no renderer-analysis or exclusion
schema.

## Pipeline contract

`scripts/docs/public-diagrams.json` owns the five family roots, their exact
`assets/diagrams` directories, Desktop version/output, export arguments, and
raster exceptions. The export wrapper:

1. accepts one lower-kebab `.drawio` below an explicit governed directory;
2. refuses repository escapes, symlinked sources/ancestors/outputs, and
   non-regular sources;
3. requires exact Desktop stdout `31.1.8` with no stderr or ambiguity;
4. invokes `--export --format svg --embed-diagram --crop --border 0`;
5. requires one well-formed page and canonical sibling/embedded equality;
6. derives `role="img"`, `aria-labelledby`, `<title>`, and `<desc>` from
   source accessibility attributes;
7. records source, embedded-source, export, version-output, version, and flag
   provenance; and
8. writes through an exclusive same-directory temporary file and atomic rename.

The checker walks the five public families but governs diagram-like files only
inside the explicit directories. It validates pairing, naming, lower-case
extensions, provenance, structural XML parity, SVG safety, accessibility, real
same-family Markdown/MDX references, and reviewed raster exceptions. It never
invokes Draw.io and does not inspect Docusaurus configuration source.

The live production count is zero, intentionally: DOCRB-007 establishes the
pipeline and DOCRB-008 owns production diagram authoring.

## Replacement RED/GREEN evidence

| Slice | RED | GREEN |
| ----- | --- | ----- |
| No checker-time Desktop dependency | Validation without a supplied fake binary produced `render-verification-unavailable` | The checker validates committed provenance without invoking Draw.io |
| No renderer-analysis schema | The live contract still contained renderer and exclusion fields | The contract test proves all fields and per-family renderer mappings are absent |
| Retained safety | Existing adversarial cases stayed green during both removals | The full focused suite passes 64/64 |

## Fresh evidence

| Gate | Result |
| ---- | ------ |
| `node --test scripts/docs/check-public-diagrams.test.mjs` | Exit 0; 64/64 tests passed |
| `pnpm docs:public:diagrams` | Exit 0; 0 errors, 0 warnings, 0 production files |
| `pnpm test:docs-check` | Exit 0; focused suite and all shell-harness cases passed |
| `pnpm docs:check` | Exit 0; 12/12 surfaces passed |
| `pnpm --filter @eddacraft/anvil-docs-private build` | Exit 0; generated static files, with the inherited `/anvil/` shell-routing warnings |
| `pnpm --filter @eddacraft/docs-public build` | Exit 0; generated static files |
| `pnpm exec oxfmt --write <four changed files>` | Exit 0; repository formatter applied after the restricted write attempt failed with `EROFS` |
| `pnpm aps:active-lint` | Exit 0; 140 files checked |
| `pnpm aps:index:check` | Exit 0; only the inherited DOCDEF `0/6` versus `4/6` advisory |
| `pnpm aps:drift --json` | Exit 0; the same inherited DOCDEF advisory |
| `git diff --check` | Exit 0 |

The first restricted `pnpm test:docs-check` attempt reached fixture case D and
failed when `chmod` met the sibling Worktrunk's read-only sandbox boundary.
The identical scoped Worktrunk-write rerun passed all cases. Both restricted
formatter and harness failures are tooling-environment evidence, not product
failures.

The DOCDEF count warning exists at the base and belongs to
`plans/modules/docs-definition-layer.aps.md`; DOCRB-007 does not absorb it.

## Rollback

Revert the DOCRB-007 commits as one documentation-tooling unit. No production
diagram, runtime state, deployment state, or package dependency requires
recovery.
