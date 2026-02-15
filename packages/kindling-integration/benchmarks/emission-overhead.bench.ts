/**
 * Emission Overhead Benchmark (KINDLING-017)
 *
 * Validates that observation emission meets the < 50ms acceptance criteria.
 * With a no-op store, emission should be < 1ms.
 *
 * Run: pnpm bench --filter kindling-integration
 *
 * @see plans/modules/kindling-integration.aps.md - KINDLING-017
 */

import { bench, describe } from 'vitest';
import {
  validateObservation,
  containsSensitiveData,
  ObservationSchema,
  SessionStartObservationSchema,
  SessionEndObservationSchema,
  GateEvaluatedObservationSchema,
  ActionExecutedObservationSchema,
  ErrorObservationSchema,
  type Observation,
  type SessionStartObservation,
  type SessionEndObservation,
  type GateEvaluatedObservation,
  type ActionExecutedObservation,
  type ErrorObservation,
} from '../src/index.js';

// =============================================================================
// Test Fixtures
// =============================================================================

const VALID_UUID = '550e8400-e29b-41d4-a716-446655440000';
const VALID_TIMESTAMP = '2026-02-15T10:00:00.000Z';

const sessionStartObs: SessionStartObservation = {
  kind: 'session_start',
  session_id: VALID_UUID,
  timestamp: VALID_TIMESTAMP,
  context: {
    working_directory: '/home/user/project',
    anvil_version: '1.0.0',
    command: 'anvil check',
    args: ['--watch', '--verbose'],
    environment: 'development',
  },
};

const sessionEndObs: SessionEndObservation = {
  kind: 'session_end',
  session_id: VALID_UUID,
  timestamp: VALID_TIMESTAMP,
  outcome: 'success',
  exit_code: 0,
  duration_ms: 5000,
  summary: {
    gates_evaluated: 3,
    gates_passed: 2,
    gates_failed: 1,
    actions_executed: 5,
    errors_encountered: 0,
  },
};

const gateEvalObs: GateEvaluatedObservation = {
  kind: 'gate_evaluated',
  session_id: VALID_UUID,
  timestamp: VALID_TIMESTAMP,
  gate_eval_id: 'gate-eval-001',
  gate_id: 'architecture',
  inputs: {
    file_count: 12,
    changed_files: ['src/index.ts', 'src/config.ts', 'src/utils.ts'],
    baseline_hash: 'abc123',
  },
  outcome: 'pass',
  rules_evaluated: ['no-circular-deps', 'layer-boundaries', 'import-restrictions'],
  enforcement: 'blocking',
  duration_ms: 250,
  violation_count: 0,
  warning_count: 1,
};

const actionExecObs: ActionExecutedObservation = {
  kind: 'action_executed',
  session_id: VALID_UUID,
  timestamp: VALID_TIMESTAMP,
  action_id: 'action-001',
  action_type: 'command',
  details: {
    command: 'npm test',
    tool_name: 'npm',
    file_paths: ['src/index.ts'],
    diff_summary: {
      additions: 10,
      deletions: 3,
      files_changed: 1,
    },
    working_directory: '/home/user/project',
  },
  outcome: 'success',
  exit_code: 0,
  duration_ms: 3000,
};

const errorObs: ErrorObservation = {
  kind: 'error',
  session_id: VALID_UUID,
  timestamp: VALID_TIMESTAMP,
  error_id: 'err-001',
  error_type: 'command_failure',
  context: {
    component: 'gate:architecture',
    action_id: 'action-001',
  },
  error_message: 'Process exited with code 1',
  error_code: 'ENOENT',
  exit_code: 1,
  recoverable: true,
};

// =============================================================================
// No-Op Store (simulates emission without I/O)
// =============================================================================

/**
 * No-op store that accepts observations but does nothing.
 * This isolates the measurement to validation + serialisation overhead.
 */
class NoOpStore {
  async emit(_obs: Observation): Promise<void> {
    // Intentionally empty -- measures pure overhead
  }
}

const noOpStore = new NoOpStore();

// =============================================================================
// Emission Pipeline (validate + sensitive check + store)
// =============================================================================

/**
 * Full emission pipeline matching what KindlingService.emit() should do:
 * 1. Validate observation via Zod
 * 2. Check for sensitive data
 * 3. Write to store (no-op here)
 */
async function fullEmissionPipeline(obs: Observation): Promise<void> {
  const validation = validateObservation(obs);
  if (!validation.success) {
    throw new Error(`Invalid observation: ${validation.error}`);
  }

  const sensitiveCheck = containsSensitiveData(validation.data!);
  if (sensitiveCheck.hasSensitiveData) {
    throw new Error(`Sensitive data detected: ${sensitiveCheck.issues.join(', ')}`);
  }

  await noOpStore.emit(validation.data!);
}

// =============================================================================
// Benchmarks: Individual Emission (Target: < 50ms, expect < 1ms with no-op)
// =============================================================================

describe('Observation emission overhead (no-op store)', () => {
  bench('emit session_start', async () => {
    await fullEmissionPipeline(sessionStartObs);
  });

  bench('emit session_end', async () => {
    await fullEmissionPipeline(sessionEndObs);
  });

  bench('emit gate_evaluated', async () => {
    await fullEmissionPipeline(gateEvalObs);
  });

  bench('emit action_executed', async () => {
    await fullEmissionPipeline(actionExecObs);
  });

  bench('emit error', async () => {
    await fullEmissionPipeline(errorObs);
  });
});

// =============================================================================
// Benchmarks: Zod Validation Alone
// =============================================================================

describe('Zod validation overhead', () => {
  bench('validate session_start via schema', () => {
    SessionStartObservationSchema.parse(sessionStartObs);
  });

  bench('validate session_end via schema', () => {
    SessionEndObservationSchema.parse(sessionEndObs);
  });

  bench('validate gate_evaluated via schema', () => {
    GateEvaluatedObservationSchema.parse(gateEvalObs);
  });

  bench('validate action_executed via schema', () => {
    ActionExecutedObservationSchema.parse(actionExecObs);
  });

  bench('validate error via schema', () => {
    ErrorObservationSchema.parse(errorObs);
  });

  bench('validate via discriminated union (ObservationSchema)', () => {
    ObservationSchema.parse(sessionStartObs);
  });

  bench('safeParse via discriminated union (ObservationSchema)', () => {
    ObservationSchema.safeParse(sessionStartObs);
  });
});

// =============================================================================
// Benchmarks: Sensitive Data Check Alone
// =============================================================================

describe('Sensitive data check overhead', () => {
  bench('containsSensitiveData (clean observation)', () => {
    containsSensitiveData(gateEvalObs);
  });

  bench('containsSensitiveData (observation with many fields)', () => {
    containsSensitiveData(actionExecObs);
  });
});

// =============================================================================
// Benchmarks: Batch Emission (10 observations)
// =============================================================================

describe('Batch emission (10 observations)', () => {
  const batch: Observation[] = [
    sessionStartObs,
    gateEvalObs,
    gateEvalObs,
    actionExecObs,
    actionExecObs,
    actionExecObs,
    errorObs,
    gateEvalObs,
    actionExecObs,
    sessionEndObs,
  ];

  bench('emit batch of 10 observations sequentially', async () => {
    for (const obs of batch) {
      await fullEmissionPipeline(obs);
    }
  });

  bench('emit batch of 10 observations in parallel', async () => {
    await Promise.all(batch.map((obs) => fullEmissionPipeline(obs)));
  });
});
