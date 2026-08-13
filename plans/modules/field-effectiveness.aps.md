<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if work items exist and status is Ready. -->

# Field Effectiveness Evidence

| ID   | Owner  | Priority | Status | Progress |
| ---- | ------ | -------- | ------ | -------- |
| FEFF | @aneki | High     | Ready  | 0/8      |

**Last reviewed:** 2026-08-13 against the index success criteria, DEVACC Tier C
boundary, ADR-107, FLEET/BACT, current insights/drift evidence, and the
operator-approved study design.

> **Provenance:** Filed after the 2026-08-12 planning review found that anvil
> has synthetic acceleration evidence, anonymous fleet usage, authenticated
> beta activity, and local value summaries, but no programme tests whether the
> product reduces unsafe drift in ordinary development without slowing merges.
>
> **Exclusive module.** One study implementation stream owns this file. Feature
> PRs update only their own item status and evidence; stored progress counts are
> reconciled separately under ADR-053.

## Purpose

Produce claim-safe field evidence for the four post-release success criteria
that remain unverified in the index:

- adoption of anvil during active development;
- no material increase in merge throughput time;
- fewer new cross-boundary edges;
- fewer warnings left unresolved or suppressed without resolution.

The first study is a **simple before/after observational study**, not a causal
experiment. It combines a retrospective Git/GitHub baseline with a short
prospective anvil-active period so useful evidence does not require waiting
eight weeks before analysis begins.

This module owns the study protocol, local evidence tooling, participant
operations, first study, and claims disposition. It does not make a release
claim and does not change anvil's default telemetry posture.

**Packages:** `crates/anvil-bench`, `crates/anvil-cli` (final ownership is
confirmed by FEFF-002 before implementation).

## Approved Study Design

| Dimension | Default |
| --------- | ------- |
| Design | Simple retrospective/prospective before-and-after comparison |
| Historical window | 6–12 weeks immediately before activation, frozen before extraction |
| Active window | 10 working days with anvil; extend only under the pre-registered low-activity rule |
| Activity gate | Use one frozen activity unit for both periods: at least 5 eligible merged PRs in each period (10 total), or 25 eligible commits in each period (50 total), with no more than 3:1 exposure imbalance |
| Cohort | Minimum one eligible repository; target three independent teams/repositories; stretch five or more |
| Export | Local, user-reviewed manual bundle only; no automatic upload |
| Claim posture | Descriptive observational evidence; never a causal claim |

The protocol records the active-period maximum before a study starts. The
default maximum is 20 working days. If the activity gate is still unmet at that
point, the result remains descriptive and is labelled insufficient for a
comparative product claim; the window is not extended selectively after
results are inspected.

The protocol selects the activity unit before extraction. Complete,
authoritative PR metadata is preferred; otherwise the study uses commits. An
eligible commit is a non-merge change to the pre-registered in-scope source,
test, architecture, or configuration paths; generated, vendored,
dependency-lock-only, study-tooling, and period-boundary adjustment commits are
excluded. These definitions and the 3:1 exposure limit are frozen before
outcomes are visible. A repository below either period's floor is
descriptive-only.

An eligible merged PR targets the frozen integration branch and contains at
least one eligible in-scope change under the same path and exclusion rules.
Documentation-only, generated-only, vendored-only, dependency-only, bot-only,
revert-only, backport/duplicate, and study-tooling PRs do not qualify. A mixed
PR counts once; linked PRs that deliver one logical change count once under a
grouping frozen before extraction.

### Cohort and claim ladder

Participant availability is an input, not a hidden success condition:

| Evidence class | Minimum achieved cohort | Permitted interpretation |
| -------------- | ----------------------- | ------------------------ |
| Exploratory | One eligible repository meeting the activity gate | Repository-specific directional evidence only |
| Multi-repository corroboration | Three independent eligible teams/repositories | Repeated directional evidence; no population or causal claim |
| Broader corroboration | Five or more independent eligible teams/repositories | Stronger external relevance, still observational |

Repositories that miss the activity gate remain in the report as transparent
descriptive cases but do not enter the comparative aggregate. Results are
reported per repository before any cross-repository summary; raw event counts
are not pooled across repositories of different sizes.

Before outcomes are inspected, the study register freezes recruitment
channels, invitees, repository and team independence rules, inclusion and
exclusion criteria, and the enrolled cohort. The default recruitment window is
20 working days and closes at its end or when five consented repositories meet
the frozen baseline-only provisional criteria, whichever comes first. That
cohort is fixed before prospective collection. Active-period eligibility is
assessed only during analysis; it cannot reopen or extend recruitment. Results
must never influence early stopping. Invited, declined, enrolled, withdrawn,
capture-failed, ineligible, and
descriptive-only cases remain in the recruitment funnel. Withdrawal controls
future use of that participant's data; only the minimal, consented
accountability record may remain. Reports include attrition and sensitivity
analyses rather than complete cases alone.

### Metric and evidence boundaries

| Outcome | Retrospective source | Prospective source | Honest claim boundary |
| ------- | -------------------- | ------------------ | --------------------- |
| Merge throughput | GitHub PR timestamps and Git history | The same frozen extraction | Report open-to-merge and review/rework distributions, stratified by change size; do not infer AI assistance |
| Architecture drift | Historical replay at a frozen cadence with one pinned anvil build, rule catalogue, and participant-approved boundary definition | The same replay over the active period | Report new and resolved stable edges plus replay coverage; configuration failures are missing data, never zero drift |
| Adoption | Not reconstructable | Participant roster plus local anvil evidence on developer-days with eligible Git activity | A prospective adoption proxy only; it cannot prove "every save" coverage |
| Warning and suppression outcomes | Frozen historical scans may expose detectable violations and escape-hatch proxies | Local study snapshots and supported governance evidence | Historical proxies and prospective warnings remain separate; no fabricated warning baseline |
| Workflow friction | Not reconstructable | Short participant closeout plus observable retries/rework where supported | Qualitative corroboration, not a performance counter |

An AI-assisted PR subset is reported only when the repository already carries a
reliable, pre-existing label or authoritative metadata. Commit authorship,
message style, diff shape, or model-like prose must never be used to infer AI
assistance.

### Criterion-to-evidence boundary

| Index success criterion | First-study capability | Completion rule |
| ----------------------- | ---------------------- | --------------- |
| At least 50% of developers run anvil on every save | Directional prospective developer-day adoption proxy only | Remains unchecked unless an authoritative developer population and independently valid per-developer every-save denominator exist |
| No increase in AI-assisted time-to-merge | Overall throughput direction, stratified by size; AI subset only with pre-existing authoritative metadata | Remains unchecked without the authoritative AI subset and its pre-registered comparator |
| New cross-boundary edges fall 30% over eight weeks | Directional 10–20-working-day transition rate | Remains unchecked until the active observation horizon is at least eight weeks under the same protocol |
| Fewer than 10% of warnings are suppressed without resolution | Prospective warning disposition only; historical deterministic violations stay a separate proxy | Remains unchecked without comparable warning denominators and resolution follow-up in both periods |

The verifier's claims allowlist mechanically separates directional field
evidence from criterion completion. The first study cannot check a criterion
merely because its directional result is favourable.

### Retrospective replay contract

1. Freeze the study windows, selection cadence, anvil version/SHA, rule
   catalogue digest, architecture configuration digest, and metric dictionary
   before inspecting results.
2. Freeze one change-transition unit per repository and use it in both periods:
   the merge parent to merged-PR head when complete PR metadata exists,
   otherwise every eligible first-parent commit's parent to result. Analyse
   both sides of every transition; weekly-tip sampling is not comparative
   evidence.
3. Record whether the frozen boundary definition was valid at each historical
   point. Report present-policy replay as retroactive conformance separately
   from contemporaneous drift, and never aggregate repositories with
   incompatible transition or policy evidence classes.
4. Materialise historical trees only through sanitised Git configuration or
   non-checkout plumbing. Disable hooks, external filters, submodules, LFS,
   credential helpers, and repository-defined executables. Never install
   dependencies or execute participant code.
5. Run only the pinned absolute analyser in a no-network, credential-stripped
   isolation boundary with a temporary state root, bounded process, time,
   memory, and disk budgets, and symlink/path-containment checks. Never switch,
   reset, clean, or write into a participant's working checkout.
6. Apply the same pinned analyser and approved boundary definition to both
   periods. A snapshot that cannot load that definition is an evidence gap.
7. Record analysed and skipped snapshots, failure reasons, files/languages in
   scope, and rule/config changes so coverage accompanies every result.
8. Default comparative coverage floors are 90% of selected transitions in each
   period and 90% of prospective eligible developer-days for metrics that
   depend on daily capture. FEFF-001 may replace these only before enrolment.
   Below-floor metrics are descriptive-only and carry worst-case or sensitivity
   bounds; missingness is never silently dropped.
9. Normalise drift counts by the same frozen eligible activity unit as well as
   reporting
   absolute counts; do not let a quieter active period look safer by default.

### Manual bundle boundary

All collection and aggregation happens locally. Nothing leaves the participant
machine until a person reviews the exact export and explicitly approves it.

The export bundle contains only schema-versioned aggregates and provenance:

- a study-and-recipient-specific pseudonym, never the repository name;
- bucketed period and activity values plus recipient-specific, unlinkable
  commitments where configuration integrity must be demonstrated;
- metric definitions, cohort/evidence class, activity denominators, coverage,
  missing-data reasons, and aggregate results;
- a manifest of the immutable payload, its canonical digest, and verifier
  outcome;
- bundle retention/deletion dates.

The export uses a recursively closed allowlist schema: unknown, misspelled,
nested, aliased, free-form, and extension-map fields fail validation. New
fields require a schema version plus renewed ADR and consent review. The
payload must not contain source text, diffs, raw file paths, repository names
or URLs, exact dates or small cells that create avoidable linkage, stable
cross-recipient fingerprints, commit/PR titles, authors, emails, hostnames,
command arguments, access tokens, raw event rows, or reversible commit
identifiers. Before export or publication, a disclosure-risk review considers
recipient knowledge and linkage to candidate repositories; explicit consent
is required for any repository-specific result that remains identifiable.

Review and approval operate on an immutable, canonically serialised payload.
The participant sees that payload and its digest. Approval is a separate
envelope that binds the digest, schema version, study-and-recipient pseudonym,
purpose, recipient, bundle ID, nonce, and initial-transfer expiry using a
participant-controlled signature or another authenticated mechanism accepted
by FEFF-001. Adding the envelope does not alter the reviewed payload. The
recipient verifies the transfer before expiry and creates an authenticated
acceptance receipt binding the payload digest, approval envelope, recipient,
nonce, and acceptance time. The verifier rejects payload or envelope
substitution, stale initial transfer, nonce reuse, and wrong-recipient or
wrong-purpose replay. After timely acceptance, the immutable payload, approval
envelope, and acceptance receipt remain verifiable as durable history; transfer
expiry does not invalidate archived evidence.

Full local receipts may retain the data needed to reproduce a bundle, subject
to the protocol's retention and deletion rules; they are never exported by
default. They live in a user-scoped, non-repository state root with owner-only
permissions or platform ACLs, no-follow create-new and atomic writes, strict
path containment, bounded age and size, and redacted errors. Deletion covers
raw receipts, caches, temporary trees, and exports; any surviving deletion
receipt is explicitly data-minimised and unlinkable.

Existing privacy controls remain authoritative. FEFF-001 defines their exact
precedence, but study start never implies an override: `DO_NOT_TRACK` is a
superset hard-off for local collection and
`ANVIL_INTERCEPT_DISABLE_OBSERVATION` is the whole-observation break-glass.
FEFF-004 checks applicable controls before every participant-source read or
ledger write, fails closed, and stops new collection immediately if they
change mid-study. Resume requires a fresh explicit action. Any exception needs
separate ADR-authorised consent and cannot be inferred from joining a study.

## In Scope

- A durable evidence/privacy decision covering consent, local processing,
  manual export, retention, deletion, and permitted claims.
- A source feasibility audit mapping every metric to current supported product,
  Git, or GitHub truth before implementation.
- Deterministic retrospective reconstruction over temporary detached worktrees.
- Prospective local snapshots for adoption proxy, warning disposition, drift,
  throughput, evidence coverage, and participant friction.
- A schema-versioned, inspectable aggregate bundle plus deterministic verifier.
- Participant onboarding, abort, review, deletion, and closeout procedures.
- Execution and publication of the first evidence-labelled study.

## Out of Scope

- Automatic or background upload of study evidence.
- Expanding or re-identifying the anonymous FLEET beacon, joining FLEET
  `install_id` to an account, or adding study data to BACT.
- Source code, paths, identities, raw Git/GitHub records, or raw governance
  events in the exported bundle.
- Randomised, matched-control, or organisation-wide causal claims from the
  first study.
- Claiming every-save adoption without an independently valid save denominator.
- Inferring AI assistance where authoritative metadata does not already exist.
- A hosted analytics dashboard, third-party analytics SDK, or general product
  telemetry system.
- Modifying participant repositories to make the historical result look
  cleaner, or silently excluding failed historical snapshots.

## Interfaces

**Depends on:**

- The index success criteria and product thesis — the outcomes being tested.
- [dev-acceleration-benchmarks](./dev-acceleration-benchmarks.aps.md) (DEVACC)
  and its claims policy — Tier C corroborates but does not replace Tier B.
- [ADR-107](../decisions/107-fleet-telemetry-consent-posture.md),
  [fleet-telemetry](./fleet-telemetry.aps.md) (FLEET), and
  [beta-account-activity](./beta-account-activity.aps.md) (BACT) — boundaries
  this module must not weaken.
- `anvil drift` snapshots/comparison, architecture boundary evidence, current
  checks, witness evidence, and supported insights/governance queries.
- Git history and optional GitHub PR metadata obtained with participant
  authorisation.

**Coordinates with:**

- [daemon-protection-observability](./daemon-protection-observability.aps.md)
  (DPO) and [kindling-product-fit](./kindling-product-fit.aps.md) (KFIT) —
  FEFF-002 must determine whether supported study-grade queries exist. FEFF
  does not add a parallel raw event store or query undocumented Kindling data.
- [usage-insights](../archive/modules/usage-insights.aps.md) (INSIGHTS) — drift
  and suppression views are inputs where measured; weekly placeholder zeroes
  are not evidence.
- GitHub privacy and rate-limit boundaries — cached input may be retained
  locally but is reduced before export.

**Exposes:**

- An accepted evidence/privacy decision and frozen study protocol.
- A local retrospective/prospective study runner and manual bundle surface.
- A deterministic verifier and evidence-classified report.
- A claims disposition that may update index success-criteria evidence without
  silently converting directional results into proof.

## Readiness and Sequencing

FEFF-001 and the synthetic/public/operator-owned portion of FEFF-002 are Ready
and may proceed in parallel. No private participant repository, GitHub
metadata, or governance data is read until FEFF-001 accepts the consent,
authority, retention, and deletion contract. No collection surface is
promoted until both close. FEFF-002 must explicitly disposition current
KFIT-007/-009/-010 and DPO-003 dependencies: if a supported study-grade source
is missing, the required upstream item or a narrowly owned FEFF adapter must be
planned before FEFF-004 becomes Ready.

FEFF-003 and FEFF-004 may then proceed in parallel. FEFF-005 validates both
outputs. FEFF-006 follows the manual export surface. FEFF-007 remains blocked
on tooling, consent, and at least one eligible participant; FEFF-008 follows the
completed study.

## Work Items

### FEFF-001: Accept the field-evidence and manual-export decision

- **Status:** Ready
- **Intent:** Establish a durable authority for what field evidence may be
  collected, retained, exported, and claimed.
- **Expected Outcome:** An accepted ADR freezes the approved before/after
  design, retrospective and prospective windows, adaptive cohort/claims
  ladder, low-activity extension rule, local-only processing, user-reviewed
  manual export, consent/withdrawal, local and exported retention/deletion,
  closed export allowlist, disclosure-risk model, immutable-payload approval
  envelope, existing privacy-control precedence, hardened local storage,
  Git/GitHub authorisation, replay isolation, activity and coverage floors,
  recruitment/stopping/attrition rules, metric definitions, missing-data
  treatment, and the rule that observational evidence cannot support a causal
  claim. The decision log and this module agree.
- **Validation:** `pnpm format:check && pnpm aps:active-lint && pnpm docs:check`
- **Files:** `plans/decisions/DECISION-LOG.md`,
  `plans/decisions/<next>-field-effectiveness-evidence.md`, this module
- **Confidence:** high
- **Dependencies:** none

### FEFF-002: Audit evidence sources and historical replay feasibility

- **Status:** Ready
- **Intent:** Prove which planned measures are reconstructable from current
  supported sources before building a study surface.
- **Expected Outcome:** A source audit maps each metric to its authoritative
  Git, GitHub, anvil, DPO/KFIT, insights, drift, or participant source; marks
  measured values versus placeholders/proxies; demonstrates isolated replay on
  at least two synthetic, public, or explicitly operator-owned historical
  commits without touching the active checkout; records coverage/failure
  behaviour; and decides the implementation owner for the
  retrospective runner and prospective evidence adapter. It explicitly
  dispositions KFIT-007/-009/-010 and DPO-003 dependencies and forbids direct
  reads of undocumented storage. Participant-data probes remain gated on
  FEFF-001.
- **Validation:** The audit records successful replay and current-surface probe
  commands; `pnpm format:check && pnpm aps:active-lint && pnpm docs:check`
- **Files:** `plans/audits/`, `crates/anvil-cli/src/commands/insights.rs`,
  `crates/anvil-cli/src/insights/`, `crates/anvil-cli/src/commands/drift.rs`,
  `crates/anvil-intercept/src/kindling_observation.rs`
- **Confidence:** medium
- **Dependencies:** none

### FEFF-003: Build the local retrospective baseline runner

- **Status:** Proposed
- **Intent:** Reconstruct a reproducible historical comparison without
  modifying participant work or exporting repository content.
- **Expected Outcome:** A user-explicit local runner freezes and validates the
  study manifest; selects the documented historical cadence; queries Git and
  optional authorised GitHub metadata; replays every frozen parent/result
  transition using the pinned analyser and boundary definition in isolated
  temporary trees; records every analysed/skipped snapshot and error; and emits
  local aggregate throughput, activity, drift,
  deterministic violation, escape-hatch proxy, and coverage records. Repeated
  runs over unchanged inputs are byte-stable apart from explicitly excluded
  local receipt timestamps. The runner uses sanitised Git, executes no
  participant code, has no network or ambient credentials, applies resource
  and path-containment bounds, and safely handles malicious snapshots and
  interrupted teardown. GitHub access is pinned independently to the consented
  repository and GitHub/GHES host, uses read-only least-privilege credentials
  and allowlisted queries/endpoints, never trusts a remote URL for credential
  routing, redacts errors, and bounds caches, retention, rate-limit, purge, and
  offline-input behaviour.
- **Validation:** `cargo test -p anvil-bench -- field_effectiveness_retrospective`
- **Files:** `crates/anvil-bench/`, `benchmarks/field-effectiveness/`, schemas
  selected by FEFF-002
- **Confidence:** medium
- **Dependencies:** FEFF-001, FEFF-002

### FEFF-004: Build the prospective local ledger and reviewed bundle export

- **Status:** Proposed
- **Intent:** Capture the active period locally and let the participant inspect
  the exact aggregate before any evidence leaves the machine.
- **Expected Outcome:** A supported user-explicit study surface starts, checks,
  closes, reviews, exports, and deletes a local study. It records bounded daily
  aggregates and evidence gaps for eligible Git activity, anvil-active
  developer-days, warning/finding disposition, suppressions, drift snapshots,
  governance observations where supported, and capture health. Export is
  impossible until local review renders the exact canonically serialised
  payload and digest and the user creates a separate authenticated approval
  envelope bound to them. Initial transfer produces the authenticated
  acceptance receipt required for durable verification. It honours existing
  privacy hard-offs before every
  read/write and on mid-study changes. Local state is owner-only,
  non-repository, no-follow, atomic, bounded, and fully deletable. No automatic
  upload or raw event/path export exists, and withdrawal/deletion leaves only a
  consented, data-minimised receipt.
- **Validation:** `cargo test -p eddacraft-anvil -- field_effectiveness`
- **Files:** `crates/anvil-cli/`, schemas and local state paths selected by
  FEFF-001/-002
- **Confidence:** low
- **Dependencies:** FEFF-001, FEFF-002; any DPO/KFIT prerequisite identified by
  FEFF-002

### FEFF-005: Verify bundles and compute evidence-classified comparisons

- **Status:** Proposed
- **Intent:** Make malformed, underpowered, incomparable, or privacy-unsafe
  evidence fail closed before it reaches a report.
- **Expected Outcome:** A deterministic verifier rejects forbidden fields,
  unknown fields at every nesting level, schema/digest or approval-envelope
  mismatch, changed definitions/configuration between periods, selectively
  extended periods, missing approval or acceptance, invalid initial-transfer
  freshness, nonce or recipient/purpose replay, replay/capture coverage below
  the metric-specific floor, absent denominators, excessive exposure imbalance,
  and activity below either period's frozen floor. Valid bundles receive an
  evidence class, per-repository before/after distributions and normalised
  rates, recruitment/attrition and missing-data tables, sensitivity bounds, and
  a claims allowlist that cannot complete an index criterion without its exact
  denominator and horizon. AI-assisted results are omitted unless authoritative
  input metadata passes the FEFF-001 rule.
- **Validation:** `cargo test -p anvil-bench -- field_effectiveness_verify`
- **Files:** `crates/anvil-bench/`, bundle schemas and fixtures
- **Confidence:** medium
- **Dependencies:** FEFF-003, FEFF-004

### FEFF-006: Publish the participant operations and consent pack

- **Status:** Proposed
- **Intent:** Make recruitment and study operation safe, reversible, and
  understandable without researcher improvisation.
- **Expected Outcome:** An operator runbook and participant-facing pack cover
  eligibility, informed consent, repository authorisation, historical replay,
  the 6–12-week/10-working-day defaults, pre-registered extension maximum,
  local disk impact, daily capture health, pause/abort/withdrawal, exact bundle
  review, manual transfer, retention/deletion, support, and closeout questions.
  It explains the exploratory/multi-repository/broader ladder and never promises
  causal proof or automatic privacy through aggregation.
- **Validation:** `pnpm format:check && pnpm docs:check`
- **Files:** `docs/runbooks/`, participant templates under a governed docs path
  selected by FEFF-001
- **Confidence:** high
- **Dependencies:** FEFF-001, FEFF-004, FEFF-005

### FEFF-007: Conduct the first field-effectiveness study

- **Status:** Blocked
- **Intent:** Produce the first participant-reviewed before/after evidence set
  under the accepted protocol.
- **Expected Outcome:** At least one eligible repository completes the frozen
  historical replay and prospective active period; target recruitment is three
  independent repositories and stretch recruitment is five or more. Each
  participant reviews and explicitly approves their aggregate bundle. The
  verifier records activity eligibility, evidence class, coverage, deviations,
  withdrawals, missing data, and qualitative friction. Low-activity cases are
  extended only under the pre-registered rule or retained as descriptive cases.
  Recruitment follows the frozen sampling frame, 20-working-day window, and
  outcome-independent stopping rule; the full invite-to-disposition funnel,
  attrition, and sensitivity analyses accompany the results.
- **Validation:** FEFF-005's verifier passes every comparative bundle; the
  study register accounts for every recruited, completed, withdrawn, excluded,
  and descriptive-only case; `pnpm docs:check`
- **Files:** Local participant bundles (untracked by default), reviewed aggregate
  evidence under `benchmarks/history/field-effectiveness/` only after explicit
  approval, study register/report under a governed docs path
- **Confidence:** low — participant availability is intentionally adaptive
- **Dependencies:** FEFF-003, FEFF-004, FEFF-005, FEFF-006, at least one eligible
  and consenting participant

### FEFF-008: Publish the claims disposition and reconcile success evidence

- **Status:** Blocked
- **Intent:** Convert the completed study into appropriately bounded product
  decisions rather than a headline selected after seeing the result.
- **Expected Outcome:** A reviewed report presents per-repository and aggregate
  results, cohort/evidence class, denominators, uncertainty, missing data,
  replay coverage, study deviations, friction themes, and limitations. It
  states which adoption, throughput, drift, and signal-quality claims are
  supported, contradicted, or still untested. The index success criteria gain
  evidence links only where the criterion-to-evidence matrix permits; the
  default first study cannot complete the every-save adoption, authoritative
  AI-assisted throughput, eight-week drift, or comparable historical warning
  criteria. Exploratory and directional evidence remains
  labelled and cannot silently check a criterion complete. Follow-up product
  work is filed in APS or GitHub rather than embedded as deferred-work notes.
- **Validation:** FEFF-005 verifier passes the report inputs;
  `pnpm format:check && pnpm aps:active-lint && pnpm aps:index:check && pnpm docs:check`
- **Files:** `plans/audits/` or `docs/reviews/`, `plans/index.aps.md`, follow-up
  owning APS modules
- **Confidence:** medium
- **Dependencies:** FEFF-007

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Before/after changes reflect seasonality, team mix, or project phase rather than anvil | High | Label observational; freeze windows; report change-size/activity distributions; no causal language |
| Current architecture rules do not apply cleanly to old commits | High | Freeze the participant-approved definition; expose unsupported snapshots and coverage; never count failures as clean |
| A quieter active period appears to reduce drift | High | Report normalised rates and activity denominators alongside absolute counts |
| Historical escape-hatch scans are mistaken for warning history | High | Keep historical proxies and prospective warning disposition in separate fields and prose |
| Small or opportunistic cohort is over-generalised | High | Freeze the recruitment frame and stopping rule; report the full funnel, attrition, sensitivity, cohort/claim ladder, and per-repository results |
| Export accidentally identifies a private repository or person | High | Closed allowlist, recipient-specific pseudonyms, bucketing/small-cell controls, disclosure-risk review, explicit consent, and verifier rejection |
| Reviewed evidence is changed or replayed after approval | High | Canonical immutable payload plus authenticated approval and timely acceptance receipts; transfer expiry does not invalidate the archive |
| Prospective evidence relies on placeholder or lossy sources | High | FEFF-002 source audit; explicit evidence gaps; promote required DPO/KFIT work before collection |
| Historical replay executes hostile repository behaviour or damages participant work | High | Sanitised materialisation, no code/network/credentials, isolated pinned analyser, resource and path limits, active-checkout refusal, clean teardown |
| Local receipts leak or are accidentally committed | High | Owner-only non-repository storage, no-follow atomic writes, bounded retention, redacted errors, and complete deletion |
| Low activity causes indefinite study extension | Medium | Pre-register a maximum (default 20 working days); otherwise descriptive-only |

## Decisions

1. The first study is simple before/after, not matched-control or randomised.
2. The baseline is retrospective Git/GitHub reconstruction, not a four-week
   waiting period.
3. The prospective default is 10 working days, with a pre-registered bounded
   low-activity extension.
4. Eligibility uses one frozen unit and requires at least 5 eligible PRs in
   each period (10 total) or 25 eligible commits in each (50 total), with no
   more than 3:1 exposure imbalance.
5. Cohort size adapts to recruitment; the achieved size controls the evidence
   label, while a frozen sampling frame, recruitment window, stopping rule, and
   full disposition funnel prevent outcome-dependent selection.
6. Evidence leaves the participant machine only in an explicitly approved,
   user-reviewed immutable aggregate payload plus its authenticated approval
   envelope.
7. The same frozen analyser/rules/configuration apply to both periods.
8. The study never infers AI assistance or claims every-save coverage from
   evidence that cannot support it.
9. Existing privacy hard-offs remain authoritative and participant-data replay
   does not begin before the evidence/privacy decision.
10. Directional evidence cannot complete an index criterion without that
    criterion's exact denominator and observation horizon.

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| Decision and truth audit | FEFF-001..002 | 2 Ready |
| Evidence tooling | FEFF-003..005 | 3 Proposed |
| Participant operations | FEFF-006 | 1 Proposed |
| Study and disposition | FEFF-007..008 | 2 Blocked |
| **Total** | **8** | **0/8 complete** |
