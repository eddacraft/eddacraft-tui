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
| Activity gate | At least 10 merged PRs or 50 relevant commits across the paired periods; neither period may be empty |
| Cohort | Minimum one eligible repository; target three independent teams/repositories; stretch five or more |
| Export | Local, user-reviewed manual bundle only; no automatic upload |
| Claim posture | Descriptive observational evidence; never a causal claim |

The protocol records the active-period maximum before a study starts. The
default maximum is 20 working days. If the activity gate is still unmet at that
point, the result remains descriptive and is labelled insufficient for a
comparative product claim; the window is not extended selectively after
results are inspected.

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

### Retrospective replay contract

1. Freeze the study windows, selection cadence, anvil version/SHA, rule
   catalogue digest, architecture configuration digest, and metric dictionary
   before inspecting results.
2. Prefer merged-PR heads when complete GitHub metadata is available; otherwise
   use a documented first-parent or weekly-tip cadence consistently for that
   repository.
3. Materialise historical commits only in bounded temporary detached
   worktrees. Never switch, reset, clean, or write into a participant's working
   checkout.
4. Apply the same pinned analyser and approved boundary definition to both
   periods. A snapshot that cannot load that definition is an evidence gap.
5. Record analysed and skipped snapshots, failure reasons, files/languages in
   scope, and rule/config changes so coverage accompanies every result.
6. Normalise drift counts by eligible change activity as well as reporting
   absolute counts; do not let a quieter active period look safer by default.

### Manual bundle boundary

All collection and aggregation happens locally. Nothing leaves the participant
machine until a person reviews the exact export and explicitly approves it.

The export bundle contains only schema-versioned aggregates and provenance:

- a random study/repository pseudonym, never the repository name;
- frozen period, selection, tool, rule, and configuration fingerprints;
- metric definitions, cohort/evidence class, activity denominators, coverage,
  missing-data reasons, and aggregate results;
- a manifest of included files plus digests and verifier outcome;
- the review/approval receipt and bundle retention/deletion dates.

The export bundle must not contain source text, diffs, raw file paths,
repository names or URLs, commit/PR titles, authors, emails, hostnames, command
arguments, access tokens, raw event rows, or reversible commit identifiers.
Full local receipts may retain the data needed to reproduce a bundle, subject
to the protocol's retention and deletion rules; they are never exported by
default.

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

FEFF-001 and FEFF-002 are Ready and may proceed in parallel. No collection
surface is promoted until both close. FEFF-002 must explicitly disposition
current KFIT-007/-009/-010 and DPO-003 dependencies: if a supported study-grade
source is missing, the required upstream item or a narrowly owned FEFF adapter
must be planned before FEFF-004 becomes Ready.

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
  forbidden fields, metric definitions, missing-data treatment, and the rule
  that observational evidence cannot support a causal claim. The decision log
  and this module agree.
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
  at least two historical commits without touching the active checkout; records
  coverage/failure behaviour; and decides the implementation owner for the
  retrospective runner and prospective evidence adapter. It explicitly
  dispositions KFIT-007/-009/-010 and DPO-003 dependencies and forbids direct
  reads of undocumented storage.
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
  optional authorised GitHub metadata; replays the pinned analyser and boundary
  definition in temporary detached worktrees; records every analysed/skipped
  snapshot and error; and emits local aggregate throughput, activity, drift,
  deterministic violation, escape-hatch proxy, and coverage records. Repeated
  runs over unchanged inputs are byte-stable apart from explicitly excluded
  local receipt timestamps. Temporary worktrees are removed only after clean,
  bounded teardown and never overlap the participant checkout.
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
  impossible until the local review renders the exact schema-versioned bundle
  and the user gives explicit approval. No automatic upload or raw event/path
  export exists, and withdrawal/deletion leaves a verifiable local receipt.
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
  schema/digest mismatch, changed definitions/configuration between periods,
  empty or selectively extended periods, missing approval, replay coverage
  below the protocol floor, absent denominators, and activity below the 10-PR
  or 50-commit gate. Valid bundles receive an evidence class, per-repository
  before/after distributions and normalised rates, missing-data table, and
  claims allowlist. AI-assisted results are omitted unless authoritative input
  metadata passes the FEFF-001 rule.
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
  evidence links only where the protocol permits; exploratory evidence remains
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
| Small or opportunistic cohort is over-generalised | High | Enforce the cohort/claim ladder and per-repository-first reporting |
| Export accidentally identifies a private repository or person | High | Denylist schema, irreversible pseudonyms, exact local review, manual approval, verifier rejection |
| Prospective evidence relies on placeholder or lossy sources | High | FEFF-002 source audit; explicit evidence gaps; promote required DPO/KFIT work before collection |
| Historical replay damages participant work or consumes unbounded disk | High | Temporary detached worktrees, active-checkout refusal, bounded concurrency/storage, clean teardown |
| Low activity causes indefinite study extension | Medium | Pre-register a maximum (default 20 working days); otherwise descriptive-only |

## Decisions

1. The first study is simple before/after, not matched-control or randomised.
2. The baseline is retrospective Git/GitHub reconstruction, not a four-week
   waiting period.
3. The prospective default is 10 working days, with a pre-registered bounded
   low-activity extension.
4. Eligibility requires 10 merged PRs or 50 relevant commits across the paired
   periods, with neither period empty.
5. Cohort size adapts to recruitment; the achieved size controls the evidence
   label and permitted interpretation.
6. Evidence leaves the participant machine only in an explicitly approved,
   user-reviewed manual aggregate bundle.
7. The same frozen analyser/rules/configuration apply to both periods.
8. The study never infers AI assistance or claims every-save coverage from
   evidence that cannot support it.

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| Decision and truth audit | FEFF-001..002 | 2 Ready |
| Evidence tooling | FEFF-003..005 | 3 Proposed |
| Participant operations | FEFF-006 | 1 Proposed |
| Study and disposition | FEFF-007..008 | 2 Blocked |
| **Total** | **8** | **0/8 complete** |
