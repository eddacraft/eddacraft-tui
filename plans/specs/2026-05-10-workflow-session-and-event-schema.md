# Workflow Session And Event Schema

Date: 2026-05-10

Status: Complete

Related work item: OPMODEL-009

Related specs:

- `plans/specs/2026-05-09-plan-build-release-operating-model.md`
- `plans/specs/2026-05-09-agentic-execution-ecosystem-architecture.md`
- `plans/specs/2026-05-09-council-agent-skill-change-proposal.md`

Related decision:

- `plans/decisions/035-three-pipe-observability-rule.md`

## Purpose

Define the durable workflow session and event records used by agent review,
planning validation, release/recovery workflows, and future orchestration.

The goal is resumability and evidence. The schema records enough state to answer
who did what, under which APS authority, from which branch/SHA, using which
playbook/tool, with which validation or approval evidence.

## Authority Boundary

Workflow session records are execution memory. They do not replace APS, ADRs,
CI, release records, or source code.

| Concern | Authority | Session role |
| --- | --- | --- |
| Intent and readiness | APS | cite `apsItems` and plan paths |
| Architectural decisions | ADRs/specs | cite `relevantDecisions` and specs |
| Validation pass/fail | local checks and CI before CGBDG; Kindling after governance bridge | record pre-bridge command/check evidence, not authority |
| Review judgement | council/review output before CGBDG; Kindling after governance bridge | store pre-bridge findings, decisions, waivers, not authority |
| Human approval | explicit approval event before CGBDG; Kindling after governance bridge | record pre-bridge actor, scope, and timestamp, not authority |
| Shipped state | release record | cite release record, do not infer shipping |

Chat history is never authoritative. If a future agent needs state after the
chat is gone, that state must be in APS, a release record, a session/event
record, a PR, or another durable artefact.

## Storage Contract

Initial repository-local schema:

- `schemas/workflow-session-event.v1.schema.json` defines workflow sessions and
  workflow events.
- Council/review sessions may write durable summaries under `plans/reviews/`.
- Future implementations may move session/event storage, but must preserve the
  schema semantics or provide an explicit migration.

The existing `.claude/council/schema.json` path remains the shared Council schema
symlink until CGBDG migrates local writers/readers. This slice does not fork that
source of truth.

## Session Record

A session record represents one resumable workflow instance.

Required fields:

- `schemaVersion`: schema version, currently `workflow-session-event/v1`.
- `sessionId`: stable id for this workflow instance.
- `sessionKind`: `code-review`, `planning`, `pre-execution`, `release`,
  `recovery`, or `implementation`.
- `workflowId`: stable workflow name, such as `pre-pr-review` or
  `release-candidate`.
- `status`: current session state.
- `currentState`: state-machine node or phase name.
- `apsItems`: APS work item ids in scope.
- `repoReality.repo`: repository owner/name.
- `repoReality.baseBranch`, `repoReality.headBranch`, and `repoReality.headSha`:
  repository reality anchor.
- `actors`: human and agent participants.
- `events`: embedded events or event references.

Recommended fields:

- `playbooks`: playbooks used during the session.
- `relevantDecisions`: ADR/spec references used as context.
- `traceparent`: W3C trace context used only for correlation.
- `evidence`: validation, PR, CI, release, or review evidence links.
- `decisions`: workflow decisions such as `proceed`, `amend`, `split`,
  `replan`, `block`, `approve`, or `needs-changes`.
- `waivers`: explicit accepted risks with approver and rationale.

## Event Record

An event records one append-only workflow observation.

Required fields:

- `eventId`: stable event id.
- `timestamp`: RFC 3339 timestamp.
- `eventType`: namespaced event type, such as `review.started`,
  `tool.completed`, `validation.failed`, or `approval.granted`.
- `workflowId`: workflow this event belongs to.
- `sessionId`: session this event belongs to.
- `actor`: human, agent, CI, hook, or system actor.
- `apsItems`: APS work item ids in scope.

Events may include `branch` and `sha` when a repository anchor exists. Session
records always carry the repository anchor through `repoReality`.

Events may additionally include:

- `stateTransition`: `from`, `to`, and `reason`.
- `toolInvocation`: tool name, command or endpoint digest, exit code, and
  duration.
- `inputDigest` and `outputDigest`: hashes and media types for large or
  sensitive inputs/outputs.
- `validationResult`: command/check name, status, and evidence URL/path.
- `error`: error class, message, retryability, and recovery hint.
- `approval`: approval decision, decision actor, scope, and expiry/review date.
- `traceparent`: cross-pipe correlation key.
- `payloadDigest`: digest for any payload not embedded directly.

Raw command output, prompts, secrets, and large payloads should not be embedded by
default. Store digests and durable references instead.

## Pipe Allocation Under ADR-035

ADR-035 defines the three-pipe observability rule. Workflow sessions and events
must allocate facts accordingly:

| Fact | Pipe |
| --- | --- |
| Governance outcomes, approvals, waivers, validation decisions | Kindling is the source of truth once bridged; session/event records are pre-bridge operational evidence only |
| User-visible progress or state changes | Notification envelope |
| Debugging breadcrumbs, timings, and correlation spans | Tracing / OTEL |

`traceparent` is the cross-pipe correlation key. It may appear in sessions,
events, notifications, and spans, but tracing is not a source of truth. Any
governance fact needed later must be bridged to Kindling when CGBDG lands, or
remain in its existing authority such as APS, PR metadata, CI output, or a
release record.

## Workflow Decision Values

Planning and pre-execution sessions use:

- `proceed`
- `amend`
- `split`
- `replan`
- `block`

Review sessions use:

- `approve`
- `needs-changes`
- `reject`
- `waived`

Release and recovery sessions use:

- `candidate-ready`
- `publish-approved`
- `verify-pass`
- `verify-fail`
- `rollback-required`
- `recovery-complete`

## Redaction And Digest Rules

- Do not store secrets, access tokens, credentials, or raw secret-detection
  payloads in session/event records.
- Prefer SHA-256 digests for large command outputs, prompts, tool results, and
  generated artefacts.
- Store paths/URLs to durable evidence when safe.
- If evidence may contain secrets, store only the digest and the command/check
  identity.
- Human approval events must include scope and decision actor; do not infer approval from
  silence or elapsed time.

## Current-State Limits

This slice defines the contract and tracked schema. It does not implement a
complete session writer, Kindling bridge, event store, or CI enforcement.

Downstream work:

- CGBDG owns durable council evidence publication/bridging.
- TRACE / ADR-035 owns pipe allocation and future trace correlation mechanics.
- OPMODEL-010 owns warning-mode drift checks.
- Release orchestration owns release-record generation and shipped-state
  reconciliation.
