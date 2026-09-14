<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Draft module: no work item is executable until the Ready checklist is cleared. -->

# Repository Ship Report

| ID      | Owner       | Priority | Status |
| ------- | ----------- | -------- | ------ |
| SHIPREP | @joshuaboys | high     | Draft  |

**Last reviewed:** 2026-09-14 — created from Elliot beta feedback and Josh's
written follow-up describing a local daily, weekly, or monthly repository ship
report. The source was an operator-provided customer message; no external
transcript or repository-local feedback artefact exists. Its requirements are
reproduced in the V1 User Contract below. This module records the customer
contract and maps it to existing anvil truth owners. It does not authorise
implementation or claim that a command or web dashboard ships today.

> **Origin:** Elliot asked for a team dashboard that answers what landed, who
> did it, whether the delivered change matched what it claimed, and whether the
> result can be trusted. Josh narrowed the first slice to one repository and a
> local command, with Markdown and JSON output. A built-in web dashboard is a
> later projection over the same data.

## Purpose

Give a team lead a deterministic, evidence-graded account of everything that
landed on one repository's default branch during a selected day, week, or
month. The report covers ordinary human work as well as AI-assisted work and
never fills evidence gaps with model inference.

The product question is:

> What landed, who did it, did it match what was claimed, and can I trust it?

This is a provenance and claim-integrity projection, not a generic engineering
metrics dashboard. It composes existing Git, conformance, graph, attribution,
test-impact, gate, review-capsule, and witness facts into one bounded report.

## V1 User Contract

### Report scope

- Run locally against one Git repository.
- Select a preset period: last day, week, or month.
- Read landed work from the repository's resolved default branch.
- Record the exact resolved default-branch tip and whether the local branch may
  be stale; a local-only report never implies that it has fetched remote work.
- Produce one row per pull request when durable local Git evidence identifies
  the pull request; otherwise fall back to one row per commit.
- Cover every landed unit in the period. Human-only, AI-assisted, planless, and
  evidence-poor changes remain visible.

### Summary row

Each row carries:

1. **Landed at** — Git time for the landed unit on the default branch.
2. **Identity** — pull-request number when proven, otherwise commit identity.
3. **Claim** — the declared change type and subject, preserving the source
   text and its provenance.
4. **Said versus shipped** — `match`, `mismatch`, or `unknown`, with reason
   codes and evidence references. Evaluation is deterministic. Natural-language
   intent is never guessed by a model.
5. **Intent link** — an explicitly named plan, ticket, or declaration when one
   exists. Absence does not remove the row or prevent Tier-0 claim checking.
6. **Who** — Git author as the universal fact, plus `human`, `AI`, `mixed`, or
   `unknown` only when an existing attribution source proves it.
7. **Model** — observed model identity when evidence exists; otherwise
   `human`, `unknown`, or `not_applicable` according to the attribution
   contract. The report must not infer a model from author names or tools.
8. **Surfaces touched** — canonical architecture surface names when available;
   otherwise deterministic repository-relative path groups.
9. **Tests** — test files landed in the change plus graph-implied affected
   tests when the graph is available and revision-bound.
10. **Provenance** — gate, review-capsule, and witness evidence observed by
    anvil; otherwise an explicit `none` or `unknown` with a reason.
11. **Completeness** — field-level proof state so a populated row cannot imply
    stronger evidence than the repository actually contains.

### Drill-down

Opening one row exposes the same contract without summary truncation:

- commits and files in the landed unit;
- resolved architecture surfaces or fallback path groups;
- Git authors and any evidence-backed human/AI/model attribution;
- tests landed and graph-implied tests, including graph coverage limits;
- per-claim said-versus-shipped dispositions and reason codes;
- linked plan, ticket, or declaration identifiers when explicit; and
- gate, capsule, review, and witness evidence trails with verification state.

### Export

- Markdown for human reading and sharing.
- Versioned JSON for customer-owned dashboards and automation.
- Human and JSON outputs are projections of one typed report contract; they
  must not recompute facts independently.

## Honesty Contract

- `unknown` is a first-class answer, never a temporary display placeholder.
- Pull-request grouping requires local durable evidence. The V1 command does
  not call GitHub to repair missing metadata.
- Conventional Commit type and subject are displayable Tier-0 claims. A free
  text subject is not semantically judged. Only structured claims with an
  accepted deterministic mapping may receive `match` or `mismatch`; all other
  claim members remain `unknown`.
- A linked plan or ticket enriches the row only when the link is explicit and
  the relevant adapter can verify it. Written plans are not required.
- Git author identity does not prove human or AI authorship. Model identity is
  reported only from evidence recognised by the attribution contract.
- Test filenames are observed facts. Graph-implied tests are separately
  labelled and carry graph revision, generation, coverage, and partiality.
- Missing gate, review, capsule, or witness evidence is shown as missing; it
  never collapses into trusted or clean.
- Gate and witness state may exist only on the machine that produced it. A
  different team lead's clone is expected to show `none` unless evidence was
  committed or exported through GITGOV's portable capsule contract.
- Completeness is field-level as well as row-level. Strong evidence for one
  field cannot upgrade unrelated unknown fields.
- Any range, time, count, graph, or output budget that truncates collection is
  disclosed at report level with its reason and omitted-unit count when known;
  a truncated report never claims every landed unit was included.
- Stable ordering, time-window boundaries, default-branch resolution, and
  report budgets must be pinned before the module becomes Ready.

## In Scope

- One-repository, bounded historical report over the default branch.
- Day, week, and month presets.
- Deterministic PR-or-commit landed-unit reconstruction from local Git facts.
- Typed summary-row, drill-down, evidence-reference, and completeness schemas.
- Tier-0 claim display and deterministic conformance projection through CONF.
- Optional explicit joins to plans, tickets, declarations, authorship/model
  evidence, graph surfaces, affected tests, gates, capsules, and witness lines.
- Markdown and JSON projections over one report model.
- A local CLI command family selected through the CLICT truth workflow.
- Performance and privacy budgets suitable for ordinary local repositories.

## Out of Scope for V1

- Multi-repository or organisation-wide aggregation.
- Live monitoring, push notifications, or scheduled remote reporting.
- GitHub CI pass/fail state or any required network call.
- Inferring features, intent, authorship, or model identity from a diff.
- Requiring a plan, ticket, conventional commit, AI agent, or anvil evidence.
- Hiding ordinary human merges or evidence-poor changes.
- A built-in web dashboard; later dashboard work must consume the same JSON
  contract rather than fork report semantics.
- Generic productivity ranking, developer scoring, velocity surveillance, or
  model-performance league tables.
- Replacing CONF, LAC, GCTX, GITGOV, or `anvil-witness` collectors and stores.

## Interfaces and Ownership

### Depends on

- **CONF / ADR-134 / ADR-135 / ADR-139** — owns canonical deterministic
  claim extraction, evidence binding, per-member conformance dispositions,
  bounded Git range evidence, and atomic report materialisation. SHIPREP
  projects CONF results; it does not create a second claim engine.
- **GITGOV / ADR-072 / ADR-074** — owns deterministic commit/range facts,
  review-capsule collection and verification, and the portable evidence
  contract.
- **`crates/anvil-witness` / ADR-037** — owns witness-chain truth. SHIPREP
  references verified witness evidence and must not create a parallel
  provenance store.
- **GV2 and GCTX** — own graph-backed architecture identities, impact, and
  affected-test evidence. SHIPREP labels graph-derived fields separately and
  degrades honestly when the graph is absent, stale, partial, or capped.
- **LAC** — owns human/AI/mixed attribution and optional model identity. Until
  LAC is respecified and evidence exists, SHIPREP reports Git authors plus
  explicit unknown attribution fields.

### Coordinates with

- **ILGOV** — optional explicit session-intent and plan/ticket correlation;
  not a V1 prerequisite and never required for a row.
- **CLICT** — command naming, registration, help, tests, and public CLI claims.
  SHIPREP owns implementation; CLICT owns the command-truth review.
- **DASH/DASHCORE and later dashboard modules** — possible future web
  projection. They consume the SHIPREP JSON contract and do not redefine it.
- **COMPLY/CEWS** — later organisational evidence consumers. SHIPREP is not a
  compliance certification and does not clear their evidence-semantics gates.
- **FEFF** — may use user-reviewed exported reports as study inputs, subject to
  its consent and privacy contract; SHIPREP does not collect telemetry.

### Exposes

- Candidate `anvil.repo-ship-report.v1` typed report schema, subject to the
  Ready-checklist schema review.
- One typed landed-unit summary and drill-down contract.
- Markdown and JSON renderers over the same report value.
- Field-level evidence/completeness states and stable reason codes.
- A future dashboard adapter seam with no dashboard dependency in V1.

## Candidate Result Shape

This is a requirements sketch, not an accepted implementation schema:

```text
RepoShipReport
  repository, default_branch, source_tip_sha, source_staleness, period, generated_at
  completeness: complete | truncated, truncation_reason, omitted_units
  landed_units[]
    identity: pr | commit
    landed_at
    claim: source, type, subject, explicit_links[]
    conformance: match | mismatch | unknown, members[], evidence[]
    authors: git[], attribution: human | ai | mixed | unknown
    models[]
    surfaces[]
    tests: changed[], implied[], coverage
    provenance: gates[], reviews[], capsules[], witness[]
    completeness: field -> proven | partial | unknown | not_applicable
```

## Open Questions

1. Does Elliot confirm one repository and day/week/month presets as the right
   V1 grain, or is multi-repository reporting required from day one?
2. Is deterministic claimed-versus-landed conformance, including planless
   human changes, the decisive check or is another team-lead question missing?
3. Which local Git shapes qualify as durable PR identity across merge,
   squash-merge, and rebase-merge histories?
4. What timezone and inclusive/exclusive boundary define day/week/month, and
   does "landed at" use committer time or another default-branch event?
5. What is the CLI noun and drill-down selector? Public naming remains open
   until the CLICT slice reviews the live registry.
6. Which existing attribution evidence is reliable enough for V1, given LAC's
   stale implementation paths and the witness `agent_tag` limitations?
7. What maximum range, commit count, Git time, graph time, and output size keep
   the local command bounded and honest?
8. Does Markdown export include evidence identifiers by default, or use a
   concise view with an explicit verbose form?
9. Which gate, witness, capsule, and review evidence is portable to another
   team lead's clone, and which remains machine-local by design?

## Ready Checklist

Change this module to **Ready** only when:

- [ ] Elliot's answers to Open Questions 1 and 2 are recorded, or Josh
      explicitly accepts the written V1 defaults without further validation.
- [ ] CONF owners confirm that SHIPREP consumes the canonical conformance and
      bounded-range contracts rather than forking them.
- [ ] Audit the live CONF implementation and freeze the exact Tier-0 claim
      members that can deterministically resolve to `match` or `mismatch`;
      every unsupported member is explicitly `unknown` in V1.
- [ ] PR-or-commit reconstruction rules and merge-strategy fixtures are frozen.
- [ ] Time-window, default-branch, ordering, and resource-budget contracts are
      explicit and covered by deterministic fixtures.
- [ ] The V1 attribution subset is audited against live evidence; unsupported
      human/AI/model claims are pinned to `unknown`.
- [ ] The typed report schema, completeness lattice, reason codes, and Markdown
      and JSON parity contract are reviewed.
- [ ] Privacy review confirms the local-only default and export minimisation.
- [ ] A CLICT slice records the selected command family before public docs
      claim that it exists.
- [ ] Exact Rust implementation homes, validators, and affected crates replace
      the candidate work-item placeholders below.

## Candidate Delivery Slices

These slices are design input, not executable work items:

1. Freeze landed-unit, merge-strategy, default-branch, and time-window
   semantics with deterministic Git fixtures.
2. Define the typed report, field-completeness lattice, evidence references,
   and stable reason codes.
3. Project CONF-owned Tier-0 and explicitly linked intent conformance without
   natural-language inference.
4. Join Git authorship, LAC attribution, GV2/GCTX surfaces and affected tests,
   GITGOV capsules, and witness evidence without strengthening weak sources.
5. Render Markdown and JSON from the same typed value, including drill-down.
6. Add a bounded, local-only CLI surface after the CLICT naming review.
7. Validate the complete workflow with Elliot or an agreed proxy repository
   before freezing the V1 claim.

## Work Items

None authorised while the module is Draft. After the Ready Checklist is
complete, decompose the accepted delivery slices above into outcome-focused
work items with exact implementation homes and validation commands, then
promote them explicitly.

No claim issue exists while there is no Ready work item; create one only when a
specific promoted item starts, per `plans/project-context.md`.
