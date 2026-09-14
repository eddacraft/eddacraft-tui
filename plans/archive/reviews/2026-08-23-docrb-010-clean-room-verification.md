# DOCRB-010 clean-room verification evidence

**Date:** 2026-08-24
**Item:** DOCRB-010
**Pinned starting receipt:** `abe6be8b657b8be68565aace3aada6056323ae61`
**Exact rebase base:** `7383f5c2490836281adc3286b2a267de8b4211e4`
**Decision:** Supported with three separately authorised residuals after the clean-room replay and post-rebase validation; independent verification and Council re-review remain lifecycle gates.

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

The missing literal clean-status receipt was repaired with a bounded replay in
a disposable local clone at exact post-rebase planning receipt
`5f4a783cca5a49dae19a651887499bea92f10560`:

- `git status --porcelain=v1` was empty before installation and after the
  replay;
- no `packages/anvil/**/dist` directory existed before validation;
- `pnpm install --frozen-lockfile` exited 0 after an initial sandbox-only
  `EROFS` project-registration failure was rerun with the required writable
  pnpm store boundary; and
- `pnpm exec nx test docs-shell` built policy, contracts, core, runtime, and
  flags-catalogue before passing 7/7 files and 60/60 tests.

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

The public-diagram semantic trace also matched current contracts:

- The anvil CREATE node is the page's guarded temporary-file creation; DETECT
  invokes `anvil check` through
  `crates/anvil-cli/src/commands/check.rs` and the compiled pattern registry;
  FIX applies the page's typed replacement; VERIFY repeats the same check and
  requires the explicit clean result. The three arrows are the tutorial's
  ordered transitions.
- The APS LINT, NEXT, START, IMPLEMENT, VALIDATE, and COMPLETE nodes match
  `plans/aps-rules.md`, `docs/public/aps/workflow.md`, and the adjacent
  getting-started lifecycle prose. The forward arrows match the canonical
  `Ready -> In Progress -> Complete` sequence, while the return arrow matches
  the documented continuation with `aps lint` and `aps next`.

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
| `pnpm exec nx test docs-shell` | 0 | Built five dependencies; 7 files, 60 tests |
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

- Fresh `pnpm docs:check` emitted 255 baselined warnings: two malformed tags
  in an archived APS module and 253 link findings (224 links and 29 anchors).
  Of those link findings, 117 are route-shaped links under live Kindling or
  edda-stack public content. A mechanical comparison against the built public
  site found all 117 corresponding HTML routes; these are repository-file
  checker false positives, not broken mounted navigation. The remaining
  baselined findings predate and are unchanged by this four-path range.

- The first `pnpm lint:check` attempt exited 1 only because every Rust clippy
  task could not open the shared cache build lock under
  `/home/aneki/.cache/anvil-targets/`: the filesystem was read-only. Oxlint,
  all 31 Nx JavaScript lint targets, and Rust format checks had passed. The
  bounded hermetic rerun above used `/tmp/docrb010-cargo-target` and exited 0
  across JavaScript/Nx lint and 37 Rust clippy/format projects. Nx labelled the
  retried tasks as flaky because of the earlier environmental failures; it
  emitted no product diagnostic.
- The private and public Docusaurus builds reported the inherited deprecated
  `onBrokenMarkdownLinks` option. Docusaurus still accepted the option and
  both builds completed; this is dependency/configuration maintenance, not a
  current navigation or output failure. The image-dimension warning is also
  non-behavioural: both SVGs carry explicit dimensions and mounted Chromium
  reported the expected natural and responsive sizes. The private `/anvil/`
  warnings are cross-application shell routes rather than missing private-app
  files; the mounted shell returned HTTP 200 for the inspected route.
- The docs-shell TypeScript project-reference warning did not prevent
  compilation, static generation, or the proxy-entitlement smoke. The Vite
  warning describes a future native config-loader incompatibility; the current
  test mode loaded the config and passed 60/60. The local-validation invalid
  shell line and isolated profile warnings are deliberate fixture evidence.
  None represents a current DOCRB navigation, accuracy, accessibility, or
  enforcement failure.
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

The verification branch rebased normally onto exact `origin/main`
`7383f5c2490836281adc3286b2a267de8b4211e4`. The resulting range remained
exactly the four approved paths, preserved the upstream FLAGCAT index truth,
and was clean relative to its merge base.

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

Immediately after the upstream v2 product-catalogue merge, the first raw
docs-shell package test loaded the new `flags/surfaces.json` through stale
pre-rebase `dist` artefacts and failed before three suites. That exposed a
validation-recipe defect: the raw package script does not build its workspace
dependencies. The action plan now uses the dependency-aware Nx target. The
clean-clone replay began with no generated `dist`, built all five
dependencies, and passed 7/7 files and 60/60 tests. No tracked file changed.

## Separately authorised residuals

The operator agreed on 2026-08-24 that these findings remain outside the
four-path verification item and authorised their independent follow-up:

- #4114 — reconcile ADR-123 after the deleted `apps/docs-site` rollback host
  and topology-side ownership closeout;
- #4115 — retain governed `infra/**` upstreams in documentation metadata and
  diagram-impact enforcement; and
- #4116 — preserve both endpoints of a Git rename in checker, local, and CI
  diagram-impact classification.

Each issue is open with `kind:bug` and `readiness:needs-triage`. DOCRB-010
does not repair, close, or silently absorb them.

## Boundary decision

DOCRB-010's clean-room acceptance is supported with the three authorised
residuals above.
The maintainer, contributor, operator, and public-reader journeys completed; the
enforcement, rendering, parity, accessibility, routing, tests, builds, and
declared validation matrix passed after the bounded environment-only lint
rerun and the clean-clone dependency-aware docs-shell replay. The warning
baseline is dispositioned above; #4114, #4115, and #4116 preserve the three
real residuals without expanding this item into repair work.

This is evidence for the pinned receipt, not a merge or completion claim.
Independent verification and Council remain lifecycle gates before publication
or completion.
