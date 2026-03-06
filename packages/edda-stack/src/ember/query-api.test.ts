import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createProposalId } from '../contracts/identifiers.js';
import type { CandidateProposal } from '../contracts/ember-proposal.js';
import {
  createExpiredProposal,
  createProposalFixture,
  createProposalsOfAllTypes,
} from '../testing/fixtures/proposals.js';
import { createMockEmberPort } from '../testing/mocks/ember.mock.js';
import { EmberQueryApi } from './query-api.js';

function buildTimeOffsets(baseTime: Date): CandidateProposal[] {
  const oneHour = 60 * 60 * 1000;

  return [
    createProposalFixture('decision', {
      summary: 'Critical architecture decision',
      confidence: 0.9,
      status: 'active',
      created_at: new Date(baseTime.getTime() - oneHour).toISOString(),
      expires_at: new Date(baseTime.getTime() + 2 * oneHour).toISOString(),
    }),
    createProposalFixture('decision', {
      summary: 'Another design decision recorded',
      confidence: 0.7,
      status: 'active',
      created_at: new Date(baseTime.getTime() - 2 * oneHour).toISOString(),
      expires_at: new Date(baseTime.getTime() + 30 * oneHour).toISOString(),
    }),
    createProposalFixture('pattern', {
      summary: 'Factory pattern recognised across modules',
      confidence: 0.8,
      status: 'promoted',
      created_at: new Date(baseTime.getTime() - 3 * oneHour).toISOString(),
      expires_at: new Date(baseTime.getTime() + 50 * oneHour).toISOString(),
    }),
    createProposalFixture('warning', {
      summary: 'Latency warning was not actioned',
      confidence: 0.4,
      status: 'expired',
      created_at: new Date(baseTime.getTime() - 96 * oneHour).toISOString(),
      expires_at: new Date(baseTime.getTime() - 2 * oneHour).toISOString(),
      resolution: {
        resolved_at: new Date(baseTime.getTime() - 2 * oneHour).toISOString(),
        resolution_reason: 'TTL expired',
      },
    }),
    createProposalFixture('lesson', {
      summary: 'Refactoring lesson from rollout',
      confidence: 0.5,
      status: 'dismissed',
      created_at: new Date(baseTime.getTime() - 5 * oneHour).toISOString(),
      expires_at: new Date(baseTime.getTime() + 10 * oneHour).toISOString(),
      resolution: {
        resolved_at: new Date(baseTime.getTime() - oneHour).toISOString(),
        resolution_reason: 'Not relevant',
      },
    }),
    createProposalFixture('anomaly', {
      summary: 'Anomalous memory spike detected',
      confidence: 0.6,
      status: 'active',
      created_at: new Date(baseTime.getTime() - 30 * 60 * 1000).toISOString(),
      expires_at: new Date(baseTime.getTime() + 20 * oneHour).toISOString(),
    }),
    createProposalFixture('decision', {
      summary: 'Highest decision confidence in this set',
      confidence: 0.95,
      status: 'active',
      created_at: new Date(baseTime.getTime() - 10 * 60 * 1000).toISOString(),
      expires_at: new Date(baseTime.getTime() + oneHour).toISOString(),
    }),
  ];
}

describe('EmberQueryApi', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-01-10T12:00:00.000Z'));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('filters by type with pagination options', async () => {
    const proposals = buildTimeOffsets(new Date('2026-01-10T12:00:00.000Z'));
    const store = createMockEmberPort({ initialProposals: proposals });
    const api = new EmberQueryApi(store);

    const firstPage = await api.listByType('decision', { limit: 2, offset: 0 });
    const secondPage = await api.listByType('decision', { limit: 2, offset: 1 });

    expect(firstPage).toHaveLength(2);
    expect(firstPage[0]?.summary).toContain('Highest decision confidence');
    expect(firstPage[1]?.summary).toContain('Critical architecture decision');

    expect(secondPage).toHaveLength(2);
    expect(secondPage[0]?.summary).toContain('Critical architecture decision');
    expect(secondPage[1]?.summary).toContain('Another design decision');
  });

  it('filters by confidence range', async () => {
    const proposals = buildTimeOffsets(new Date('2026-01-10T12:00:00.000Z'));
    const store = createMockEmberPort({ initialProposals: proposals });
    const api = new EmberQueryApi(store);

    const minOnly = await api.listByConfidence(0.9);
    const bounded = await api.listByConfidence(0.7, 0.9);

    expect(minOnly.map((proposal) => proposal.confidence).sort()).toEqual([0.9, 0.95]);
    expect(bounded.map((proposal) => proposal.confidence).sort()).toEqual([0.7, 0.8, 0.9]);
  });

  it('lists active proposals expiring soon with default and custom window', async () => {
    const proposals = buildTimeOffsets(new Date('2026-01-10T12:00:00.000Z'));
    const store = createMockEmberPort({ initialProposals: proposals });
    const api = new EmberQueryApi(store);

    const soonDefault = await api.listExpiringSoon();
    const soonTwoHours = await api.listExpiringSoon(2);

    expect(soonDefault).toHaveLength(3);
    expect(soonDefault.map((proposal) => proposal.type)).toEqual([
      'decision',
      'decision',
      'anomaly',
    ]);
    expect(soonTwoHours).toHaveLength(2);
    expect(soonTwoHours.map((proposal) => proposal.summary)).toEqual([
      'Highest decision confidence in this set',
      'Critical architecture decision',
    ]);
  });

  it('lists most recent proposals with default limit', async () => {
    const proposals = buildTimeOffsets(new Date('2026-01-10T12:00:00.000Z'));
    const extra = createProposalsOfAllTypes().map((proposal, index) =>
      createProposalFixture(proposal.type, {
        created_at: new Date(Date.now() - (index + 6) * 24 * 60 * 60 * 1000).toISOString(),
      })
    );
    const store = createMockEmberPort({ initialProposals: [...proposals, ...extra] });
    const api = new EmberQueryApi(store);

    const recent = await api.listRecent();
    const topThree = await api.listRecent(3);

    expect(recent).toHaveLength(10);
    expect(recent[0]?.summary).toContain('Highest decision confidence');
    expect(topThree.map((proposal) => proposal.summary)).toEqual([
      'Highest decision confidence in this set',
      'Anomalous memory spike detected',
      'Critical architecture decision',
    ]);
  });

  it('searches summaries with case-insensitive substring matching', async () => {
    const proposals = buildTimeOffsets(new Date('2026-01-10T12:00:00.000Z'));
    const store = createMockEmberPort({ initialProposals: proposals });
    const api = new EmberQueryApi(store);

    const decisionMatches = await api.searchBySummary('  DeCiSiOn  ');
    const noMatches = await api.searchBySummary('not-present');
    const blank = await api.searchBySummary('   ');

    expect(decisionMatches).toHaveLength(3);
    expect(noMatches).toEqual([]);
    expect(blank).toEqual([]);
  });

  it('returns proposal context and handles missing proposal ids', async () => {
    const proposals = buildTimeOffsets(new Date('2026-01-10T12:00:00.000Z'));
    const target = proposals.find(
      (proposal) => proposal.summary === 'Critical architecture decision'
    );
    const highest = proposals.find((proposal) =>
      proposal.summary.includes('Highest decision confidence')
    );
    const store = createMockEmberPort({ initialProposals: proposals });
    const api = new EmberQueryApi(store);

    const context = await api.getProposalWithContext(target!.id);
    const highestContext = await api.getProposalWithContext(highest!.id);
    const missing = await api.getProposalWithContext(
      createProposalId('00000000-0000-4000-8000-000000000000')
    );

    expect(context).not.toBeNull();
    expect(context?.relatedCount).toBe(2);
    expect(context?.averageTypeConfidence).toBeCloseTo(0.85, 5);
    expect(context?.isHighestConfidence).toBe(false);

    expect(highestContext?.isHighestConfidence).toBe(true);
    expect(missing).toBeNull();
  });

  it('returns summary stats with nearest expiry and 24-hour count', async () => {
    const proposals = buildTimeOffsets(new Date('2026-01-10T12:00:00.000Z'));
    const store = createMockEmberPort({ initialProposals: proposals });
    const api = new EmberQueryApi(store);

    const stats = await api.getSummaryStats();

    expect(stats.totalActive).toBe(4);
    expect(stats.totalExpired).toBe(1);
    expect(stats.totalPromoted).toBe(1);
    expect(stats.byType).toEqual({
      decision: 3,
      pattern: 1,
      warning: 1,
      lesson: 1,
      anomaly: 1,
      constraint: 0,
    });
    expect(stats.averageConfidence).toBeCloseTo(4.85 / 7, 5);
    expect(stats.nearestExpiry).toBe('2026-01-10T13:00:00.000Z');
    expect(stats.expiringWithin24h).toBe(3);
  });

  it('handles empty stores and all-expired edge cases', async () => {
    const emptyStore = createMockEmberPort();
    const emptyApi = new EmberQueryApi(emptyStore);

    expect(await emptyApi.listRecent()).toEqual([]);
    expect(await emptyApi.listByType('constraint')).toEqual([]);
    expect(await emptyApi.listExpiringSoon()).toEqual([]);
    expect(await emptyApi.listByConfidence(0.2)).toEqual([]);

    const emptyStats = await emptyApi.getSummaryStats();
    expect(emptyStats.totalActive).toBe(0);
    expect(emptyStats.totalExpired).toBe(0);
    expect(emptyStats.totalPromoted).toBe(0);
    expect(emptyStats.averageConfidence).toBe(0);
    expect(emptyStats.nearestExpiry).toBeNull();
    expect(emptyStats.expiringWithin24h).toBe(0);

    const expiredOnlyStore = createMockEmberPort({
      initialProposals: [
        createExpiredProposal('warning'),
        createExpiredProposal('decision', { status: 'expired' }),
      ],
    });
    const expiredOnlyApi = new EmberQueryApi(expiredOnlyStore);
    const expiredStats = await expiredOnlyApi.getSummaryStats();

    expect(await expiredOnlyApi.listExpiringSoon()).toEqual([]);
    expect(expiredStats.totalActive).toBe(0);
    expect(expiredStats.totalExpired).toBe(2);
    expect(expiredStats.nearestExpiry).toBeNull();
    expect(expiredStats.expiringWithin24h).toBe(0);
  });
});
