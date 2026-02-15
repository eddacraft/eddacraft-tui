/**
 * Gate Emitter (KINDLING-004)
 *
 * Emits gate_evaluated observations when gates are checked.
 * Gate evaluations form the governance record -- why things were
 * allowed or blocked.
 */

import { randomUUID } from 'node:crypto';
import type { KindlingService } from '../kindling-service.js';
import type { GateEvaluatedObservation } from '../observation-contract.js';

// =============================================================================
// Input Types
// =============================================================================

/**
 * Gate evaluation result to be recorded
 */
export interface GateResult {
  session_id: string;
  gate_id: string;
  gate_version?: string;
  inputs: {
    file_count?: number;
    changed_files?: string[];
    baseline_hash?: string;
  };
  outcome: 'pass' | 'fail' | 'error' | 'skipped';
  rules_evaluated: string[];
  rules_violated?: string[];
  enforcement: 'blocking' | 'warning' | 'informational';
  duration_ms: number;
  violation_count?: number;
  warning_count?: number;
}

// =============================================================================
// Emitter
// =============================================================================

/**
 * Emit a gate_evaluated observation.
 *
 * @param service - KindlingService instance
 * @param gateResult - Gate evaluation result
 * @returns The generated gate_eval_id
 */
export function emitGateEvaluated(service: KindlingService, gateResult: GateResult): string {
  const gateEvalId = randomUUID();

  const observation: GateEvaluatedObservation = {
    kind: 'gate_evaluated',
    session_id: gateResult.session_id,
    timestamp: new Date().toISOString(),
    gate_eval_id: gateEvalId,
    gate_id: gateResult.gate_id,
    gate_version: gateResult.gate_version,
    inputs: {
      file_count: gateResult.inputs.file_count,
      changed_files: gateResult.inputs.changed_files,
      baseline_hash: gateResult.inputs.baseline_hash,
    },
    outcome: gateResult.outcome,
    rules_evaluated: gateResult.rules_evaluated,
    rules_violated: gateResult.rules_violated,
    enforcement: gateResult.enforcement,
    duration_ms: gateResult.duration_ms,
    violation_count: gateResult.violation_count,
    warning_count: gateResult.warning_count,
  };

  // Fire-and-forget
  service.emit(observation).catch(() => {
    // Silently swallow
  });

  return gateEvalId;
}
