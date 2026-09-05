# Converged App Decisions and Contract Verification

| ID | Owner | Status |
| --- | --- | --- |
| CONV | @joshuaboys | In Progress |

## Purpose

Reconcile the converged application with existing anvil authorities and turn
allomorph EXP-002 evidence into an anvil-owned command contract.
The [decision register](../specs/converged-app/05-decisions-risks-and-open-questions.md)
owns the A–J disposition. This module owns executable follow-through.

## Scope

Decision reconciliation, shared contracts, bounded production verification,
APS authority/public boundary, document/workspace mutation, daemon lifecycle,
native framework evidence and capability/evidence integration.

## Non-goals

No allomorph dependency before Gate E; no Flow/Scene concepts in product
contracts; no new global daemon or second governance store; no bulk RPC rewrite;
no default-on desktop release; no fabricated human evaluation.
This planning change implements no runtime behaviour and moves no APS source.

## Interfaces

Depends on the [two-track strategy](../specs/2026-08-07-two-track-ui-strategy.md)
and [ADR-140](../decisions/140-converged-command-event-contract.md).
Coordinates with ACTMO/JOURNEY for activation reliability, KFIT for governance
storage and identity, SETCON for settings, and the owning feature catalogue
and flag modules. Existing work remains with its owner; CONV adds only app gaps.
CONV-002's first internal extraction is scoped in its execution plan. A new
application ingress still requires exact schema, bounds and compatibility gates.
The other implementation items remain Proposed.

## Work Items

### CONV-001: Reconcile decisions and record the shared command contract

- **Status:** Done
- **Intent:** Incorporate EXP-002 evidence while preserving existing anvil authorities.
- **Expected Outcome:** ADR-B represented by ADR-140; A–J dispositions and remaining
  work linked from the existing register and strategy, with no runtime readiness claim.
- **Files:** `plans/decisions/140-converged-command-event-contract.md`,
  `plans/decisions/DECISION-LOG.md`, `plans/specs/converged-app/`,
  `plans/specs/2026-08-07-two-track-ui-strategy.md`, this module and APS index.
- **Validation:** `pnpm adr:check`, `pnpm docs:check`, `pnpm aps:active-lint`,
  `pnpm aps:drift --json`; Council review and required PR checks.
- **Evidence:** ADR integrity and whitespace checks passed locally; Council found
  no critical/major issues. Full repository validation is delegated to PR CI.
  Done describes the prepared decision records, not ADR acceptance or runtime delivery.
- **Dependencies:** allomorph EXP-002 evidence (PR #8).
- **Confidence:** high
- **Claim:** [#4400](https://github.com/eddacraft/anvil-001/issues/4400)
- **Release:** changeType docs; releaseIntent never; releaseScope none.

### CONV-002: Prove the shared contract against one real anvil command

- **Status:** In Progress
- **Claim:** [#4407](https://github.com/eddacraft/anvil-001/issues/4407)
- **Evidence:** ADR-140 accepted by @joshuaboys on 2026-09-05. The
  [execution plan](../execution/CONV-002.actions.md) inventories existing paths
  and scopes the first CLI/MCP shared source-scan handler increment. Generated
  application schema, identity and compatibility gates remain outstanding;
  this item is not complete when the internal extraction merges.
- **Intent:** Select a bounded existing command and freeze exact DTO/schema placement.
- **Expected Outcome:** One handler serves CLI and a second client via generated
  schemas; typed IDs, semantic errors, version negotiation and existing exit-code
  mappings have executable fixtures. Inventory existing RPCs before adding types.
- **Files:** First increment: `crates/anvil-cli/src/services/antipattern_scan.rs`,
  its module registration, CLI/MCP check adapters and stdio parity tests.
  Exact public DTO/schema placement remains gated in the execution plan;
  no new crate or application logic in intercept.
- **Validation:** ADR-140 shared-command, identity, compatibility and dependency gates;
  round-trip/generated-client fixtures and unchanged legacy command tests.
- **Dependencies:** CONV-001 and ADR-140 acceptance.
- **Confidence:** medium

### CONV-003: Decide and prove durable operation, approval and reconnect boundaries

- **Status:** Proposed
- **Intent:** Resolve remaining ADR-C/F storage and lifecycle details with a crash spike.
- **Expected Outcome:** Explicit store/transaction ownership, numeric retry/retention
  windows, global/per-scope queue and payload limits, durable admission and approval
  consumption; snapshot/cursor resync; client closure leaves admitted work alive.
  Unknown post-effect outcomes require reconciliation, never blind retry.
- **Validation:** ADR-140 duplicate/lost-reply/crash/expiry/cancel/reconnect matrix;
  slow-client and restart tests; tray-only startup, tray crash/reconnect and
  quit-with-live-work cases; existing save-time latency/resource gates.
- **Dependencies:** CONV-002; existing ADR-036/064/067/116 boundaries.
- **Confidence:** medium

### CONV-004: Settle APS canonical source and public distribution

- **Status:** Proposed
- **Intent:** Resolve ADR-A before moving source authority.
- **Expected Outcome:** Dedicated ADR covers import history, canonical ownership,
  allow-listed public dependency graph, public contribution import, independent
  versioning, release provenance and rollback. Reuse relevant distribution precedents
  without assuming the eddacraft-tui mirror decision already authorises APS.
- **Validation:** Clean-room public export/build and standalone CLI/TUI parity plan
  with exact commands and retained attribution, before any source move.
- **Dependencies:** CONV-001; current APS upstream inventory.
- **Confidence:** medium

### CONV-005: Settle planning mutation and workspace ownership

- **Status:** Proposed
- **Intent:** Resolve ADR-D/E with a real concurrent planning/worktree scenario.
- **Expected Outcome:** Markdown authority, content-hash compare-and-swap, atomic
  deterministic patches, branch-switch refresh and visible conflict; single-repository
  worktree ownership/cleanup first, preserved history, no direct UI file writes.
  Audit existing repository and worktree identity/safety contracts before extension.
- **Validation:** External edit, concurrent client, stale version, branch switch,
  unsafe path, shared checkout and archive-with-active-run cases.
- **Dependencies:** CONV-002; CONV-004 before changing canonical APS source.
- **Confidence:** medium

### CONV-006: Decide native framework and native/web sharing through the hard-component spike

- **Status:** Proposed
- **Intent:** Resolve ADR-G/J using the converged-app surface requirements.
- **Expected Outcome:** Dioxus versus Tauri/React or constrained hybrid measured on
  terminals, diffs, virtual lists, accessibility, Windows/macOS/Linux tray behaviour
  and packaging. Record component/code cost and explicit go/no-go results.
  Reuse ADR-104's existing React/Vite/OpenAPI dashboard seam as the baseline.
  Decide native component sharing and browser transport/auth/write expansion
  separately; this work does not revoke the current read-only dashboard boundary.
- **Validation:** Real interactive tasks and platform matrix, not static mock-ups;
  framework decision and shared-UI disposition recorded in ADRs. Include a
  tray-only installation with full desktop absent: login/startup policy, compact
  approval/recovery, notification delivery failure and OS tray support/fallback
  on each supported platform. A hidden main window is not this proof.
- **Dependencies:** CONV-001; typed fixture from CONV-002 for integration proof.
- **Confidence:** medium

### CONV-007: Reconcile capabilities, evidence and first-slice release exposure

- **Status:** Proposed
- **Intent:** Close app-specific ADR-H/I gaps by composing existing owners.
- **Expected Outcome:** Capability/action and entitlement mapping, server enforcement,
  ADR-035 pipe routing, ADR-116 governance retention and authorised artefact access.
  Decide operational metadata/large-artefact storage separately; no automatic
  governance expiry. Register app.converged only when first shell code lands,
  default disabled with review date; sub-flags do not grant permissions.
- **Validation:** Denied actions through every ingress, redaction failure, capacity
  refusal/gap evidence, expired grants, stale settings attestation and absent MCP.
  First vertical slice must survive client closure and preserve standalone APS parity
  before explicit default-on UX/release review. Verify the essential tray-only
  journey without desktop/browser dependencies and publish its supported action
  matrix; optional richer-surface handoffs cannot conceal unsupported actions.
- **Dependencies:** CONV-002; CONV-003 for durable guarantees; CONV-006 for shell exposure.
- **Confidence:** medium

### CONV-008: Make tray-only operation explicit in the design

- **Status:** Done
- **Intent:** Require the tray to operate as the only graphical client.
- **Expected Outcome:** ADR-140 and the surface spec require independent tray
  startup, compact approvals/recovery, daemon-owned lifecycle and optional full-UI
  handoffs; CONV-003/006/007 own executable proofs.
- **Validation:** `pnpm adr:check`, `pnpm docs:check`, `pnpm aps:active-lint`;
  focused Council review and required PR checks.
- **Dependencies:** CONV-001.
- **Confidence:** high
- **Claim:** [#4404](https://github.com/eddacraft/anvil-001/issues/4404)
- **Evidence:** Operator clarification on 2026-09-05. Done means the specification
  amendment is prepared, not that tray-only behaviour is implemented or released.
- **Release:** changeType docs; releaseIntent never; releaseScope none.
