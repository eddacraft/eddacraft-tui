import { clampConfidence, weightedConfidence } from '../contracts/confidence.js';
import type {
  CandidateProposal,
  EvaluationSignal,
  ProposalType,
} from '../contracts/ember-proposal.js';
import type { ObservationGroup } from './aggregator-service.js';

export interface EvaluationRule {
  name: string;
  description: string;
  weight: number;
  evaluate(group: ObservationGroup, context: EvaluationContext): EvaluationResult;
}

export interface EvaluationContext {
  existingProposals: CandidateProposal[];
  config: EmberEvaluationConfig;
}

export interface EmberEvaluationConfig {
  min_confidence: number;
  repetition_threshold: number;
  escalation_window_hours: number;
}

export interface EvaluationResult {
  fired: boolean;
  contribution: number;
  context?: Record<string, unknown>;
}

export interface EvaluationOutcome {
  group: ObservationGroup;
  confidence: number;
  signals: EvaluationSignal[];
  meetsThreshold: boolean;
  suggestedType: ProposalType;
}

const RULE_TYPE_MAP: Record<string, ProposalType> = {
  repetition: 'pattern',
  escalation: 'warning',
  resolution: 'lesson',
  convergence: 'decision',
  surprise: 'anomaly',
};

const FALLBACK_TYPE: ProposalType = 'pattern';

export class EvaluatorService {
  private rules: EvaluationRule[] = [];

  constructor(rules?: EvaluationRule[]) {
    if (rules) {
      this.rules = [...rules];
    }
  }

  registerRule(rule: EvaluationRule): void {
    const index = this.rules.findIndex((item) => item.name === rule.name);
    if (index >= 0) {
      this.rules[index] = rule;
      return;
    }
    this.rules.push(rule);
  }

  removeRule(name: string): void {
    this.rules = this.rules.filter((rule) => rule.name !== name);
  }

  getRules(): ReadonlyArray<EvaluationRule> {
    return this.rules;
  }

  evaluate(group: ObservationGroup, context: EvaluationContext): EvaluationOutcome {
    const scores: Array<{ score: number; weight: number }> = [];
    const signals: EvaluationSignal[] = [];

    for (const rule of this.rules) {
      const result = rule.evaluate(group, context);
      if (!result.fired) {
        continue;
      }

      const contribution = clampConfidence(result.contribution);
      scores.push({ score: contribution, weight: rule.weight });
      signals.push({
        rule: rule.name,
        contribution,
        weight: rule.weight,
        context: result.context,
      });
    }

    const confidence = clampConfidence(weightedConfidence(scores));
    const meetsThreshold = confidence >= context.config.min_confidence;

    return {
      group,
      confidence,
      signals,
      meetsThreshold,
      suggestedType: this.resolveSuggestedType(group, signals),
    };
  }

  evaluateAll(groups: ObservationGroup[], context: EvaluationContext): EvaluationOutcome[] {
    return groups.map((group) => this.evaluate(group, context));
  }

  private resolveSuggestedType(group: ObservationGroup, signals: EvaluationSignal[]): ProposalType {
    const byType = new Map<ProposalType, number>();

    if (group.suggested_type) {
      byType.set(group.suggested_type, 0.25);
    }

    for (const signal of signals) {
      const contextSuggested = signal.context?.suggested_type;
      const suggestedType = this.isProposalType(contextSuggested)
        ? contextSuggested
        : (RULE_TYPE_MAP[signal.rule] ?? group.suggested_type ?? FALLBACK_TYPE);

      const current = byType.get(suggestedType) ?? 0;
      byType.set(suggestedType, current + signal.contribution * signal.weight);
    }

    let highestType: ProposalType = group.suggested_type ?? FALLBACK_TYPE;
    let highestScore = byType.get(highestType) ?? Number.NEGATIVE_INFINITY;

    for (const [type, score] of byType.entries()) {
      if (score > highestScore) {
        highestType = type;
        highestScore = score;
      }
    }

    return highestType;
  }

  private isProposalType(value: unknown): value is ProposalType {
    return (
      value === 'decision' ||
      value === 'pattern' ||
      value === 'warning' ||
      value === 'lesson' ||
      value === 'anomaly' ||
      value === 'constraint'
    );
  }
}
