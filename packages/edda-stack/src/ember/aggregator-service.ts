import { createHash } from 'node:crypto';
import type { ProposalType } from '../contracts/ember-proposal.js';
import { SessionIdSchema } from '../contracts/identifiers.js';
import type { IKindlingPort, Observation } from '../contracts/ports/kindling.port.js';

export interface ObservationGroup {
  id: string;
  grouping_type: 'session' | 'temporal' | 'pattern' | 'agent';
  observation_ids: string[];
  session_ids: string[];
  earliest: string;
  latest: string;
  count: number;
  suggested_type?: ProposalType;
  signals: string[];
}

const DEFAULT_TEMPORAL_WINDOW_MS = 5 * 60 * 1000;
const DEFAULT_REPETITION_THRESHOLD = 2;

export class AggregatorService {
  constructor(private readonly kindlingPort: IKindlingPort) {}

  async groupBySession(sessionId: string): Promise<ObservationGroup[]> {
    const parsedSessionId = SessionIdSchema.parse(sessionId);
    const observations = await this.kindlingPort.getSessionObservations(parsedSessionId);
    if (observations.length === 0) {
      return [];
    }

    return [this.buildGroup(observations, 'session', ['session_cluster'])];
  }

  async groupByTemporalProximity(
    observations: Observation[],
    windowMs: number
  ): Promise<ObservationGroup[]> {
    if (observations.length === 0) {
      return [];
    }

    const sorted = [...observations].sort(
      (a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime()
    );

    const clusters: Observation[][] = [];
    let currentCluster: Observation[] = [sorted[0]];

    for (let index = 1; index < sorted.length; index++) {
      const previousTimestamp = new Date(sorted[index - 1].timestamp).getTime();
      const currentTimestamp = new Date(sorted[index].timestamp).getTime();
      if (currentTimestamp - previousTimestamp <= windowMs) {
        currentCluster.push(sorted[index]);
      } else {
        clusters.push(currentCluster);
        currentCluster = [sorted[index]];
      }
    }
    clusters.push(currentCluster);

    return clusters.map((cluster) =>
      this.buildGroup(cluster, 'temporal', [`temporal_window_${windowMs}`])
    );
  }

  async groupByKind(observations: Observation[]): Promise<ObservationGroup[]> {
    if (observations.length === 0) {
      return [];
    }

    const byKind = new Map<string, Observation[]>();
    for (const observation of observations) {
      const existing = byKind.get(observation.kind);
      if (existing) {
        existing.push(observation);
      } else {
        byKind.set(observation.kind, [observation]);
      }
    }

    return Array.from(byKind.entries()).map(([kind, kindObservations]) =>
      this.buildGroup(kindObservations, 'pattern', [`kind_${kind}`])
    );
  }

  async detectRepetitions(
    observations: Observation[],
    threshold: number
  ): Promise<ObservationGroup[]> {
    if (observations.length === 0 || threshold < 2) {
      return [];
    }

    const buckets = new Map<string, Observation[]>();
    for (const observation of observations) {
      const fingerprint = this.repetitionFingerprint(observation);
      const existing = buckets.get(fingerprint);
      if (existing) {
        existing.push(observation);
      } else {
        buckets.set(fingerprint, [observation]);
      }
    }

    return Array.from(buckets.entries())
      .filter(([, repeatedObservations]) => repeatedObservations.length >= threshold)
      .map(([fingerprint, repeatedObservations]) =>
        this.buildGroup(
          repeatedObservations,
          'pattern',
          ['repetition_detected', fingerprint],
          'pattern'
        )
      );
  }

  async aggregate(sessionId: string): Promise<ObservationGroup[]> {
    const parsedSessionId = SessionIdSchema.parse(sessionId);
    const queried = await this.kindlingPort.queryObservations({ session_id: parsedSessionId });
    const observations = queried.observations;
    if (observations.length === 0) {
      return [];
    }

    const [kindGroups, temporalGroups, repetitionGroups] = await Promise.all([
      this.groupByKind(observations),
      this.groupByTemporalProximity(observations, DEFAULT_TEMPORAL_WINDOW_MS),
      this.detectRepetitions(observations, DEFAULT_REPETITION_THRESHOLD),
    ]);

    const merged = this.mergeOverlappingGroups([
      ...kindGroups,
      ...temporalGroups,
      ...repetitionGroups,
    ]);
    const deduplicated = this.deduplicateGroups(merged);

    return deduplicated.sort((left, right) => {
      if (right.count !== left.count) {
        return right.count - left.count;
      }
      return new Date(right.latest).getTime() - new Date(left.latest).getTime();
    });
  }

  private buildGroup(
    observations: Observation[],
    groupingType: ObservationGroup['grouping_type'],
    baseSignals: string[],
    suggestedType?: ProposalType
  ): ObservationGroup {
    const observationIds = [...new Set(observations.map((item) => item.id))];
    const sessionIds = [...new Set(observations.map((item) => item.session_id))];

    const timestamps = observations
      .map((item) => new Date(item.timestamp).getTime())
      .filter((value) => Number.isFinite(value));

    const earliest = new Date(Math.min(...timestamps)).toISOString();
    const latest = new Date(Math.max(...timestamps)).toISOString();

    const signals = [...new Set([...baseSignals, ...this.deriveSignals(observations)])];
    const resolvedType = suggestedType ?? this.inferSuggestedType(observations, signals);

    return {
      id: this.groupId(groupingType, observationIds),
      grouping_type: groupingType,
      observation_ids: observationIds,
      session_ids: sessionIds,
      earliest,
      latest,
      count: observationIds.length,
      suggested_type: resolvedType,
      signals,
    };
  }

  private mergeOverlappingGroups(groups: ObservationGroup[]): ObservationGroup[] {
    const merged: ObservationGroup[] = [];

    for (const group of groups) {
      const overlaps = merged.filter((candidate) => this.hasOverlap(candidate, group));
      if (overlaps.length === 0) {
        merged.push(group);
        continue;
      }

      const base = overlaps.shift() as ObservationGroup;
      const mergedGroup = overlaps.reduce(
        (accumulator, overlap) => {
          const index = merged.findIndex((entry) => entry.id === overlap.id);
          if (index >= 0) {
            merged.splice(index, 1);
          }
          return this.mergeTwo(accumulator, overlap);
        },
        this.mergeTwo(base, group)
      );

      const baseIndex = merged.findIndex((entry) => entry.id === base.id);
      if (baseIndex >= 0) {
        merged.splice(baseIndex, 1, mergedGroup);
      } else {
        merged.push(mergedGroup);
      }
    }

    return merged;
  }

  private deduplicateGroups(groups: ObservationGroup[]): ObservationGroup[] {
    const unique = new Map<string, ObservationGroup>();

    for (const group of groups) {
      const key = this.observationSetKey(group.observation_ids);
      const existing = unique.get(key);
      if (!existing || group.count > existing.count) {
        unique.set(key, group);
      }
    }

    return Array.from(unique.values());
  }

  private hasOverlap(left: ObservationGroup, right: ObservationGroup): boolean {
    const rightIds = new Set(right.observation_ids);
    return left.observation_ids.some((id) => rightIds.has(id));
  }

  private mergeTwo(left: ObservationGroup, right: ObservationGroup): ObservationGroup {
    const observationIds = [...new Set([...left.observation_ids, ...right.observation_ids])];
    const sessionIds = [...new Set([...left.session_ids, ...right.session_ids])];
    const signals = [...new Set([...left.signals, ...right.signals])];
    const earliest =
      new Date(left.earliest).getTime() <= new Date(right.earliest).getTime()
        ? left.earliest
        : right.earliest;
    const latest =
      new Date(left.latest).getTime() >= new Date(right.latest).getTime()
        ? left.latest
        : right.latest;

    const suggestion = right.suggested_type ?? left.suggested_type;
    const groupingType = this.strongerGroupingType(left.grouping_type, right.grouping_type);

    return {
      id: this.groupId(groupingType, observationIds),
      grouping_type: groupingType,
      observation_ids: observationIds,
      session_ids: sessionIds,
      earliest,
      latest,
      count: observationIds.length,
      suggested_type: suggestion,
      signals,
    };
  }

  private strongerGroupingType(
    left: ObservationGroup['grouping_type'],
    right: ObservationGroup['grouping_type']
  ): ObservationGroup['grouping_type'] {
    const ranking: Record<ObservationGroup['grouping_type'], number> = {
      pattern: 4,
      temporal: 3,
      session: 2,
      agent: 1,
    };
    return ranking[left] >= ranking[right] ? left : right;
  }

  private groupId(
    groupingType: ObservationGroup['grouping_type'],
    observationIds: string[]
  ): string {
    const seed = `${groupingType}:${this.observationSetKey(observationIds)}`;
    return createHash('sha256').update(seed).digest('hex').slice(0, 16);
  }

  private observationSetKey(observationIds: string[]): string {
    return [...observationIds].sort().join('|');
  }

  private repetitionFingerprint(observation: Observation): string {
    const normalisedSummary = observation.summary
      .toLowerCase()
      .replace(/\s+/g, ' ')
      .trim()
      .slice(0, 48);
    return `${observation.kind}:${normalisedSummary}`;
  }

  private deriveSignals(observations: Observation[]): string[] {
    const signals: string[] = [];

    if (
      observations.some((item) => item.kind === 'error_recorded' || item.kind === 'action_failed')
    ) {
      signals.push('failure_signal');
    }

    if (
      observations.some((item) => item.kind === 'plan_completed' || item.kind === 'action_executed')
    ) {
      signals.push('success_signal');
    }

    const kinds = new Set(observations.map((item) => item.kind));
    if (kinds.size > 1) {
      signals.push('mixed_kinds');
    }

    return signals;
  }

  private inferSuggestedType(observations: Observation[], signals: string[]): ProposalType {
    const kinds = new Set(observations.map((item) => item.kind));

    if (signals.includes('failure_signal') && signals.includes('success_signal')) {
      return 'lesson';
    }

    if (kinds.has('error_recorded') || kinds.has('action_failed')) {
      return 'warning';
    }

    if (kinds.has('constraint_applied')) {
      return 'constraint';
    }

    if (observations.length >= 3) {
      return 'pattern';
    }

    return 'decision';
  }
}
