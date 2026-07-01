<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Policy Federation

| ID  | Owner | Priority | Status | Progress |
| ------ | ----- | -------- | ------ | -------- |
| POLFED | —     | medium   | Draft  | 0/8      |

**Last reviewed:** 2026-04-26

> **Policy-solution validation (2026-06-24):** POLFED should federate signed
> Rego packs, lifecycle state, and pack metadata; evaluation remains the
> ADR-040/POLENG regorus facade at each consuming repository. Go OPA may be a
> reference compatibility check during pack publication, but not the fleet
> runtime. Keep the OPAE/POLFED bundle-boundary ADR as a promotion gate.
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
> **Conceptual model:** OPAE owns bundle primitives (data model,
> versioning, inheritance — the "what"); POLFED owns the operational
> federation layer (publish workflow, subscribe sync, fleet aggregation,
> CLI commands — the "how to distribute"). They are layered, not
> duplicate.
>
> **Rescope work pending** (tracked separately, see followup list):
> 1. Author ADR codifying the OPAE/POLFED boundary so future refactors
>    don't re-litigate ownership of bundle primitives vs federation
>    workflow.
> 2. Coordinate POLFED-002 with the post-POLRESET bundle schema (where bundles live in a
>    central repo: directory layout, manifest format).
> 3. Coordinate POLFED-006 with the post-POLRESET version-resolution semantics
>    (subscriber pin vs publisher version).
> 4. Retarget validations to `cargo test -p eddacraft-anvil-policy` once OPAE
>    bundle primitives land in `crates/anvil-policy`.
> 5. Confirm POLFED-007 (cross-repo fleet) and COMPLY-007 (single-repo
>    historical) stay distinct — they shouldn't merge.

## Purpose

Allow organisations to manage policies as shared artefacts across a fleet of
repositories. A central policy repository acts as the source of truth;
individual repos subscribe to policy channels and receive updates through
bundle sync. Federation covers publishing, discovery, subscription, and
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

- `policy-value-enforcement-reset` — first policy-value slice and OPAE reset
- `opa-enhancements` — first-wave local authoring/runtime UX; remote bundle
  infrastructure is no longer owned by OPAE after the 2026-07-02 reset
- `org-policy-hierarchy` — Hierarchy resolution for federated policies
- `policy-lifecycle` — Lifecycle state gates publishing
- `policy-pack-validation` — Validate before publish

**Exposes:**

- `PolicyPublisher` — Publish policy packs to channels
- `PolicySubscriber` — Subscribe repos to channels and sync
- `FleetComplianceAggregator` — Collect compliance data across repos
- `anvil policy publish` — Publish a policy pack to a channel
- `anvil policy subscribe` — Subscribe to a policy channel
- `anvil policy fleet` — Show fleet-wide compliance status

## Acceptance Criteria

- [ ] Central repo can host multiple policy channels with versioned packs
- [ ] Subscribing repos pull latest channel version on `anvil policy bundle sync`
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
- **Scope:** `crates/anvil-policy/src/`
- **Non-scope:** Policy authoring
- **Dependencies:** POLFED-001
- **Validation:** `cargo test -p eddacraft-anvil-policy -- central_repo`
- **Confidence:** high

### POLFED-003: Policy publisher

- **Intent:** Package and publish policy packs to channels with validation
- **Expected Outcome:** Publisher validates, versions, and pushes packs to registry
- **Scope:** `crates/anvil-policy/src/`
- **Non-scope:** Approval workflow
- **Dependencies:** POLFED-001, POLFED-002
- **Validation:** `cargo test -p eddacraft-anvil-policy -- policy_publisher`
- **Confidence:** high

### POLFED-004: Publish approval gate

- **Intent:** Require reviewer approval before policies go live on a channel
- **Expected Outcome:** Approval tracked in manifest; unapproved publishes blocked
- **Scope:** `crates/anvil-policy/src/`
- **Non-scope:** Notification delivery
- **Dependencies:** POLFED-003
- **Validation:** `cargo test -p eddacraft-anvil-policy -- publish_approval`
- **Confidence:** medium

### POLFED-005: Policy subscriber

- **Intent:** Allow repos to subscribe to channels and sync policy bundles
- **Expected Outcome:** Subscriber fetches, caches, and applies channel policies
- **Scope:** `crates/anvil-policy/src/`
- **Non-scope:** Hierarchy resolution
- **Dependencies:** POLFED-001
- **Validation:** `cargo test -p eddacraft-anvil-policy -- policy_subscriber`
- **Confidence:** high

### POLFED-006: Subscription version pinning

- **Intent:** Let repos pin to a specific channel version for stability
- **Expected Outcome:** Pinned repos skip newer versions until pin is updated
- **Scope:** `crates/anvil-policy/src/`
- **Non-scope:** Auto-update logic
- **Dependencies:** POLFED-005
- **Validation:** `cargo test -p eddacraft-anvil-policy -- version_pinning`
- **Confidence:** high

### POLFED-007: Fleet compliance aggregator

- **Intent:** Collect compliance data across all subscribed repos for org visibility
- **Expected Outcome:** Aggregator produces fleet-wide posture summary
- **Scope:** `crates/anvil-policy/src/`
- **Non-scope:** Dashboard rendering
- **Dependencies:** POLFED-005
- **Validation:** `cargo test -p eddacraft-anvil-policy -- fleet_compliance`
- **Confidence:** medium

### POLFED-008: CLI federation commands

- **Intent:** Expose publish, subscribe, and fleet status via the CLI
- **Expected Outcome:** `anvil policy publish`, `subscribe`, and `fleet` commands work
- **Scope:** `crates/anvil-cli/src/commands/`
- **Non-scope:** TUI visualisation
- **Dependencies:** POLFED-003, POLFED-005, POLFED-007
- **Validation:** `cargo test -p eddacraft-anvil -- policy_federation`
- **Confidence:** high
