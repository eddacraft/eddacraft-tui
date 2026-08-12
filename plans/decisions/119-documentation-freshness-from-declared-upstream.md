# ADR-119: Documentation freshness is decided from declared upstream, gated by granularity, and verified for public docs at the release boundary

## Status

Accepted 2026-08-12 (owner). Reviewed on PR #3732, which corrected the D6
anchor from the release's `previous_tag` to each page's own `verified_against`
before acceptance.

## Date

2026-08-11

## Context

Ten `docs:check` surfaces validate documentation today. Every one of them asks
the same class of question: is this document internally well-formed? Metadata
present (`check-metadata.mjs`), tags known (`check-tags.mjs`), links resolve
(`check-links.mjs`), cited paths exist (`check-asbuilt-paths.mjs`), generated
indexes current (`check-index-freshness.mjs`). None of them asks the question
that actually makes documentation wrong over time: **has the thing this document
describes changed since anyone last checked the document against it?**

The information needed to answer that is already in the corpus and has never
been read. The DOCGOV metadata convention
(`docs/guides/documentation-governance.md`) requires each governed document to
name its `Upstream` sources — defined there as "canonical source(s) this doc must
not contradict" — and a freshness line carrying `Last reviewed YYYY-MM-DD`.
`check-asbuilt-paths.mjs` walks those same references but only asks whether they
*resolve*; an upstream file can be rewritten end to end and still pass.

### Measured state

`scripts/docs/check-docs-owed.mjs` joins the declared upstream paths to
`git log`. **Measured at `24070b867` (2026-08-12); reproduce with
`pnpm docs:owed`:**

```
83 owed, 10 review, 103 checked, 30 uncheckable,
94 without governance metadata, of 228 documents
```

These figures are a reading of a moving corpus, not constants, which is why they
carry a commit anchor: every merge to `main` can move a document between classes.
The first measurement of this corpus, four days earlier, read 84/7 — the totals
drift, the shape does not. Cite the anchor when quoting them, and re-run rather
than trusting the number.

`owed` means the upstream moved and the document has not been committed since —
nobody has looked. `review` means the document was committed after its upstream
moved, so it may already be reconciled with a stale date. The split is heavily
weighted to `owed`, which says this is overwhelmingly untouched documentation
rather than date-keeping noise.

Hand-verified example: `docs/guides/git-hook-compatibility.md` declares
`crates/anvil-cli/src/commands/hooks.rs` upstream and was reviewed 2026-05-25.
That file has taken more than ten behaviour-changing commits since, including
`fix(hooks): honest config-mode doctor/status` and `fix(cli): detect sh-less git
before relying on #!/bin/sh hooks`. A compatibility-policy document is
describing hook behaviour that changed underneath it.

### Why the drift is structural, not a discipline problem

Every heavy step of the `Docs Lint` job is gated on `markdownlint-required`
(`.github/workflows/ci.yml:172-213`), which `scripts/ci/classify-changes.sh:333`
sets only for the `docs` path class. A pull request that rewrites
`crates/anvil-cli/src/commands/hooks.rs` runs **zero** documentation checks. The
trigger sits on the wrong side of the edge: documentation is validated when
documentation changes, and staled when code changes.

This repository has already found and fixed one instance of the class.
`.github/workflows/public-reference-regen.yml:1-22` records how cutting
`v0.9.3-beta` silently staled `docs/public/anvil/reference/cli.md` with no
commit and no diff, surfacing only when an unrelated `.md` pull request
inherited the red check (#3676). That fix covered one generator. The general
case was left open.

### Why a calendar rule is not available

ADR-117 holds that a merge gate may only fail for something a commit did,
assessed per assertion. "Reviewed more than N days ago" violates that outright —
it goes red while everyone sleeps. Upstream-moved does not: the commit that
edits `hooks.rs` is precisely what invalidates the hook compatibility policy, so
the failure is caused by, and fixable inside, the pull request that causes it.

### Why public docs need a different answer

Public documentation is the highest-cost drift surface and currently the least
governed. All 91 files under `docs/public/**` carry no governance metadata at
all: the visible five-column table was deliberately removed because it rendered
badly on the published site. Public docs today have no declared owner, no
declared upstream, and no recorded verification anchor.

They also cannot use the model above unchanged, because **public docs describe
the released product, not `main`**. An as-built document describes the current
tree; `docs/public/anvil/operations/git-hooks.md` describes the behaviour a
customer gets from the installed binary. An upstream commit landing on `main` is
therefore not yet a public-doc defect — it becomes one at the tag cut. That is
exactly the #3676 failure, and it is why a PR-time freshness rule would be both
noisy and wrong for this surface.

Three concrete symptoms of the ungoverned state:

- Version information exists only as prose, so correctness can only be assessed
  by hand, one reference at a time. A manual audit of the 18 version references
  outside the changelog found the corpus mostly sound — the "`0.9.1-beta` and
  later" form used in `overview.md`, `security.md`, `skills.md`, and
  `telemetry.md` is a durable capability claim and stays correct across
  releases — and exactly one defect: `integrations/mcp.md` asserted dual-era MCP
  support for `0.9.1-beta` with no "and later", leaving a reader on `0.9.4-beta`
  unable to tell whether it still applied. That audit is unrepeatable and
  nothing would have caught the defect. The problem is the absence of a
  mechanism, not a mass of stale numbers.
- The governance guide already requires public docs to "cite the release or
  product version they describe". Nothing enforces it, and there is nowhere
  structured to put it — which is why the audit above had to read prose.
- The strongest check in the whole documentation system — `docs:public:commands`,
  which executes every `anvil …` example in the public corpus against a real
  released binary — is pinned to `ANVIL_DOCS_VERSION: 0.9.1-beta`
  (`.github/workflows/ci.yml:191`) while the repository is at `0.9.4-beta`. The
  best evidence available is being gathered against a binary three releases old.

## Decision

### D1 — Freshness is derived from declared upstream and git history

Add a `docs-owed` surface (`scripts/docs/check-docs-owed.mjs`). For each
governed document it reads the `Upstream` references and the `Last reviewed`
date, and reports any upstream path with a commit newer than that date. Bare
invocation reports the corpus backlog; `--since <ref>` scopes to a diff range,
which is the gate shape.

This is a closeout-enforcement check and therefore falls inside the ADR-042
carve-out from ADR-002: it exits non-zero on violation by design. ADR-002
continues to govern runtime warnings on user code, which this is not.

### D2 — Granularity decides what binds

| Upstream shape                                | Posture      |
| --------------------------------------------- | ------------ |
| Resolves to a file (blob)                     | **Gating**   |
| Resolves to a directory, or contains a glob   | **Advisory** |

A file-level upstream is a claim precise enough to act on. A directory-level
upstream fires on any commit anywhere beneath it and cannot be acted on. The
measured distribution makes this concrete: the two highest-frequency triggers in
the corpus are `crates/anvil-cli` (7 findings) and `crates/anvil-checks` (6),
both whole-crate references. Of the 83 owed findings at that same anchor, 47
cite file-level upstreams only, 20 cite directories only, and 16 are mixed.

A mixed document gates on its file-level components and reports the rest. A
directory-only document is never red.

### D3 — Only the high-confidence class gates

`owed` (the document has not been committed since its upstream moved) is
gate-eligible. `review` (the document was committed after, so the date may
simply be stale) is advisory date-hygiene. Measured 83 versus 10 at the anchor above.

### D4 — Same-day counts as reviewed

`Last reviewed` is a date; commits carry timestamps. A same-day upstream commit
is treated as reviewed, so the check deliberately under-reports. A false "go
review this" costs more gate credibility than a missed one.

### D5 — Public-doc governance lives in frontmatter, never in a visible table

Public documents adopt governance fields as YAML frontmatter keys, which
Docusaurus does not render:

```yaml
owner: GHOOK
upstream:
  - crates/anvil-cli/src/commands/hooks.rs
verified_against: 0.9.4-beta
```

The pattern is already proven in this repository: `public_unlisted` is a
non-standard key that survives Docusaurus frontmatter validation, is carried
into the built `frontMatter` object, and is read by
`scripts/docs/check-public-docs.mjs:182`. Governance fields ride the same rails
at zero visual cost. The five-column table is not reintroduced to
`docs/public/**` under any circumstance.

### D6 — Public docs are verified against the release boundary

`verified_against` names the product version a page was last checked against, so
**that** is the diff base — one per page, not one per release. At release
readiness, for each public document:

```
git diff v<verified_against> <candidate_tag> -- <declared upstream paths>
```

A non-empty diff means the page is owed re-verification before the release ships.

Anchoring on the page's own value rather than the release's `previous_tag` is
load-bearing, not a detail. A page verified against `0.9.1-beta` whose upstream
changed in `0.9.2-beta` produces an *empty* `0.9.3-beta..0.9.4-beta` diff and
would be reported clean while remaining stale. Per-page anchoring is also the
only version of this check that expresses what the metadata actually claims: the
alternative — asserting an invariant that every governed page is re-verified
before each release — would force the whole public corpus through re-verification
every cut, which is precisely the untargeted burden this ADR exists to avoid.

Two failure modes must be loud rather than silent, because both would otherwise
resolve to "no diff, therefore clean":

- `verified_against` naming a tag that does not exist (a typo, or a version
  never cut) is an error, not a pass.
- A page with no `verified_against` at all is unverifiable, and counts against
  coverage under D9. It is never treated as verified.

The release's own `previous_tag` input keeps a narrower job: reporting the
release-over-release delta for operators. It is not the correctness anchor.
`.github/workflows/release-readiness.yml` already accepts it, so the workflow
plumbing exists either way.

The consequences of that placement are deliberate:

- Owed public docs block the **release**, never an ordinary pull request. A
  public page becomes wrong when the product ships, so that is where the check
  belongs.
- A pull request that moves a public page's declared upstream gets an advisory
  note that the page will be owed at the next cut. Never red.
- This generalises `public-reference-regen.yml` from one generator to the whole
  public surface. Any artefact whose inputs are frozen by a tag inherits the
  same boundary check instead of needing its own bespoke workflow.

### D7 — Executable verification outranks declared verification

Where a public claim can be executed, executing it is the primary check and the
frontmatter is bookkeeping. `docs:public:commands` is the strongest instrument
in the system and its authority depends entirely on the binary it probes, so
`ANVIL_DOCS_VERSION` must track the current release. Pinning it three releases
behind (D5 context) silently downgrades every public command assertion. Keeping
that pin current becomes a release-checklist item.

The standing preference follows: where a documented surface can be generated
from shipped sources with a `--check` mode, generate it rather than adding
another review obligation. Generated documents cannot drift.

### D8 — Ratchet from a baseline, do not big-bang

The owed findings present at adoption are baselined the way `links` already carries 130
entries in `docs/governance/docs-check.baseline.json`. New violations gate from
day one; the backlog burns down by owner. The surface is `baselineable: true` in
`DEFAULT_SURFACES`.

### D9 — Coverage is reported, always

Only 103 of 228 documents are checkable at all. An uncheckable document is
invisible to this check, not clean, so the summary always prints the
`uncheckable`, `withoutGovernanceMetadata`, and directory-only counts. Coverage
must never be silently traded for a green result.

## Rationale

The decision is really one idea applied twice: **the moment a document becomes
wrong is a specific, observable event, and the check belongs at that event.**
For an internal document, the event is a commit to its upstream on `main`. For a
public document, the event is a tag cut that changes what customers receive.
Putting each check at its own event is what makes both of them ADR-117-legal and
what keeps the public-doc check quiet during ordinary development.

The granularity split (D2) is the difference between a gate that survives and
one that gets switched off. Gating on `crates/anvil-cli` would fire on nearly
every Rust pull request, and a gate that fires on everything teaches people to
bypass it. Gating on `crates/anvil-cli/src/commands/hooks.rs` fires when the
hook compatibility policy genuinely came into question.

### Alternatives Considered

| Option | Pros | Cons |
| --- | --- | --- |
| **Chosen:** declared upstream + git history, granularity-split, public docs at the release boundary | Uses metadata that already exists; commit-caused so ADR-117-legal; version-correct for public docs; noise bounded by measurement | Depends on hand-declared upstream, so coverage is a corpus problem; creates an incentive to declare vaguely |
| Calendar staleness ("review every 90 days") | Trivial to implement; catches documents with no upstream at all | Violates ADR-117 — goes red with no commit; reviews get rubber-stamped to clear the board |
| Gate all upstream granularities equally | Simpler rule; no metadata-authoring burden | Measured to fire on whole-crate references (13 findings from two directory upstreams); the gate would be disabled within a release |
| Content-hash the cited source spans (`path:12-40`) | Far more precise than "the file moved"; line-range syntax already appears in the corpus | Brittle to reformatting and refactors; a rename or a `cargo fmt` invalidates every anchor |
| Couple docs to code symbols through anvil's own graph | Dogfoods the product; survives moves and renames | Requires the daemon in CI; large scope; worth revisiting once graph coverage is broad |
| Apply the PR-time model to public docs too | One mechanism for the whole corpus | Wrong by construction — public docs describe the shipped release, not `main`; would be noisy and would still miss the tag-cut event that caused #3676 |
| Reinstate the visible metadata table on public docs | Consistent with internal docs; one parser for everything | Rejected once already on rendering grounds; frontmatter achieves the same validation invisibly |

## Consequences

- **Positive:** the `Upstream` column stops being decorative and becomes a
  machine-checked dependency graph. Documentation drift is caught by the change
  that causes it rather than discovered by a reader.
- **Positive:** public docs gain an owner, a declared upstream, and a
  verification anchor for the first time, at zero cost to the rendered page.
- **Positive:** the tag-cut blind spot behind #3676 is closed as a class rather
  than per generator.
- **Negative:** a backlog of roughly ninety findings becomes visible and needs owners. Baselining
  defers it; it does not remove it.
- **Negative:** metadata authoring gets stricter. Documents with no file-level
  upstream stay unchecked until someone declares one.
- **Risks:** the gate creates a perverse incentive — declaring a *directory*
  upstream is a way to opt out of gating. The check must report directory-only
  and uncheckable counts per owner (D9) so opting out is visible rather than
  quiet. Widening `asbuilt-paths`-style "must cite at least one path" to Guides
  and Policies limits the crudest form of it.
- **Risks:** documentation review becomes a merge dependency for source changes,
  which can pressure authors to bump the date without reading. `review`-class
  reporting exists partly to make a bumped-but-untouched date visible.
- **Risks:** `git log` per upstream path costs wall-clock on a large corpus.
  Measured acceptable at 228 documents with per-path caching; revisit if the
  corpus or the CI budget changes (`docs/policies/resource-budget.md`).
- **Mitigations:** report-only first (already built), baseline the backlog,
  gate only the `owed` × file-level intersection, and keep the public-doc check
  on the release boundary where its blast radius is one release rather than
  every pull request.

## References

- Related ADRs: ADR-117 (repo-state checks are not per-PR gates), ADR-042
  (closeout-enforcement exit codes as an ADR-002 carve-out), ADR-002 (warnings
  over blocks)
- Prior art in-repo: `.github/workflows/public-reference-regen.yml` (#3676),
  `scripts/docs/check-public-docs.mjs`, `scripts/docs/check-asbuilt-paths.mjs`
- Governance: `docs/guides/documentation-governance.md`
- APS: DOCSYNC (`plans/modules/documentation-sync.aps.md`) owns public content;
  DOCGOV (`plans/archive/modules/documentation-governance.aps.md`, Complete)
  seeded the metadata convention. Execution of this ADR needs a new module.
- Prototype: `scripts/docs/check-docs-owed.mjs`
