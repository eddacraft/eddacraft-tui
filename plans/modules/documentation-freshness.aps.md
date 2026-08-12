<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Documentation Freshness

| ID        | Owner | Priority | Status | Progress |
| --------- | ----- | -------- | ------ | -------- |
| DOCFRESH  | —     | high     | In Progress | 0/8 |

**Last reviewed:** 2026-08-12 — DOCFRESH-004 **In Progress**. Coverage moves
from **104 to 119** checkable documents (of 228). Composition of the gap was
mis-stated in the item and is corrected there: no document was missing a review
date. What actually blocked coverage was `Upstream` cells carrying prose or
markdown-link labels instead of the backticked repository paths the parser
recognises — `[label](../../plans/x.md)` is invisible to it.

Two real defects fell out of widening `check-asbuilt-paths.mjs` to Guide, Policy
and Spec: three documents still cite `plans/modules/documentation-governance.aps.md`,
which moved under `plans/archive/` when DOCGOV was archived, and two PocketFlow
references claim `packages/kindling-adapter-pocketflow/`, which has never existed
in this tree. Both corrected.

**Limitation found by dogfooding, and it will bite -005 harder.** The gate
treats *any* commit to a declared upstream as invalidating, including a change
to that upstream's own governance metadata. This pull request tripped its own
gate that way: it added `docs/architecture/dev-acceleration-benchmark-spec.md`
as a declared upstream of the DEVACC evidence note and, in the same change,
edited only that spec's Upstream cell. Nothing the evidence note depends on
moved, but the check cannot tell a metadata edit from a substantive one.
Resolved here by verifying and re-dating the one affected document.

DOCFRESH-005 edits frontmatter on 91 public documents, so the same effect will
cascade much wider there. Worth deciding before -005 starts whether the check
should ignore commits that touch only an upstream's metadata block — noting the
"metadata-only" test is itself fragile, which is why it is recorded rather than
implemented on the fly here.

**Coverage has a real ceiling.** Sixteen documents — vision, strategy, internal
briefs, third-party references — have no in-repo upstream that could be cited.
They are baselined rather than given invented paths, so they stay countable as
known gaps instead of being dressed up as covered. `docs-owed` known debt rises
62 → 67 for the same reason: more documents visible means more staleness
visible, not more staleness.

**Earlier — 2026-08-12** — DOCFRESH-003 **Merged via PR #3800**: the trigger moved
off `markdownlint-required` onto `code-changed`, which is the change that
actually closes the structural gap. `source-changed` was the obvious candidate
and is wrong — it resolves to format|lint|typecheck|unit-tests, which a
Rust-only or workflow-only pull request never sets, and Rust-only is the
motivating case. Diff-scoped with `--fail-on-owed`, so it can fail only for
unbaselined gate-eligible findings that the commit itself caused (ADR-117 plus
D2/D8).

Dogfood note: this change edits `.github/workflows/ci.yml`, a declared upstream
of `docs/guides/testing.md`, so the new gate fires on the very pull request that
introduces it. Resolved the way the gate intends — the guide was read against the
change (it documents test conventions rather than CI job structure, so no
content change was needed) and its review date bumped.

**Earlier — 2026-08-12** — DOCFRESH-001 **Merged via PR #3787**;
DOCFRESH-002 **Merged via PR #3795**. The D2 granularity split landed: severity is
the AND of confidence and granularity, so only an `owed` finding backed by a
file-level upstream can reach ERROR. Glob upstreams are classified rather than
discarded — the previous behaviour silently dropped the `scripts/release/*`
declaration on `docs/runbooks/release-runbook.md`, making a real dependency
invisible. Measured at that split: 83 owed = **62 gating + 21 advisory by
granularity**, plus 10 review. The baseline narrows from 83 entries to 62,
because only gate-eligible findings need absorbing.

Per-item counts are left to the periodic reconcile (ADR-053); this change does
not bump the module header or the index `N/M`.

**Earlier — 2026-08-12** — ADR-119 **Accepted**, so the execution authority this
module was waiting on now exists. The Draft hold recorded on 2026-08-11 is
lifted: -002/-003/-004/-005/-007 promoted to **Ready**. DOCFRESH-006 stays Draft
behind its -005 dependency, and DOCFRESH-008 stays Draft because it needs its
own decision rather than execution.

**Earlier — 2026-08-11** — Module created to execute
[ADR-119](../decisions/119-documentation-freshness-from-declared-upstream.md).
Every item stayed Draft until the ADR was Accepted; the ADR is the execution
authority for the gating posture, and promoting items before it landed would
have authorised a merge gate nobody had agreed to.

## Purpose

Documentation drift in this repository is structural, not a discipline problem.
Every heavy step of the `Docs Lint` job is gated on `markdownlint-required`
(`.github/workflows/ci.yml:172-213`), which `scripts/ci/classify-changes.sh:333`
sets only for the `docs` path class, so a pull request that rewrites Rust
sources runs **zero** documentation checks. Documentation is validated when
documentation changes, and staled when code changes.

Measured by `pnpm docs:owed` at `24070b867` (2026-08-12) — a reading of a
moving corpus, so re-run rather than trusting the figures:

```
83 owed, 10 review, 103 checked, 30 uncheckable,
94 without governance metadata, of 228 documents
```

The information needed to detect this is already declared in the corpus and has
never been read: each governed document names its `Upstream` sources and a
`Last reviewed YYYY-MM-DD` date. `check-asbuilt-paths.mjs` walks the same
references but only asks whether they resolve, so an upstream file can be
rewritten end to end and still pass.

This module makes the declaration machine-checked, and gives public docs — the
highest-cost drift surface, currently carrying no governance metadata at all —
a verification model appropriate to the fact that they describe the shipped
release rather than `main`.

## In Scope

- A `docs-owed` `docs:check` surface deriving freshness from declared upstream
  paths versus git history.
- The ADR-119 granularity split: file-level upstreams gate, directory and glob
  upstreams are advisory.
- Moving the trigger for documentation validation from "docs changed" to
  "a declared upstream changed".
- Governance metadata for `docs/public/**` as non-rendered YAML frontmatter,
  and release-boundary verification driven from it.
- Backfilling review dates and upstream paths so the checkable corpus grows.
- Keeping the `ANVIL_DOCS_VERSION` probe pin current, because executable
  verification outranks declared verification (ADR-119 D7).

## Out of Scope

- **Calendar-based staleness.** ADR-117 forbids it on a merge gate. A scheduled
  `Repo Health` report is the correct home and is not this module's work.
- **Reinstating a visible metadata table on `docs/public/**`.** It was removed
  on rendering grounds and does not come back; frontmatter achieves the same
  validation invisibly.
- **Editing the substance of stale documents.** This module makes drift visible
  and assigns it; the owning module fixes its own content.
- **MDGOV M2 stale-claim detection.** MDGOV is a *product* capability — Anvil
  checking a customer's markdown as a governance surface. DOCFRESH is this
  repository's own tooling. They share vocabulary, not code.
- **Rewriting `docs:check` surface orchestration.** `docs-owed` conforms to the
  existing contract in `scripts/docs/lib/surface-delegate.mjs`.

## Interfaces

- `scripts/docs/check-docs-owed.mjs` — the surface (prototype exists)
- `scripts/docs/docs-check.mjs` — `DEFAULT_SURFACES` registration
- `scripts/docs/check-public-docs.mjs` — public frontmatter validation
- `scripts/ci/classify-changes.sh` — the trigger change
- `.github/workflows/release-readiness.yml` — release-boundary check
- `docs/governance/docs-check.baseline.json` — the ratchet
- `docs/guides/documentation-governance.md` — the convention being enforced

## Work Items

### DOCFRESH-001: Promote the docs-owed prototype to a governed surface — Merged

- **Intent:** Make the existing report-only probe a first-class `docs:check`
  surface so its output is trustworthy before anything depends on it.
- **Expected Outcome:** `docs-owed` is registered in `DEFAULT_SURFACES` as
  `baselineable: true`, honours `--json`, `--root`, `--baseline` and
  `--no-baseline` like its siblings, exits 2 only when it could not run
  (CIB-278 taxonomy), and is covered by fixture tests in the style of
  `scripts/docs/docs-check.test.sh`. It reports and does not gate.
- **Scope:** `scripts/docs/check-docs-owed.mjs`, `scripts/docs/docs-check.mjs`,
  `scripts/docs/docs-check.test.sh`, `package.json`
- **Non-scope:** Any CI trigger change; any gating behaviour
- **Validation:** `pnpm docs:check` reports 11/11 surfaces passed
- **Confidence:** high
- **Status:** Merged 2026-08-12 via PR #3787

### DOCFRESH-002: Split findings by upstream granularity — Merged

- **Intent:** Ensure only claims precise enough to act on can ever turn a check
  red, per ADR-119 D2.
- **Expected Outcome:** Upstream references resolving to a blob are gate-eligible;
  those resolving to a tree, or containing a glob, are advisory and can never
  raise an ERROR. Mixed documents gate on their file-level components only. A
  fixture proves a directory-only upstream stays advisory no matter how far the
  review date has slipped.
- **Scope:** `scripts/docs/check-docs-owed.mjs`, fixture tests
- **Non-scope:** Changing any document's declared upstream
- **Validation:** `bash scripts/docs/docs-check.test.sh` — includes a
  directory-upstream fixture asserting WARN, never ERROR
- **Confidence:** high
- **Status:** Merged 2026-08-12 via PR #3795

### DOCFRESH-003: Move the trigger from docs-changed to upstream-changed — Merged

- **Intent:** Close the structural gap — a code change that stales a document
  must run the check that notices.
- **Expected Outcome:** `docs-owed` runs in CI when source paths change, not
  only when `markdownlint-required` is set, using `--since` against the merge
  base so it reports what *this* change owes. The findings present at adoption are
  seeded into `docs/governance/docs-check.baseline.json` so only new violations
  surface. A Rust-only pull request that moves a declared file-level upstream
  produces a finding; one that does not, stays silent.
- **Scope:** `.github/workflows/ci.yml`, `scripts/ci/classify-changes.sh`,
  `scripts/ci/classify-changes.test.sh`, `docs/governance/docs-check.baseline.json`
- **Non-scope:** Making the check a required merge context
- **Dependencies:** DOCFRESH-001, DOCFRESH-002
- **Validation:** `pnpm test:ci-classify` plus a dry run of
  `node scripts/docs/check-docs-owed.mjs --since origin/main`
- **Confidence:** medium
- **Status:** Merged 2026-08-12 via PR #3800

### DOCFRESH-004: Grow the checkable corpus — In Progress

- **Intent:** Coverage is the ceiling on this whole model; 125 of 228 documents
  are currently invisible to it.
- **Expected Outcome:** Documents carrying governance metadata but no resolvable
  upstream path are backfilled, and the "must cite at least one source path"
  rule in `check-asbuilt-paths.mjs` widens from As-built/Runbook to Guide,
  Policy, and Spec. The `uncheckable` count in the `docs-owed` summary falls and
  the reduction is recorded.
- **Correction (2026-08-12):** this item was written against a wrong diagnosis.
  It claimed 30 documents lacked "a review date or a resolvable upstream path";
  the audit found **zero** missing review dates. The real composition was 3
  READMEs with no governance table at all, 27 whose `Upstream` cell held prose
  or markdown-link labels rather than the backticked repository paths the parser
  recognises, and 2 citing a package that has never existed in this tree.
  Delivered against the measured composition rather than the assumed one.
- **Scope:** `docs/**/*.md` metadata tables, `scripts/docs/check-asbuilt-paths.mjs`
- **Non-scope:** `docs/public/**` (DOCFRESH-005 owns that surface); document
  content
- **Dependencies:** DOCFRESH-001
- **Validation:** `pnpm docs:check` green, and
  `node scripts/docs/check-docs-owed.mjs --json` shows a lower `uncheckable`
- **Confidence:** medium
- **Status:** In Progress

### DOCFRESH-005: Public-doc governance in frontmatter — Ready

- **Intent:** Give the 91 public documents an owner, a declared upstream, and a
  verification anchor without putting anything on the rendered page.
- **Expected Outcome:** `docs/public/**` pages carry `owner`, `upstream`, and
  `verified_against` YAML frontmatter keys, validated by
  `check-public-docs.mjs` and invisible to readers — the pattern
  `public_unlisted` already proves end to end (`check-public-docs.mjs:182`).
  The visible five-column table is not reintroduced.
- **Scope:** `docs/public/**/*.md`, `scripts/docs/check-public-docs.mjs`
- **Non-scope:** Docusaurus theme or sidebar changes; the internal metadata table
- **Dependencies:** DOCFRESH-001
- **Validation:** `pnpm docs:public:check` and a docs-site build
- **Confidence:** medium
- **Status:** Ready

### DOCFRESH-006: Verify public docs at the release boundary — Draft

- **Intent:** Public docs describe the shipped release, so check them when the
  release changes rather than on every pull request.
- **Expected Outcome:** Release readiness runs
  `git diff v<verified_against> <candidate> -- <declared upstream>` per public
  document — anchored on each page's own last-verified version, **not** the
  release's `previous_tag`, which reports a page clean whenever its upstream
  moved during a release that page skipped — and reports pages owed
  re-verification, blocking the **release** rather than any pull request. A
  `verified_against` that is missing, or names a tag that does not exist, is an
  error rather than a pass. A pull request that moves a public page's upstream
  gets an advisory note only. `public-reference-regen.yml` becomes one case of
  this check rather than a bespoke workflow.
- **Scope:** `.github/workflows/release-readiness.yml`,
  `scripts/ci/release-readiness-workflow.test.sh`, new checker script
- **Non-scope:** Retiring `public-reference-regen.yml` (follow-up once the
  general check is proven)
- **Dependencies:** DOCFRESH-005
- **Validation:** `pnpm test:release-readiness-workflow`
- **Confidence:** medium
- **Status:** Draft

### DOCFRESH-007: Keep the public command-probe pin current — Ready

- **Intent:** `docs:public:commands` is the strongest instrument in the system
  and its authority is only as good as the binary it probes.
- **Expected Outcome:** `ANVIL_DOCS_VERSION` (`.github/workflows/ci.yml:191`)
  tracks the newest version in `docs/public/anvil/releases/changelog.md`,
  enforced by a check rather than remembered. It currently pins `0.9.1-beta`
  against a `0.9.4-beta` repository, so every public command assertion is being
  validated three releases behind.
- **Scope:** `.github/workflows/ci.yml`, release checklist, new assertion
- **Non-scope:** Changing what `docs:public:commands` probes
- **Validation:** the new assertion fails when the pin and the newest changelog
  heading disagree
- **Confidence:** high
- **Status:** Ready

### DOCFRESH-008: Decide the model for out-of-tree public sections — Draft

- **Intent:** ADR-119 D6 assumes a document's upstream lives in this repository.
  For three public sections it does not, and that hole needs a decision rather
  than a silent gap.
- **Expected Outcome:** A recorded decision covering `docs/public/kindling/**`,
  `docs/public/aps/**`, and `docs/public/edda-stack/**`, whose sources are not
  in this tree (no `kindling-*` or `aps` crates exist here), so
  `git diff <tag>..<tag>` cannot verify them. Concrete symptoms today:
  `docs/public/kindling/reference/crates.md:11` claims workspace **0.2.0** and
  `docs/public/aps/getting-started.md:55` claims `aps 0.6.0`, and neither can be
  checked from this repository. Options to weigh: an external-upstream
  declaration with a manual `verified_against` attestation, a cross-repo check,
  or explicit exclusion with a named owner. Whatever is chosen must make the
  exclusion visible in the coverage counts, not silent.
- **Scope:** ADR follow-up, `docs/public/kindling/**`, `docs/public/aps/**`,
  `docs/public/edda-stack/**`
- **Non-scope:** Changing version claims in those sections without a source of
  truth to check them against
- **Dependencies:** DOCFRESH-005
- **Validation:** decision recorded in `plans/decisions/` and indexed in
  `DECISION-LOG.md`; `pnpm adr:check` clean
- **Confidence:** low
- **Status:** Draft

## Acceptance Criteria

- A source-only pull request that moves a declared file-level upstream produces
  a `docs-owed` finding; one that moves only a directory-level upstream does not.
- No `docs-owed` finding can turn a merge gate red for anything a commit did not
  do (ADR-117).
- Every public document declares an owner, an upstream, and the product version
  it was verified against.
- A release cannot be declared ready while a public page's declared upstream
  changed between **that page's own `verified_against` tag** and the candidate
  without re-verification. The release's `previous_tag` is explicitly not the
  anchor: a page that skipped the release in which its upstream changed would
  produce an empty previous-tag diff and pass while stale (ADR-119 D6).
- A public page whose `verified_against` is missing, or names a tag that does
  not exist, fails the release check rather than passing it.
- The `uncheckable` and directory-only counts are reported on every run, so
  coverage cannot be silently traded for a green result.

## References

- [ADR-119](../decisions/119-documentation-freshness-from-declared-upstream.md)
  — execution authority for this module
- [ADR-117](../decisions/117-repo-state-checks-are-not-per-pr-gates.md) — why
  the trigger is commit-caused rather than calendar-based
- [ADR-042](../decisions/042-closeout-enforcement-exit-codes.md) — the ADR-002
  carve-out that lets a closeout check exit non-zero
- `docs/guides/documentation-governance.md` — the convention being enforced
- DOCSYNC (`documentation-sync.aps.md`) owns public *content*; DOCFRESH owns
  whether that content is known to be current
- DOCGOV (`../archive/modules/documentation-governance.aps.md`, Complete) seeded
  the metadata convention this module makes executable
- `.github/workflows/public-reference-regen.yml` (#3676) — the one-generator
  precedent DOCFRESH-006 generalises
