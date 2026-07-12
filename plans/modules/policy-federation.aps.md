<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Policy Federation

| ID  | Owner | Priority | Status | Progress |
| ------ | ----- | -------- | ------ | -------- |
| POLFED | —     | medium   | Draft  | 0/8      |

**Last reviewed:** 2026-07-11 (post-POLRESET downstream coherence review —
`plans/reviews/2026-07-11-polreset-downstream-coherence.md`: module
**re-based**. The old "OPAE bundle primitives" prerequisite is void —
`bundle.rs` was deleted by ADR-098 PR-C and post-reset OPAE explicitly
excludes remote bundles/federation. The pack "what" is owned by POLVAL's
shipped admission in `crates/anvil-policy-engine/src/pack/`.)

> **Reset posture (POLRESET-010 / ADR-098, 2026-07-04; re-based 2026-07-11):**
> post-first-slice expansion — not a prerequisite for first policy value. The
> first slice has shipped; the live prerequisites are **POLVAL pack
> primitives** (shipped — manifest/metadata/versioning/discovery in
> `crates/anvil-policy-engine/src/pack/`), **POLLC lifecycle state** (Draft),
> and **ORGHIER** (Draft, demand-gated). Coordinated by
> [`POLRESET`](../archive/modules/policy-value-enforcement-reset.aps.md).

> **Policy-solution validation (2026-06-24):** POLFED should federate signed
> Rego packs, lifecycle state, and pack metadata; evaluation remains the
> ADR-040/POLENG regorus facade at each consuming repository. Go OPA may be a
> reference compatibility check during pack publication, but not the fleet
> runtime. Keep the pack-boundary ADR as a promotion gate — re-titled
> **POLVAL/POLFED** (2026-07-11): the pack data model shipped under POLVAL,
> so the boundary to codify is POLVAL-pack-primitives vs POLFED-distribution,
> not OPAE vs POLFED.
>
> **Audit note (2026-04-26):** Tier C (parking lot, post-launch).
> Multi-repo / fleet federation is an enterprise feature, not RTAI-blocking.
>
> Council C recommended dissolving POLFED into OPAE on the grounds that
> "POLFED-001..006 are duplicates of remote-bundle work that used to sit in
> OPAE." That was
> overstated. The actual overlap is narrow:
> - **POLFED-002** (central repo conventions) ↔ the old OPAE org-bundles item
>   — partial overlap on *where bundles live*. Coordinate schema.
> - **POLFED-006** (subscription version pinning) ↔ the old OPAE bundle-versioning
>   versioning) — different angle (consume vs publish). Coordinate
>   version-resolution semantics.
>
> The other 6 POLFED tasks have no OPAE equivalent: channel schema
> (POLFED-001), publisher workflow (POLFED-003), publish approval gate
> (POLFED-004), subscriber workflow (POLFED-005), fleet aggregation
> (POLFED-007 — cross-repo, distinct from COMPLY-007's single-repo
> historical), CLI federation commands (POLFED-008 — `publish/subscribe/
> fleet`, distinct from OPAE's `browse/install`).
>
> **Conceptual model (re-based 2026-07-11 — the pre-reset version of this
> paragraph said "OPAE owns bundle primitives"; that ownership dissolved with
> the OPAE reset and PR-C):** **POLVAL's shipped pack module** owns pack
> primitives — manifest, metadata, versioned packs, discovery
> (`crates/anvil-policy-engine/src/pack/`, the "what"); POLFED owns the
> operational federation layer (publish workflow, subscribe sync, fleet
> aggregation, CLI commands — the "how to distribute"). They are layered, not
> duplicate.
>
> **Rescope work pending** (tracked separately, see followup list):
> 1. Author the ADR codifying the **POLVAL/POLFED boundary** so future
>    refactors don't re-litigate ownership of pack primitives vs federation
>    workflow.
> 2. Coordinate POLFED-002 with the **shipped pack manifest**
>    (`crates/anvil-policy-engine/src/pack/manifest.rs`): central-repo
>    directory layout must compose with it, not fork a second manifest.
> 3. Coordinate POLFED-006 with the shipped pack metadata/version semantics
>    (subscriber pin vs publisher version).
> 4. ~~Retarget validations once OPAE bundle primitives land~~ — resolved
>    2026-07-11: validations retargeted to
>    `cargo test -p eddacraft-anvil-policy-engine` in this pass; no bundle
>    primitives are coming from OPAE.
> 5. Confirm POLFED-007 (cross-repo fleet) and COMPLY-007 (single-repo
>    historical) stay distinct — they shouldn't merge.

## Purpose

Allow organisations to manage policies as shared artefacts across a fleet of
repositories. A central policy repository acts as the source of truth;
individual repos subscribe to policy channels and receive updates through
pack sync. Federation covers publishing, discovery, subscription, and
fleet-wide compliance visibility.

## In Scope

- Central policy repository conventions and manifest format
- Policy channel model (e.g. `security/baseline`, `quality/strict`)
- Subscription configuration in `.anvilrc` referencing channels
- Fleet sync command that checks all subscribed repos for compliance
- Policy publishing workflow from authoring repo to registry
- Fleet-wide compliance dashboard data aggregation
- Approval gates for publishing new policy versions to channels
- CLI commands for publishing, subscribing, and fleet status

## Out of Scope

- Hosted SaaS registry (file-based and Git-based distribution only)
- Real-time push notifications (pull-based sync)
- Cross-organisation federation (single-org scope)
- Billing or access control beyond Git permissions

## Interfaces

**Depends on:**

- `policy-value-enforcement-reset` — satisfied: the first policy-value slice
  and OPAE reset both shipped (POLRESET Done 10/10, 2026-07-05)
- `policy-pack-validation` — **the pack-primitive owner**: publish must route
  through the shipped admission (manifest/validator/test enforcement in
  `crates/anvil-policy-engine/src/pack/`); the channel manifest composes with
  the shipped pack manifest, never forks it
- `opa-enhancements` — first-wave local authoring/runtime UX only; remote
  bundle infrastructure is owned by no one — OPAE excludes it post-reset and
  PR-C deleted the OPA bundle code
- `org-policy-hierarchy` — Hierarchy resolution for federated policies
- `policy-lifecycle` — Lifecycle state gates publishing
- ADR-100 (committed-authority provenance) — publish approval evidence
  (POLFED-004) must be readable from committed trees, matching the exception
  store's trust model

**Exposes:**

- `PolicyPublisher` — Publish policy packs to channels
- `PolicySubscriber` — Subscribe repos to channels and sync
- `FleetComplianceAggregator` — Collect compliance data across repos
- `anvil policy publish` — Publish a policy pack to a channel
- `anvil policy subscribe` — Subscribe to a policy channel
- `anvil policy fleet` — Show fleet-wide compliance status

## Acceptance Criteria

- [ ] Central repo can host multiple policy channels with versioned packs
- [ ] Subscribing repos pull the latest channel version on a future
      `anvil policy sync` (no `bundle` subcommand exists — the old
      `anvil policy bundle sync` wording named deleted OPAE plan-ware)
- [ ] Publishing requires pack validation and lifecycle state check
- [ ] Approval gate blocks publish until designated reviewer approves
- [ ] Fleet status aggregates compliance posture across all subscribed repos
- [ ] Subscription pinning allows repos to stay on a specific channel version
- [ ] Channel discovery lists available channels with descriptions

## Risks & Mitigations

| Risk                                     | Mitigation                                    |
| ---------------------------------------- | --------------------------------------------- |
| Stale subscriptions miss critical updates | Warn on outdated subscriptions in gate output |
| Publishing untested policies             | Validation and lifecycle gates block publish  |
| Fleet data privacy across teams          | Aggregate scores only; no raw violation data  |
| Git-based registry scalability           | Shallow clones; bundle caching in this module |

## Work Items

### POLFED-001: Policy channel schema

- **Intent:** Define the channel model for grouping and versioning policy packs
- **Expected Outcome:** Schema supports channel name, description, version, and metadata
- **Scope:** `crates/anvil-kernel-types/src/`
- **Non-scope:** Transport layer
- **Validation:** `cargo test -p eddacraft-anvil-kernel-types -- policy_channel`
- **Confidence:** high

### POLFED-002: Central repository conventions

- **Intent:** Establish directory structure and manifest format for policy repos
- **Expected Outcome:** Convention documented; manifest loader supports the format
- **Scope:** `crates/anvil-policy-engine/src/`
- **Non-scope:** Policy authoring
- **Dependencies:** POLFED-001
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- central_repo`
- **Confidence:** high

### POLFED-003: Policy publisher

- **Intent:** Package and publish policy packs to channels with validation
- **Expected Outcome:** Publisher validates, versions, and pushes packs to registry
- **Scope:** `crates/anvil-policy-engine/src/`
- **Non-scope:** Approval workflow
- **Dependencies:** POLFED-001, POLFED-002
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- policy_publisher`
- **Confidence:** high

### POLFED-004: Publish approval gate

- **Intent:** Require reviewer approval before policies go live on a channel
- **Expected Outcome:** Approval tracked in manifest and readable from the
  committed tree (ADR-100 committed-authority alignment); unapproved publishes
  blocked
- **Scope:** `crates/anvil-policy-engine/src/`
- **Non-scope:** Notification delivery
- **Dependencies:** POLFED-003
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- publish_approval`
- **Confidence:** medium

### POLFED-005: Policy subscriber

- **Intent:** Allow repos to subscribe to channels and sync policy bundles
- **Expected Outcome:** Subscriber fetches, caches, and applies channel policies
- **Scope:** `crates/anvil-policy-engine/src/`
- **Non-scope:** Hierarchy resolution
- **Dependencies:** POLFED-001
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- policy_subscriber`
- **Confidence:** high

### POLFED-006: Subscription version pinning

- **Intent:** Let repos pin to a specific channel version for stability
- **Expected Outcome:** Pinned repos skip newer versions until pin is updated
- **Scope:** `crates/anvil-policy-engine/src/`
- **Non-scope:** Auto-update logic
- **Dependencies:** POLFED-005
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- version_pinning`
- **Confidence:** high

### POLFED-007: Fleet compliance aggregator

- **Intent:** Collect compliance data across all subscribed repos for org visibility
- **Expected Outcome:** Aggregator produces fleet-wide posture summary
- **Scope:** `crates/anvil-policy-engine/src/`
- **Non-scope:** Dashboard rendering
- **Dependencies:** POLFED-005
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- fleet_compliance`
- **Confidence:** medium

### POLFED-008: CLI federation commands

- **Intent:** Expose publish, subscribe, and fleet status via the CLI
- **Expected Outcome:** `anvil policy publish`, `subscribe`, and `fleet` commands work
- **Scope:** `crates/anvil-cli/src/commands/`
- **Non-scope:** TUI visualisation
- **Dependencies:** POLFED-003, POLFED-005, POLFED-007
- **Validation:** `cargo test -p eddacraft-anvil -- policy_federation`
- **Confidence:** high
