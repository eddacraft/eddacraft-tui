# Agent baseline assurance

| Field | Value |
| --- | --- |
| Status | Final, operator-approved 2026-09-16 |
| Date | 2026-09-16 |
| Decision | [ADR-147](../decisions/147-agent-baseline-governance-composition.md) |
| APS | [ABASE](../modules/agent-baseline-assurance.aps.md) |

## 1. Summary

ABASE lets an organisation publish an approved agent baseline, resolve the
parts that apply to one repository, detect missing or stale material, and
reconcile managed content without erasing repository-owned additions.

The structural authority chain is organisation → business area → pod → team →
project. Functional authorities such as security and platform contribute
independently scoped overlays. A central baseline steward composes those inputs
into an immutable approved baseline for a governance context. The roles are
logical: in a small organisation or solo project, one principal may hold every
role without changing the contract.

The baseline may project managed sections into documents such as `AGENTS.md`
and READMEs, and may install whole managed artefacts such as skill directories
or agent definitions. anvil consumes already-materialised projections. It does
not author neutral content or compile one source into every harness format.

## 2. Product outcome

For one repository and governance context, anvil answers:

- which baseline components are required;
- which immutable versions were resolved from the approved composite;
- whether the expected projections are missing, stale, drifted, incompatible,
  conflicted, or current;
- which update would restore the approved baseline;
- which local content remains outside baseline ownership; and
- which exact baseline and exceptions were present when later work was
  witnessed.

The initial customer is the agentic or platform engineering team that owns the
organisation's harness and agent-content distribution. The initial adoption
wedge is coexistence: anvil provides deterministic composition, freshness,
drift, reconciliation, and evidence around an existing producer or internal
distribution system rather than replacing it.

Before ABASE becomes executable, the demand hypothesis must be checked with
the originating mature organisation and at least one less-mature organisation.
At least one design partner must provide representative materialised artefacts
for the bounded pilot described below.

## 3. Authority and context

### 3.1 Logical roles

| Role | Responsibility |
| --- | --- |
| Customer or organisation authority | Root of trust for one governance context. |
| Structural contributor | Publishes organisation, business-area, pod, team, or project requirements. |
| Functional contributor | Publishes independently scoped security, platform, or similar overlays. |
| Baseline steward | Operates composition, slot ordering, publication, conflict workflow, and update policy. |
| Repository owner | Reviews or applies the resolved update for one repository. |
| Producer | Creates already-materialised projections. It is not automatically an authority. |
| Publisher | Signs and versions a contribution. Its signature proves origin, not applicability or authority. |

One principal may hold several or all roles. Self-approval is valid when the
same principal genuinely owns the contributing and consuming authorities, but
the resulting decision remains versioned and auditable.

### 3.2 Governance context

A tracked repository descriptor selects the governance context. It identifies:

- the customer or organisation trust root;
- business area, pod, team, and project identities where applicable;
- selected functional authorities;
- the authority and delegation graph;
- trust-anchor references;
- target harnesses and repository characteristics used by selectors; and
- the approved composite channel or exact baseline pin.

There is no installation-wide current organisation. Missing or ambiguous
context fails closed. The descriptor cannot authenticate its own trust root:
its root reference must validate against a pre-established customer anchor or
root metadata signed by an already trusted root. Root rotation is a distinct,
authorised, auditable operation; replacing the descriptor and its referenced
key together never establishes a new root.

Every credential handle, private-material cache entry, channel resolution,
policy result, and evidence namespace is keyed by the governance-context and
trust-root digests. Clearing state during a repository or worktree switch is
defence in depth, not the isolation boundary. Concurrent contexts must be as
isolated as sequential ones.

### 3.3 Delegation and conflict authority

The customer or organisation authority is the trust root. A consultancy or
other external producer contributes only through signed, scoped, revocable
delegation.

Delegation is add-only by default. Relaxing or replacing another authority's
requirement needs a separate capability naming the affected requirement,
scope, approving authority, and expiry.

The baseline steward may adjudicate conflicts by publishing a governed,
versioned resolution. Stewardship is not blanket override authority: the
steward cannot silently weaken a contributor's requirement without the
required delegation or approval. Unresolved contradictions block composition.

## 4. Contributions and selectors

Each authority publishes immutable signed contribution versions and may move a
mutable `approved` channel between them. A contribution contains:

- publisher-qualified identity and version;
- manifest and projection content digests;
- publisher signature and trust-anchor reference;
- compatibility bounds;
- explicit selectors;
- one or more already-materialised projections;
- declared authority, layer, component, and target surface;
- component identity shared with SKPKG and SKOBS; and
- references to policy packs or other separately owned artefacts by digest.

Selectors evaluate only explicit fields from the tracked governance context,
including organisation, business area, pod, team, project, harness, and target
surface. Filesystem position, login order, authentication order, and account
history never select authority or applicability.

Each authority owns its immutable versions and approved channel. Channel
movement is an authenticated statement by the delegated channel owner. It
contains a monotonic generation, predecessor statement digest, target digest,
issued/expiry bounds, and authority-graph reference. Replaying an older
statement is rejected. An intentional rollback is a new higher-generation
statement pointing to the earlier immutable version, not reuse of old channel
metadata.

The baseline steward resolves applicable contributions, detects
contradictions, and publishes one steward-authenticated immutable composite
binding the governance-context digest, authority-graph digest, slot-schema
version, contribution digests, and authority resolutions. A repository may
select that composite by exact digest or through an authenticated `approved`
channel resolved to an immutable digest before inspection or mutation.
Delegation revocation invalidates cached channel statements and composites
derived from that delegation. Evidence distinguishes valid-at-resolution from
valid-now, and a source that cannot be refreshed reports cached/unknown
freshness rather than `current` approval.

## 5. Projection ownership and composition

### 5.1 Documents

Shared documents use namespaced managed sections addressed by:

```text
authority / layer / component / target-surface
```

The composite baseline owns a versioned slot schema. Contributors target named
slots they are permitted to use; they do not choose arbitrary numeric
precedence. Rendering is deterministic.

Content outside managed sections remains repository-owned. A repository that
wants a governed local addition publishes a project-layer contribution through
the same contract. Ordinary unmanaged prose remains preserved but is not
claimed as baseline-controlled.

### 5.2 Whole managed artefacts

Skills, agent definitions, and similar directory projections use
authority-qualified ownership identities and whole-artefact digests. Publisher
identity remains separate provenance and signature-origin metadata. Distinct
ownership identities coexist. Two applicable contributions that target the
same ownership identity conflict unless the existing authority explicitly
delegates replacement.

Skill contents are never mechanically merged. Whole-file or whole-directory
replacement is permitted only when the installed artefact still matches the
previous managed digest and the authority graph permits the transition.

### 5.3 Shared component identity

ABASE owns the baseline envelope, authority graph, selectors, composite, and
assurance state. It does not create a second skill or agent identity schema.
SKPKG, SKOBS, and ABASE must share a neutral component identity covering at
least authority-qualified ownership identity, source, publisher provenance,
version, digest, target, capability declarations, and observed-state
references.

SKPKG continues to own anvil-authored bundled skills. SKOBS continues to own
observed inventory. ABASE owns the desired composition and comparison between
resolved and observed state.

## 6. Assurance dimensions

The product and machine contracts use **resolved baseline**, not “effective
baseline”, to preserve ADR-132's configured/resolved/active vocabulary.

The machine contract reports orthogonal facts rather than one lossy state enum:

| Dimension | Values and meaning |
| --- | --- |
| Selection | `not-required`, `required`, or `resolved` to an authenticated immutable digest. |
| Projection | `missing`, `installed`, `stale`, `drifted`, or `current`; stale and drifted remain factual even when excepted. |
| Composition | `compatible`, `incompatible`, or `conflicted`. |
| Runtime evidence | `unknown`, `discoverable`, or `loaded`; each value records its evidence strength and time. |
| Compliance disposition | `satisfied`, `violating`, or `excepted` with a verified exception reference. |
| Approval freshness | `current`, `cached`, `expired`, `revoked`, or `unknown` for the channel and authority evidence used at resolution. |

Presence, configuration, or process detection never upgrades `discoverable` to
`loaded`. Whether an agent obeyed material is outcome evidence outside the
ABASE state machine and is never inferred from installation or loading.

## 7. Reconciliation and updates

Inspection is read-only by default. When a repository is not current, anvil
produces a reviewable update plan containing:

- the current and target composite digests;
- every selected contribution and channel resolution;
- exact managed-section and managed-artefact changes;
- preserved unmanaged content;
- conflicts, incompatibilities, and required approvals;
- the expected post-apply assurance state; and
- validation and rollback information.

The first supported automation mode, after the local pilot, opens a pull
request containing the reviewable plan and changes. Direct silent fleet
rewriting is out of scope. A solo owner may explicitly opt into stronger
automation for layers they wholly control, without changing the underlying
authority and evidence contract.

Mutation reuses PSCAF's additive, race-safe, per-projection reconciliation
substrate. ABASE supplies resolved desired components; it does not create a
second general scaffolder. ABASE owns a transaction coordinator and durable
recovery journal above those adapters. The contract explicitly accounts for
external writers outside the shared lock. An apply either publishes the
complete verified target state and receipt or recovers every projection to its
verified previous state; if recovery cannot be proved, it reports an
indeterminate partial state and blocks further mutation pending repair.

Locally modified managed content is never overwritten automatically. Digest or
identity mismatch produces a reviewable conflict plan. Protection-affecting
settings and hooks remain inspect-only until the established settings and
policy mutation workflows can perform the update without a second approval
path.

Shared projections are committed by default. A manifest may declare a
harness-required machine-local projection, but local projections cannot contain
credentials or secrets and cannot become hidden authority.

## 8. Exceptions and receipts

Temporary deviations extend the Git-native EXCEPT contract. An exception
names the governance context, requirement or component, repository/path scope,
approving authority, reason, and expiry. It is valid only when approved by the
requirement owner or by an explicit scoped, expiring exception delegation in
the authority graph. ADR-100 committed-tip provenance, revocation, expiry, and
fail-safe unreadable-state rules continue to apply. Local hidden ignores and
publisher-authored self-exemptions are invalid.

Each inspection or apply may emit a content-addressed
`anvil.agent-baseline-receipt.v1` companion payload containing:

- governance-context and authority-graph digests;
- selected composite and contribution digests;
- channel-resolution evidence;
- projection outcomes and observed digests;
- reconciliation result;
- exception references; and
- compatibility, partiality, and unknown-state disclosures.

The existing ADR-037 witness envelope references the receipt digest and
location. The receipt does not create a parallel ledger. Witness and receipt
hashes provide tamper-evident provenance, not proof that a publisher was
authorised or that an agent obeyed the material.

## 9. Product-learning checkpoint and integrated pilot

The earliest product-learning checkpoint is read-only. With representative
partner artefacts, it resolves organisation/team requirements plus security or
platform overlays, inspects one managed document surface and one managed skill,
reports their freshness and conflicts, and emits a reviewable update plan. It
does not wait for mutation, receipts, or automatic pull requests before testing
whether the result answers the design partner's real freshness problem.

The later integrated pilot uses one local repository, a customer-controlled Git
source, and two distinct governance contexts. It must prove:

1. One context descriptor resolves structural layers and functional overlays
   to one immutable composite digest.
2. Signed manifests and content digests reject an untrusted publisher,
   unauthorised relaxation, and mutated projection.
3. One managed document section and one managed skill directory reconcile
   without modifying unmanaged content.
4. Selection, projection, composition, runtime-evidence, compliance, and
   approval-freshness dimensions are deterministic.
5. A complete update applies transactionally and emits a verifiable receipt.
6. Switching contexts cannot reuse the previous context's private artefacts,
   credentials, channel resolution, policy, or evidence, under both concurrent
   and sequential evaluation.
7. The same contract works when every role maps to one principal.
8. A descriptor cannot substitute its own trust root; root rotation is
   authorised and auditable.
9. Replayed channel statements, unauthorised rollback, and revoked delegation
   cannot produce currently approved results.
10. Filesystem projections remain inside the canonical workspace and reject
    symlinked targets and path components without race-prone check-then-write.

Automatic pull requests are a follow-on projection after the local contract is
reliable. Remote fleet administration, dashboards, broad harness coverage, and
runtime capability enforcement remain later work.

## 10. Validation and readiness

The design is approved, but ABASE remains Draft until:

- the mature and less-mature design-partner interviews are recorded;
- one partner supplies representative materialised artefacts and agrees to the
  bounded pilot;
- ADR-147 is accepted;
- the shared component-identity and manifest contracts are reviewed with
  SKPKG and SKOBS ownership;
- ORGHIER and POLLC coordination records which semantics are shared and which
  remain policy-specific;
- PSCAF confirms the required managed-section and managed-directory adapter
  boundary;
- EXCEPT and witness extensions are reviewed by their owners; and
- exact implementation homes, fixtures, and validation commands replace the
  Draft candidate slices in the ABASE module.

## 11. Non-goals

- Neutral agent-content authoring or harness compilation.
- A public marketplace or arbitrary package manager.
- Last-write-wins composition.
- Hidden local authority or exceptions.
- Automatic direct fleet rewriting.
- Hosted identity, credential storage, or cross-customer trust brokerage.
- Claiming that installed instructions were loaded or obeyed.
- Replacing customer-specific producers, Delivery Shadow, SKPKG, SKOBS,
  PSCAF, SETGOV, EXCEPT, or the witness chain.
