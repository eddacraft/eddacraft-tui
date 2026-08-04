<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if work items exist and status is Ready. -->

# Kindling Product Fit and Anvil Integration

| ID   | Owner  | Priority | Status | Progress |
| ---- | ------ | -------- | ------ | -------- |
| KFIT | @aneki | high     | In Progress | 0/11     |

**Last reviewed:** 2026-08-03 — the operator accepted the KFIT-001 product,
retention, admission, migration, and ownership contract. PR #3489 merged the
default-off embedded-runtime consumption seam, and `kindling-runtime` 0.3.0 is
published; neither fact activates the embedded profile or completes KFIT-005 or
KFIT-006. The original 2026-07-18 usefulness and fit-for-purpose review found a
sound daemon/client/spool slice for daemon-originated `command.invoked` rows,
but not the coherent memory product or authoritative governance store described
by the two repositories.

## Purpose

Make Kindling useful in both roles it currently claims:

1. a standalone, local memory product that helps a developer recover useful
   context across coding sessions; and
2. Anvil's durable, queryable system of record for governance facts under
   [ADR-035](../decisions/035-three-pipe-observability-rule.md).

This is one vertical module because the product contract and the downstream
integration constrain each other. Kindling must expose reliable capture,
retrieval, lifecycle, and embedded-runtime mechanisms; Anvil must consume those
mechanisms through one truthful storage path without teaching Kindling Anvil
policy.

## Problem

Kindling's core storage, daemon, client, spool, retrieval, and embedded runtime
are technically credible. Its standalone usefulness is weakened by incomplete
or narrower-than-advertised adapter journeys, weak cross-session explanation,
and unclear lifecycle feedback.

Anvil uses only a narrow part of that engine. Its daemon-backed sink stores
`command.invoked` rows, while ordinary CLI usage, save-time verdicts, fences,
audit-chain facts, and false-positive reports remain split across several
NDJSON sidecars. Mid-edit and post-hook observation builders exist but do not
have a production Kindling delivery path. The only first-class reader provides
command-usage views. This does not satisfy ADR-035's retained, queryable
source-of-truth commitment or ADR-116's normal-operation append-only rule.

The result is a contract gap in both directions: Anvil is not a convincing
proof of Kindling's memory usefulness, and Kindling is not yet Anvil's single
governance record.

## Success Criteria

- [ ] A supported standalone install-to-recall journey works end to end for
      each integration advertised as supported; unsupported paths are labelled
      honestly.
- [ ] Retrieval can recover relevant context across sessions and explain why
      each result was returned.
- [ ] Users can see memory health, current summary, retention, redaction, and
      deletion state without inspecting SQLite or spool files.
- [ ] Anvil starts or attaches to Kindling without requiring a separately
      installed `kindling` executable.
- [ ] Every Anvil governance observation declared in scope reaches one
      daemon-backed Kindling store or is explicitly removed from the contract.
- [ ] Production code contains no no-op Kindling producer for an observation
      claimed as captured.
- [ ] `usage.ndjson`, `audit-chain.ndjson`, and false-positive sidecars are no
      longer parallel sources of truth; migration and rollback are explicit.
- [ ] Repository, session, event-kind, and correlation identifiers are stable
      across CLI, daemon, hook, and read paths.
- [ ] Anvil exposes command usage and governance-history queries plus daemon,
      spool, replay, redaction, and dropped-row diagnostics.
- [ ] The TypeScript integration package and public documentation describe the
      shipped Rust architecture, or are deprecated and removed.
- [ ] Clean-install and outage/recovery tests prove the complete Kindling to
      Anvil flow without mutating a real user profile.

## In Scope

### Track A — Kindling product usefulness

- Standalone product positioning and supported-journey truth.
- Claude Code and VS Code-family capture/onboarding contracts.
- Cross-session retrieval, result explanation, summaries, and lifecycle
  visibility.
- Retention, redaction, forgetting, health, and spool diagnostics needed to
  trust the local memory.
- The published `kindling-runtime` contract Anvil consumes.

### Track B — Anvil-backed Kindling

- Embedded runtime adoption and stable repository routing.
- One typed mapping for every in-scope Anvil observation kind.
- Real sink wiring for CLI, JSON-RPC, save-time, mid-edit, fence, hook,
  audit-chain, and false-positive producers.
- Sidecar migration and retirement.
- Governance queries and operator diagnostics that unblock DPO consumers.
- Contract/package/documentation reconciliation.

## Out of Scope

- Cloud-hosted memory, cross-machine synchronisation, embeddings, or model-led
  interpretation inside Kindling.
- Moving Anvil policy or aggregation semantics into Kindling; Kindling remains
  mechanism, not policy.
- Replacing Anvil's notification or tracing pipes; ADR-035's three-pipe split
  remains authoritative.
- Implementing the DPO dashboard components themselves. DPO-004/-005 retain
  ownership after this module supplies their durable read foundation.
- Remote fleet telemetry. FLEET remains a separate consent and egress posture.

## Interfaces

**Depends on:**

- Kindling `05-rust-port`, `06-downstream-integration-surface` (KINTEG), and
  `08-conversion-surface` (CONV) modules in the sibling repository.
- Published `kindling-runtime`, `kindling-client`, daemon list API, capability
  handshake, deduplicated replay, and bounded spool support.
- [ADR-035](../decisions/035-three-pipe-observability-rule.md) — Kindling owns
  durable governance facts.
- [ADR-064](../decisions/064-intercept-graph-cache-crate-boundary.md) — Anvil's
  intercept crate remains transport-independent.
- [ADR-088](../decisions/088-dpo-observation-kind-taxonomy.md) — save-time and
  fence observation taxonomy.

**Coordinates with:**

- [usage-analytics](../archive/modules/usage-analytics.aps.md) (USAGE) — command
  shape, privacy, principal, flags, and existing usage views.
- [kindling-daemon-sink](../archive/modules/kindling-daemon-sink.aps.md) (KDS) —
  the proven command-only daemon/spool path this module generalises.
- [daemon-protection-observability](./daemon-protection-observability.aps.md)
  (DPO) — owns governance-history consumer semantics and dashboard components;
  its stale KDS blockers must be reconciled during KFIT-001.
- MLP2 observation builders and producers — mid-edit, post-hook, and audit-chain
  facts already modelled in Anvil.
- TRACE — cross-pipe `traceparent` correlation and redaction parity.

**Exposes:**

- A truthful supported-integration matrix and standalone memory journey.
- A current, published embedded Kindling runtime contract.
- One Anvil governance observation sink and one retained store.
- Stable Anvil usage/governance query and diagnostic surfaces.
- Migration evidence sufficient to retire the legacy sidecars and stale
  TypeScript contract.

## Ready Checklist

- [x] KFIT-001 product/ownership decision accepted by the operator.
- [x] The authoritative Anvil observation-kind inventory is agreed.
- [x] Retention policy is resolved for memory facts versus governance facts.
- [x] Sidecar migration and rollback posture is decided.
- [ ] The planned `kindling-runtime` 0.4.0 / `generic-event@1` compatibility
      floor is confirmed by a linked Kindling KINTEG/CONV change and fresh
      `aps lint plans` evidence.
- [x] KFIT-002..011 are revalidated against current Kindling and Anvil `main`.

## Work Items

### Phase 0 — Contract and ownership

#### KFIT-001: Reconcile the product contract and APS authority

- **Status:** In Progress
- **Intent:** Decide the supported Kindling product journeys and the exact
  governance record Anvil requires before extending either implementation.
- **Expected Outcome:** ADR-116 records one kindling product with standalone
  memory and embedded anvil-governance profiles; a generic kindling event
  carrying the Rust-authoritative `anvil.governance.v1` envelope; the selected
  governance inventory and durable-admission rule; explicit-prune-only
  governance retention, 30-day usage retention, and a 7-day/64 MiB outage
  spool; separate canonical repository/worktree identities and local
  authorisation; a crash-durable gap ledger and bounded store quotas; explicit,
  idempotent, forward-only-after-cutover sidecar migration with dry-run default,
  atomic state, backup, ambiguous-scope skips, and one-release dual-read without
  dual-write; immediate TypeScript integration-package removal at cutover; and
  ownership across KFIT, KINTEG, CONV, DPO, KDS, USAGE, and MLP2. Stale “blocked
  on KDS” and “source of truth” claims are corrected without rewriting shipped
  history.
- **Validation:**
  - `pnpm aps:active-lint`
  - `pnpm aps:index:check`
  - `pnpm docs:check`
  - `pnpm adr:check`
  - `aps lint plans` in `eddacraft/kindling`
- **Files:** `plans/decisions/116-kindling-product-profiles-and-governance-record.md`,
  the narrow ADR-035/088/089 amendments, `plans/decisions/DECISION-LOG.md`, this
  module, and affected KINTEG/CONV plan records in `eddacraft/kindling`.
- **Dependencies:** None for the contract; implementation items consume the
  accepted ADR in dependency order.
- **Design Source:** Operator-approved design session on 2026-08-03; ADR-116 is
  the durable deliverable.
- **Pull Request:** [#3506](https://github.com/eddacraft/anvil-001/pull/3506)
  (draft; base `main`).
- **Risk:** high
- **PR Base:** `main`
- **Confidence:** high

### Track A — Kindling product usefulness

#### KFIT-002: Make advertised capture journeys truthful end to end

- **Status:** Draft
- **Intent:** A developer can install an advertised adapter and capture useful
  events without discovering a stub, incompatible hook contract, or hidden
  manual setup.
- **Expected Outcome:** Claude Code initialisation and hook payloads work against
  the current CLI, VS Code-family capture has a supported setup path, and the
  integration matrix is generated or tested from executable capability truth.
  Any intentionally unsupported path is labelled unsupported rather than
  advertised.
- **Validation:** Fresh-profile integration tests drive install, initialise,
  capture, status, and retrieval for every supported adapter.
- **Dependencies:** KFIT-001
- **Repositories:** `eddacraft/kindling`
- **Confidence:** medium

#### KFIT-003: Deliver useful cross-session retrieval and explanation

- **Status:** Draft
- **Intent:** Retrieval should restore relevant working context rather than
  merely prove that observations were stored.
- **Expected Outcome:** CLI and editor searches can cross session boundaries
  within an explicit repository scope; pins, current summary, and ranked hits
  are distinguishable; each result exposes deterministic provenance and ranking
  reasons sufficient for a user to judge relevance.
- **Validation:** Retrieval tests cover prior-session recall, repository
  isolation, pin/summary precedence, deterministic ranking, and explanation
  output in CLI and VS Code-family surfaces.
- **Dependencies:** KFIT-001
- **Repositories:** `eddacraft/kindling`
- **Confidence:** medium

#### KFIT-004: Make memory lifecycle and trust visible

- **Status:** Draft
- **Intent:** Users should understand what Kindling retained, redacted, forgot,
  summarised, or failed to deliver without reading internal files.
- **Expected Outcome:** Status and browse surfaces report active repository and
  session scope, current summary, retention posture, redacted/forgotten counts,
  daemon ownership, spool pending/dropped/replay state, and actionable degraded
  conditions. Forget and retention operations have observable confirmation.
- **Validation:** Lifecycle integration tests cover normal, redacted, forgotten,
  expired, daemon-down, spooled, replayed, and dropped states.
- **Dependencies:** KFIT-001
- **Repositories:** `eddacraft/kindling`
- **Confidence:** medium

#### KFIT-005: Harden the published embedded downstream runtime

- **Status:** In Progress
- **Intent:** Rust consumers should get one supported dependency for lifecycle,
  durable append, capability negotiation, reads, and diagnostics.
- **Expected Outcome:** Planned `kindling-runtime` 0.4.0 exposes
  `generic-event@1` plus attach-or-start, embedded daemon ownership, spooled
  append/flush/status, capability checks, and observation list/retrieval
  delegates without requiring the CLI on `PATH`. The generic outer envelope
  preserves an opaque, size-bounded Anvil payload without adding Anvil policy to
  Kindling. The publishing KINTEG/CONV change, compatibility floor, capability
  negotiation, shutdown semantics, and replay behaviour are documented and
  tested for Anvil; releases through 0.3.x remain explicitly incompatible.
- **Validation:** `cargo test -p kindling-runtime --all-features && cargo clippy -p kindling-runtime --all-features -- -D warnings`, package-content check, and clean scratch-crate install.
- **Performance Contract:** On the 2026-08-03 16-logical-CPU Linux reference
  host, release-mode cold start p95 stays below 50 ms; warm append p95 below 1
  ms; 500-row daemon page p95 below 10 ms; ranked retrieval p95 below 50 ms at
  about 25k rows; outage append p95 below 5 ms with no positive backlog slope
  through 100k rows; replay above 2k rows/s. Full scans are export/projection
  rebuild operations, not interactive query primitives. Resource release gates
  require isolated-process deltas; shared-process values remain directional.
- **Current Slice:** Kindling PR
  [#143](https://github.com/eddacraft/kindling/pull/143) **merged** to `main`
  at `f6dcd7d` (KINTEG-013/014, private `kindling-bench`, isolated-process +
  logical I/O). Post-merge isolated re-bench filed as
  `benchmarks/history/kindling/2026-08-04.json` (all budgeted workloads pass).
  Anvil evidence PR
  [#3515](https://github.com/eddacraft/anvil-001/pull/3515). `cargo package
  --no-verify` succeeds for core crates; full `--verify` + clean scratch
  install + **0.4.0-class publish** under release authority remain open before
  KFIT-006 activation.
- **Dependencies:** KFIT-001; KFIT-006 completion, KFIT-007 implementation, and
  KFIT-011 release activation require the published 0.4.0-or-newer compatible
  release, not merely this draft contract.
- **Repositories:** `eddacraft/kindling`
- **Confidence:** high

### Track B — Anvil-backed Kindling

#### KFIT-006: Adopt the default-off embedded runtime facade

- **Status:** In Progress
- **Intent:** Anvil should compile and test one supported embedded Kindling
  lifecycle facade without depending on an external executable or activating
  governance admission prematurely.
- **Expected Outcome:** The shipped Anvil binary contains a default-off embedded
  facade that can start or attach once in injected tests, accepts a supplied
  canonical repository/worktree scope, uses owner-restricted local IPC, and
  shuts down only a daemon it owns. This item does not resolve identity, wire the
  governance sink, migrate a profile, or activate the release path; KFIT-007,
  KFIT-008/-009, and KFIT-011 own those later boundaries.
- **Validation:** Focused Anvil tests cover clean-host injected startup with no
  `kindling` binary, attach to an existing daemon, supplied-scope forwarding,
  owner-restricted IPC, owned-versus-attached shutdown, and proof that release
  defaults never activate the facade.
- **Current Slice:** Merged 2026-08-03 via PR #3489: add
  `kindling-runtime` as an optional Cargo dependency, compile a typed consumption
  seam only under an opt-in Cargo feature, and register an active rollout flag
  that resolves disabled by default. Do not enable that Cargo feature in release
  builds or start/attach an embedded daemon in the default path. KFIT-005 gates
  completion of this facade after runtime hardening is published; KFIT-007 owns
  the default-off typed sink, KFIT-009 owns local cutover, and KFIT-011 owns
  release activation.
- **Pull Request:** [#3489](https://github.com/eddacraft/anvil-001/pull/3489)
  (merged to `main`; unreleased).
- **Slice Validation:** `cargo test -p eddacraft-anvil feature_flags && cargo
  test -p eddacraft-anvil --features kindling-embedded-runtime kindling_ &&
  cargo check -p eddacraft-anvil && cargo check -p eddacraft-anvil --features
  kindling-embedded-runtime && cargo hakari generate --diff && cargo hakari
  verify`, plus the flags-catalogue tests and APS/docs checks.
- **Files:** `crates/anvil-cli/Cargo.toml`, `crates/anvil-cli/src/feature_flags.rs`,
  `crates/anvil-cli/src/kindling_runtime.rs`, `crates/anvil-cli/src/main.rs`,
  `flags/manifest.json`, `packages/anvil/flags-catalogue/tests/manifest.test.ts`,
  `Cargo.lock`
- **Dependencies:** KFIT-001 and KFIT-005. The merged disabled compile seam is a
  precursor, not activation evidence; KFIT-008 identity precedes KFIT-007 sink
  implementation and KFIT-009 cutover, while KFIT-011 owns release activation.
- **Repositories:** `eddacraft/anvil-001`
- **Confidence:** medium

#### KFIT-007: Implement one typed governance sink behind the default-off gate

- **Status:** Draft
- **Intent:** Every governance fact Anvil claims to capture should reach the
  daemon-backed store through one non-blocking, privacy-preserving contract.
- **Expected Outcome:** CLI and JSON-RPC commands, save-time and mid-edit gates,
  fences, post-hooks, audit-chain runs, and false-positive reports either map to
  a reviewed Anvil-owned `anvil.governance.v1` payload kind and provenance shape
  carried by upstream `generic-event@1`, or are explicitly removed from the
  capture contract. No production producer uses a no-op sink.
  A selected event counts as recorded only after kindling or the bounded spool
  accepts it. Admission failure does not change anvil's verdict, but it creates
  queryable `recording_gap` health evidence through a preallocated, checksummed,
  crash-durable ledger. This item includes the minimum status surface for gap,
  quota, spool, and ledger health; redaction and strict schema validation are
  consistent across producer, store, replay, read, and migration boundaries;
  save-time I/O remains within its latency budget. The embedded profile remains
  air-gapped and enforces the 1 GiB profile, 256 MiB repository, 256 KiB event,
  and low-disk ceilings from ADR-116.
- **Validation:** A table-driven parity suite emits every supported Anvil kind,
  reads it back from a real in-process runtime, and proves redaction, durable
  admission, visible recording gaps, owner isolation, generated-contract parity,
  malicious-input rejection, crash/restart recovery, quota/low-disk behaviour,
  two-phase spool expiry/eviction accounting with a crash at every transition,
  64-way concurrent intent saturation, cross-run tuple-table overflow,
  same-scope/same-sequence/different-run deduplication, two separated aggregate
  overflow episodes across restart with distinct incident IDs, and bounded
  save-time latency. The air-gap harness proves no socket or egress
  regression. At-rest tests cover wrong owner/DACL, permissive mode, symlink,
  hard link, and pre-created store, spool,
  ledger, registry, and receipt paths on Unix and Windows. Receipt tests cover
  zero-result refusal, reserve exhaustion, atomic refusal, explicit reserve
  increase, continuity, repository-scoped and aggregate-gap round trips, and
  rejection of profile-administrative scope on any other row. Tests prove
  production routing remains default-off after the sink, ledger, and minimum
  status exist.
- **Dependencies:** KFIT-001, KFIT-005, KFIT-006, KFIT-008.
- **Repositories:** `eddacraft/anvil-001`
- **Confidence:** medium

#### KFIT-008: Consolidate session, scope, and correlation identity

- **Status:** Draft
- **Intent:** Stored events should join into meaningful Anvil runs and
  repository histories instead of carrying unrelated per-call UUIDs.
- **Expected Outcome:** ADR-036's tracked `anvil/project-id` `project_uuid`
  remains the repository identity, while a subordinate owner-only UUID in each
  per-worktree Git administrative directory replaces raw-root worktree identity
  and current-working-directory fallback. An owner-only registry outside Git
  binds each active pair to one Git-admin instance; copied metadata fails closed
  until an explicit move/copy rebind.
  Work-session/run identity, event identity, `gate_eval_id`, and `traceparent`
  rules apply across CLI, daemon, hooks, audit, spool replay, and reads. The
  daemon registers and authorises the exact repository/worktree pair. Clones and
  forks inherit project identity under ADR-036 but receive new local worktree
  IDs; nested repositories, linked worktrees, copied metadata, symlinks, owner
  mismatch, and ambiguous scope fail closed with a visible gap.
- **Validation:** Cross-surface correlation and isolation tests cover moves,
  clones/forks inheriting ADR-036 project identity, explicit new-project
  identity, recursive filesystem copies with the original both present and
  absent, copy/move rebind, nested repositories, linked worktrees,
  subdirectories, symlinks, owner mismatch, daemon registration, replay, and
  migration; related events join and unrelated scopes do not collide.
- **Dependencies:** KFIT-001, KFIT-005, and the default-off KFIT-006 consumption
  seam. KFIT-007 implementation depends on this item.
- **Repositories:** `eddacraft/anvil-001`, `eddacraft/kindling`
- **Confidence:** low

#### KFIT-009: Migrate and retire parallel observation sidecars

- **Status:** Draft
- **Intent:** Kindling should become Anvil's actual retained source of truth
  without silently losing existing local history or rollback safety.
- **Expected Outcome:** `anvil kindling migrate` is dry-run by default and a
  bounded, idempotent `--apply` migration imports supported rows from
  `usage.ndjson`, `audit-chain.ndjson`, and false-positive storage with
  stable IDs and repository scope; readers remain compatible during the
  transition; ambiguously scoped rows are reported and skipped, source files
  are backed up, and a persisted `discovered -> validated -> backed_up ->
  importing -> verified -> fenced -> cutover` state machine makes interruption
  resumable.
  Owner, regular-file, no-symlink/hardlink, path-confinement, checksum, fsync,
  canonical row-digest parity, generation ownership, redaction, and divergence
  checks guard the transaction. Existing IDs are duplicates only when the full
  post-redaction row digest matches; collisions block. One release keeps
  dual-read compatibility without dual-write. Rollback is supported only
  before writer cutover; after the synchronised cutover marker, downgrade is
  unsupported and recovery is forward-only. Successful local cutover retires
  production sidecar writers and atomically activates canonical routing for that
  migrated profile; KFIT-011 later owns default release activation. The
  payload-free manifest remains verification-only: it cannot replay lost rows,
  and post-cleanup canonical loss produces degraded status plus bounded or
  unknown `recording_gap` evidence rather than a false restoration claim.
- **Validation:** Migration fixtures cover mixed versions, legacy
  `gate.evaluated` mapping, hostile and pre-redaction rows, corrupt lines,
  duplicates, missing repository identity, symlink/hardlink/cross-root inputs,
  permission mismatch, interruption at every checkpoint, late sidecar writes,
  concurrent supported writers at the final generation fence,
  identical-ID/different-payload collision, generation-owned rollback that
  preserves identical pre-existing rows, post-cutover forward repair, prune
  refusal while any legacy source/backup survives, cleanup of both retired
  sources and backups, post-cleanup canonical loss without manifest replay, and
  count/digest read parity.
- **Dependencies:** KFIT-007, KFIT-008
- **Repositories:** `eddacraft/anvil-001`
- **Confidence:** low

#### KFIT-010: Expose governance queries and operational status

- **Status:** Draft
- **Intent:** Developers and operators should be able to use and trust the
  retained governance record through supported Anvil commands.
- **Expected Outcome:** Existing usage views read the canonical store, a
  governance-history surface exposes gate/fence/action/audit facts with bounded
  filters and evidence-preserving output, and an Anvil status surface reports
  runtime health, canonical scope, store/repository quotas, reserved integrity
  space, spool depth, replay failures, redaction evidence, prune receipts, and
  `recording_gap` rows. KFIT-007's minimum gap-health surface is a prerequisite,
  not work deferred to this item. Aggregation policy remains in Anvil:
  interactive counts use an Anvil-owned incremental projection/cache fed by
  typed observations, while bounded Kindling repo/kind/time pages support recent
  history and projection rebuilds. No Kindling server-side
  command/flag/principal vocabulary is added. DPO-003 consumes this foundation;
  DPO-004/-005 remain DPO-owned.
- **Validation:** CLI integration tests cover every view, daemon-down/read-only
  degradation, pagination completeness, JSON stability, repository isolation,
  and status transitions through outage and replay.
- **Dependencies:** KFIT-007, KFIT-009
- **Repositories:** `eddacraft/anvil-001`
- **Confidence:** medium

#### KFIT-011: Reconcile packages, documentation, and release evidence

- **Status:** Draft
- **Intent:** Public and developer-facing contracts should describe the system
  users actually receive after the cutover.
- **Expected Outcome:** `@eddacraft/anvil-kindling-integration` is removed in the
  cutover release immediately after Rust-generated TypeScript/JSON schemas and
  fixtures replace its useful contract; there is no deprecation release.
  That same release activates the embedded profile by default only after
  KFIT-009's fenced local cutover path and KFIT-010's supported query/status
  surface are green. Activation is per profile: a new profile with no legacy
  state or one with a valid persisted cutover marker selects canonical routing;
  any existing profile with a legacy source or backup and no marker stays on the
  legacy writer until explicit migration, with no automatic migration or
  dual-write.
  Anvil and Kindling docs describe the same install, capture, retention, query,
  privacy, and failure behaviour; release records name the compatibility floor
  and migration path.
- **Validation:** `pnpm format:check && pnpm aps:active-lint && pnpm aps:index:check && pnpm docs:check` in Anvil; Kindling format, docs, workspace tests, and package-readiness checks; one clean-install end-to-end acceptance run.
- **Dependencies:** KFIT-002, KFIT-003, KFIT-004, KFIT-005, KFIT-006, KFIT-007,
  KFIT-008, KFIT-009, KFIT-010
- **Repositories:** `eddacraft/anvil-001`, `eddacraft/kindling`
- **Confidence:** medium

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| The standalone-memory and governance-ledger roles force incompatible retention or query semantics | Medium | High | KFIT-001 defines separate consumer profiles and retention classes before implementation |
| Cross-repository changes ship in the wrong order | High | High | Record a Kindling compatibility floor; publish upstream before Anvil consumes it |
| Sidecar migration duplicates or mis-scopes facts | Medium | High | Stable IDs, idempotent import, dry-run report, backup, and parity fixtures |
| Embedded runtime regresses daemon latency or shutdown | Medium | High | Preserve Anvil's non-blocking boundary and test owned-versus-attached lifecycle |
| Richer capture weakens privacy | Medium | High | Default-minimal fields, shared redaction, explicit path/snippet opt-ins, evidence tests |
| A malicious repository exhausts the local store or crosses another repository's scope | Medium | High | Owner-restricted local IPC, canonical repository/worktree registration, strict event limits, hard profile/repository quotas, low-disk refusal, and reserved gap-ledger capacity |
| DPO and KFIT duplicate query/dashboard ownership | Medium | Medium | KFIT owns storage and query foundation; DPO owns dashboard consumer semantics |
| Documentation continues to describe aspirational adapters | High | Medium | Test the supported matrix and remove unsupported claims in the same release |

## Decisions

1. **One vertical module, two repository tracks.** The user outcome crosses the
   Kindling product and Anvil integration boundary; splitting them would hide
   the compatibility and release-order gates.
2. **Kindling remains mechanism, not Anvil policy.** Anvil owns event selection,
   aggregation, and governance presentation; Kindling owns reliable local
   capture, lifecycle, storage, retrieval, and diagnostics.
3. **No dashboard duplication.** This module unblocks DPO's readers and
   dashboards but does not absorb DPO-004/-005.
4. **No new sidecar is an acceptable completion state.** Temporary migration
   files may exist, but the terminal architecture has one retained store and a
   bounded outage spool.
5. **One product, two profiles.** Standalone memory and embedded anvil
   governance share kindling's mechanism and lifecycle; embedded governance is
   a deployment profile inside the single shipped `anvil` binary.
6. **Selective durable governance.** The closed governance inventory is
   `gate_evaluated`, `constraint_applied`, `action_executed`,
   `false_positive_reported`, `recording_gap`, and the non-prunable
   `governance_pruned` receipt. Retain gate `fail`/`error` outcomes plus every
   explicit audit or pre-push outcome; enforcement remains independent context
   and does not select rows. Routine successful or skipped high-frequency
   mid-edit/save-time/pre-commit checks remain tracing-only; `command.invoked`
   is a separate short-retention usage envelope. Legacy `gate.evaluated` is a
   migration/dual-read alias only.
7. **Generic transport, anvil-owned schema.** Kindling stores a generic event;
   anvil owns the closed, versioned `anvil.governance.v1` envelope and generates
   TypeScript/JSON contracts from Rust authority. Upstream `generic-event@1`
   and the planned `kindling-runtime` 0.4.0 compatibility floor must publish
   before activation.
8. **Explicit retention, capacity, and migration.** Governance evidence has no
   automatic expiry and only the authenticated local governance-profile prune
   command can remove it, leaving a non-prunable receipt. This explicitly
   amends ADR-035/089 immutability wording. Usage rolls for 30 days; the store,
   per-repository payload, free-space, and 7-day/64 MiB outage-spool budgets are
   bounded. Sidecars migrate through a dry-run-first, resumable transaction;
   rollback ends at writer cutover and recovery is forward-only thereafter.
9. **Durable admission with crash-visible gaps.** Kindling or its bounded spool
   must accept selected evidence before it counts as recorded. A preallocated,
   checksummed write-ahead gap ledger survives crash, quota rejection, expiry,
   and eviction. Admission failure never changes the anvil verdict, but remains
   visible as status and a deduplicated `recording_gap` query fact.
10. **Ownership and sequence.** Runtime/transport stay in `anvil-cli`;
    `anvil-intercept` exposes transport-free traits; DPO owns dashboards. The
    default-off runtime seam and canonical identity precede the typed sink;
    local canonical routing activates only through the fenced migration cutover
    or clean initialisation with no legacy state. Full queries/status follow,
    then KFIT-011 owns per-profile release-default selection, package removal,
    docs, and release closeout without bypassing an existing profile's marker.
