# ADR-072: Git-Native Governance Substrate

## Status

**Accepted** — 2026-06-08, full council review (accept-with-changes; the
required changes — named scan-on-write enforcement surfaces — are applied in
the accepting commit)

## Date

2026-06-06

## Context

Anvil already enforces AI-assisted-development governance across save-time
(L2 daemon), pre-commit (L3 hook), pre-push/CI (L4), and audit (L5) layers, and
already emits durable, tracked evidence:

- the hash-chained witness ledger under `anvil/witness/` (ADR-037, the
  `anvil-witness` crate),
- the adoption baseline at `anvil/baseline.json` (ADR-039, `anvil-baseline`),
- the append-only drift edge-delta ledger at `anvil/drift/edges.ndjson`
  (ADR-052),
- project identity at `anvil/project-id`.

Each of these is already a *tracked, in-tree, content-addressable* governance
artefact. What is missing is a stated decision that ties them together: that Git
itself — not a hosted service — is Anvil's durable trust substrate, and that new
governance evidence (review capsules, exceptions, sealed Edda provenance,
release attestations) should follow the same posture rather than reaching for a
cloud datastore.

This matters now because the next product claim — "a governed AI-assisted change
can be packaged and verified locally, without trusting Anvil Cloud" (the Review
Capsule wedge, GITGOV) — requires a substrate decision before any new evidence
schema is frozen. Without it, each new evidence type risks inventing its own
storage location and trust model.

The brainstorm pack at
[`plans/brainstorms/git-native-governance/`](../brainstorms/git-native-governance/)
captures the full strategic framing. This ADR records only the substrate
decision; the durable/local path boundary is ADR-073 and the capsule v0 format
is ADR-074.

## Decision

**Anvil uses Git as the durable governance substrate.** Durable governance
evidence is stored as tracked, content-addressed, tamper-evident artefacts in
the repository — files under `anvil/` for v0, with Git refs/notes reserved for a
later phase — and is verifiable offline using the repository itself.

This applies to:

- witness evidence (already shipped — ADR-037),
- policy, rule-set, and baseline history (already tracked),
- drift ledgers (already shipped — ADR-052),
- exception grants/revocations (ADR-073 moves these in-tree),
- review capsules (ADR-074),
- sealed Edda memory/provenance (future EDDA-SEAL),
- release attestations and supplier bundles (future).

Boundaries this decision sets:

1. **Git proves; it does not serve hot reads.** High-frequency telemetry,
   mutable queues, and millisecond hot-path queries stay in local stores
   (`.anvil/kindling.db`, `.anvil/ember.db`, `.anvil/cache/`). The
   Kindling/Ember/Graph-V2 split is unchanged; Git is the canonical *evidence
   and transport* layer, and local SQLite/cache stores are projections, never
   authority. This is consistent with the three-pipe rule (ADR-035): Kindling
   stays the governance-fact pipe of record.
2. **Cloud is an optional amplifier, never the source of truth.** Any future
   Anvil Cloud surface (fleet view, GitHub App, supplier portal) federates or
   amplifies repository evidence; local enforcement and verification must never
   depend on it. This preserves the local-first / air-gapped posture (ADR-001).
3. **No secrets in durable Git evidence.** Durable artefacts store redacted
   summaries, digests, policy-relevant fields, and source pointers — never raw
   secrets, environment dumps, token-bearing output, or unredacted runtime
   state. This generalises the privacy line already drawn for the Graph-V2
   snapshot (ADR-069 §"Privacy line").
   **Enforcement surfaces (testable, not aspirational):** every free-text
   field headed for durable evidence is a scan-on-write surface — exception
   `reason` strings (EXCEPT), Edda `statement`/`context`/`metadata` prose
   (EDDA-SEAL-001 acceptance criterion, ADR-073), and capsule evidence files
   (GITGOV-012 includes a secret-in-evidence test). Secret findings are
   already redacted at the producer (`anvil-checks` `redact_secret` /
   `redacted_match`), so capsule SARIF inherits redaction; scan-on-write
   covers content that never passed through a producer.
4. **Honest verdicts only.** Verification of durable evidence uses closed-state
   verdicts where missing/incomplete evidence is `degraded`, not `pass`
   (formalised for capsules in ADR-074). Consistent with Anvil's "tooling
   honesty" doctrine and ADR-002 (the verdict is advisory evidence, not a new
   blocking gate on user code).

## Rationale

Git is already present in every target workflow and natively provides content
addressing, tamper-evident history, distributed replication, offline transfer,
durable refs/tags, evidence attachment (notes), and a review model developers
understand. Anvil already relies on exactly these properties for the witness
chain and drift ledger; this ADR ratifies the pattern rather than inventing one.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| Git as durable substrate (chosen) | Local-first, air-gap friendly, offline-verifiable, reuses shipped witness/baseline/drift posture, no new infra | Git storage can get noisy without discipline; redaction must be strict; evidence schemas must be migration-aware |
| Hosted evidence datastore (Anvil Cloud canonical) | Central query, fleet analytics, easy cross-repo | Breaks air-gapped/local-first (ADR-001); makes verification depend on a service; weakens the product claim |
| Local SQLite as canonical durable store | Fast queries, already used by Kindling | Not portable/tamper-evident across machines; not reviewable in PRs; doesn't travel with the repo |
| Do nothing (leave each evidence type ad hoc) | No upfront work | Every new evidence type re-decides storage + trust model; guarantees drift (e.g. exceptions already landed in the wrong tree — ADR-073) |

## Consequences

- **Positive:** A single, stated trust model for all governance evidence;
  offline/air-gapped verification; portable review/audit artefacts; no cloud
  dependency for enforcement or verification; reuses primitives already in
  production.
- **Positive:** New evidence types (capsules, exceptions, sealed provenance) get
  a default home and posture, avoiding per-feature storage decisions.
- **Negative:** Imposes redaction and schema-migration discipline on every
  durable artefact; working-tree noise must be managed (mitigated by generating
  capsules on demand and keeping high-volume evidence out of the tree until
  refs/notes land).
- **Risks:** Premature scope creep toward a full GRC platform; freezing evidence
  schemas against fictional shapes (e.g. the brainstorm's `WitnessExtract` does
  not match the real `WitnessLine`).
- **Mitigations:** Keep v0 file-first and inspectable (ADR-074); reconcile every
  capsule sub-schema against the real producing crate before freezing;
  refs/notes, release seals, and supplier bundles stay explicitly deferred.

## References

- Related ADRs: ADR-001 (planless-first / local-first), ADR-002 (warnings over
  blocks), ADR-035 (three-pipe observability), ADR-037 (witness chain & L4),
  ADR-039 (baseline policy), ADR-052 (drift edge-delta ledger), ADR-069 (GV2
  persistence privacy line), ADR-073 (durable vs local state), ADR-074 (capsule
  v0 format)
- APS modules: GITGOV (`plans/archive/modules/git-native-governance.aps.md`), EXCEPT
  (`plans/modules/git-native-exceptions.aps.md`)
- Brainstorm: `plans/brainstorms/git-native-governance/`
