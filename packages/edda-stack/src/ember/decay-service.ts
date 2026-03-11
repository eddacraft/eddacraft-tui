import type { CandidateProposal } from '../contracts/ember-proposal.js';
import type { IEmberPort } from '../contracts/ports/ember.port.js';
import { now } from '../contracts/temporal.js';

const HOUR_MS = 60 * 60 * 1000;
const DAY_MS = 24 * 60 * 60 * 1000;
const DEFAULT_CHECK_INTERVAL_MS = 60 * 60 * 1000;
/** Default age threshold (in days) for pruning resolved proposals */
export const DEFAULT_PRUNE_DAYS = 90;

export interface DecayServiceConfig {
  checkIntervalMs?: number;
}

export class DecayService {
  readonly checkIntervalMs: number;

  constructor(
    private readonly store: IEmberPort,
    config: DecayServiceConfig = {}
  ) {
    this.checkIntervalMs = config.checkIntervalMs ?? DEFAULT_CHECK_INTERVAL_MS;
  }

  async processExpired(): Promise<number> {
    return this.store.processExpiredProposals();
  }

  async pruneOld(olderThanDays: number): Promise<number> {
    const cutoff = new Date(Date.now() - olderThanDays * DAY_MS).toISOString();
    return this.store.pruneProposals(cutoff);
  }

  async getExpiringSoon(withinHours: number): Promise<CandidateProposal[]> {
    const active = await this.store.getActiveProposals();
    const currentTime = new Date(now()).getTime();
    const threshold = currentTime + withinHours * HOUR_MS;

    return active.filter((proposal) => {
      const expiresAt = new Date(proposal.expires_at).getTime();
      return expiresAt > currentTime && expiresAt <= threshold;
    });
  }

  async run(): Promise<{ expired: number; pruned: number }> {
    const expired = await this.processExpired();
    const pruned = await this.pruneOld(DEFAULT_PRUNE_DAYS);
    return { expired, pruned };
  }

  async getDecayStats(): Promise<{
    totalActive: number;
    expiringSoon: number;
    recentlyExpired: number;
  }> {
    const [active, expiringSoon, recentlyExpired] = await Promise.all([
      this.store.getActiveProposals(),
      this.getExpiringSoon(24),
      this.store.countProposals('expired'),
    ]);

    return {
      totalActive: active.length,
      expiringSoon: expiringSoon.length,
      recentlyExpired,
    };
  }
}
