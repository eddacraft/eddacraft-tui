<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Policy Federation

| Scope  | Owner | Priority | Status |
| ------ | ----- | -------- | ------ |
| POLFED | —     | medium   | Draft  |

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

- `opa-enhancements` — Remote bundle infrastructure (OPAE-034–036)
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
| Git-based registry scalability           | Shallow clones; bundle caching per OPAE-034   |

## Tasks

### POLFED-001: Policy channel schema

- **Intent:** Define the channel model for grouping and versioning policy packs
- **Expected Outcome:** Schema supports channel name, description, version, and metadata
- **Scope:** `packages/anvil/contracts/src/types/`
- **Non-scope:** Transport layer
- **Validation:** `nx test contracts --testNamePattern="policy-channel"`
- **Confidence:** high

### POLFED-002: Central repository conventions

- **Intent:** Establish directory structure and manifest format for policy repos
- **Expected Outcome:** Convention documented; manifest loader supports the format
- **Scope:** `packages/anvil/policy/src/`
- **Non-scope:** Policy authoring
- **Dependencies:** POLFED-001
- **Validation:** `nx test policy --testNamePattern="central-repo"`
- **Confidence:** high

### POLFED-003: Policy publisher

- **Intent:** Package and publish policy packs to channels with validation
- **Expected Outcome:** Publisher validates, versions, and pushes packs to registry
- **Scope:** `packages/anvil/policy/src/`
- **Non-scope:** Approval workflow
- **Dependencies:** POLFED-001, POLFED-002
- **Validation:** `nx test policy --testNamePattern="policy-publisher"`
- **Confidence:** high

### POLFED-004: Publish approval gate

- **Intent:** Require reviewer approval before policies go live on a channel
- **Expected Outcome:** Approval tracked in manifest; unapproved publishes blocked
- **Scope:** `packages/anvil/policy/src/`
- **Non-scope:** Notification delivery
- **Dependencies:** POLFED-003
- **Validation:** `nx test policy --testNamePattern="publish-approval"`
- **Confidence:** medium

### POLFED-005: Policy subscriber

- **Intent:** Allow repos to subscribe to channels and sync policy bundles
- **Expected Outcome:** Subscriber fetches, caches, and applies channel policies
- **Scope:** `packages/anvil/policy/src/`
- **Non-scope:** Hierarchy resolution
- **Dependencies:** POLFED-001
- **Validation:** `nx test policy --testNamePattern="policy-subscriber"`
- **Confidence:** high

### POLFED-006: Subscription version pinning

- **Intent:** Let repos pin to a specific channel version for stability
- **Expected Outcome:** Pinned repos skip newer versions until pin is updated
- **Scope:** `packages/anvil/policy/src/`
- **Non-scope:** Auto-update logic
- **Dependencies:** POLFED-005
- **Validation:** `nx test policy --testNamePattern="version-pinning"`
- **Confidence:** high

### POLFED-007: Fleet compliance aggregator

- **Intent:** Collect compliance data across all subscribed repos for org visibility
- **Expected Outcome:** Aggregator produces fleet-wide posture summary
- **Scope:** `packages/anvil/runtime/src/`
- **Non-scope:** Dashboard rendering
- **Dependencies:** POLFED-005
- **Validation:** `nx test runtime --testNamePattern="fleet-compliance"`
- **Confidence:** medium

### POLFED-008: CLI federation commands

- **Intent:** Expose publish, subscribe, and fleet status via the CLI
- **Expected Outcome:** `anvil policy publish`, `subscribe`, and `fleet` commands work
- **Scope:** `apps/anvil-cli/src/commands/`
- **Non-scope:** TUI visualisation
- **Dependencies:** POLFED-003, POLFED-005, POLFED-007
- **Validation:** `nx test cli --testNamePattern="policy federation"`
- **Confidence:** high
