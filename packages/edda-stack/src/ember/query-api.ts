import type { ProposalId } from '../contracts/identifiers.js';
import type {
  CandidateProposal,
  ProposalQuery,
  ProposalType,
} from '../contracts/ember-proposal.js';
import type { IEmberPort } from '../contracts/ports/ember.port.js';

const DEFAULT_PAGE_SIZE = 100;
const DEFAULT_RECENT_LIMIT = 10;
const DEFAULT_EXPIRY_WINDOW_HOURS = 24;
const HOUR_MS = 60 * 60 * 1000;

const PROPOSAL_TYPES: ProposalType[] = [
  'decision',
  'pattern',
  'warning',
  'lesson',
  'anomaly',
  'constraint',
];

export interface ProposalWithContext {
  proposal: CandidateProposal;
  relatedCount: number;
  averageTypeConfidence: number;
  isHighestConfidence: boolean;
}

export interface EmberSummaryStats {
  totalActive: number;
  totalExpired: number;
  totalPromoted: number;
  byType: Record<ProposalType, number>;
  averageConfidence: number;
  nearestExpiry: string | null;
  expiringWithin24h: number;
}

export class EmberQueryApi {
  constructor(private readonly store: IEmberPort) {}

  async listByType(
    type: ProposalType,
    options?: { limit?: number; offset?: number }
  ): Promise<CandidateProposal[]> {
    const query: ProposalQuery = {
      types: [type],
      include_expired: true,
      limit: options?.limit ?? DEFAULT_PAGE_SIZE,
      offset: options?.offset ?? 0,
      sort_by: 'created_at',
      sort_order: 'desc',
    };

    const result = await this.store.queryProposals(query);
    return result.proposals;
  }

  async listByConfidence(
    minConfidence: number,
    maxConfidence?: number
  ): Promise<CandidateProposal[]> {
    const proposals = await this.getAllProposals();
    return proposals.filter((proposal) => {
      if (proposal.confidence < minConfidence) {
        return false;
      }

      if (maxConfidence !== undefined && proposal.confidence > maxConfidence) {
        return false;
      }

      return true;
    });
  }

  async listExpiringSoon(withinHours = DEFAULT_EXPIRY_WINDOW_HOURS): Promise<CandidateProposal[]> {
    const nowMs = Date.now();
    const thresholdMs = nowMs + withinHours * HOUR_MS;
    const active = await this.store.getActiveProposals();

    return active
      .filter((proposal) => {
        const expiresAtMs = new Date(proposal.expires_at).getTime();
        return expiresAtMs >= nowMs && expiresAtMs <= thresholdMs;
      })
      .sort(
        (left, right) => new Date(left.expires_at).getTime() - new Date(right.expires_at).getTime()
      );
  }

  async listRecent(limit = DEFAULT_RECENT_LIMIT): Promise<CandidateProposal[]> {
    const result = await this.store.queryProposals({
      include_expired: true,
      sort_by: 'created_at',
      sort_order: 'desc',
      limit,
      offset: 0,
    });

    return result.proposals;
  }

  async searchBySummary(searchTerm: string): Promise<CandidateProposal[]> {
    const normalisedSearch = searchTerm.trim().toLowerCase();
    if (normalisedSearch.length === 0) {
      return [];
    }

    const proposals = await this.getAllProposals();
    return proposals.filter((proposal) =>
      proposal.summary.toLowerCase().includes(normalisedSearch)
    );
  }

  async getProposalWithContext(id: ProposalId): Promise<ProposalWithContext | null> {
    const proposal = await this.store.getProposal(id);
    if (!proposal) {
      return null;
    }

    const proposals = await this.getAllProposals();
    const sameType = proposals.filter((candidate) => candidate.type === proposal.type);
    const relatedCount = Math.max(sameType.length - 1, 0);

    const totalConfidence = sameType.reduce((sum, candidate) => sum + candidate.confidence, 0);
    const averageTypeConfidence = sameType.length > 0 ? totalConfidence / sameType.length : 0;
    const highestConfidence = sameType.reduce(
      (max, candidate) => (candidate.confidence > max ? candidate.confidence : max),
      Number.NEGATIVE_INFINITY
    );

    return {
      proposal,
      relatedCount,
      averageTypeConfidence,
      isHighestConfidence: proposal.confidence >= highestConfidence,
    };
  }

  async getSummaryStats(): Promise<EmberSummaryStats> {
    const proposals = await this.getAllProposals();
    const nowMs = Date.now();
    const within24hMs = nowMs + DEFAULT_EXPIRY_WINDOW_HOURS * HOUR_MS;

    const byType = PROPOSAL_TYPES.reduce<Record<ProposalType, number>>(
      (record, type) => {
        record[type] = 0;
        return record;
      },
      {} as Record<ProposalType, number>
    );

    let totalActive = 0;
    let totalExpired = 0;
    let totalPromoted = 0;
    let confidenceTotal = 0;
    let nearestExpiryMs: number | null = null;
    let expiringWithin24h = 0;

    for (const proposal of proposals) {
      byType[proposal.type] += 1;
      confidenceTotal += proposal.confidence;

      if (proposal.status === 'active') {
        totalActive += 1;
      } else if (proposal.status === 'expired') {
        totalExpired += 1;
      } else if (proposal.status === 'promoted') {
        totalPromoted += 1;
      }

      if (proposal.status !== 'active') {
        continue;
      }

      const expiresAtMs = new Date(proposal.expires_at).getTime();
      if (expiresAtMs < nowMs) {
        continue;
      }

      if (nearestExpiryMs === null || expiresAtMs < nearestExpiryMs) {
        nearestExpiryMs = expiresAtMs;
      }

      if (expiresAtMs <= within24hMs) {
        expiringWithin24h += 1;
      }
    }

    return {
      totalActive,
      totalExpired,
      totalPromoted,
      byType,
      averageConfidence: proposals.length > 0 ? confidenceTotal / proposals.length : 0,
      nearestExpiry: nearestExpiryMs ? new Date(nearestExpiryMs).toISOString() : null,
      expiringWithin24h,
    };
  }

  private async getAllProposals(): Promise<CandidateProposal[]> {
    const proposals: CandidateProposal[] = [];
    let offset = 0;
    let hasMore = true;

    while (hasMore) {
      const result = await this.store.queryProposals({
        include_expired: true,
        limit: DEFAULT_PAGE_SIZE,
        offset,
        sort_by: 'created_at',
        sort_order: 'desc',
      });

      proposals.push(...result.proposals);
      offset += result.proposals.length;
      hasMore = result.has_more;

      if (result.proposals.length === 0) {
        break;
      }
    }

    return proposals;
  }
}
