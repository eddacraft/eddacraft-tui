import { randomUUID } from 'node:crypto';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMemoryId, createProposalId, createSessionId } from '../contracts/identifiers.js';
import type { CreateProposalInput, ProposalType } from '../contracts/ember-proposal.js';
import { ProposalAlreadyResolvedError } from '../contracts/ember-proposal.js';
import { ProposalStore } from './proposal-store.js';

function createInput(type: ProposalType = 'pattern', sessionId?: string): CreateProposalInput {
  const currentSessionId = sessionId ?? createSessionId(randomUUID());
  return {
    type,
    summary: `${type} summary`,
    rationale: `${type} rationale`,
    confidence: 0.7,
    ttl_days: 30,
    metadata: { source: type },
    signals: [{ rule: 'test-rule', contribution: 0.7, weight: 1 }],
    provenance: {
      observation_ids: [randomUUID()],
      session_ids: [currentSessionId],
      earliest_observation: new Date('2026-01-01T00:00:00.000Z').toISOString(),
      latest_observation: new Date('2026-01-01T00:05:00.000Z').toISOString(),
    },
  };
}

describe('ProposalStore', () => {
  let store: ProposalStore;

  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-01-01T00:00:00.000Z'));
    store = ProposalStore.createInMemory();
  });

  afterEach(() => {
    store.close();
    vi.useRealTimers();
  });

  it('creates, reads, and checks existence of proposals', async () => {
    const created = await store.createProposal(createInput('decision'));
    const fetched = await store.getProposal(created.id);

    expect(created.status).toBe('active');
    expect(fetched?.id).toBe(created.id);
    expect(fetched?.summary).toBe('decision summary');
    expect(await store.proposalExists(created.id)).toBe(true);
  });

  it('updates proposal fields and sets updated timestamp', async () => {
    const created = await store.createProposal(createInput('lesson'));
    vi.setSystemTime(new Date('2026-01-02T00:00:00.000Z'));

    const updated = await store.updateProposal(created.id, {
      summary: 'updated summary',
      confidence: 0.95,
      metadata: { refreshed: true },
    });

    expect(updated).not.toBeNull();
    expect(updated?.summary).toBe('updated summary');
    expect(updated?.confidence).toBe(0.95);
    expect(updated?.metadata).toEqual({ refreshed: true });
    expect(updated?.updated_at).toBe('2026-01-02T00:00:00.000Z');
  });

  it('resolves proposals as promoted and dismissed', async () => {
    const promoted = await store.createProposal(createInput('pattern'));
    const dismissed = await store.createProposal(createInput('warning'));
    const memoryId = createMemoryId(randomUUID());

    await store.markPromoted(promoted.id, memoryId, 'agent/promoter');
    await store.markDismissed(dismissed.id, 'Not useful', 'agent/reviewer');

    const promotedResult = await store.getProposal(promoted.id);
    const dismissedResult = await store.getProposal(dismissed.id);

    expect(promotedResult?.status).toBe('promoted');
    expect(promotedResult?.resolution?.memory_id).toBe(memoryId);
    expect(dismissedResult?.status).toBe('dismissed');
    expect(dismissedResult?.resolution?.resolution_reason).toBe('Not useful');
  });

  it('filters by type, status, confidence, date range, and session', async () => {
    const sharedSession = createSessionId(randomUUID());

    vi.setSystemTime(new Date('2026-01-01T00:00:00.000Z'));
    const first = await store.createProposal(createInput('pattern', sharedSession));
    vi.setSystemTime(new Date('2026-01-01T01:00:00.000Z'));
    const second = await store.createProposal(createInput('decision', sharedSession));
    vi.setSystemTime(new Date('2026-01-01T02:00:00.000Z'));
    const third = await store.createProposal({ ...createInput('warning'), confidence: 0.2 });

    await store.resolveProposal(second.id, {
      status: 'dismissed',
      resolved_by: 'agent/reviewer',
      resolution_reason: 'duplicate',
    });

    const result = await store.queryProposals({
      types: ['pattern', 'decision'],
      statuses: ['active', 'dismissed'],
      min_confidence: 0.6,
      created_after: '2026-01-01T00:30:00.000Z',
      created_before: '2026-01-01T01:30:00.000Z',
      session_id: sharedSession,
      include_expired: true,
      limit: 10,
      offset: 0,
      sort_by: 'created_at',
      sort_order: 'asc',
    });

    expect(result.total).toBe(1);
    expect(result.proposals[0]?.id).toBe(second.id);
    expect(result.proposals[0]?.status).toBe('dismissed');
    expect(result.proposals.find((proposal) => proposal.id === first.id)).toBeUndefined();
    expect(result.proposals.find((proposal) => proposal.id === third.id)).toBeUndefined();

    const bySession = await store.getProposalsBySession(sharedSession);
    expect(bySession).toHaveLength(2);
  });

  it('supports sorting and pagination', async () => {
    const low = await store.createProposal({ ...createInput('anomaly'), confidence: 0.2 });
    const mid = await store.createProposal({ ...createInput('anomaly'), confidence: 0.6 });
    const high = await store.createProposal({ ...createInput('anomaly'), confidence: 0.9 });

    const paged = await store.queryProposals({
      include_expired: true,
      sort_by: 'confidence',
      sort_order: 'asc',
      limit: 1,
      offset: 1,
    });

    expect(paged.total).toBe(3);
    expect(paged.has_more).toBe(true);
    expect(paged.proposals).toHaveLength(1);
    expect([low.id, mid.id, high.id]).toContain(paged.proposals[0]?.id);
    expect(paged.proposals[0]?.confidence).toBeCloseTo(0.6, 5);
  });

  it('throws on invalid sort_by field', async () => {
    await store.createProposal({ ...createInput('pattern'), confidence: 0.5 });

    expect(() =>
      store.queryProposals({
        include_expired: true,
        sort_by: 'not-a-real-field' as unknown as 'created_at',
        sort_order: 'desc',
        limit: 10,
        offset: 0,
      })
    ).toThrow('Invalid sort field: not-a-real-field');
  });

  it('throws on invalid sort_order direction', async () => {
    await store.createProposal({ ...createInput('pattern'), confidence: 0.5 });

    expect(() =>
      store.queryProposals({
        include_expired: true,
        sort_by: 'created_at',
        sort_order: 'sideways' as unknown as 'asc',
        limit: 10,
        offset: 0,
      })
    ).toThrow('Invalid sort direction: sideways');
  });

  it('finds and processes expired proposals', async () => {
    await store.createProposal({ ...createInput('constraint'), ttl_days: 1 });

    vi.setSystemTime(new Date('2026-01-03T00:00:00.000Z'));
    const expiredBefore = await store.getExpiredProposals();
    const processed = await store.processExpiredProposals();
    const expiredCount = await store.countProposals('expired');

    expect(expiredBefore).toHaveLength(1);
    expect(processed).toBe(1);
    expect(expiredCount).toBe(1);

    const expiredViaAlias = await store.expireStaleProposals();
    expect(expiredViaAlias).toBe(0);
  });

  it('returns aggregate stats', async () => {
    const active = await store.createProposal({ ...createInput('pattern'), confidence: 0.6 });
    const promoted = await store.createProposal({ ...createInput('decision'), confidence: 0.9 });
    await store.createProposal({ ...createInput('warning'), confidence: 0.4, ttl_days: 1 });

    await store.markPromoted(promoted.id, createMemoryId(randomUUID()), 'agent/promoter');
    await store.resolveProposal(active.id, {
      status: 'dismissed',
      resolved_by: 'agent/reviewer',
      resolution_reason: 'insufficient evidence',
    });

    vi.setSystemTime(new Date('2026-01-03T00:00:00.000Z'));
    await store.processExpiredProposals();

    const stats = await store.getStats();

    expect(stats.total_proposals).toBe(3);
    expect(stats.by_status.find((item) => item.status === 'promoted')?.count).toBe(1);
    expect(stats.by_status.find((item) => item.status === 'dismissed')?.count).toBe(1);
    expect(stats.by_status.find((item) => item.status === 'expired')?.count).toBe(1);
    expect(stats.by_type.find((item) => item.type === 'decision')?.count).toBe(1);
    expect(stats.promotion_rate).toBeCloseTo(1 / 3, 5);
    expect(stats.most_recent).toBeDefined();
  });

  it('prunes old resolved proposals and keeps active records', async () => {
    const resolved = await store.createProposal(createInput('lesson'));
    const active = await store.createProposal(createInput('pattern'));

    await store.resolveProposal(resolved.id, {
      status: 'dismissed',
      resolved_by: 'agent/reviewer',
      resolution_reason: 'obsolete',
    });

    vi.setSystemTime(new Date('2026-02-15T00:00:00.000Z'));
    const pruned = await store.pruneProposals('2026-01-20T00:00:00.000Z');

    expect(pruned).toBe(1);
    expect(await store.getProposal(resolved.id)).toBeNull();
    expect(await store.getProposal(active.id)).not.toBeNull();
  });

  it('handles non-existent IDs and duplicate primary keys', async () => {
    const missingId = createProposalId(randomUUID());

    expect(await store.getProposal(missingId)).toBeNull();
    expect(await store.updateProposal(missingId, { summary: 'missing' })).toBeNull();
    expect(await store.resolveProposal(missingId, { status: 'dismissed' })).toBeNull();
    await expect(
      store.markPromoted(missingId, createMemoryId(randomUUID()), 'agent/promoter')
    ).rejects.toThrow(`Proposal not found: ${missingId}`);
    await expect(store.markDismissed(missingId, 'obsolete', 'agent/reviewer')).rejects.toThrow(
      `Proposal not found: ${missingId}`
    );
    expect(await store.proposalExists(missingId)).toBe(false);

    const created = await store.createProposal(createInput('decision'));
    expect(await store.getProposal(created.id)).not.toBeNull();

    const created2 = await store.createProposal(createInput('pattern'));
    expect(created2.id).not.toBe(created.id);
  });

  it('reports availability and can close cleanly', async () => {
    expect(await store.isAvailable()).toBe(true);
    store.close();
    expect(await store.isAvailable()).toBe(false);
  });

  describe('terminal-state immutability (CIB-118)', () => {
    it('refuses to re-resolve a proposal that is already terminal', async () => {
      const proposal = await store.createProposal(createInput('pattern'));
      const memoryId = createMemoryId(randomUUID());

      const resolved = await store.resolveProposal(proposal.id, {
        status: 'promoted',
        resolved_by: 'agent/promoter',
        memory_id: memoryId,
      });
      expect(resolved?.status).toBe('promoted');

      await expect(
        store.resolveProposal(proposal.id, {
          status: 'dismissed',
          resolved_by: 'agent/reviewer',
          resolution_reason: 'changed my mind',
        })
      ).rejects.toThrow(ProposalAlreadyResolvedError);

      const current = await store.getProposal(proposal.id);
      expect(current?.status).toBe('promoted');
      expect(current?.resolution?.memory_id).toBe(memoryId);
    });

    it('returns null when resolving a proposal that does not exist', async () => {
      const missingId = createProposalId(randomUUID());
      const result = await store.resolveProposal(missingId, {
        status: 'dismissed',
        resolved_by: 'agent/reviewer',
      });
      expect(result).toBeNull();
    });

    it('treats a markPromoted replay with the same memory id as a no-op', async () => {
      const proposal = await store.createProposal(createInput('decision'));
      const memoryId = createMemoryId(randomUUID());

      vi.setSystemTime(new Date('2026-01-01T01:00:00.000Z'));
      await store.markPromoted(proposal.id, memoryId, 'agent/promoter');

      vi.setSystemTime(new Date('2026-01-01T02:00:00.000Z'));
      await expect(
        store.markPromoted(proposal.id, memoryId, 'agent/promoter')
      ).resolves.toBeUndefined();

      const current = await store.getProposal(proposal.id);
      expect(current?.status).toBe('promoted');
      expect(current?.resolution?.memory_id).toBe(memoryId);
      // The first resolution record wins; the replay must not rewrite it.
      expect(current?.resolution?.resolved_at).toBe('2026-01-01T01:00:00.000Z');
    });

    it('refuses markPromoted with a different memory id on an already promoted proposal', async () => {
      const proposal = await store.createProposal(createInput('decision'));
      const firstMemoryId = createMemoryId(randomUUID());
      const secondMemoryId = createMemoryId(randomUUID());

      await store.markPromoted(proposal.id, firstMemoryId, 'agent/promoter');

      await expect(
        store.markPromoted(proposal.id, secondMemoryId, 'agent/promoter')
      ).rejects.toThrow(ProposalAlreadyResolvedError);

      const current = await store.getProposal(proposal.id);
      expect(current?.resolution?.memory_id).toBe(firstMemoryId);
    });

    it('treats a markDismissed replay as a no-op that preserves the first resolution', async () => {
      const proposal = await store.createProposal(createInput('warning'));

      vi.setSystemTime(new Date('2026-01-01T01:00:00.000Z'));
      await store.markDismissed(proposal.id, 'first reason', 'agent/reviewer');

      vi.setSystemTime(new Date('2026-01-01T02:00:00.000Z'));
      await expect(
        store.markDismissed(proposal.id, 'second reason', 'agent/other')
      ).resolves.toBeUndefined();

      const current = await store.getProposal(proposal.id);
      expect(current?.status).toBe('dismissed');
      expect(current?.resolution?.resolution_reason).toBe('first reason');
      expect(current?.resolution?.resolved_by).toBe('agent/reviewer');
      expect(current?.resolution?.resolved_at).toBe('2026-01-01T01:00:00.000Z');
    });

    it('refuses markDismissed on a promoted proposal', async () => {
      const proposal = await store.createProposal(createInput('lesson'));
      const memoryId = createMemoryId(randomUUID());

      await store.markPromoted(proposal.id, memoryId, 'agent/promoter');

      await expect(
        store.markDismissed(proposal.id, 'no longer wanted', 'agent/reviewer')
      ).rejects.toThrow(ProposalAlreadyResolvedError);

      const current = await store.getProposal(proposal.id);
      expect(current?.status).toBe('promoted');
      expect(current?.resolution?.memory_id).toBe(memoryId);
    });

    it('dismissing an expired proposal is an idempotent no-op that keeps the expiry record', async () => {
      const proposal = await store.createProposal(createInput('pattern'));
      vi.setSystemTime(new Date('2026-02-05T00:00:00.000Z'));
      expect(await store.processExpiredProposals()).toBe(1);

      await expect(
        store.markDismissed(proposal.id, 'sweeping old proposals', 'agent/reviewer')
      ).resolves.toBeUndefined();

      const current = await store.getProposal(proposal.id);
      expect(current?.status).toBe('expired');
      expect(current?.resolution?.resolution_reason).toBe('TTL expired');
    });

    it('resolveProposal(dismissed) on an expired proposal succeeds without overwriting the expiry record', async () => {
      const proposal = await store.createProposal(createInput('warning'));
      vi.setSystemTime(new Date('2026-02-05T00:00:00.000Z'));
      expect(await store.processExpiredProposals()).toBe(1);

      const result = await store.resolveProposal(proposal.id, {
        status: 'dismissed',
        resolved_by: 'agent/reviewer',
        resolution_reason: 'sweeping old proposals',
      });

      expect(result?.status).toBe('expired');
      expect(result?.resolution?.resolution_reason).toBe('TTL expired');
    });

    it('refuses markPromoted on a dismissed proposal', async () => {
      const proposal = await store.createProposal(createInput('anomaly'));
      await store.markDismissed(proposal.id, 'not useful', 'agent/reviewer');

      await expect(
        store.markPromoted(proposal.id, createMemoryId(randomUUID()), 'agent/promoter')
      ).rejects.toThrow(ProposalAlreadyResolvedError);

      const current = await store.getProposal(proposal.id);
      expect(current?.status).toBe('dismissed');
    });
  });
});
