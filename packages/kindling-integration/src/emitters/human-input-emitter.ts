/**
 * Human Input Emitter (KINDLING-007a)
 *
 * Emits human_input observations when a user makes a decision
 * (approval, override, rejection, manual edit, confirmation, cancellation).
 */

import { randomUUID } from 'node:crypto';
import type { KindlingService } from '../kindling-service.js';
import type { HumanInputObservation } from '../observation-contract.js';

// =============================================================================
// Input Types
// =============================================================================

/**
 * Human input details to be recorded
 */
export interface HumanInputDetails {
  session_id: string;
  input_type:
    | 'approval'
    | 'override'
    | 'rejection'
    | 'manual_edit'
    | 'confirmation'
    | 'cancellation';
  context: {
    prompt?: string;
    target?: string;
  };
  decision: string;
  reason?: string;
  user_identifier: string;
}

// =============================================================================
// Emitter
// =============================================================================

/**
 * Emit a human_input observation.
 *
 * @param service - KindlingService instance
 * @param input - Human input details
 * @returns The generated input_id, persisted on the observation for linking (CIB-118)
 */
export function emitHumanInput(service: KindlingService, input: HumanInputDetails): string {
  const inputId = randomUUID();

  const observation: HumanInputObservation = {
    kind: 'human_input',
    session_id: input.session_id,
    timestamp: new Date().toISOString(),
    input_id: inputId,
    input_type: input.input_type,
    context: {
      prompt: input.context.prompt,
      target: input.context.target,
    },
    decision: input.decision,
    reason: input.reason,
    user_identifier: input.user_identifier,
  };

  // Fire-and-forget
  service.emit(observation).catch(() => {
    // Silently swallow
  });

  return inputId;
}
