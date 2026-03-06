import type {
  CandidateProposal,
  CreateProposalInput,
  ProposalQuery,
  ProposalQueryResult,
  ProposalType,
} from '../contracts/ember-proposal.js';
import type { EmberStats, IEmberPort, UpdateProposalInput } from '../contracts/ports/ember.port.js';
import type { IKindlingPort, Observation } from '../contracts/ports/kindling.port.js';
import { createProposalCreatedEvent, type IStackEventBus } from '../contracts/events.js';
import type { MemoryId, ProposalId, SessionId } from '../contracts/identifiers.js';
import { clampConfidence } from '../contracts/confidence.js';
import type { Timestamp } from '../contracts/temporal.js';
import { DEFAULT_PRUNE_DAYS } from './decay-service.js';

export interface EmberServiceConfig {
  evaluation: {
    min_confidence: number;
    repetition_threshold: number;
    escalation_window_hours: number;
  };
  decay: {
    default_ttl_days: number;
    min_ttl_days: number;
    max_ttl_days: number;
  };
  limits: {
    max_candidates: number;
  };
}

export const DEFAULT_EMBER_SERVICE_CONFIG: EmberServiceConfig = {
  evaluation: {
    min_confidence: 0.3,
    repetition_threshold: 3,
    escalation_window_hours: 24,
  },
  decay: {
    default_ttl_days: 30,
    min_ttl_days: 7,
    max_ttl_days: 90,
  },
  limits: {
    max_candidates: 1000,
  },
};

export interface SimpleObservationGroup {
  key: Observation['kind'];
  observations: Observation[];
}

export interface EvaluatedCandidate {
  should_propose: boolean;
  confidence: number;
  type: ProposalType;
  summary: string;
  rationale: string;
  metadata?: Record<string, unknown>;
  ttl_days?: number;
}

export interface CandidateAggregator {
  aggregateSession(
    sessionId: SessionId,
    observations: Observation[]
  ): Promise<SimpleObservationGroup[]>;
}

export interface CandidateEvaluator {
  evaluateGroup(group: SimpleObservationGroup): Promise<EvaluatedCandidate | null>;
}

export interface CandidateServiceDeps {
  store: IEmberPort;
  kindlingPort?: IKindlingPort;
  eventBus?: IStackEventBus;
  config?: EmberServiceConfig;
  aggregator?: CandidateAggregator;
  evaluator?: CandidateEvaluator;
}

export class CandidateService {
  private readonly config: EmberServiceConfig;

  constructor(private readonly deps: CandidateServiceDeps) {
    this.config = deps.config ?? DEFAULT_EMBER_SERVICE_CONFIG;
  }

  async createProposal(input: CreateProposalInput): Promise<CandidateProposal> {
    const count = await this.deps.store.countProposals();
    if (count >= this.config.limits.max_candidates) {
      throw new Error('Maximum candidate limit reached');
    }

    const proposal = await this.deps.store.createProposal({
      ...input,
      confidence: clampConfidence(input.confidence),
      ttl_days: this.clampTtlDays(input.ttl_days),
    });

    if (this.deps.eventBus) {
      await this.deps.eventBus.publish(
        createProposalCreatedEvent({
          proposal_id: proposal.id,
          proposal_type: proposal.type,
          confidence: proposal.confidence,
          summary: proposal.summary,
          expires_at: proposal.expires_at,
          source_observation_ids: proposal.provenance.observation_ids,
        })
      );
    }

    return proposal;
  }

  async getProposal(id: string): Promise<CandidateProposal | null> {
    return this.deps.store.getProposal(id as ProposalId);
  }

  async queryProposals(query: ProposalQuery): Promise<ProposalQueryResult> {
    return this.deps.store.queryProposals(query);
  }

  async getActiveProposals(): Promise<CandidateProposal[]> {
    return this.deps.store.getActiveProposals();
  }

  async updateProposal(id: string, input: UpdateProposalInput): Promise<CandidateProposal | null> {
    return this.deps.store.updateProposal(id as ProposalId, input);
  }

  async promoteProposal(id: string, memoryId: string, resolvedBy: string): Promise<void> {
    await this.deps.store.markPromoted(id as ProposalId, memoryId as MemoryId, resolvedBy);
  }

  async dismissProposal(id: string, reason: string, resolvedBy: string): Promise<void> {
    await this.deps.store.markDismissed(id as ProposalId, reason, resolvedBy);
  }

  async runDecayCycle(): Promise<{ expired: number; pruned: number }> {
    const expired = await this.deps.store.processExpiredProposals();
    const pruneThreshold = new Date();
    pruneThreshold.setDate(pruneThreshold.getDate() - DEFAULT_PRUNE_DAYS);
    const pruned = await this.deps.store.pruneProposals(pruneThreshold.toISOString() as Timestamp);
    return { expired, pruned };
  }

  async getStats(): Promise<EmberStats> {
    return this.deps.store.getStats();
  }

  async isAvailable(): Promise<boolean> {
    return this.deps.store.isAvailable();
  }

  /**
   * Process a session's observations into candidate proposals.
   *
   * NOTE: This method is not concurrency-safe. The read-evaluate-write cycle
   * can race if two calls process the same session simultaneously. With SQLite
   * WAL mode this is low-risk (serialised writes), but a future backend swap
   * would require an advisory lock or compare-and-swap pattern.
   */
  async processSession(sessionId: string): Promise<CandidateProposal[]> {
    if (!this.deps.kindlingPort) {
      return [];
    }

    const typedSessionId = sessionId as SessionId;
    const observations = await this.deps.kindlingPort.getObservationsBySession(typedSessionId);
    if (observations.length === 0) {
      return [];
    }

    const groups = this.deps.aggregator
      ? await this.deps.aggregator.aggregateSession(typedSessionId, observations)
      : this.aggregateByKind(observations);

    const created: CandidateProposal[] = [];
    for (const group of groups) {
      if (group.observations.length === 0) {
        continue;
      }

      const evaluated = this.deps.evaluator
        ? await this.deps.evaluator.evaluateGroup(group)
        : this.evaluateGroup(group);

      if (!evaluated || !evaluated.should_propose) {
        continue;
      }

      const confidence = clampConfidence(evaluated.confidence);
      if (confidence < this.config.evaluation.min_confidence) {
        continue;
      }

      const proposal = await this.createProposal({
        type: evaluated.type,
        summary: evaluated.summary,
        rationale: evaluated.rationale,
        confidence,
        metadata: evaluated.metadata,
        ttl_days: this.clampTtlDays(evaluated.ttl_days),
        provenance: this.buildProvenance(group.observations, typedSessionId),
      });
      created.push(proposal);
    }

    return created;
  }

  private clampTtlDays(ttlDays?: number): number {
    const value = ttlDays ?? this.config.decay.default_ttl_days;
    return Math.max(
      this.config.decay.min_ttl_days,
      Math.min(this.config.decay.max_ttl_days, value)
    );
  }

  private aggregateByKind(observations: Observation[]): SimpleObservationGroup[] {
    const grouped = new Map<Observation['kind'], Observation[]>();
    for (const observation of observations) {
      const existing = grouped.get(observation.kind) ?? [];
      existing.push(observation);
      grouped.set(observation.kind, existing);
    }

    return Array.from(grouped.entries()).map(([key, groupedObservations]) => ({
      key,
      observations: groupedObservations,
    }));
  }

  private evaluateGroup(group: SimpleObservationGroup): EvaluatedCandidate {
    const confidence = clampConfidence(
      group.observations.length / Math.max(this.config.evaluation.repetition_threshold, 1)
    );

    return {
      should_propose: true,
      confidence,
      type: this.mapObservationKindToProposalType(group.key),
      summary: `Session produced repeated ${group.key} observations`,
      rationale: `${group.observations.length} ${group.key} observations were recorded in one session`,
      metadata: {
        observation_kind: group.key,
        occurrence_count: group.observations.length,
      },
      ttl_days: this.config.decay.default_ttl_days,
    };
  }

  private mapObservationKindToProposalType(kind: Observation['kind']): ProposalType {
    if (kind === 'error_recorded' || kind === 'action_failed') {
      return 'warning';
    }
    if (kind === 'constraint_applied') {
      return 'constraint';
    }
    if (kind === 'plan_completed') {
      return 'lesson';
    }
    if (kind === 'plan_started') {
      return 'decision';
    }
    return 'pattern';
  }

  private buildProvenance(
    observations: Observation[],
    sessionId: SessionId
  ): {
    observation_ids: string[];
    session_ids: string[];
    earliest_observation: Timestamp;
    latest_observation: Timestamp;
  } {
    const timestamps = observations.map((observation) => new Date(observation.timestamp).getTime());
    return {
      observation_ids: observations.map((observation) => observation.id),
      session_ids: [sessionId],
      earliest_observation: new Date(Math.min(...timestamps)).toISOString() as Timestamp,
      latest_observation: new Date(Math.max(...timestamps)).toISOString() as Timestamp,
    };
  }
}
