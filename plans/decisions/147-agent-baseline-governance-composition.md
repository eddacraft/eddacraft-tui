# ADR-147: Agent baseline governance composition

## Status

Proposed

## Date

2026-09-16

## Context

An enterprise agentic-engineering lead described an internal system that keeps
agent instructions and skills current across an organisation of roughly 900
people. The material must differ by organisation, business area, pod, team,
project, and cross-cutting functions such as security and platform. The same
problem also exists when every role belongs to one person: approved material
still becomes stale, local edits still drift, and updates still need an honest
ownership boundary.

ABASE already identifies the product boundary: anvil governs materialised
agent baseline artefacts and does not become a neutral authoring or
harness-compilation system. The Draft module did not yet decide how structural
and functional authorities compose, who resolves conflicts, what “latest”
means, how local ownership survives updates, or how a one-person context avoids
enterprise-only ceremony.

Existing decisions constrain the answer. ADR-132 separates resolved state from
runtime evidence. ADR-143 makes PSCAF the additive local reconciliation
substrate. ADR-106 owns anvil-authored managed skill installation. ADR-037 owns
the canonical witness chain. ORGHIER, POLLC, SKPKG, SKOBS, SETGOV, and EXCEPT
own adjacent policy, packaging, observation, mutation, and exception concerns.
ABASE must compose with them rather than creating rival systems.

## Decision

### 1. Structural hierarchy plus functional overlays

The structural lineage is organisation → business area → pod → team → project.
Security, platform, and other cross-cutting authorities publish functional
overlays selected independently against the repository's governance context.

The authority graph is explicit. Filesystem placement, login identity,
authentication order, and configuration discovery order never grant authority
or precedence. Contradictory applicable requirements block composition.

Authority roles are logical rather than person-count requirements. One
principal may own every authority and self-approve within that genuine scope;
the resulting resolution remains versioned and auditable.

### 2. Customer root and governed stewardship

The customer or organisation authority is the governance trust root. External
producers, consultancies, and subordinate authorities contribute through
signed, scoped, revocable delegation. Delegation permits additions by default.
Relaxation or replacement requires a separate capability naming its scope,
approving authority, and expiry.

A baseline steward operates composition, the slot schema, publication, and the
conflict workflow. The steward may publish a governed resolution but cannot
silently weaken another authority's requirement without the necessary
delegation or approval.

### 3. Explicit context and selectors

Each repository carries a tracked governance-context descriptor naming the
trust root, structural lineage, selected functional authorities, delegation
graph, trust-anchor references, target harnesses, and composite selection.
Missing or ambiguous context fails closed.

The tracked descriptor does not bootstrap its own trust. Its root reference
must validate against a pre-established customer anchor or root metadata signed
by an already trusted root. Root rotation is a separate authorised operation;
changing the descriptor and referenced key together cannot establish trust.

Contributions declare explicit selectors over that descriptor. Repository
allowlist maintenance and filesystem-position inference are not the primary
selection model.

One installation may serve unrelated contexts. Governance-context and
trust-root digests namespace every credential handle, private-material cache,
channel resolution, policy result, and evidence record. Switch-time clearing
is defence in depth; concurrent and sequential contexts share no authority or
private state.

### 4. Federated publication, central composition

Each authority publishes signed immutable contribution versions and may
advance its own `approved` channel. A contribution contains already-materialised
target projections. anvil does not compile neutral source into harness formats.

The baseline steward resolves applicable contributions and publishes one
steward-authenticated immutable composite binding context, authority graph,
slot schema, contribution digests, and authority resolutions. Repositories use
an exact composite digest or an `approved` channel resolved to an immutable
digest before inspection or mutation. There is no unqualified “latest”.

Every channel movement is authenticated by its delegated owner and binds a
monotonic generation, predecessor statement, target digest, validity bounds,
and authority graph. Replay is rejected. Intentional rollback uses a new
higher-generation statement. Delegation revocation invalidates derived channel
and composite caches; evidence distinguishes valid-at-resolution from
valid-now and reports unknown freshness when the source cannot be refreshed.

The first transport is a customer-controlled Git source. The signed manifest
and content digests establish publisher origin and immutability; Git transport
alone grants no authority.

### 5. Namespaced document sections and whole artefacts

Document projections use namespaced managed sections keyed by authority,
layer, component, and target surface. A steward-owned, versioned named-slot
schema determines deterministic order. Unmanaged content remains
repository-owned. Governed repository-specific additions are project-layer
contributions.

Whole managed artefacts such as skills use authority-qualified ownership
identities and whole-artefact digests. Publisher identity remains separate
signature-origin and provenance metadata. Distinct ownership identities
coexist. A duplicate identity conflicts unless its authority explicitly
delegates replacement. Skill contents are never mechanically merged.

ABASE, SKPKG, and SKOBS share one neutral component-identity contract. ABASE
owns desired composition and assurance; SKPKG retains anvil-authored bundle
ownership; SKOBS retains observed inventory ownership.

### 6. Honest freshness and update posture

ABASE reports orthogonal selection, projection condition, composition,
runtime-evidence, compliance-disposition, and approval-freshness dimensions.
An exception changes compliance disposition without erasing factual stale or
drifted state. “Resolved baseline” replaces “effective baseline” in product and
machine contracts. Presence never proves loading, and obedience remains
outcome evidence outside the ABASE state machine.

The default response to stale or drifted state is a reviewable update plan.
Policy may additionally open an automatic pull request. Direct silent fleet
rewriting is not the default. Solo owners may explicitly select stronger
automation for layers they wholly own.

Mutation reuses PSCAF's per-projection compare-and-swap substrate. ABASE owns a
transaction coordinator and durable recovery journal above the adapters,
including external-writer and indeterminate-recovery semantics. Locally
modified managed content produces a conflict plan rather than being
overwritten. Protection-affecting settings and hooks remain inspect-only until
their established governed mutation paths can perform the change.

### 7. Existing exception and provenance systems

Temporary deviations extend EXCEPT with governance context, requirement,
scope, authority, reason, and expiry. Approval must come from the requirement
owner or a specifically delegated, scoped, expiring exception capability;
ADR-100 committed-tip, revocation, expiry, and fail-safe rules remain binding.
Hidden local ignores and publisher-authored self-exemptions are invalid.

Detailed baseline evidence lives in a content-addressed companion receipt. The
ADR-037 witness envelope references its digest and location. ABASE creates no
parallel provenance ledger and makes no authentication or obedience claim from
hash chaining alone.

The complete product and pilot contract is
[`plans/specs/2026-09-16-agent-baseline-assurance.md`](../specs/2026-09-16-agent-baseline-assurance.md).

## Rationale

A structural hierarchy alone cannot model security and platform requirements
that span many teams. Unordered overlays cannot explain precedence or conflict.
The explicit authority graph and steward-owned slot schema preserve both
cross-cutting contribution and deterministic output.

Separating producer, publisher, authority, and steward prevents a valid
signature from becoming accidental permission. Federated publication lets each
domain owner update its material, while an immutable composite gives repository
owners one reviewable baseline.

Namespaced sections make shared documents additive without pretending every
projection is a text block. Whole managed skills retain digest and ownership
semantics instead of unsafe line merging. Reusing PSCAF, EXCEPT, settings
governance, and witness keeps ABASE focused on desired-state resolution and
freshness rather than rebuilding the surrounding platform.

### Alternatives considered

| Option | Benefit | Why rejected |
| --- | --- | --- |
| Structural hierarchy plus functional overlays and governed stewardship | Models organisation and cross-cutting ownership; deterministic; works for one or many people | Requires explicit context, delegation, selectors, and conflict records |
| One linear hierarchy including security and platform | Simple precedence | Misrepresents cross-cutting ownership and forces arbitrary placement |
| Most-specific or last writer wins | Minimal conflict workflow | Lets lower authority silently erase requirements and makes output order authority |
| Central authority publishes every contribution | One control point | Bottlenecks domain owners and obscures provenance of security/platform material |
| Repository teams fork the baseline | Maximum local autonomy | Recreates the staleness and drift problem ABASE exists to solve |
| anvil compiles neutral source into harness formats | One authoring model | Turns anvil into a skill authoring/compiler platform and couples it to every harness |
| Automatic direct updates by default | Fast convergence | Removes repository review, amplifies bad central changes, and conflicts with governed mutation |

## Consequences

- **Positive:** organisations can compose centrally governed and domain-owned
  material without whole-file replacement.
- **Positive:** the same contract collapses cleanly to a solo operator.
- **Positive:** “latest” and “current” become immutable, evidence-backed claims.
- **Positive:** repository-local additions and unmanaged prose remain visible
  and preserved.
- **Negative:** contributors need explicit identities, selectors, signatures,
  and slot assignments.
- **Negative:** conflicts require a real stewardship workflow rather than an
  automatic precedence shortcut.
- **Risk:** a steward is mistaken for an all-powerful override authority.
- **Mitigation:** relaxation remains capability-scoped and authority-approved.
- **Risk:** component identity diverges across ABASE, SKPKG, and SKOBS.
- **Mitigation:** one shared neutral identity is a readiness gate.
- **Risk:** projection currency is misread as runtime loading or obedience.
- **Mitigation:** preserve orthogonal assurance dimensions and explicit unknown
  evidence; keep obedience outside the ABASE state machine.
- **Risk:** the first release expands into remote fleet administration.
- **Mitigation:** prove one local repository and two contexts before automatic
  PRs, dashboards, or remote distribution.

## References

- [Agent baseline assurance specification](../specs/2026-09-16-agent-baseline-assurance.md)
- [ABASE module](../modules/agent-baseline-assurance.aps.md)
- [ADR-037](037-witness-chain-and-l4-policy.md)
- [ADR-106](106-agent-integration-registry-and-managed-installers.md)
- [ADR-132](132-settings-truth-contract.md)
- [ADR-143](143-project-scaffold-reconciliation.md)
