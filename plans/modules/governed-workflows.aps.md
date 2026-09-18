<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Governed Workflows

| ID | Owner | Priority | Status | Progress |
| --- | --- | --- | --- | --- |
| GOVWF | @joshuaboys | P1 | Draft | 0/12 |

**Last reviewed:** 2026-09-18 — operator-approved design recorded in the
[governed workflows specification](../specs/2026-09-18-governed-workflows.md)
and [ADR-150](../decisions/150-governed-workflow-transition-authority.md)
proposed.

> **Posture:** approved design with Draft delivery slices. Nothing in this
> module is executable or authorised for implementation. Promote items only
> after the architecture, schema, authority, evidence, and pilot gates in the
> Ready Checklist are closed.

## Purpose

Let a team declare an engineering workflow as a versioned DAG and let anvil
deterministically allow or deny transitions using independently verifiable
evidence, policy, approvals, and exceptions. The first enforcement boundary is
an exact-head GitHub pull-request required check governing merge/completion.

## Origin And Demand Signal

The requirement arose separately from ABASE: agent instructions such as “run
this script, inspect its artefact, and never close the work while errors remain”
drift as agents, prompts, tools, and workflows change. The required control must
therefore live outside the actor performing the work.

The supplied product requirement calls for a deterministic, agent-independent,
evidence-driven, policy-driven, versioned, auditable, composable, extensible,
enforceable, and explainable governance layer. The design was resolved through
operator Q&A on 2026-09-18. A design-partner repository is still required
before implementation promotion.

## Product Boundary

Agents, humans, CI, and automation execute work. anvil admits their evidence
and governs state transitions. This module does not build a workflow runner,
CI/CD service, issue tracker, task scheduler, or agent orchestrator.

`/dev-loop` is internal developer tooling. It may dogfood the provider-neutral
transition API after the product slice exists, but it is not the product
contract, an implementation prerequisite, or delivery evidence for GOVWF.

## Approved Design Positions

- Start repository-scoped with an exact GitHub pull-request base/head instance.
- Resolve workflow authority from the current protected base; a pull request
  cannot weaken its own gate.
- Revise the instance and re-evaluate evidence when the protected base changes.
- Support a true DAG with parallel branches and joins from the first schema.
- Reference registered, versioned evidence capabilities; do not run inline
  commands from workflow definitions.
- Keep action execution external and make evidence admission explicit.
- Evaluate acceptance policy deterministically through Regorus.
- Represent GitHub exact-head review as a provider-neutral approval receipt.
- Reuse EXCEPT for scoped, expiring, authority-valid deviations.
- Reuse Kindling for local governance facts and ADR-037 witness plus a
  content-addressed companion receipt for portable proof.
- Treat host administrator bypass as an observable ungoverned outcome, not a
  governed success.
- Reserve a later local CLI/MCP transition projection and organisation-level
  composition without making either part of the first slice.

## In Scope

- Versioned workflow definition and DAG validation.
- Repository/pull-request/base/head/workflow instance identity.
- Completed-node state and available/blocked transition frontier.
- Registered evidence-producer capability contract and receipt admission.
- Regorus transition policy and explainable allow/deny/degraded/unsatisfiable
  decisions.
- Exact-head GitHub approval evidence.
- EXCEPT-backed transition deviations.
- Required-check enforcement of merge/completion eligibility.
- Durable transition decision evidence through Kindling and witness.
- Workflow drift and satisfiability diagnostics.

## Out Of Scope

- Running arbitrary workflow actions or inline shell commands.
- Replacing GitHub Actions, Temporal, CI/CD, issue trackers, or agent
  orchestrators.
- Requiring APS identity for customer workflow instances.
- Organisation-level workflow composition in the first slice.
- A hosted workflow state service or new provenance ledger.
- Treating LLM judgement as authoritative policy evaluation.
- Claiming to prevent a forge administrator from bypassing controls outside
  anvil's enforcement boundary.

## Relationships And Ownership

| System | GOVWF relationship |
| --- | --- |
| Regorus policy layer | Authoritative deterministic transition-policy evaluation; GOVWF owns its workflow-specific input/output contract. |
| EXCEPT | Owns exception storage, authority, expiry, revocation, and committed provenance; GOVWF consumes valid dispositions. |
| Kindling | Owns durable local governance facts; GOVWF defines transition fact payloads through that contract. |
| ADR-037 witness | Owns portable tamper-evident proof envelope; GOVWF supplies the companion transition receipt. |
| ORGHIER/POLLC/POLFED | Own future organisation-level authority and policy composition; GOVWF starts repository-scoped. |
| GitHub | First enforcement and approval adapter, not the core domain model. |
| `/dev-loop` | Future internal consumer only; no product ownership. |

## Work Items

### GOVWF-001: Workflow definition and DAG validation

- **Intent:** Define the versioned workflow schema and deterministic static
  validation contract.
- **Expected Outcome:** Cycles, unknown references, invalid joins, missing
  terminal routes, and unsupported schema versions produce stable diagnostics.
- **Dependencies:** ADR-150 accepted; schema/versioning readiness gates closed.
- **Non-scope:** Runtime transition evaluation and action execution.
- **Validation:** Draft intent; concrete implementation home and commands must
  be named before Ready promotion.
- **Status:** Draft

### GOVWF-002: Trusted-base authority and instance revision

- **Intent:** Resolve the protected-base workflow and identify instances by
  repository, pull request, exact base/head, and workflow version.
- **Expected Outcome:** A pull request cannot self-weaken; a base update creates
  a revision and invalidates mismatched evidence deterministically.
- **Dependencies:** GOVWF-001; GitHub ruleset/bootstrap design approved.
- **Non-scope:** Organisation-level authority composition.
- **Validation:** Draft intent; add fork, rebase, force-push, and base-change
  fixtures before promotion.
- **Status:** Draft

### GOVWF-003: Evidence capability registry

- **Intent:** Define registered producer capability identity, versioning,
  schema, availability, and trust admission.
- **Expected Outcome:** Workflows reference governed capabilities and cannot
  introduce arbitrary inline execution.
- **Dependencies:** GOVWF-001; registry owner and mutation authority decided.
- **Non-scope:** Running capability commands.
- **Validation:** Draft intent; registry conformance suite required before
  Ready promotion.
- **Status:** Draft

### GOVWF-004: Evidence receipt and admission contract

- **Intent:** Admit structured receipts bound to producer, tool, capability,
  repository, base/head, workflow, policy, validity, and content digest.
- **Expected Outcome:** Missing, stale, malformed, unverified, or mismatched
  receipts cannot satisfy node requirements.
- **Dependencies:** GOVWF-002, GOVWF-003; cross-CI durability and trust design
  approved.
- **Non-scope:** Producer execution and hosted artefact storage.
- **Validation:** Draft intent; adversarial freshness, replay, substitution, and
  wrong-change fixtures required before promotion.
- **Status:** Draft

### GOVWF-005: DAG state and transition-frontier evaluator

- **Intent:** Evaluate completed nodes, parallel branches, joins, and available
  or blocked transitions from admitted facts.
- **Expected Outcome:** Identical workflow state and evidence produce an
  identical transition frontier and reasons.
- **Dependencies:** GOVWF-001, GOVWF-004.
- **Non-scope:** Policy acceptance thresholds.
- **Validation:** Draft intent; property tests for ordering independence,
  monotonic completion, joins, and unreachable terminal states required.
- **Status:** Draft

### GOVWF-006: Regorus transition-policy contract

- **Intent:** Define bounded Regorus inputs and stable decision/reason outputs
  for each requested edge.
- **Expected Outcome:** Only `allow` satisfies enforcement; deny, degraded, and
  unsatisfiable states remain blocking and explainable.
- **Dependencies:** GOVWF-004, GOVWF-005; policy schema and resource bounds
  approved.
- **Non-scope:** LLM-authored authoritative decisions.
- **Validation:** Draft intent; deterministic golden fixtures and resource
  limit tests required before promotion.
- **Status:** Draft

### GOVWF-007: GitHub exact-head approval adapter

- **Intent:** Map GitHub review identity, authority, state, and exact head into
  the provider-neutral approval receipt.
- **Expected Outcome:** Dismissed, stale, unauthorised, or wrong-head reviews do
  not satisfy an approval node.
- **Dependencies:** GOVWF-002, GOVWF-004; review-authority mapping approved.
- **Non-scope:** Other forge adapters.
- **Validation:** Draft intent; GitHub review-state fixture matrix required.
- **Status:** Draft

### GOVWF-008: Transition exception integration

- **Intent:** Evaluate applicable EXCEPT grants without erasing the underlying
  failed or missing fact.
- **Expected Outcome:** Only valid, scoped, attributed, unexpired,
  authority-valid grants alter compliance disposition and every use is visible.
- **Dependencies:** GOVWF-006; EXCEPT owner review and receipt mapping approved.
- **Non-scope:** A workflow-specific bypass store.
- **Validation:** Draft intent; expiry, revocation, scope, committed-authority,
  and absent-store fixtures required.
- **Status:** Draft

### GOVWF-009: Transition fact and portable receipt

- **Intent:** Record transition requests and decisions through Kindling and the
  ADR-037 witness model with a content-addressed companion receipt.
- **Expected Outcome:** An operator can reconstruct the instance, evidence,
  policy, approvals, exceptions, decision, actor, and reasons without a new
  event store.
- **Dependencies:** GOVWF-004, GOVWF-006, GOVWF-008; cross-machine durability
  and privacy mapping approved.
- **Non-scope:** Hosted workflow ledger or analytics dashboard.
- **Validation:** Draft intent; round-trip, tamper, redaction, and missing-
  companion fixtures required.
- **Status:** Draft

### GOVWF-010: GitHub required-check enforcement

- **Intent:** Publish the exact-head transition decision as a required check
  governing pull-request merge/completion.
- **Expected Outcome:** Only an exact-current `allow` check passes; every other
  state remains non-passing with actionable reasons.
- **Dependencies:** GOVWF-002, GOVWF-006, GOVWF-007, GOVWF-008, GOVWF-009;
  installation/ruleset bootstrap approved.
- **Non-scope:** Administrator policy bypass.
- **Validation:** Draft intent; integration fixtures for rebase, force-push,
  base update, rerun, cancellation, and unavailable evaluator required.
- **Status:** Draft

### GOVWF-011: End-to-end pilot and adversarial closure

- **Intent:** Prove a branching test/security/review workflow on one protected
  design-partner repository.
- **Expected Outcome:** The specification's first-slice acceptance scenario is
  reproduced, including stale/mismatched evidence, self-weakening attempts,
  EXCEPT use, drift, and observable host bypass.
- **Dependencies:** GOVWF-001..010.
- **Non-scope:** Organisation rollout and other forge providers.
- **Validation:** Draft intent; an approved pilot repository and executable
  scenario must be named before promotion.
- **Status:** Draft

### GOVWF-012: Provider-neutral local transition projection

- **Intent:** After the GitHub pilot, expose the same evaluator through a local
  CLI/MCP request for tools and internal dogfooding.
- **Expected Outcome:** A tool can query available/blocked transitions and
  request evaluation without becoming the decision authority or gaining an
  implicit action runner.
- **Dependencies:** GOVWF-011; post-pilot operator promotion.
- **Non-scope:** `/dev-loop` orchestration, organisation composition, and
  automatic action execution.
- **Validation:** Draft future intent; command/API home and consumer contract
  deliberately deferred until the core receipt model is proven.
- **Status:** Draft

## Ready Checklist

- [x] Product boundary and non-goals approved.
- [x] Exact-head GitHub merge/completion wedge approved.
- [x] Repository-scoped instance identity and trusted-base authority approved.
- [x] True DAG, registered capabilities, external execution, Regorus policy,
  exact-head approvals, EXCEPT reuse, and Kindling/witness reuse approved.
- [x] `/dev-loop` classified as internal future dogfooding, not delivery.
- [ ] ADR-150 accepted.
- [ ] Workflow schema, versioning, migration, and reason taxonomy approved.
- [ ] GitHub protected-base resolution and ruleset/bootstrap design approved.
- [ ] Capability registry owner, mutation authority, and producer admission
  contract approved.
- [ ] Receipt authentication/content-addressing, freshness, replay defence, and
  cross-CI durability approved.
- [ ] Regorus input/output schema and resource bounds approved.
- [ ] GitHub review authority mapping approved.
- [ ] EXCEPT integration reviewed by its owner.
- [ ] Kindling/witness payload, privacy, and cross-machine mapping approved.
- [ ] Concrete implementation homes and validation commands named.
- [ ] Design-partner pilot repository and acceptance fixture approved.

## Open Questions

1. What canonical schema and migration policy identify workflow definitions,
   nodes, edges, requirements, joins, and terminal states?
2. Which repository authority admits or revokes evidence capabilities, and how
   is that registry protected from self-modification?
3. Which signing, identity, and transport mechanisms make CI receipts durable
   and independently attributable across machines?
4. What exact Regorus document and reason-code schema remains stable across
   CLI, required-check, Kindling, and witness projections?
5. Which GitHub identities and team relationships count as authorised approval
   for each workflow scope?
6. How is required-check installation/ruleset drift detected without claiming
   control over administrator bypass?
7. Which design-partner repository supplies the first real branching workflow
   and evidence producers?

## Designs

- [Governed workflows specification](../specs/2026-09-18-governed-workflows.md)
- [ADR-150: Governed workflow transition authority](../decisions/150-governed-workflow-transition-authority.md)
- [ADR-037: Witness chain and L4 policy](../decisions/037-witness-chain-and-l4-policy.md)
- [ADR-098: Policy enforcement reset](../decisions/098-policy-enforcement-reset-gate.md)
- [Git-native exceptions](./git-native-exceptions.aps.md)
- [Organisational policy hierarchy](./org-policy-hierarchy.aps.md)
