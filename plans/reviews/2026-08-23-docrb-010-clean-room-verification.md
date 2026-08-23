# DOCRB-010 clean-room verification evidence

**Date:** 2026-08-23
**Item:** DOCRB-010
**Pinned starting receipt:** `abe6be8b657b8be68565aace3aada6056323ae61`
**Exact rebase base:** `930900af4a88d594a173c0c541809ca4752ff050`
**Decision:** Supported after the clean-room and post-rebase exact-head validation; independent verification and Council remain lifecycle gates.

## Scope and method

This verification used the dedicated
`docs/docrb-010-clean-room-verification` Worktrunk. The only repository
postimage owned by the verifier is this report. The module, index, and action
plan were already the three intended planning paths. No product, documentation,
diagram, checker, build, workflow, dependency, issue, claim, or checkpoint was
changed to make a journey pass.

The clean-room journeys were executed independently against the repository
contracts and current source. Existing fixtures and isolated `/tmp` browser
and build state supplied failure-mode evidence; no fixture or source was edited.

## Clean-room baseline

- `git rev-parse HEAD` returned
  `abe6be8b657b8be68565aace3aada6056323ae61`.
- Node was `v24.18.0`; pnpm was `11.9.0`.
- `pnpm install --frozen-lockfile` exited 0 and reported the lockfile and
  installation already up to date.
- `pnpm exec mmdc --version` exited 0 with exact stdout `11.16.0`.
- The isolated Worktrunk contained only the three intended DOCRB-010 planning
  postimages before this report.

## Representative journeys

### Maintainer

The component-local Mermaid authority in
`apps/docs-shell/ARCHITECTURE.md` was traced from its request, path-class,
session, `docs.access`, OAuth callback, and proxy-response nodes and arrows to:

- `apps/docs-shell/proxy.ts` for the `/anvil` route, session cookie, proxy
  branches, header filtering, and timeout;
- `apps/docs-shell/lib/jwt.ts` and
  `apps/docs-shell/lib/feature-flags.ts` for licence verification and the
  `DOCS_ACCESS_FLAG` decision;
- `infra/src/vercel.ts` for the docs-shell domain, public/private upstream
  applications, secrets, and watch paths; and
- the private and public middleware protected-upstream-secret contracts.

The corresponding source/build/deploy and request arrows in
`docs/architecture/docs-delivery.md` matched that implementation trace.

### Contributor

The root `AGENTS.md` route led to the documentation-governance and
architecture-diagram authorities. The focused enforcement suite proved that a
relevant declared-upstream change or deletion fails when its owner is
untouched, an updated owner passes, and an unrelated change passes without a
waiver. It also proved fail-closed diff-base handling and that candidate content
cannot authorise the conditional Chromium sandbox fallback.

### Operator

The live docs-shell/private/public topology was traced through the same proxy,
feature-flag, deployment, and middleware sources. The private sidebar mounts
`first-gate` beneath the `/anvil/` site base, and the public APS sidebar mounts
`getting-started` beneath `/aps/`.

### Public reader

Built applications were served locally and inspected with headless Chromium
152.0.7977.42 through Puppeteer 25.8.0. The nested host sandbox could not start,
so the already-governed `--no-sandbox --disable-setuid-sandbox` browser
fallback was used for this isolated read-only inspection.

- `/anvil/first-gate` and `/aps/getting-started` both returned HTTP 200 with
  the expected title and heading and were not fallback 404 pages.
- Both routes applied real dark and light themes. The page backgrounds read
  `rgb(13, 13, 15)` in dark mode and `rgb(250, 250, 250)` in light mode.
- The anvil diagram exposed meaningful alternative text, loaded at
  `966 x 338`, rendered responsively at approximately `823 x 288`, and
  remained within the article.
- The APS diagram exposed meaningful alternative text, loaded at
  `1096 x 350`, rendered responsively at approximately `823 x 263`, and
  remained within the article.
- Both pages had `clientWidth === scrollWidth === 1440`; neither diagram was
  clipped. Manual readback of all four theme screenshots found legible text,
  complete flows, and no visual obstruction.

Screenshots were retained outside the repository at
`/tmp/docrb010-{anvil,aps}-{dark,light}.png`.

## Rendering, parity, and accessibility

- The corpus checker rendered 17 governed documents and 24 Mermaid fences with
  zero findings using exact Mermaid 11.16.0. It transparently reported the
  conditional no-sandbox fallback required by this host.
- The public-diagram checker accepted four governed files with zero errors and
  zero warnings. The committed Draw.io sources and SVG exports retained paired
  provenance; the sources have opaque white canvases, and the accessible
  title/description metadata agrees with the mounted output.
- `pnpm test:docs-check` passed all 98 Node tests and every shell fixture
  (cases 1-19 and A-AK), including SVG structure, security, accessibility,
  provenance/parity, change/deletion/update/unaffected, docs-owed, mutation,
  and fail-closed cases.

## Validation evidence

| Validation | Exit | Fresh result |
| --- | ---: | --- |
| `pnpm install --frozen-lockfile` | 0 | Lockfile/install already up to date |
| `pnpm exec mmdc --version` | 0 | Exact `11.16.0` |
| `node --test scripts/docs/check-diagram-impact.test.mjs` | 0 | 16/16 |
| `node scripts/docs/check-diagram-impact.mjs --json` | 0 | 17 documents, 24 fences, 0 findings |
| `pnpm test:docs-check` | 0 | 98/98 plus all shell fixtures |
| `pnpm docs:check` | 0 | 13/13 surfaces |
| `pnpm docs:public:check` | 0 | 98 files, 0 errors |
| `pnpm docs:public:diagrams` | 0 | 4 files, 0 errors, 0 warnings |
| `pnpm docs:owed --since abe6be8b657b8be68565aace3aada6056323ae61 --fail-on-owed` | 0 | Provisional dirty-range result: 0 owed, 0 review, 0 baselined |
| `pnpm docs:index:check` | 0 | 6 files, 0 errors, 0 warnings |
| `pnpm test:ci-classify` | 0 | All classifier fixtures passed |
| `pnpm test:validate-local` | 0 | All local-validation fixtures passed |
| `pnpm test:ci-integration` | 0 | All integration fixtures passed |
| `pnpm --filter @eddacraft/docs-shell test` | 0 | 7 files, 60 tests |
| `pnpm --filter @eddacraft/anvil-docs-private build` | 0 | Production build completed |
| `pnpm --filter @eddacraft/docs-public build` | 0 | Production build completed |
| `pnpm --filter @eddacraft/docs-shell build` | 0 | Production build and proxy entitlement smoke completed |
| `pnpm validate:changed` | 0 | Changed-path validation passed |
| `pnpm format:check` | 0 | 1,686 files |
| `CARGO_TARGET_DIR=/tmp/docrb010-cargo-target pnpm lint:check` | 0 | JavaScript/Nx lint and Rust fmt/clippy passed |
| `pnpm aps:active-lint` | 0 | 148 active plan files clean |
| `pnpm aps:index:check` | 0 | Check passed with inherited advisories |
| `pnpm aps:drift --json` | 0 | No new drift; three inherited warnings |
| `git diff --check` | 0 | Clean |

## Inherited and environmental observations

- The first `pnpm lint:check` attempt exited 1 only because every Rust clippy
  task could not open the shared cache build lock under
  `/home/aneki/.cache/anvil-targets/`: the filesystem was read-only. Oxlint,
  all 31 Nx JavaScript lint targets, and Rust format checks had passed. The
  bounded hermetic rerun above used `/tmp/docrb010-cargo-target` and exited 0
  across JavaScript/Nx lint and 37 Rust clippy/format projects. Nx labelled the
  retried tasks as flaky because of the earlier environmental failures; it
  emitted no product diagnostic.
- The private and public Docusaurus builds reported the inherited deprecated
  `onBrokenMarkdownLinks` option and could not infer one diagram's image
  dimensions. The private build also reported inherited `/anvil/` shell-route
  link warnings. Both builds completed, and mounted-route inspection proved the
  two in-scope pages and diagrams load correctly.
- The docs-shell build retained its TypeScript project-reference warning; the
  build and proxy entitlement smoke passed. The docs-shell tests retained the
  Vite native-config warning. The local-validation fixture retained its
  deliberate invalid-shell line and isolated profile warnings.
- APS index checking retained GTAO and POLFIT stored-progress advisories. APS
  drift retained those two progress warnings and the inherited IMPV-001
  validation-evidence warning. None is caused by this four-path verification
  diff.
- The pre-commit `docs:owed --since` result is explicitly provisional because
  the working-tree postimages are not part of `<base>...HEAD`. It must be
  rerun after the report and planning state are committed and after rebasing.
- During verification, `origin/main` advanced to
  `126cc4e681fbffb95b384fd1eb1ebcb585c84c7b` via PR #4111. The overlap is
  confined to `plans/index.aps.md`; no implementation or evidence source
  overlaps this report.

## Post-rebase exact-head closeout

The single verification commit rebased normally onto exact `origin/main`
`930900af4a88d594a173c0c541809ca4752ff050`. The resulting range remained
exactly the four approved paths, preserved the upstream FLAGCAT index truth,
and was clean and one commit ahead of its merge base.

Fresh post-rebase validation repeated the complete binding matrix. The focused
diagram tests passed 16/16; corpus rendering passed for 17 documents and 24
fences at exact Mermaid 11.16.0; all 98 docs-check Node tests and every shell
fixture passed; all 13 docs surfaces, public checks, public parity, classifier,
local-validator, CI-integration, and generated-index checks passed. Exact
committed-range `docs:owed --since origin/main --fail-on-owed` reported 0
owed, 0 gating, and 0 advisory findings. The private, public, and shell
production builds passed; `validate:changed`, format over 1,689 files, the
full hermetic JS/Nx and Rust lint matrix, APS active/index/drift checks, and
range diff integrity all passed with only the inherited warnings above.

Immediately after the upstream v2 product-catalogue merge, the first
docs-shell test attempt loaded the new `flags/surfaces.json` through stale
pre-rebase `dist` artefacts and failed before three suites. Rebuilding the
merged `@eddacraft/anvil-contracts` and
`@eddacraft/anvil-flags-catalogue` packages changed no tracked file; the
unchanged docs-shell test then passed 7/7 files and 60/60 tests. The shell
production build performs the catalogue prerequisite itself and passed its
proxy-entitlement smoke. This was a local build-order boundary, not a DOCRB
product or documentation defect.

## Boundary decision

DOCRB-010's clean-room acceptance is supported on the pinned starting receipt.
The maintainer, contributor, operator, and public-reader journeys completed; the
enforcement, rendering, parity, accessibility, routing, tests, builds, and
declared validation matrix passed after the bounded environment-only lint
rerun. No residual in-contract documentation gap was found, so this item makes
no repair and creates no tracker.

This is evidence for the pinned receipt, not a merge or completion claim.
Independent verification and Council remain lifecycle gates before publication
or completion.
