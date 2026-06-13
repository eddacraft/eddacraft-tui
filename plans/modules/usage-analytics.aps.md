<!--
APS Module: Usage Analytics
===========================
Cross-cutting durable usage observations on Kindling — command
invocations, inline flag-context snapshots, dev-investment query views.
Touches every command surface (CLI / JSON-RPC) and records the
invocation's resolved flag context inline per ADR-041. Third trial of
the cross-cutting module convention promoted under ADR-034.

Cross-cutting convention: see plans/aps-rules.md#cross-cutting-modules.
-->

# Usage Analytics

| ID    | Owner      | Status | Progress |
| ----- | ---------- | ------ | -------- |
| USAGE | @eddacraft | Ready  | 0/3      |

**Last reviewed:** 2026-05-11

> **Provenance:** Founder request 2026-05-10. Current observability story
> (TRACE-001 + JSON logs) gives a debug surface but no durable answer to
> "who is using what" for dev-investment decisions. Per
> [ADR-035](../decisions/035-three-pipe-observability-rule.md), usage
> facts are governance-shaped (durable, queryable, source-of-truth) and
> belong on Kindling, not on the tracing pipe. This module is the
> **third trial** of the cross-cutting module convention promoted under
> [ADR-034](../decisions/034-cross-cutting-modules-as-aps-primitive.md).

## Cross-cutting convention

This module follows the cross-cutting convention. The normative spec
lives in [`plans/aps-rules.md#cross-cutting-modules`][rules] and is
cited by anchor link wherever a callout is used in a task body.

[rules]: ../aps-rules.md#cross-cutting-modules

> **Anti-drift hook (per ADR-034):** changes to the
> `## Cross-Cutting Modules` section of `aps-rules.md` update this
> module's header reference, `launch-flow-readiness.aps.md`, and
> `tracing-foundation.aps.md` in the same PR.

## Purpose

Give the founder a durable, queryable answer to **what is being used
and what is not** so dev-investment decisions are evidence-based
rather than gut-led. Specifically:

- Record every CLI command invocation and JSON-RPC method call as a
  Kindling row carrying the active flag context and the cross-pipe
  `traceparent`.
- Record the inline `flag_set` context defined by ADR-041 so the founder
  can ask "which gates fired for whom this week" without rebuilding the
  index from spans.
- Publish canned query views answering "top N commands", "commands
  never invoked", "flag-dependent paths exercised vs not".

This module is **explicitly not** about result/output capture — it
records *that* a command ran, not *what it produced*. See **Privacy
contract** in USAGE-001.

## In scope

- A new Kindling observation kind for command invocation (or reuse of
  an existing kind, decided in USAGE-001), carrying: command name,
  anonymised principal, timestamp, redacted argument *shape* (arg
  names but not values), active flag set, `traceparent`.
- A producer wired into the CLI entrypoint and the JSON-RPC dispatcher
  emitting one observation per user-initiated invocation. Library
  crates do not emit; only entrypoints.
- Query helpers / canned views — at minimum a `kindling`-style query
  surface and a runbook describing the top-N / never-invoked queries.
- Alignment with ADR-019 / ADR-041 so gate-affecting flag facts join by
  manifest `key`, while non-gate flag context stays inline on the usage
  row.
- A privacy contract documenting the redaction rule (the same
  `SENSITIVE_FIELDS` deny-list `anvil-observability` exposes today).
- A new `anvil.usage.*` namespace registry entry (per ADR-035 / the
  namespace registry doc) for any tracing-pipe attributes the
  producer also emits for debugging — Kindling-side fields are the
  source-of-truth, the tracing-pipe attributes are breadcrumbs.

## Out of scope

- Identity provider / principal minting — assumes a stable
  user/session ID is already available on the call path. If it is
  not, USAGE-001 surfaces that as a blocker; it does not solve it.
- Result or output capture — the founder's stated requirement is
  invocation, not results. Adding result capture later is a follow-up
  task with its own privacy review.
- Dashboard rendering of usage data — that lives with the dashboard
  surface (TUIDASH / dashboard-ops-views) and is post-launch.
- Exporting usage data to an external analytics pipeline (Mixpanel /
  PostHog / a warehouse). Out of scope for v1; revisit if
  evidence-based dev-investment decisions need cross-system joins.
- Production tracing sink choice — owned by EXPORT.

## Interfaces

**Depends on:**

- Kindling — the source-of-truth pipe per ADR-035; USAGE writes here
  exclusively.
- TRACE-001 — `anvil-observability::TraceContext` provides the
  `traceparent` correlation key every usage row carries.
- ADR-019 / ADR-041 — gate-affecting flag facts join by manifest
  `key`; non-gate flag context is inline on the usage row only.

**Coordinates with:**

- TRACE-004 — every usage observation needs an active `traceparent`
  at write time. TRACE-004 binds the incoming context; USAGE-001
  reads it. The two land in either order, but both must be in place
  before the first end-to-end "trace ↔ usage" join works.
- FLAGS module — the active flag set on each usage row is the inline
  resolved context USAGE captures from the resolver at the invocation
  boundary; USAGE-002 closes the loop.
- TRACE-003 — argument redaction shares the `SENSITIVE_FIELDS`
  deny-list. USAGE inherits the advisory-only constraint until
  TRACE-003 lands the layer; producers MUST NOT bypass it.
- OBS-001 (post-launch) — once OBS goes Ready, the usage signal
  inventory becomes a row in OBS-001's contract; until then USAGE
  publishes its own narrow contract.
- INTD-013 / INTD-015 — usage observations carry no
  `notification.context` payload, so the redaction risk on that
  field is not relevant here. Documented to forestall confusion.

**Exposes:**

- A Kindling observation kind for command invocation (kind name
  decided in USAGE-001).
- A query helper (`kindling`-style) and a runbook describing the
  canned dev-investment views.
- The privacy contract: invocation-only, args redacted by the same
  deny-list `anvil-observability` carries.
- A new `anvil.usage.*` row in
  `docs/observability/namespace-registry.md` (tracing-pipe
  breadcrumbs only; Kindling rows are not registry-tracked).

## Ready Checklist

This module is **Ready** when:

- [x] Founder confirmed 2026-05-11: privacy contract (invocation-only,
      args redacted by `SENSITIVE_FIELDS`, arg shape only — no values
      by default, no result capture) reflects intent. Codified in the
      Privacy Contract section below; USAGE-001 publishes the same
      text at `docs/observability/usage-analytics.md`.
- [x] OQ2 resolved 2026-05-11: principal anonymised via one-way hash
      with a per-deployment salt held in the existing secrets store.
      Salt rotation is a deliberate privacy reset, not routine.
      USAGE-001's contract test pins the hash function and asserts the
      raw principal never lands in a Kindling row.
- [x] OQ5 resolved: FLAGS cross-clarification ([`FLAGCAT-007`](./feature-flag-catalogue.aps.md#flagcat-007))
      — [ADR-041](../decisions/041-flag-snapshot-usage-join-contract.md)
      says USAGE stores resolved flag context inline on the usage row,
      joins by manifest `key`, and ADR-019 stays gate-affecting-only for
      standalone Kindling flag facts.
- [ ] OQ1 resolved: reuse-existing vs new `command.invoked` kind
      (founder lean 2026-05-10 → new kind; OQ5 now resolved).
- [ ] At least one task approved for execution.

## Privacy contract

Founder-confirmed 2026-05-11. USAGE-001 publishes the same text at
`docs/observability/usage-analytics.md`. Changes to this section
require founder review.

**Captured per invocation:**

- Command name (e.g. `scan`, `kindling.list`).
- Anonymised principal — one-way hash (per OQ2: per-deployment salt).
- Timestamp.
- Per-argument metadata only: argument name, plus shape fields
  (length / type / presence). **Raw argument values are never
  recorded.**
- For arguments whose **name** matches
  `anvil_observability::redaction::SENSITIVE_FIELDS`, even the shape
  fields are elided and replaced with the literal `<redacted>` marker
  (the value of the `REDACTED` constant). A sensitive argument's
  *existence* (its name) is still visible; nothing about its value
  or shape leaks via metadata.
- Active flag set — the inline resolved flag context defined by ADR-041.
- `traceparent` (cross-pipe correlation per ADR-035).

**NOT captured:**

- Raw argument values, ever. Widening to a fuller capture is a
  follow-up review, not a routine code change — and is the only path
  that introduces value content into a usage row.
- Shape fields for sensitive-named arguments (see above — these
  collapse to `<redacted>`).
- Command results, output, stdout, stderr.
- File contents touched by the command.
- Network traffic.
- Stack traces or error messages — those stay on the tracing pipe.

**Retention:** defers to Kindling's retention policy. *Open: confirm
Kindling's policy is acceptable for usage rows specifically; tighten
under a follow-up if not.*

**Change control:** any change to the captured / not-captured lists
requires founder review. The contract doc lives in
`docs/observability/`, separate from code, so a PR diff is visible.

## Work Items

> Status: Ready. USAGE-001 authorised Ready 2026-05-30 via operator
> Draft-readiness review — the founder OQ1 observation-kind decision is
> folded into the task's scope (Expected Outcome records reuse-vs-new
> inline). USAGE-002 and USAGE-003 remain Draft and follow once
> invocations land. Scoped into the `v0.9.0-beta` release window as
> additive scope (operator, 2026-06-13); USAGE-001 stays `Ready` and
> unblocked — independent of the graph entry gates, so it can be picked
> up whenever it is scheduled.

### USAGE-001: Command-invocation observation kind and producer

- **Intent:** Every CLI command and JSON-RPC method call lands as one
  Kindling row carrying enough context to answer "who ran what"
  later.
- **Concrete failure mode (today):** The founder asks "which commands
  did anyone run last week?" and the only available data is JSON log
  lines on stderr (ephemeral) or `traceparent`-stamped envelopes
  (also ephemeral, no archive). The answer is unrecoverable.
- **Expected Outcome:**
  - Decision recorded (in the PR or a thin ADR) on whether to reuse
    an existing Kindling observation kind or introduce a new one
    (working title `command.invoked`).
  - A producer at the CLI entrypoint
    (`crates/anvil-cli/src/main.rs`) and the JSON-RPC dispatcher
    (`crates/anvil-intercept/src/ipc.rs`) emits exactly one
    observation per user-initiated invocation with: command name,
    principal (one-way hash + per-deployment salt per OQ2 — salt
    sourced from the existing secrets store; raw principal MUST NOT
    appear in any field), timestamp, redacted argument shape, active
    flag set (inline `flag_set` per ADR-041), `traceparent`.
  - Argument redaction defers to
    `anvil_observability::redaction`. Raw argument values are never
    recorded; non-sensitive arguments contribute *shape* metadata
    only (name + length + type + presence). For arguments whose
    *name* matches `SENSITIVE_FIELDS`, the shape metadata is elided
    and the per-arg payload is the literal `<redacted>` marker (the
    value of the `REDACTED` constant). A fuller value capture
    requires a follow-up review and is out of scope for this task.
  - A privacy contract published at
    `docs/observability/usage-analytics.md` (new) covering: what is
    captured, what is not, anonymisation policy, retention
    expectation, and the change-control rule (founder review for
    contract changes).
  - Contract pinned by a Kindling-side fixture: a known invocation
    produces a known row, asserted with a test analogous to the
    INTD-014 conformance fixture. The fixture iterates the
    *registered* command/method list — adding a command without an
    observation fails the test (R2 mitigation).
- **Coordinates with:** TRACE-004 — incoming `traceparent` is on the
  current span; USAGE reads from there.
- **Coordinates with:** FLAGS — active flag set is the inline resolved
  flag context defined by ADR-041.
- **Files (best-effort):** `crates/anvil-cli/src/main.rs`,
  `crates/anvil-intercept/src/ipc.rs`,
  `crates/<kindling-crate>/src/<observation-module>` (path TBD when
  the observation-kind decision lands), tests in the same crates,
  `docs/observability/usage-analytics.md` (new),
  `docs/observability/namespace-registry.md` (new
  `anvil.usage.*` row for any tracing-pipe breadcrumbs).
- **Validation:** TBD when picked up — at minimum (a) a unit test
  that a CLI invocation produces one Kindling row with the expected
  fields; (b) a unit test that a JSON-RPC call produces one row with
  the matching `traceparent`; (c) a unit test that a sensitive arg
  name results in a redacted value; (d) a unit test that the raw
  principal never appears in any captured field (only the salted
  hash); (e) the contract test against the registered command/method
  list.
- **Confidence:** medium — OQ2 and OQ5 are resolved; OQ1 (reuse vs new
  observation kind) is folded into this task's scope per the 2026-05-30
  operator authorisation and is decided as the first step of execution.
- **Status:** Ready

---

### USAGE-002: Flag-context correlation on usage rows

- **Intent:** Every usage row carries the resolved flag context active
  at invocation time so "which gates fired for whom" can join through
  the canonical flag key without duplicating standalone flag-evaluation
  facts.
- **Expected Outcome:**
  - The active flag set is captured as a stable, queryable inline
    `flag_set` field on every USAGE-001 observation, per
    [ADR-041](../decisions/041-flag-snapshot-usage-join-contract.md):
    sorted by manifest `key`, with resolved variant, resolution source,
    and whether the entry was gate-affecting. The field contains flags
    resolved or inherited as active context for that invocation; it is
    not a full dump of every manifest entry, and producers must not
    re-evaluate unrelated flags solely to populate it.
  - A canned query / view in
    `docs/observability/usage-analytics.md` answering "for command
    X, which flag set was active across the invocations this week?".
  - Canned gate-affecting queries join usage rows to ADR-019
    `flags_consulted` data by manifest `key`. Non-gate-affecting flags
    are available only as inline invocation context, not as standalone
    Kindling join rows.
- **Coordinates with:** USAGE-001 (extends the row shape).
- **Coordinates with:** FLAGS module (consumes the resolver's resolved
  values at the invocation boundary; does not re-evaluate or require a
  separate FLAGS-published snapshot row).
- **Files (best-effort):**
  `crates/<kindling-crate>/src/<observation-module>`,
  `docs/observability/usage-analytics.md`,
  `plans/decisions/019-flags-observability-alignment.md`
  (cross-link only),
  `plans/decisions/041-flag-snapshot-usage-join-contract.md`.
- **Validation:** TBD when picked up — at minimum a query test that
  joins a known invocation to its known flag set and returns the
  expected pairing.
- **Confidence:** medium
- **Status:** Draft

---

### USAGE-003: Dev-investment query views

- **Intent:** The founder has a small, named set of queries answering
  "what is being used vs not" without writing ad-hoc SQL each time.
- **Expected Outcome:**
  - A runbook / docs page enumerating: top N commands by invocation
    count (week / month / since launch), commands never invoked,
    flag-dependent paths exercised vs not, principals by activity
    level (per the OQ2 anonymisation contract).
  - Query helpers shipped either as CLI subcommands
    (`anvil kindling usage <view>`) or as a documented `kindling`
    query — decided in this task.
  - A short "how to use this for dev investment decisions" note in
    the same doc — explicitly: this is *signal*, not *evidence*.
    Small populations, flag bias, and survivorship effects mean the
    views inform direction, not decisions in isolation.
- **Coordinates with:** USAGE-001, USAGE-002.
- **Coordinates with:** OBS-001 — when OBS reaches Ready, this view
  set becomes a row in its signal inventory contract.
- **Files (best-effort):**
  `docs/observability/usage-analytics.md`,
  `crates/anvil-cli/src/...` (only if CLI subcommand is the chosen
  shape).
- **Validation:** TBD when picked up — at minimum a smoke test for
  each canned view returning a non-empty result against a fixture
  Kindling state.
- **Confidence:** medium-low — depends on USAGE-001/-002 shapes.
- **Status:** Draft

## Risks

- **R1 (privacy):** Recording who ran which command is itself
  sensitive, even without args or results. Mitigation: OQ2's
  anonymisation strategy plus the `SENSITIVE_FIELDS` deny-list on
  any arg-shape capture. Founder reviews the privacy contract before
  USAGE-001 reaches Ready.
- **R2 (silent drift — new commands skip USAGE):** A new CLI
  subcommand or JSON-RPC method ships without emitting a usage
  observation, and the dev-investment views silently undercount it.
  Mitigation: USAGE-001's contract test iterates the *registered*
  command/method list — adding one without an observation fails the
  test. Decide the registration shape in USAGE-001.
- **R3 (Kindling row volume):** High-frequency JSON-RPC calls
  (per-keystroke completions, scan probes) could fill Kindling with
  per-invocation noise the way ADR-019 explicitly pushed back on for
  flag evaluations. Mitigation: USAGE-001 emits one row **per
  user-initiated command**, not per internal call. Internal RPCs
  triggered by a single user command share that command's row via
  `traceparent` (the debug breadcrumbs stay on the tracing pipe).
- **R4 (anonymisation drift):** The principal field's anonymisation
  policy gets weakened over time as someone adds "just one debug
  field". Mitigation: the privacy contract is a separate doc, not
  code comments, and changes to it require founder review.

## Open questions

- **OQ1 (founder lean 2026-05-10 → new kind; OQ5 resolved):**
  Introduce a new `command.invoked` Kindling observation kind. Final
  decision recorded in USAGE-001; ADR-041 confirms FLAGS does not
  already publish a row shape that covers USAGE-002.
- **OQ2 (resolved 2026-05-11):** Principal anonymised via one-way
  hash with a per-deployment salt held in the existing secrets
  store. Salt rotation is a deliberate privacy reset, not routine.
  Same-person → same hash within a deployment; no cross-deployment
  join; distinct-user counts remain answerable; raw IDs never land
  in logs. Codified in the Privacy Contract section above.
- **OQ3:** Whether USAGE-003's canned views ship as CLI subcommands,
  documented `kindling` queries, or both. Decided in USAGE-003.
- **OQ4 (deferred):** External analytics pipeline (Mixpanel /
  PostHog / warehouse). Out of scope for v1. Revisit when
  evidence-based dev-investment decisions need cross-system joins
  or when a third party (board / investor) asks for usage rollups
  outside Anvil.
- **OQ5 (resolved 2026-05-11 by [ADR-041](../decisions/041-flag-snapshot-usage-join-contract.md)
  via [`FLAGCAT-007`](./feature-flag-catalogue.aps.md#flagcat-007)):**
  The catalogue/FLAGS do not publish a separate per-invocation resolved
  snapshot row. USAGE-002 stores the resolved flag context inline on the
  usage row as `flag_set`, sorted by manifest `key`, with resolved
  variant, resolution source, and gate-affecting marker. It contains
  flags resolved or inherited at the invocation boundary, not every
  manifest flag. The manifest `key` is the stable join key; changing it
  creates a new logical flag, retired keys are reserved for historical
  queries, and `createdFor` is provenance only. ADR-019 is not widened:
  non-gate-affecting flags have inline usage context only and no
  standalone Kindling row to join to.
