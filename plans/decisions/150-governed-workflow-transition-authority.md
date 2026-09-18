# ADR-150: Governed workflow transition authority

## Status

Proposed

## Date

2026-09-18

## Context

Teams express engineering workflow controls through prompts, documentation,
scripts, CI jobs, issue templates, and repository conventions. Those controls
are fragmented and often advisory. A human or agent can claim an action
occurred, reuse stale evidence, reinterpret an instruction, or skip a step as
tools and models change. The organisation cannot deterministically prove that
the required process governed the resulting transition.

anvil already has relevant primitives: Regorus-based deterministic policy
evaluation, git-native exceptions, Kindling governance facts, and the ADR-037
witness chain. It does not have an authoritative workflow instance, DAG
transition evaluator, evidence-capability contract, or independently
enforceable completion boundary. `/dev-loop` is internal developer tooling and
cannot stand in for the product capability.

The architecture must add transition governance without turning anvil into a
workflow automation service or coupling it to one agent, CI provider, or
planning system.

## Decision

### 1. anvil governs transitions; external systems execute work

Workflows declare required actions, evidence, approvals, and policy. Humans,
agents, CI, and other automation execute those actions. anvil admits evidence
and evaluates whether a requested edge is allowed. Transition evaluation does
not implicitly execute commands.

### 2. Start with exact-head GitHub merge/completion governance

The first enforceable slice is a GitHub required check for a repository-scoped
pull request. A workflow instance is identified by repository, pull request,
exact base commit, exact head commit, and workflow version. APS or issue
identity may be attached but is not required.

The workflow definition and policy resolve from the protected base authority,
not the pull-request head. The change under evaluation therefore cannot weaken
its own controls. A base update creates a new instance revision; mismatched
evidence does not carry forward silently.

### 3. Model workflows as true DAGs

The model supports parallel branches and joins. State is the completed-node set
plus the available and blocked transition frontier, not one mutable “current
step”. Static validation rejects cycles, missing references, invalid joins, and
unreachable valid terminal states.

### 4. Reference registered capabilities, not inline commands

Workflow nodes name registered, versioned evidence-producing capabilities.
External producers execute the capability and submit a structured receipt.
The registry pins producer identity, schema, version semantics, and trust
requirements. Untrusted workflow definitions cannot introduce arbitrary shell
execution into the governance evaluator.

### 5. Evidence is admitted and bound to the current change

Receipts bind producer and tool identity, capability version, generation time,
validity, repository, base, head, workflow, policy, result, and content digest
or equivalent attribution. Assertions are insufficient. Missing, stale,
unverified, malformed, or mismatched evidence cannot satisfy a requirement.

### 6. Regorus is the authoritative policy evaluator

The existing deterministic Rego/Regorus layer evaluates transition policy.
LLMs may explain or propose remediation but cannot authoritatively decide that
a transition is permitted. Only `allow` satisfies an enforcement boundary;
deny, degraded, and unsatisfiable outcomes remain blocking and explain why.

### 7. Approvals and exceptions reuse governed evidence contracts

The first approval adapter binds GitHub reviewer identity, authority, review
state, and exact head, behind a provider-neutral approval receipt. Deviations
reuse EXCEPT's scoped, attributed, expiring, revocable authority model. A host
administrator bypass is outside anvil's prevention boundary and is recorded as
ungoverned rather than silently treated as success.

### 8. Reuse Kindling and witness provenance

Transition decisions create durable governance facts through the established
Kindling contract. Portable tamper-evident proof uses the ADR-037 witness model,
with a content-addressed companion receipt for detailed evidence. GOVWF creates
no independent event store or provenance ledger. Cross-machine receipt
durability remains a readiness gate for the first implementation slice.

### 9. Reserve provider-neutral and organisational expansion

The decision model is provider-neutral even though GitHub is the first adapter.
A future local CLI/MCP transition API, other hosts, deployment gates, and
`/dev-loop` dogfooding consume the same evaluator. Organisation-level mandatory
workflow composition is deferred to ORGHIER/POLLC/POLFED; lower scopes will not
be permitted to remove higher-scope mandatory controls silently.

The full contract is
[`plans/specs/2026-09-18-governed-workflows.md`](../specs/2026-09-18-governed-workflows.md).

## Rationale

The exact-head pull-request boundary is the smallest place where anvil can
actually prevent a prohibited engineering progression using a host's normal
merge controls. It creates customer value without requiring anvil to schedule
or run every action. A true DAG and provider-neutral receipts avoid baking a
temporary linear checklist or GitHub-specific event shape into the core.

Resolving authority from the protected base closes the self-modification hole.
Registered capabilities close the arbitrary-execution hole. Reusing Regorus,
EXCEPT, Kindling, and witness preserves one policy, exception, governance-fact,
and portable-proof model.

### Alternatives considered

| Option | Benefit | Why rejected |
| --- | --- | --- |
| Prompt and documentation rules | Cheap and flexible | Advisory, agent-dependent, and not independently enforceable |
| GitHub Actions alone | Existing execution and merge controls | Does not provide a portable workflow/evidence/policy authority and is host-specific |
| anvil runs every action | Central control | Turns anvil into a workflow automation engine and expands the trusted execution surface |
| Inline shell commands in workflow definitions | Simple authoring | Lets untrusted definitions introduce execution and makes producers unauditable |
| Linear state machine only | Smaller first implementation | Cannot represent required parallel checks and joins without later breaking the model |
| Bind every workflow to APS | Reuses internal planning identity | Couples the product to anvil's own development process and excludes ordinary repositories |
| Pin only the workflow present when a PR opens | Stable instance | Permits stale governance after the protected base changes |
| Create a new hosted workflow ledger | Central query surface | Conflicts with local-first and existing Kindling/witness ownership before the core model is proven |

## Consequences

- **Positive:** governance no longer depends on agent obedience or prompt
  stability.
- **Positive:** the first slice has a real denial boundary and exact-change
  evidence.
- **Positive:** CI providers and future agents remain replaceable behind
  capability receipts.
- **Positive:** existing policy, exception, governance-fact, and witness
  substrates remain authoritative.
- **Negative:** teams must govern capability registration and evidence
  producer trust, not only workflow YAML.
- **Negative:** rebasing or changing the protected base may invalidate prior
  evidence and require reruns.
- **Risk:** a passing required check is misread as proof that GitHub admin
  bypass is impossible.
- **Mitigation:** state the enforcement boundary precisely and record observed
  host bypasses as ungoverned outcomes.
- **Risk:** the workflow model becomes an orchestration platform by accretion.
- **Mitigation:** retain the evidence-admission boundary; new execution
  ownership requires a superseding ADR.
- **Risk:** `/dev-loop` dogfooding is mistaken for customer delivery.
- **Mitigation:** keep `/dev-loop` explicitly downstream of the product API and
  outside GOVWF acceptance evidence.

## References

- [Governed workflows specification](../specs/2026-09-18-governed-workflows.md)
- [GOVWF module](../modules/governed-workflows.aps.md)
- [ADR-037](037-witness-chain-and-l4-policy.md)
- [ADR-040](040-rust-policy-engine-regorus.md)
- [ADR-072](072-git-native-governance-substrate.md)
- [ADR-098](098-policy-enforcement-reset-gate.md)
- [ADR-100](100-committed-exception-store-provenance.md)
