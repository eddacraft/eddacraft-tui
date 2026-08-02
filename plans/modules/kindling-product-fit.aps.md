<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if work items exist and status is Ready. -->

# Kindling Product Fit and Anvil Integration

| ID   | Owner  | Priority | Status | Progress |
| ---- | ------ | -------- | ------ | -------- |
| KFIT | @aneki | high     | In Progress | 0/11     |

**Last reviewed:** 2026-08-03 — KFIT-006 entered In Progress for the
operator-approved, default-off embedded-runtime consumption seam. The original
2026-07-18 usefulness and fit-for-purpose review of the sibling
`eddacraft/kindling` repository and anvil's live kindling integration found a
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
command-usage views. This does not satisfy ADR-035's write-once, retained,
queryable source-of-truth commitment.

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

- [ ] KFIT-001 product/ownership decision accepted by the operator.
- [ ] The authoritative Anvil observation-kind inventory is agreed.
- [ ] Retention policy is resolved for memory facts versus governance facts.
- [ ] Sidecar migration and rollback posture is decided.
- [ ] Cross-repository release order and compatibility floor are recorded.
- [ ] KFIT-002..011 are revalidated against current Kindling and Anvil `main`.

## Work Items

### Phase 0 — Contract and ownership

#### KFIT-001: Reconcile the product contract and APS authority

- **Status:** Draft
- **Intent:** Decide the supported Kindling product journeys and the exact
  governance record Anvil requires before extending either implementation.
- **Expected Outcome:** An accepted decision/spec defines standalone memory
  users, Anvil's consumer profile, the authoritative observation inventory,
  retention classes, identifier semantics, migration posture, and ownership
  across KFIT, KINTEG, CONV, DPO, KDS, USAGE, and MLP2. Stale “blocked on KDS”
  and “source of truth” claims are corrected without rewriting shipped history.
- **Validation:** `pnpm aps:active-lint && pnpm aps:index:check && pnpm docs:check`
  in Anvil, plus the sibling Kindling APS lint.
- **Files:** `plans/specs/` or `designs/`, this module, affected module/index
  records in both repositories.
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

#### KFIT-005: Publish and harden the embedded downstream runtime

- **Status:** Draft
- **Intent:** Rust consumers should get one supported dependency for lifecycle,
  durable append, capability negotiation, reads, and diagnostics.
- **Expected Outcome:** A current `kindling-runtime` release exposes
  attach-or-start, embedded daemon ownership, spooled append/flush/status,
  capability checks, and observation list/retrieval delegates without requiring
  the CLI on `PATH`. Its compatibility floor and shutdown semantics are
  documented and tested for Anvil.
- **Validation:** `cargo test -p kindling-runtime --all-features && cargo clippy -p kindling-runtime --all-features -- -D warnings`, package-content check, and clean scratch-crate install.
- **Dependencies:** KFIT-001
- **Repositories:** `eddacraft/kindling`
- **Confidence:** high

### Track B — Anvil-backed Kindling

#### KFIT-006: Adopt the embedded runtime with stable repository routing

- **Status:** In Progress
- **Intent:** Anvil should own a usable Kindling runtime without depending on an
  external executable or daemon-start working directory.
- **Expected Outcome:** The shipped Anvil binary uses the supported embedded
  runtime, starts or attaches once, routes each observation by canonical
  workspace identity, and shuts down only a daemon it owns. CLI and subdirectory
  invocations resolve to the same repository scope.
- **Validation:** Focused Anvil tests cover clean-host startup with no `kindling`
  binary, attach to an existing daemon, subdirectory routing, two-repository
  isolation, and owned-versus-attached shutdown.
- **Current Slice:** Operator-approved 2026-08-03 precursor: add
  `kindling-runtime` as an optional Cargo dependency, compile a typed consumption
  seam only under an opt-in Cargo feature, and register an active rollout flag
  that resolves disabled by default. Do not enable that Cargo feature in release
  builds or start/attach an embedded daemon in the default path. KFIT-005 still
  gates activation and release packaging after the next kindling publication.
- **Pull Request:** [#3489](https://github.com/eddacraft/anvil-001/pull/3489)
  (draft; base `main`).
- **Slice Validation:** `cargo test -p eddacraft-anvil feature_flags && cargo
  test -p eddacraft-anvil --features kindling-embedded-runtime kindling_ &&
  cargo check -p eddacraft-anvil && cargo check -p eddacraft-anvil --features
  kindling-embedded-runtime && cargo hakari generate --diff && cargo hakari
  verify`, plus the flags-catalogue tests and APS/docs checks.
- **Files:** `crates/anvil-cli/Cargo.toml`, `crates/anvil-cli/src/feature_flags.rs`,
  `crates/anvil-cli/src/kindling_runtime.rs`, `crates/anvil-cli/src/main.rs`,
  `flags/manifest.json`, `packages/anvil/flags-catalogue/tests/manifest.test.ts`,
  `Cargo.lock`
- **Dependencies:** KFIT-005 for activation and release packaging; the disabled
  compile seam may land before KFIT-005 under the 2026-08-03 operator direction.
- **Repositories:** `eddacraft/anvil-001`
- **Confidence:** medium

#### KFIT-007: Route every declared Anvil observation through one typed sink

- **Status:** Draft
- **Intent:** Every governance fact Anvil claims to capture should reach the
  daemon-backed store through one non-blocking, privacy-preserving contract.
- **Expected Outcome:** CLI and JSON-RPC commands, save-time and mid-edit gates,
  fences, post-hooks, audit-chain runs, and false-positive reports either map to
  a reviewed Kindling kind/provenance shape and persist, or are explicitly
  removed from the capture contract. No production producer uses a no-op sink.
  Back-pressure, redaction, and drop semantics are consistent across kinds.
- **Validation:** A table-driven parity suite emits every supported Anvil kind,
  reads it back from a real in-process runtime, and proves redaction and
  non-blocking behaviour.
- **Dependencies:** KFIT-001, KFIT-006
- **Repositories:** `eddacraft/anvil-001`
- **Confidence:** medium

#### KFIT-008: Consolidate session, scope, and correlation identity

- **Status:** Draft
- **Intent:** Stored events should join into meaningful Anvil runs and
  repository histories instead of carrying unrelated per-call UUIDs.
- **Expected Outcome:** Canonical repository identity, work-session/run
  identity, event identity, `gate_eval_id`, and `traceparent` rules apply across
  CLI, daemon, hooks, audit, spool replay, and reads. Kindling capsules are used
  only if KFIT-001 decides they add real lifecycle value; otherwise the
  non-capsule contract is explicit.
- **Validation:** Cross-surface correlation tests prove related events join and
  unrelated repositories/sessions do not collide.
- **Dependencies:** KFIT-001, KFIT-007
- **Repositories:** `eddacraft/anvil-001`, `eddacraft/kindling`
- **Confidence:** low

#### KFIT-009: Migrate and retire parallel observation sidecars

- **Status:** Draft
- **Intent:** Kindling should become Anvil's actual retained source of truth
  without silently losing existing local history or rollback safety.
- **Expected Outcome:** A bounded, idempotent migration imports supported rows
  from `usage.ndjson`, `audit-chain.ndjson`, and false-positive storage with
  stable IDs and repository scope; readers remain compatible during the
  transition; successful cutover retires production sidecar writers and
  documents rollback and backup behaviour.
- **Validation:** Migration fixtures cover mixed versions, corrupt lines,
  duplicates, missing repository identity, interrupted retry, and read parity
  before/after cutover.
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
  runtime health, scope, spool depth, replay failures, redaction evidence, and
  dropped rows. DPO-003 consumes this foundation; DPO-004/-005 remain DPO-owned.
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
- **Expected Outcome:** `@eddacraft/anvil-kindling-integration` is either reduced
  to an actively consumed compatibility contract or deprecated and removed;
  TypeScript/Rust kind registries cannot drift; Anvil and Kindling docs describe
  the same install, capture, retention, query, privacy, and failure behaviour;
  release records name the compatibility floor and migration path.
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

## Open Questions

1. Is Kindling one product with standalone-memory and governance-ledger
   profiles, or should the embedded runtime be positioned as a substrate under
   a separately named standalone UX?
2. Which Anvil facts are governance-grade enough for retention, and which are
   tracing-only noise under ADR-035?
3. Should Anvil-specific kinds become typed Kindling kinds, generic kinds with
   typed provenance, or a versioned Anvil payload registry over generic kinds?
4. What retention class applies to governance evidence versus ordinary coding
   memory, and which operator can change it?
5. Should legacy sidecars be auto-imported, explicitly imported, or only read
   during a bounded compatibility window?
6. Does `@eddacraft/anvil-kindling-integration` have a live consumer after the
   Rust cutover, or should its remaining contract move to generated fixtures and
   Rust types?
