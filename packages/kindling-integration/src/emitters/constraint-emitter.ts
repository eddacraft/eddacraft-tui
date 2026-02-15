/**
 * Constraint Emitter (KINDLING-007b)
 *
 * Emits constraint_applied observations when Anvil prevents an action
 * due to policy, scope, environment, or approval requirements.
 */

import { randomUUID } from 'node:crypto';
import type { KindlingService } from '../kindling-service.js';
import type { ConstraintAppliedObservation } from '../observation-contract.js';

// =============================================================================
// Input Types
// =============================================================================

/**
 * Constraint application details to be recorded
 */
export interface ConstraintDetails {
  session_id: string;
  constraint_type: 'policy' | 'rule' | 'scope' | 'environment' | 'approval_required';
  prevented_action: {
    action_type: string;
    action_target?: string;
  };
  reason: string;
  scope?: string;
  environment?: string;
  options_available?: string[];
  options_allowed?: string[];
}

// =============================================================================
// Emitter
// =============================================================================

/**
 * Emit a constraint_applied observation.
 *
 * @param service - KindlingService instance
 * @param constraint - Constraint application details
 * @returns The generated constraint_id
 */
export function emitConstraintApplied(
  service: KindlingService,
  constraint: ConstraintDetails
): string {
  const constraintId = randomUUID();

  const observation: ConstraintAppliedObservation = {
    kind: 'constraint_applied',
    session_id: constraint.session_id,
    timestamp: new Date().toISOString(),
    constraint_id: constraintId,
    constraint_type: constraint.constraint_type,
    prevented_action: {
      action_type: constraint.prevented_action.action_type,
      action_target: constraint.prevented_action.action_target,
    },
    reason: constraint.reason,
    scope: constraint.scope,
    environment: constraint.environment,
    options_available: constraint.options_available,
    options_allowed: constraint.options_allowed,
  };

  // Fire-and-forget
  service.emit(observation).catch(() => {
    // Silently swallow
  });

  return constraintId;
}
