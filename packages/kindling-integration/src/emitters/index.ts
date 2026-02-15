/**
 * Emitters Barrel Export (KINDLING-003 through 008)
 *
 * All emitter functions and their input types, re-exported for convenience.
 */

// Session (KINDLING-003)
export {
  emitSessionStart,
  emitSessionEnd,
  type SessionStartContext,
  type SessionEndOutcome,
} from './session-emitter.js';

// Gate (KINDLING-004)
export { emitGateEvaluated, type GateResult } from './gate-emitter.js';

// Action (KINDLING-005)
export { emitActionExecuted, type ActionDetails } from './action-emitter.js';

// Plan (KINDLING-006)
export {
  emitPlanCreated,
  emitPlanEdited,
  emitPlanApproved,
  emitPlanRejected,
  type PlanCreatedInput,
  type PlanEditedInput,
  type PlanApprovedInput,
  type PlanRejectedInput,
} from './plan-emitter.js';

// Human Input (KINDLING-007a)
export { emitHumanInput, type HumanInputDetails } from './human-input-emitter.js';

// Constraint (KINDLING-007b)
export { emitConstraintApplied, type ConstraintDetails } from './constraint-emitter.js';

// Error (KINDLING-008)
export { emitError, type ErrorDetails } from './error-emitter.js';
