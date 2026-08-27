# ADR-133: Field-effectiveness evidence, local processing, and reviewed manual export

## Status

Accepted

## Date

2026-08-27

## Context

Four of the index's post-release success criteria — save-time adoption, no
increase in AI-assisted time-to-merge, falling new cross-boundary edges, and
warnings not left suppressed without resolution — are unverified. Anvil has
synthetic acceleration benchmarks, anonymous fleet counters, authenticated beta
activity, and local value summaries, but nothing that tests whether the product
reduces unsafe drift in ordinary development without slowing merges.

The `field-effectiveness` (FEFF) module proposes a simple before/after
observational study: a retrospective Git/GitHub baseline replayed locally, plus
a short prospective anvil-active period. That programme touches participants'
private repositories and their developers' working behaviour, so it cannot
start on informal agreement. Several forces pull against each other:

- **Evidence value** wants rich, joinable, per-event data.
- **Participant risk** — private source, repository identity, individual
  developer behaviour — wants the least data that can answer the question.
- **Claim safety** wants a mechanical barrier between a favourable directional
  result and a marketing claim the design cannot support.
- **Existing privacy posture** (`DO_NOT_TRACK`, `ANVIL_USAGE_DISABLE`,
  `ANVIL_INTERCEPT_DISABLE_OBSERVATION`, ADR-089's local-only false-positive
  reports) already promises users that anvil does not phone home. A study must
  not become the exception that quietly breaks that promise.
- **Reversibility** is poor: once repository-identifiable evidence leaves a
  participant machine, consent cannot be un-given.

A decision is needed now because FEFF-003 through FEFF-007 build collection and
export machinery. Deciding the boundary after the machinery exists means the
machinery decides.

This ADR does not change anvil's default telemetry posture, does not make a
release claim, and does not authorise any collection that is not part of an
explicitly joined study.

## Decision

The following are frozen for the first field-effectiveness study and for every
study run under this ADR until it is superseded.

### D-1 — Study design is observational, never causal

The first study is a before/after observational comparison, not an experiment.
No randomisation, no control arm, no counterfactual. Any output — report,
website copy, release note, or slide — may state direction and magnitude with
its denominators and uncertainty, and may never state or imply that anvil
*caused* the change. "Because of anvil", "anvil reduced", and "prevented" are
forbidden framings for observational output.

### D-2 — Frozen windows and adaptive ladder

The retrospective window, prospective window, selection cadence, anvil
version/SHA, rule catalogue digest, architecture configuration digest, metric
dictionary, activity unit, exclusion rules, and the 3:1 exposure limit are
frozen in the study manifest before any outcome is visible.

Prospective default: 10 working days, with a pre-registered maximum of 20
working days. The extension rule is pre-registered: it may fire only on an
unmet **activity** gate, never after inspecting outcomes. A repository still
below the activity floor at the maximum stays descriptive-only.

The cohort/claims ladder (exploratory → multi-repository corroboration →
broader corroboration) is as tabled in the module. Achieved cohort is an input,
not a success condition; a smaller cohort narrows permitted interpretation
rather than failing the study.

### D-3 — Recruitment, stopping, and attrition are pre-registered

Recruitment channels, invitees, independence rules, inclusion/exclusion
criteria, and the enrolled cohort are frozen in the study register before
prospective collection. Default recruitment window is 20 working days, closing
at its end or at five consented repositories meeting baseline-only provisional
criteria, whichever comes first. Stopping is outcome-independent. Active-period
eligibility is assessed only at analysis and can never reopen or extend
recruitment. The invite-to-disposition funnel — invited, declined, enrolled,
withdrawn, capture-failed, ineligible, descriptive-only — is reported in full,
with attrition and sensitivity analyses rather than complete cases alone.

### D-4 — All processing is local; export is manual and user-reviewed

Collection, replay, and aggregation happen on the participant machine. There is
no automatic upload, no background transmission, and no networked collector.
Nothing leaves the machine until a person has reviewed the exact export payload
and explicitly approved it.

Review operates on an **immutable, canonically serialised** payload. The
participant sees that payload and its digest. Approval is a **separate
envelope** binding the payload digest, schema version, study-and-recipient
pseudonym, purpose, recipient, bundle ID, nonce, and initial-transfer expiry,
using a participant-controlled signature or another authenticated mechanism.
Adding the envelope must not alter the reviewed payload. The recipient verifies
before expiry and issues an authenticated acceptance receipt binding payload
digest, approval envelope, recipient, nonce, and acceptance time. After timely
acceptance the triple remains verifiable as durable history; transfer expiry
does not invalidate archived evidence.

### D-5 — Closed export allowlist

The export schema is a **recursively closed allowlist**. Unknown, misspelled,
nested, aliased, free-form, and extension-map fields fail validation at every
nesting level. Adding a field requires a schema version bump plus renewed ADR
and consent review — a new field may never ride along on an existing consent.

The payload may contain only: the study-and-recipient-specific pseudonym;
bucketed period and activity values; recipient-specific unlinkable commitments
where configuration integrity must be demonstrated; metric definitions;
cohort/evidence class; activity denominators; coverage and missing-data
reasons; aggregate results; the payload manifest, canonical digest, and
verifier outcome; and retention/deletion dates.

The payload must never contain source text, diffs, raw or hashed file paths,
repository names or URLs, exact dates, small cells that create avoidable
linkage, stable cross-recipient fingerprints, commit or PR titles, authors,
emails, hostnames, command arguments, tokens, raw event rows, or reversible
commit identifiers.

### D-6 — Disclosure-risk model

Aggregation is not privacy. Before export or publication, a disclosure-risk
review considers what the recipient already knows and whether the aggregate
links to a candidate repository. Any repository-specific result that remains
identifiable after bucketing and small-cell control requires explicit separate
consent to publish. Pseudonyms are derived per study **and** per recipient so
two recipients cannot join their bundles.

### D-7 — Existing privacy controls outrank study membership

Joining a study never implies an override of an existing control. Precedence,
highest first:

1. `DO_NOT_TRACK=1` — superset hard-off for local study collection.
2. `ANVIL_INTERCEPT_DISABLE_OBSERVATION=1` — whole-observation break-glass.
3. `ANVIL_USAGE_DISABLE=1` — usage-surface off.
4. Study consent — permits only what this ADR's allowlist describes.

The study surface checks the applicable controls **before every participant
source read and before every ledger write**, and fails closed. If a control
changes mid-study, new collection stops immediately; resuming requires a fresh
explicit participant action, never an inferred one. Any exception needs its own
ADR-authorised consent and cannot be inferred from joining a study.

Note the current implementation contract: these variables are honoured only at
the exact value `1` (`crates/anvil-cli/src/usage.rs`, `usage_collection_disabled`).
The study surface must treat **any non-empty value** of `DO_NOT_TRACK` as a
hard-off, because the wider convention is presence-based and a participant who
sets `DO_NOT_TRACK=true` has plainly declined.

### D-8 — Consent, withdrawal, retention, deletion

Consent is informed, written, per repository, and names the exact metrics,
retention, recipient, and permitted claims. Withdrawal is unilateral, needs no
reason, and controls **future use** of that participant's data; on withdrawal
only the minimal consented accountability record may remain.

Local receipts may retain what is needed to reproduce a bundle, subject to the
protocol's retention rules, and are never exported by default. They live in a
user-scoped, **non-repository** state root with owner-only permissions or
platform ACLs, no-follow create-new and atomic writes, strict path containment,
bounded age and size, and redacted errors. Deletion covers raw receipts,
caches, temporary trees, and exports; any surviving deletion receipt is
explicitly data-minimised and unlinkable. Exported bundles carry their own
retention and deletion dates.

### D-9 — Git and GitHub authorisation

GitHub access is pinned independently to the consented repository and to the
consented GitHub/GHES host, uses read-only least-privilege credentials, and
issues only allowlisted queries against allowlisted endpoints. A remote URL
found in the repository is never trusted for credential routing. Errors are
redacted. Caches, retention, rate-limit handling, purge, and offline-input
behaviour are bounded. Absent authorisation, the study runs Git-only and
records the reduced coverage rather than substituting a proxy.

### D-10 — Replay isolation

Historical trees are materialised only through sanitised Git configuration or
non-checkout plumbing, with hooks, external filters, submodules, LFS,
credential helpers, and repository-defined executables disabled. Dependencies
are never installed and participant code is never executed. Only the pinned
absolute analyser runs, inside a no-network, credential-stripped boundary with
a temporary state root and bounded process, time, memory, and disk budgets,
plus symlink and path-containment checks. The participant's working checkout is
never switched, reset, cleaned, or written to. Teardown is safe under
interruption.

**Composite gate commands are not analysers.** Any surface that shells out to
package managers, linters, test runners, or other repository-declared tooling
executes participant code by definition and is barred from replay, whatever its
name. FEFF-002 records which concrete anvil surfaces satisfy this clause.

### D-11 — Coverage and activity floors, and missing data

Default comparative coverage floors are 90% of selected transitions in each
period and 90% of prospective eligible developer-days for daily-capture
metrics. These may be replaced only before enrolment. A metric below its floor
is descriptive-only and carries worst-case or sensitivity bounds; missingness
is never silently dropped.

A configuration or analyser failure at a snapshot is **missing data**. It is
never recorded as zero drift, and a skipped check is never recorded as a pass.
Where a surface cannot distinguish "not measured" from "measured zero", the
runner must establish the distinction independently before consuming the value.

Both periods must clear the frozen activity floor; a repository below either
floor is descriptive-only. Drift counts are reported both absolutely and
normalised by the same frozen eligible activity unit.

### D-12 — Metric definitions and the AI-assisted rule

Metric definitions, sources, and honest claim boundaries are as tabled in the
module's "Metric and evidence boundaries" section, which this ADR adopts by
reference and freezes.

An AI-assisted PR subset is reported **only** where the repository already
carries a reliable, pre-existing label or authoritative metadata. Commit
authorship, message style, diff shape, prose register, and model-like phrasing
must never be used to infer AI assistance.

### D-13 — Claims allowlist and criterion completion

The verifier emits a claims allowlist. A claim that is not on it cannot be
made. No index success criterion may be checked complete without its exact
denominator and observation horizon. Specifically, the default first study
cannot complete the every-save adoption criterion, the authoritative
AI-assisted throughput criterion, the eight-week drift criterion, or the
comparable historical warning criterion. Directional and exploratory evidence
stays labelled as such and can never silently check a criterion complete.

### D-14 — Fail-closed verification

The verifier rejects, rather than downgrades: forbidden fields; unknown fields
at any nesting level; schema, digest, or approval-envelope mismatch; changed
definitions or configuration between periods; selectively extended periods;
missing approval or acceptance; invalid initial-transfer freshness; nonce
reuse; wrong-recipient or wrong-purpose replay; coverage below the
metric-specific floor; absent denominators; excessive exposure imbalance; and
activity below either period's frozen floor.

## Rationale

The binding constraint is not measurement difficulty — it is that the study's
subjects are private codebases and identifiable teams, and that the party who
wants the evidence is the party selling the product. Every choice above
resolves that conflict in the participant's favour and against the strength of
the claim.

Local-only processing with reviewed manual export (D-4) is the only design
where the participant can verify, rather than trust, what leaves their machine.
A closed allowlist (D-5) is the only schema posture where "we added a field"
cannot silently widen an existing consent. The claims allowlist (D-13) exists
because the honest failure mode of an in-house study is not fabricated data —
it is a true directional result promoted into a criterion it cannot carry.

Freezing before outcomes are visible (D-2, D-3) is what separates a study from
a search for a favourable cut. The extension rule is deliberately keyed to
activity, an input, rather than to results.

D-10's second paragraph and D-11's third are written from FEFF-002's measured
findings rather than from principle; both name concrete failure modes that the
current product surfaces exhibit.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Chosen: local processing, closed-allowlist reviewed manual export, observational, claims allowlist** | Participant can verify what leaves; consent cannot be silently widened; claims bounded by design | Slower; manual step per participant; smaller evidence surface |
| Opt-in telemetry upload with server-side aggregation | Far richer data; no manual step; scales past a handful of repositories | Breaks the no-phone-home posture; participant cannot inspect what left; irreversible on mistake; would need its own consent regime |
| Open export schema with a denylist of sensitive fields | Easier to extend; less schema churn | Every new upstream field defaults to exportable; one omission leaks; consent scope becomes unknowable |
| Public-repository-only study | No consent burden; fully reproducible | Public OSS is not the population the criteria describe; no prospective adoption or warning-disposition evidence |
| Randomised controlled trial | Supports causal claims | Requires withholding a safety tool from a control arm; infeasible at this cohort size; ethically poor |
| Defer the decision until FEFF-003/-004 exist | Faster to first code | The machinery would set the boundary; unwinding an export surface after the fact is the expensive direction |

## Consequences

- **Positive:** Participants can join without trusting anvil's good intentions —
  the review step makes the export inspectable and the allowlist makes it
  bounded. Existing privacy promises survive the study intact.
- **Positive:** The claims allowlist gives the project a mechanical answer to
  "can we say this?", removing a recurring judgement call under launch
  pressure.
- **Positive:** Freezing the design before results removes the strongest
  temptation an in-house study creates.
- **Negative:** The manual export step does not scale; recruiting five
  repositories is materially more work than a telemetry switch.
- **Negative:** The evidence will be weaker than the criteria want. Several
  criteria will remain unchecked after the first study by design.
- **Negative:** A closed allowlist means schema churn is expensive; adding one
  aggregate needs a consent review.
- **Risks:** A favourable directional result gets quoted without its
  denominators; participants over-estimate their privacy because "it's only
  aggregates"; the study surface drifts from this ADR as later items land.
- **Mitigations:** D-13's allowlist is machine-enforced, not editorial (FEFF-005
  owns it); D-6 requires an explicit disclosure-risk review rather than trusting
  aggregation; FEFF-005's verifier rejects definition or configuration drift
  between periods, and FEFF-008 reconciles the index criteria only through the
  criterion-to-evidence matrix.

## References

- Related ADRs: ADR-089 (local-only false-positive reports), ADR-107,
  ADR-053 (module progress reconciliation)
- APS modules: FEFF-001 (this decision), FEFF-002 (source and replay audit),
  FEFF-003, FEFF-004, FEFF-005, FEFF-006, FEFF-007, FEFF-008
- Module: [`plans/modules/field-effectiveness.aps.md`](../modules/field-effectiveness.aps.md)
- Audit: [`plans/audits/2026-08-27-feff-002-source-and-replay-audit.md`](../audits/2026-08-27-feff-002-source-and-replay-audit.md)
- Implementation contract for the privacy controls named in D-7:
  `crates/anvil-cli/src/usage.rs`
