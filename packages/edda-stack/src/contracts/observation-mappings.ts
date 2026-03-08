/**
 * Observation-to-proposal type mapping contracts.
 *
 * Defines conversion rules from Kindling observation kinds to Ember proposal
 * types, both for single observations and aggregated multi-kind inputs.
 *
 * @module observation-mappings
 * @see STACK-006
 */

import type { ProposalType } from './ember-proposal.js';
import type { ObservationKind } from './ports/kindling.port.js';

export const OBSERVATION_KIND_TO_PROPOSAL_TYPE_MAPPING: Record<ObservationKind, ProposalType> = {
  gate_evaluated: 'pattern',
  action_executed: 'pattern',
  action_failed: 'warning',
  plan_started: 'decision',
  plan_completed: 'lesson',
  constraint_applied: 'constraint',
  error_recorded: 'warning',
  metric_recorded: 'pattern',
  custom: 'pattern',
} as const;

export function mapObservationKindToProposalType(observationKind: ObservationKind): ProposalType {
  return OBSERVATION_KIND_TO_PROPOSAL_TYPE_MAPPING[observationKind];
}

export function mapObservationKindsToProposalType(
  observationKinds: ReadonlyArray<ObservationKind>
): ProposalType {
  const kinds = new Set(observationKinds);
  const hasFailureSignal = kinds.has('error_recorded') || kinds.has('action_failed');
  const hasSuccessSignal = kinds.has('action_executed') || kinds.has('plan_completed');

  if (hasFailureSignal && hasSuccessSignal) {
    return 'lesson';
  }

  if (hasFailureSignal) {
    return 'warning';
  }

  if (kinds.has('plan_completed')) {
    return 'lesson';
  }

  if (kinds.has('constraint_applied')) {
    return 'constraint';
  }

  if (kinds.has('plan_started')) {
    return 'decision';
  }

  return 'pattern';
}
