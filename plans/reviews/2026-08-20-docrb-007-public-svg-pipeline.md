# DOCRB-007 public Draw.io-to-SVG pipeline evidence — 2026-08-20

| Type   | Authority | Owner | Status |
| ------ | --------- | ----- | ------ |
| Review | Advisory  | DOCRB | Open   |

## Scope and source revision

DOCRB-007 started from exact `origin/main`
`e23a60200093dab330b5d61c92a0ae0fdc2a9d85` in the fresh Worktrunk
`/home/aneki/Projects/src/anvil-001.docs-docrb-007-public-svg-pipeline` on
`docs/docrb-007-public-svg-pipeline`. Before publication, the completed
series was rebased onto exact `origin/main`
`8c8825389d41da72f575704eec34b214a60987ad`. The rebased implementation
lineage is:

- `27903654f16a85e783ce6edcfdf97da1c71f6a6d` — initial pipeline;
- `4b566d0940af7e52eb48dae977ec58e1477a9394` — committed Council safety
  repair, including renderer AST and exclusion governance;
- `5e36d2f67f1c2857befa273355442087fd21119b` — committed evidence head for
  that renderer-governance repair;
- `ace8aa35dd1a1845f2d035bb429b69fe7b7459f1` — exact
  operator-approved contraction candidate;
- `04f33177bd3fc65ff63e2a2029c9fe84c764849b` — retained-scope SVG
  safety, directory-boundary, and freshness repair; and
- `5c2f5854e79b70e96ac7264ccef34c7d6a6430e8` — manifest-root
  preflight repair; and
- `aedebcc4e7de479b2b60a351ba89bf2e314a6365` — PR review repair for
  confined summary counting, upper-case invalid-file accounting, and complete
  exporter usage.

The first Git-reproducible retained-scope repair range is
`ace8aa35dd1a1845f2d035bb429b69fe7b7459f1..04f33177bd3fc65ff63e2a2029c9fe84c764849b`.
The final manifest-root repair range is
`e591abe0dbf0d47d009ec469336a826c766c3890..5c2f5854e79b70e96ac7264ccef34c7d6a6430e8`.

`4b566d0940af7e52eb48dae977ec58e1477a9394` committed the Docusaurus
configuration AST analysis, exact mount-set enforcement, and renderer/root
exclusion-schema governance; `5e36d2f67f1c2857befa273355442087fd21119b`
then committed its evidence report. Only after that commit did a transient
uncommitted repair wave add checker-time
trusted Draw.io re-export and render attestation. The operator explicitly
approved removal of both scope expansions after the broader-need assessment:

1. the committed renderer AST, exact-set, and exclusion governance; and
2. the later uncommitted checker-time re-export and attestation.

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
| SVG active-content and external-reference rejection | Kept | DOM, namespace, entity, CSS, fragment-only `url(...)` in every attribute, and outright `ping` checks |
| Path confinement, symlink refusal, exclusive temporary file, atomic rename | Kept | Every manifest root is preflighted before traversal; exporter covers source/ancestor/output paths; checker enforcement is limited to explicit diagram roots |
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

Core LOC is the sum of the checker, exporter, shared library, and focused test
file. The previously reported 2,081 LOC and 72 tests were observed after
`5e36d2f67f1c2857befa273355442087fd21119b` in the transient uncommitted
checker-time re-export wave, layered
on the already committed renderer AST/exclusion governance. No Git object
captures that snapshot, so those figures are retained only as a
non-reproducible observation and are not presented as an exact commit
comparison.

| Git-reproducible snapshot | Core LOC | Focused tests | Meaning |
| ------------------------- | -------: | ------------: | ------- |
| `5e36d2f67f1c2857befa273355442087fd21119b` | 1,434 | 44 | Committed renderer AST/exclusion head, before the transient re-export wave |
| `ace8aa35dd1a1845f2d035bb429b69fe7b7459f1` | 1,670 | 64 | Exact operator-approved contraction candidate |
| `04f33177bd3fc65ff63e2a2029c9fe84c764849b` | 1,688 | 69 | First retained-scope repair head |
| `5c2f5854e79b70e96ac7264ccef34c7d6a6430e8` | 1,810 | 71 | Final manifest-root preflight repair head |
| `aedebcc4e7de479b2b60a351ba89bf2e314a6365` | 1,858 | 74 | PR review repair head |

The exact `ace8aa35d..04f33177b` repair adds 18 core lines and five focused
tests. The exact `e591abe0d..5c2f5854e` manifest-root repair adds 122 core
lines and two focused tests. Relative to the transient observation, the
contraction candidate removed 411 core lines and eight tests, but that delta is
not a Git-reproducible range. The removed AST, render-attestation, and
exclusion-schema tests remain absent.

The PR review repair adds 48 core lines and three focused tests without
reintroducing either removed subsystem.

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

Before any content walk or read, the checker preflights every manifest family
and diagram root for normalised repository-relative syntax, absence of parent
traversal, realpath confinement, and non-symlink ancestors; invalid roots emit
a contract finding and are not traversed. It then scans regular Markdown/MDX
files in the five public families but skips unrelated descendant symlinks
without reporting or following them. Descendant symlink enforcement applies
only to the five explicit diagram directories. Inside that boundary it
validates pairing, naming, lower-case
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
| Generic SVG attribute URLs | Provenance-correct `shape-inside`, `shape-subtract`, and arbitrary-attribute external URLs produced no `unsafe-svg`; `ping` was accepted | Every attribute containing `url(...)` is fragment-only and `ping` is rejected outright |
| Family reference symlink boundary | An unrelated family symlink produced `symlink-path` | Markdown/MDX discovery skips it without reporting or traversal; explicit diagram-root links still fail closed |
| Manifest-root preflight | An absolute family root produced no contract finding, and a parent-traversing diagram root reached its symlink tripwire | Both emit `invalid-contract` before content traversal; the tripwire produces no `symlink-path` |
| Summary confinement | The checker summary traversed an invalid parent-relative diagram root after validation rejected it | Summary counting reuses validated manifest roots and reports zero checked files for the invalid root |
| Summary invalid-extension accounting | An upper-case governed `.SVG` produced a finding but was omitted from `filesChecked` | The failing file is included in the summary count |
| Exporter usage | The supported `--root <path>` option was absent from the usage error | Usage lists both `--root` and `--drawio-bin` |
| Retained safety | Existing adversarial cases stayed green during all repairs | The full focused suite passes 74/74 |

## Fresh evidence

These results were captured at exact implementation head
`aedebcc4e7de479b2b60a351ba89bf2e314a6365` after the publication rebase:

| Gate | Result |
| ---- | ------ |
| `node --test scripts/docs/check-public-diagrams.test.mjs` | Exit 0; 74/74 tests passed |
| `pnpm docs:public:diagrams` | Exit 0; 0 errors, 0 warnings, 0 production files |
| `pnpm test:docs-check` | Exit 0; focused suite and all shell-harness cases through `AK` passed |
| `pnpm docs:check` | Exit 0; 12/12 surfaces passed |
| `pnpm --filter @eddacraft/anvil-docs-private build` | Exit 0; generated static files, with the inherited `/anvil/` shell-routing warnings |
| `pnpm --filter @eddacraft/docs-public build` | Exit 0; generated static files |
| `pnpm format:check` | Exit 0; 1,695 files checked |
| `pnpm docs:index:check` | Exit 0; 0 errors, 0 warnings, 6 files checked |
| `pnpm docs:owed --since 8c8825389` | Exit 0; 0 owed across 3 checked documents |
| `pnpm aps:active-lint` | Exit 0; 140 files checked |
| `pnpm aps:index:check` | Exit 0; stored counts match lifecycle truth |
| `pnpm aps:drift` | Exit 0; 0 findings |
| `git diff --check` | Exit 0 |

The first restricted `pnpm test:docs-check` attempt reached fixture case D and
failed when `chmod` met the sibling Worktrunk's read-only sandbox boundary.
The identical scoped Worktrunk-write rerun passed all cases. Both restricted
formatter and harness failures are tooling-environment evidence, not product
failures.

## Rollback

Revert the DOCRB-007 commits as one documentation-tooling unit. No production
diagram, runtime state, deployment state, or package dependency requires
recovery.
