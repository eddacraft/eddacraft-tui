import { randomUUID } from 'node:crypto';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createSessionId } from '../contracts/identifiers.js';
import type { CreateProposalInput, ProposalType } from '../contracts/ember-proposal.js';
import { DecayService, DEFAULT_PRUNE_DAYS } from './decay-service.js';
import { ProposalStore } from './proposal-store.js';

const DAY_MS = 24 * 60 * 60 * 1000;
const BASE_TIME = new Date('2026-01-01T00:00:00.000Z');

function createInput(type: ProposalType = 'pattern', ttlDays = 30): CreateProposalInput {
  return {
    type,
    summary: `${type} summary`,
    rationale: `${type} rationale`,
    confidence: 0.7,
    ttl_days: ttlDays,
    provenance: {
      observation_ids: [randomUUID()],
      session_ids: [createSessionId(randomUUID())],
      earliest_observation: new Date('2026-01-01T00:00:00.000Z').toISOString(),
      latest_observation: new Date('2026-01-01T00:01:00.000Z').toISOString(),
    },
    signals: [],
  };
}

describe('DecayService', () => {
  let store: ProposalStore;
  let decayService: DecayService;

  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-01-01T00:00:00.000Z'));
    store = ProposalStore.createInMemory();
    decayService = new DecayService(store, { checkIntervalMs: 5000 });
  });

  afterEach(() => {
    store.close();
    vi.useRealTimers();
  });

  it('processExpired marks active expired proposals', async () => {
    await store.createProposal(createInput('warning', 1));
    vi.setSystemTime(new Date('2026-01-03T00:00:00.000Z'));

    const processed = await decayService.processExpired();

    expect(processed).toBe(1);
    expect(await store.countProposals('expired')).toBe(1);
  });

  it('pruneOld removes old resolved proposals', async () => {
    const resolved = await store.createProposal(createInput('lesson', 30));

    await store.resolveProposal(resolved.id, {
      status: 'dismissed',
      resolved_by: 'agent/reviewer',
      resolution_reason: 'obsolete',
    });

    vi.setSystemTime(new Date('2026-02-15T00:00:00.000Z'));
    const pruned = await decayService.pruneOld(30);

    expect(pruned).toBe(1);
    expect(await store.getProposal(resolved.id)).toBeNull();
  });

  it('getExpiringSoon returns only proposals expiring in the window', async () => {
    await store.createProposal(createInput('pattern', 1));
    await store.createProposal(createInput('decision', 3));

    const expiringsSoon = await decayService.getExpiringSoon(24);

    expect(expiringsSoon).toHaveLength(1);
    expect(expiringsSoon[0]?.type).toBe('pattern');
  });

  it('run executes full decay cycle', async () => {
    await store.createProposal(createInput('warning', 1));
    const oldResolved = await store.createProposal(createInput('lesson', 30));

    await store.resolveProposal(oldResolved.id, {
      status: 'dismissed',
      resolved_by: 'agent/reviewer',
      resolution_reason: 'stale',
    });

    vi.setSystemTime(new Date(BASE_TIME.getTime() + (DEFAULT_PRUNE_DAYS + 1) * DAY_MS));
    const result = await decayService.run();

    expect(result.expired).toBe(1);
    expect(result.pruned).toBe(1);
    expect(await store.countProposals('expired')).toBe(1);
  });

  it('run honours custom pruneDays config', async () => {
    const customDecay = new DecayService(store, { pruneDays: 10 });

    const resolved = await store.createProposal(createInput('lesson', 30));
    await store.resolveProposal(resolved.id, {
      status: 'dismissed',
      resolved_by: 'agent/reviewer',
      resolution_reason: 'stale',
    });

    // At 11 days: custom pruneDays=10 should prune, but default (90) would not
    vi.setSystemTime(new Date(BASE_TIME.getTime() + 11 * DAY_MS));
    const result = await customDecay.run();

    expect(result.pruned).toBe(1);
    expect(await store.getProposal(resolved.id)).toBeNull();
  });

  it('getDecayStats returns current active, expiring soon, and expired counts', async () => {
    await store.createProposal(createInput('pattern', 1));
    await store.createProposal(createInput('decision', 3));
    await store.createProposal(createInput('warning', 1));

    vi.setSystemTime(new Date('2026-01-03T00:00:00.000Z'));
    await decayService.processExpired();

    const stats = await decayService.getDecayStats();

    expect(stats.totalActive).toBe(1);
    expect(stats.expiringSoon).toBe(1);
    expect(stats.recentlyExpired).toBe(2);
  });
});
