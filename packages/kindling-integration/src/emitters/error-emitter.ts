/**
 * Error Emitter (KINDLING-008)
 *
 * Emits error observations when failures occur.
 * "Errors are not noise, they are data."
 */

import { randomUUID } from 'node:crypto';
import type { KindlingService } from '../kindling-service.js';
import type { ErrorObservation } from '../observation-contract.js';
import { createDebugger } from '../utils/debug.js';

const debug = createDebugger('kindling');

// =============================================================================
// Input Types
// =============================================================================

/**
 * Error details to be recorded
 */
export interface ErrorDetails {
  session_id: string;
  error_type:
    | 'command_failure'
    | 'tool_error'
    | 'aborted_execution'
    | 'partial_state'
    | 'validation_failure';
  context: {
    component: string;
    action_id?: string;
    gate_id?: string;
  };
  error_message: string;
  error_code?: string;
  exit_code?: number;
  recoverable: boolean;
  partial_state_description?: string;
}

// =============================================================================
// Emitter
// =============================================================================

/**
 * Emit an error observation.
 *
 * @param service - KindlingService instance
 * @param errorDetails - Error details
 * @returns The generated error_id
 */
export function emitError(service: KindlingService, errorDetails: ErrorDetails): string {
  const errorId = randomUUID();
  debug('emitting error observation', {
    errorId,
    errorType: errorDetails.error_type,
    component: errorDetails.context.component,
    recoverable: errorDetails.recoverable,
  });

  const observation: ErrorObservation = {
    kind: 'error',
    session_id: errorDetails.session_id,
    timestamp: new Date().toISOString(),
    error_id: errorId,
    error_type: errorDetails.error_type,
    context: {
      component: errorDetails.context.component,
      action_id: errorDetails.context.action_id,
      gate_id: errorDetails.context.gate_id,
    },
    error_message: errorDetails.error_message,
    error_code: errorDetails.error_code,
    exit_code: errorDetails.exit_code,
    recoverable: errorDetails.recoverable,
    partial_state_description: errorDetails.partial_state_description,
  };

  // Fire-and-forget
  service.emit(observation).catch(() => {
    // Silently swallow
  });

  return errorId;
}
