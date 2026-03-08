import { afterEach, describe, expect, it, vi } from 'vitest';
import type { CandidateProposal, MemoryObject, PromoteProposalInput } from '../contracts/index.js';
import { createObservationId, createProposalId, createSessionId } from '../contracts/index.js';
import type { CreateMemoryInput, IEmberPort } from '../contracts/ports/index.js';
import type { IVersionTracker, IMemoryStoreOperations } from './store-interfaces.js';
import { PromotionService } from './promotion-service.js';

afterEach(() => {
  vi.restoreAllMocks();
});

describe('PromotionService', () => {
  it('promotes an active proposal into a memory', async () => {
    const proposal = createProposal();
    const store = createStoreMock();
    const emberPort = createEmberPortMock(proposal);
    const versionTracker = createVersionTrackerMock();
    const service = new PromotionService({
      store,
      emberPort,
      versionTracker,
      config: {
        require_reason: true,
        require_attribution: true,
        min_ember_confidence: 0.5,
      },
    });

    const memory = await service.promoteProposal(createPromotionInput(proposal.id));

    expect(memory.status).toBe('active');
    expect(memory.provenance.ember_source?.proposal_id).toBe(proposal.id);
    expect(store.saveMemory).toHaveBeenCalledTimes(1);
    expect(emberPort.markPromoted).toHaveBeenCalledWith(proposal.id, memory.id, 'joshua');
    expect(versionTracker.trackChange).toHaveBeenCalledTimes(1);
  });

  it('rejects promotion input that violates required reason/attribution', async () => {
    const service = new PromotionService({
      store: createStoreMock(),
      config: {
        require_reason: true,
        require_attribution: true,
        min_ember_confidence: 0.2,
      },
    });

    const invalidInput = createPromotionInput(
      createProposalId('550e8400-e29b-41d4-a716-446655440002')
    );
    invalidInput.reason = '';
    invalidInput.promoted_by = '';

    const validation = service.validatePromotionInput(invalidInput);
    expect(validation.valid).toBe(false);
    expect(validation.errors).toContain('Promotion reason is required');
    expect(validation.errors).toContain('Promotion attribution is required');
    await expect(service.promoteProposal(invalidInput)).rejects.toThrow('Invalid promotion input');
  });

  it('fails when proposal is missing in Ember', async () => {
    const service = new PromotionService({
      store: createStoreMock(),
      emberPort: createEmberPortMock(null),
    });

    await expect(
      service.promoteProposal(
        createPromotionInput(createProposalId('550e8400-e29b-41d4-a716-446655440003'))
      )
    ).rejects.toThrow('Proposal not found');
  });

  it('rejects promotion when Ember confidence is below configured minimum', async () => {
    const lowConfidenceProposal = createProposal({ confidence: 0.2 });
    const service = new PromotionService({
      store: createStoreMock(),
      emberPort: createEmberPortMock(lowConfidenceProposal),
      config: {
        require_reason: true,
        require_attribution: true,
        min_ember_confidence: 0.5,
      },
    });

    await expect(
      service.promoteProposal(createPromotionInput(lowConfidenceProposal.id))
    ).rejects.toThrow('below minimum');
  });

  it('creates a memory directly without Ember proposal promotion', async () => {
    const store = createStoreMock();
    const service = new PromotionService({ store, versionTracker: createVersionTrackerMock() });

    const memory = await service.createMemory(createDirectMemoryInput());

    expect(memory.attribution.actor).toBe('joshua');
    expect(memory.type).toBe('decision');
    expect(store.saveMemory).toHaveBeenCalledWith(memory);
  });

  it('promotes without emberPort using deterministic provenance derived from proposal ID', async () => {
    const store = createStoreMock();
    const service = new PromotionService({ store });

    const proposalId = createProposalId('550e8400-e29b-41d4-a716-446655440099');
    const input = createPromotionInput(proposalId);
    const memory = await service.promoteProposal(input);

    expect(memory.status).toBe('active');
    expect(memory.provenance.kindling_sources.length).toBeGreaterThanOrEqual(1);
    expect(memory.provenance.source_sessions.length).toBeGreaterThanOrEqual(1);
    // Offline provenance uses the proposal ID itself — no random UUIDs
    expect(memory.provenance.kindling_sources[0].observation_id).toBe(proposalId);
    expect(memory.provenance.source_sessions[0]).toBe(proposalId);
    expect(store.saveMemory).toHaveBeenCalledWith(memory);
  });

  it('rejects promotion when proposal status is not active', async () => {
    const dismissedProposal = createProposal({ status: 'dismissed' });
    const service = new PromotionService({
      store: createStoreMock(),
      emberPort: createEmberPortMock(dismissedProposal),
    });

    await expect(
      service.promoteProposal(createPromotionInput(dismissedProposal.id))
    ).rejects.toThrow('Proposal is not active');
  });

  it('promotes a proposal without Ember port fallback by using synthetic proposal', async () => {
    const store = createStoreMock();
    const versionTracker = createVersionTrackerMock();
    const activeProposal = createProposal();
    const emberPort = createEmberPortMock(activeProposal);
    const service = new PromotionService({
      store,
      emberPort,
      versionTracker,
      config: {
        require_reason: true,
        require_attribution: true,
        min_ember_confidence: 0.5,
      },
    });

    const input = createPromotionInput(activeProposal.id);
    const memory = await service.promoteProposal(input);

    expect(memory.status).toBe('active');
    expect(memory.confidence).toBe('high');
    expect(memory.provenance.ember_source?.proposal_id).toBe(input.proposal_id);
    expect(store.saveMemory).toHaveBeenCalledTimes(1);
    expect(versionTracker.trackChange).toHaveBeenCalledTimes(1);
    expect(emberPort.markPromoted).toHaveBeenCalledWith(
      input.proposal_id,
      memory.id,
      input.promoted_by
    );
  });

  it('validates promotion input with relaxed configuration allowing empty reason and attribution', () => {
    const service = new PromotionService({
      store: createStoreMock(),
      config: {
        require_reason: false,
        require_attribution: false,
        min_ember_confidence: 0.2,
      },
    });

    const inputWithEmptyFields = createPromotionInput(
      createProposalId('550e8400-e29b-41d4-a716-446655440005')
    );
    inputWithEmptyFields.reason = '';
    inputWithEmptyFields.promoted_by = '';

    const validation = service.validatePromotionInput(inputWithEmptyFields);

    expect(validation.valid).toBe(true);
    expect(validation.errors).toHaveLength(0);
  });

  it('rejects promotion input with Zod schema violations', () => {
    const service = new PromotionService({
      store: createStoreMock(),
    });

    const validInput = createPromotionInput(
      createProposalId('550e8400-e29b-41d4-a716-446655440006')
    );
    const inputMissingProposalId = { ...validInput } as unknown as typeof validInput;
    delete (inputMissingProposalId as Partial<typeof validInput>).proposal_id;

    const validation = service.validatePromotionInput(inputMissingProposalId as typeof validInput);

    expect(validation.valid).toBe(false);
    expect(validation.errors.length).toBeGreaterThan(0);
  });

  it('handles invalid when date in context by falling back to current timestamp', async () => {
    const store = createStoreMock();
    const versionTracker = createVersionTrackerMock();
    const proposal = createProposal();
    const emberPort = createEmberPortMock(proposal);
    const service = new PromotionService({
      store,
      emberPort,
      versionTracker,
      config: {
        require_reason: true,
        require_attribution: true,
        min_ember_confidence: 0.5,
      },
    });

    const input = createPromotionInput(proposal.id);
    input.context.when = 'not-a-valid-date';

    const memory = await service.promoteProposal(input);

    expect(memory.status).toBe('active');
    expect(memory.provenance.ember_source?.created_at).toBeDefined();
    expect(store.saveMemory).toHaveBeenCalledTimes(1);
  });
});

function createStoreMock(): IMemoryStoreOperations {
  return {
    getMemory: vi.fn(async () => null),
    saveMemory: vi.fn(async (_memory: MemoryObject) => undefined),
    getMemoryByProposalId: vi.fn(async () => null),
    queryMemories: vi.fn(async () => ({
      memories: [],
      total: 0,
      limit: 100,
      offset: 0,
      has_more: false,
    })),
    getActiveMemories: vi.fn(async () => []),
    getMemoriesByType: vi.fn(async () => []),
    searchMemories: vi.fn(async () => []),
    memoryExists: vi.fn(async () => false),
    countMemories: vi.fn(async () => 0),
    getStats: vi.fn(async () => ({
      total_memories: 0,
      by_status: [],
      by_type: [],
      by_confidence: [],
      active_count: 0,
      superseded_count: 0,
      retired_count: 0,
      unique_tags_count: 0,
    })),
    isAvailable: vi.fn(async () => true),
    exportMemories: vi.fn(async () => []),
    importMemories: vi.fn(async () => 0),
  };
}

function createVersionTrackerMock(): IVersionTracker {
  return {
    init: vi.fn(async () => undefined),
    trackChange: vi.fn(async () => 'hash-1'),
    getHistory: vi.fn(async () => []),
    isInitialised: vi.fn(async () => true),
  };
}

function createEmberPortMock(proposal: CandidateProposal | null): IEmberPort {
  return {
    createProposal: vi.fn(async () => {
      throw new Error('Not used in this test');
    }),
    updateProposal: vi.fn(async () => null),
    resolveProposal: vi.fn(async () => null),
    getProposal: vi.fn(async () => proposal),
    queryProposals: vi.fn(async () => ({
      proposals: [],
      total: 0,
      limit: 100,
      offset: 0,
      has_more: false,
    })),
    getActiveProposals: vi.fn(async () => []),
    getProposalsBySession: vi.fn(async () => []),
    proposalExists: vi.fn(async () => proposal !== null),
    markPromoted: vi.fn(async () => undefined),
    markDismissed: vi.fn(async () => undefined),
    getExpiredProposals: vi.fn(async () => []),
    processExpiredProposals: vi.fn(async () => 0),
    expireStaleProposals: vi.fn(async () => 0),
    isAvailable: vi.fn(async () => true),
    getStats: vi.fn(async () => ({
      total_proposals: 0,
      by_status: [],
      by_type: [],
      expiring_soon: 0,
    })),
    countProposals: vi.fn(async () => 0),
    pruneProposals: vi.fn(async () => 0),
  };
}

function createProposal(overrides?: Partial<CandidateProposal>): CandidateProposal {
  return {
    id: createProposalId('550e8400-e29b-41d4-a716-446655440001'),
    type: 'decision',
    status: 'active',
    summary: 'Adopt strict linting for memory services',
    rationale: 'Prevents drift and enforces quality expectations',
    confidence: 0.8,
    signals: [],
    provenance: {
      observation_ids: ['550e8400-e29b-41d4-a716-446655440010'],
      session_ids: ['550e8400-e29b-41d4-a716-446655440020'],
      proposal_id: '550e8400-e29b-41d4-a716-446655440001',
      earliest_observation: '2026-03-01T10:00:00.000Z',
      latest_observation: '2026-03-01T11:00:00.000Z',
    },
    created_at: '2026-03-01T12:00:00.000Z',
    expires_at: '2026-03-31T12:00:00.000Z',
    ttl_days: 30,
    ...overrides,
  };
}

function createPromotionInput(
  proposalId: PromoteProposalInput['proposal_id']
): PromoteProposalInput {
  return {
    proposal_id: proposalId,
    type: 'decision',
    statement: 'Use strict linting for Edda service implementations',
    confidence: 'high',
    confidence_rationale: 'Directly affirmed during review',
    context: {
      when: '2026-03-01T12:00:00.000Z',
      why: 'Consistency and auditability are required',
      conditions: ['Service layer changes'],
      tags: ['quality', 'edda'],
    },
    promoted_by: 'joshua',
    reason: 'This guidance is now stable enough for canonical memory',
  };
}

function createDirectMemoryInput(): CreateMemoryInput {
  return {
    type: 'decision',
    statement: 'Always use explicit dependency interfaces in Edda services',
    context: {
      when: '2026-03-02T09:00:00.000Z',
      why: 'Prevents coupling during phased delivery',
      conditions: ['Parallel implementation phases'],
      tags: ['architecture', 'interfaces'],
    },
    confidence: 'high',
    confidence_rationale: 'Repeatedly validated in integration work',
    provenance: {
      kindling_sources: [
        {
          observation_id: createObservationId('550e8400-e29b-41d4-a716-446655440100'),
          session_id: createSessionId('550e8400-e29b-41d4-a716-446655440101'),
          kind: 'action_executed',
          timestamp: '2026-03-02T09:00:00.000Z',
        },
      ],
      source_sessions: [createSessionId('550e8400-e29b-41d4-a716-446655440101')],
    },
    created_by: 'joshua',
    reason: 'Core implementation boundary should be explicit and durable',
  };
}
