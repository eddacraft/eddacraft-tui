/**
 * Action Emitter (KINDLING-005)
 *
 * Emits action_executed observations when commands, tool invocations,
 * file operations, or diff applications occur.
 */

import { randomUUID } from 'node:crypto';
import type { KindlingService } from '../kindling-service.js';
import type { ActionExecutedObservation } from '../observation-contract.js';
import { createDebugger } from '../utils/debug.js';

const debug = createDebugger('kindling');

// =============================================================================
// Input Types
// =============================================================================

/**
 * Action execution details to be recorded
 */
export interface ActionDetails {
  session_id: string;
  action_type: 'command' | 'tool_invocation' | 'file_write' | 'file_delete' | 'diff_apply';
  details: {
    command?: string;
    tool_name?: string;
    file_paths?: string[];
    diff_summary?: {
      additions: number;
      deletions: number;
      files_changed: number;
    };
    working_directory: string;
    environment_target?: string;
  };
  governed_by_gate_id?: string;
  governed_by_plan_id?: string;
  outcome: 'success' | 'failure' | 'partial';
  exit_code?: number;
  duration_ms: number;
}

// =============================================================================
// Emitter
// =============================================================================

/**
 * Emit an action_executed observation.
 *
 * @param service - KindlingService instance
 * @param actionDetails - Action execution details
 * @returns The generated action_id
 */
export function emitActionExecuted(service: KindlingService, actionDetails: ActionDetails): string {
  const actionId = randomUUID();
  debug('emitting action_executed', {
    actionId,
    actionType: actionDetails.action_type,
    outcome: actionDetails.outcome,
  });

  const observation: ActionExecutedObservation = {
    kind: 'action_executed',
    session_id: actionDetails.session_id,
    timestamp: new Date().toISOString(),
    action_id: actionId,
    action_type: actionDetails.action_type,
    details: {
      command: actionDetails.details.command,
      tool_name: actionDetails.details.tool_name,
      file_paths: actionDetails.details.file_paths,
      diff_summary: actionDetails.details.diff_summary,
      working_directory: actionDetails.details.working_directory,
      environment_target: actionDetails.details.environment_target,
    },
    governed_by_gate_id: actionDetails.governed_by_gate_id,
    governed_by_plan_id: actionDetails.governed_by_plan_id,
    outcome: actionDetails.outcome,
    exit_code: actionDetails.exit_code,
    duration_ms: actionDetails.duration_ms,
  };

  // Fire-and-forget
  service.emit(observation).catch(() => {
    // Silently swallow
  });

  return actionId;
}
