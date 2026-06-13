/**
 * GV2-014 — Plan/provenance graph contract: worked-trace fixture.
 *
 * Realises the Graph v2 spine's worked join trace
 * (docs/architecture/graph-v2-foundation-spec.md, "Worked join trace") as an
 * executable fixture, proving the plan/provenance join is followable end-to-end
 * using the *shared* edda-stack provenance contract (`ProvenanceChain`) — not a
 * second provenance type — with anchors crossing the Rust↔TS boundary by
 * reference only.
 *
 * The join row (`ChangeJoinTrace`) is **anchor-only**: every hop is a reference,
 * and the Edda provenance body is reached only by resolving `memory` (a
 * `MemoryId`) against the TS-authoritative store — modelled here by
 * `resolveProvenance`, a projection-tier read returning the shared
 * `ProvenanceChain`. One code change links to an APS item (optional), a commit, a
 * graph delta, a trust-posture change, a policy verdict, and the resolved
 * provenance record. A planless variant (no APS anchor) still resolves, proving
 * APS is not a runtime prerequisite (planless-first).
 *
 * The `ChangeJoinTrace` / anchor interfaces below are **test-fixture-only
 * illustration shapes**, not shipped contracts — the only provenance contract is
 * the reused `ProvenanceChain` from `./provenance.ts` (criterion: share one
 * contract, do not design a second).
 *
 * @module @eddacraft/anvil-edda-stack/contracts/provenance-join.test
 */
import { describe, expect, it } from 'vitest';
import { ProvenanceChainSchema, type ProvenanceChain } from './provenance.js';
import type { MemoryId, ObservationId, PlanId, ProposalId, SessionId } from './identifiers.js';
import type { Timestamp } from './temporal.js';

// =============================================================================
// Anvil-side anchors
//
// Rust-owned in production (semantic / dependency / control / trust graphs).
// Modelled here only as the reference keys that cross the language boundary —
// never the bodies. These are illustrative anchor shapes, not new shipped
// contracts.
// =============================================================================

/** Stable symbol identity — GV2-002 `(file, kind, name, ordinal)`. */
interface SymbolIdentityAnchor {
  file: string;
  kind: string;
  name: string;
  ordinal: number;
}

/** Graph-state change — GV2-003 delta ref (`schema_version` + touched identities). */
interface GraphDeltaAnchor {
  schema_version: number;
  touched: SymbolIdentityAnchor[];
}

/** Trust-posture change — GV2-012 `TrustPostureChange`, identity-anchored. */
interface TrustPostureAnchor {
  symbol: SymbolIdentityAnchor;
  change: 'Classified' | 'Reclassified' | 'Declassified';
}

/** Certify/policy verdict — Anvil's warn-over-block enforcement decision. */
type PolicyVerdict = 'allow' | 'warn' | 'block';

/**
 * The plan/provenance join row: **anchor-only**. Every hop is a reference; there
 * is no inlined provenance body — the Edda record is reached only by resolving
 * `memory` (off the hot path). `apsItem` and `trustPosture` are optional anchors
 * (APS is planless-first; a posture change is emitted only when trust moves).
 */
interface ChangeJoinTrace {
  change: { file: string; symbol: SymbolIdentityAnchor };
  apsItem?: PlanId; // e.g. 'PAY-007' — optional enrichment
  commit: string; // git SHA
  delta: GraphDeltaAnchor;
  trustPosture?: TrustPostureAnchor; // emitted only when trust moves
  policy: PolicyVerdict;
  memory: MemoryId; // Edda ref → resolve (projection tier) to the ProvenanceChain
}

// =============================================================================
// Fixture
// =============================================================================

const ts = '2026-06-13T10:00:00.000Z' as Timestamp;
const observationId = '550e8400-e29b-41d4-a716-446655440000' as ObservationId;
const sessionId = '550e8400-e29b-41d4-a716-446655440010' as SessionId;
const proposalId = '550e8400-e29b-41d4-a716-446655440020' as ProposalId;
const memoryId = '550e8400-e29b-41d4-a716-446655440030' as MemoryId;
const PAY_007 = 'PAY-007' as PlanId;

/** The changed symbol from the spine's worked trace. */
const chargeCard: SymbolIdentityAnchor = {
  file: 'src/pay.ts',
  kind: 'function',
  name: 'chargeCard',
  ordinal: 0,
};

/** Build the shared `ProvenanceChain` (Kindling → Ember → Edda) for the change. */
function buildProvenance(opts: { aps?: PlanId } = {}): ProvenanceChain {
  return ProvenanceChainSchema.parse({
    ember_source: {
      proposal_id: proposalId,
      proposal_type: 'decision',
      confidence: 0.82,
      created_at: ts,
    },
    kindling_sources: [
      {
        observation_id: observationId,
        session_id: sessionId,
        kind: 'file_modified',
        timestamp: ts,
      },
    ],
    source_sessions: [sessionId],
    ...(opts.aps ? { related_plans: [opts.aps] } : {}),
  });
}

/**
 * Stand-in for the TS-authoritative Edda store: `MemoryId` → `ProvenanceChain`.
 * `resolveProvenance` is the projection-tier read that follows the anchor's
 * `memory` ref to the body — the body never travels with the anchor.
 */
function resolveProvenance(
  memory: MemoryId,
  opts: { aps?: PlanId } = {}
): ProvenanceChain | undefined {
  const store = new Map<MemoryId, ProvenanceChain>([[memoryId, buildProvenance(opts)]]);
  return store.get(memory);
}

/** Assemble the anchor-only worked-trace join row (`apsItem` omitted when absent). */
function buildTrace(opts: { aps?: PlanId } = {}): ChangeJoinTrace {
  return {
    change: { file: 'src/pay.ts', symbol: chargeCard },
    ...(opts.aps ? { apsItem: opts.aps } : {}),
    commit: 'a1b2c3d4e5f6',
    delta: { schema_version: 1, touched: [chargeCard] },
    trustPosture: { symbol: chargeCard, change: 'Reclassified' },
    policy: 'allow',
    memory: memoryId,
  };
}

// =============================================================================
// The worked trace
// =============================================================================

describe('GV2-014 plan/provenance join — worked-trace fixture', () => {
  it('links one code change to APS item, commit, graph delta, trust-posture change, policy, and a ProvenanceChain-validated Edda record', () => {
    const trace = buildTrace({ aps: PAY_007 });

    // Every hop of the spine's worked trace is followable by a defined anchor.
    expect(trace.change.symbol.name).toBe('chargeCard');
    expect(trace.apsItem).toBe('PAY-007');
    expect(trace.commit).toMatch(/^[0-9a-f]{7,40}$/);
    expect(trace.delta.touched).toContainEqual(chargeCard);
    expect(trace.trustPosture).toEqual({ symbol: chargeCard, change: 'Reclassified' });
    expect(trace.policy).toBe('allow');

    // Resolving the `memory` ref (projection tier) yields the shared, validated
    // provenance contract — joined to this change via the APS anchor.
    const resolved = resolveProvenance(trace.memory, { aps: PAY_007 });
    expect(resolved).toBeDefined();
    expect(ProvenanceChainSchema.safeParse(resolved).success).toBe(true);
    expect(resolved?.related_plans).toEqual(['PAY-007']);
    expect(resolved?.kindling_sources[0]?.kind).toBe('file_modified');
  });

  it('resolves the same trace planless — no APS anchor — proving APS is not a prerequisite', () => {
    const planless = buildTrace(); // no aps
    const planned = buildTrace({ aps: PAY_007 });

    // The optional APS anchor is genuinely absent, not present-but-undefined.
    expect(planless).not.toHaveProperty('apsItem');
    expect(planned.apsItem).toBe('PAY-007');

    const planlessProv = resolveProvenance(planless.memory);
    const plannedProv = resolveProvenance(planned.memory, { aps: PAY_007 });
    expect(planlessProv?.related_plans).toBeUndefined();
    expect(plannedProv?.related_plans).toEqual(['PAY-007']);

    // Both validate against the shared contract: the APS anchor is enrichment, not
    // a requirement — the join stays followable from source + git + Edda alone.
    expect(ProvenanceChainSchema.safeParse(planlessProv).success).toBe(true);
    expect(ProvenanceChainSchema.safeParse(plannedProv).success).toBe(true);
    expect(planless.commit).toMatch(/^[0-9a-f]{7,40}$/);
  });

  it('keeps the join ref-only — the anchor row carries no provenance body, only a MemoryId (C-6)', () => {
    const trace = buildTrace({ aps: PAY_007 });

    // Compile-time guarantee: `ChangeJoinTrace` has no provenance field, so the
    // anchor row structurally cannot inline a body — the only provenance link is
    // the `memory` ref. (This `satisfies` fails to compile if a body is added.)
    const anchorOnly = trace satisfies ChangeJoinTrace;
    expect(anchorOnly).not.toHaveProperty('provenance');
    expect(typeof trace.memory).toBe('string');

    // The body is reachable only by the separate projection-tier resolution, and
    // it validates against the shared contract.
    expect(
      ProvenanceChainSchema.safeParse(resolveProvenance(trace.memory, { aps: PAY_007 })).success
    ).toBe(true);
  });
});
