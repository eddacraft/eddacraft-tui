/**
 * Session Emitter (KINDLING-003)
 *
 * Emits session_start and session_end observations.
 * Sessions form the "spine" of Kindling -- every other observation
 * is linked to a session via session_id.
 */

import { randomUUID } from 'node:crypto';
import type { KindlingService } from '../kindling-service.js';
import type { SessionStartObservation, SessionEndObservation } from '../observation-contract.js';
import { createDebugger } from '../utils/debug.js';

const debug = createDebugger('kindling');

// =============================================================================
// Input Types
// =============================================================================

/**
 * Context needed to start a session observation
 */
export interface SessionStartContext {
  working_directory: string;
  git_ref?: string;
  git_dirty?: boolean;
  anvil_version: string;
  command: string;
  args: string[];
  environment: 'development' | 'ci' | 'production' | 'unknown';
  plan_id?: string;
}

/**
 * Outcome data for ending a session
 */
export interface SessionEndOutcome {
  outcome: 'success' | 'failure' | 'partial' | 'cancelled';
  exit_code: number;
  duration_ms: number;
  summary: {
    gates_evaluated: number;
    gates_passed: number;
    gates_failed: number;
    actions_executed: number;
    errors_encountered: number;
  };
}

// =============================================================================
// Emitters
// =============================================================================

/**
 * Emit a session_start observation.
 *
 * Generates a new session_id and emits fire-and-forget.
 * Returns the generated session_id so callers can link subsequent observations.
 *
 * @param service - KindlingService instance
 * @param context - Session start context
 * @returns The generated session_id
 */
export function emitSessionStart(service: KindlingService, context: SessionStartContext): string {
  const sessionId = randomUUID();
  debug('emitting session_start', {
    sessionId,
    command: context.command,
    environment: context.environment,
  });

  const observation: SessionStartObservation = {
    kind: 'session_start',
    session_id: sessionId,
    timestamp: new Date().toISOString(),
    context: {
      working_directory: context.working_directory,
      git_ref: context.git_ref,
      git_dirty: context.git_dirty,
      anvil_version: context.anvil_version,
      command: context.command,
      args: context.args,
      environment: context.environment,
    },
    plan_id: context.plan_id,
  };

  // Fire-and-forget: catch errors silently to not affect main flow
  service.emit(observation).catch(() => {
    // Silently swallow -- Kindling must never break the host
  });

  return sessionId;
}

/**
 * Emit a session_end observation.
 *
 * @param service - KindlingService instance
 * @param sessionId - The session_id from the corresponding session_start
 * @param outcome - Session outcome data
 * @returns The session_id (same as input, for chaining)
 */
export function emitSessionEnd(
  service: KindlingService,
  sessionId: string,
  outcome: SessionEndOutcome
): string {
  debug('emitting session_end', {
    sessionId,
    outcome: outcome.outcome,
    duration_ms: outcome.duration_ms,
  });

  const observation: SessionEndObservation = {
    kind: 'session_end',
    session_id: sessionId,
    timestamp: new Date().toISOString(),
    outcome: outcome.outcome,
    exit_code: outcome.exit_code,
    duration_ms: outcome.duration_ms,
    summary: outcome.summary,
  };

  // Fire-and-forget
  service.emit(observation).catch(() => {
    // Silently swallow
  });

  return sessionId;
}
