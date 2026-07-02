/**
 * Observation Contract (v1)
 *
 * Defines the 13 observation kinds that Anvil must emit to Kindling.
 * This is the write-only contract - what gets recorded, not how it's queried.
 *
 * Based on: "What Kindling is used for in Anvil v1" specification
 *
 * @see query-contract.ts for the read-only query surface
 */

import { z } from 'zod';

// =============================================================================
// Schema Version
// =============================================================================

export const OBSERVATION_CONTRACT_VERSION = '1.0.0';

// =============================================================================
// 1. Session Recording (The Spine)
// =============================================================================

/**
 * Session Start: Every Anvil run opens a session capsule
 */
export const SessionStartObservationSchema = z.object({
  kind: z.literal('session_start'),
  session_id: z.string().uuid().describe('Unique session identifier'),
  timestamp: z.string().datetime().describe('When session started'),

  // Context at session start
  context: z.object({
    working_directory: z.string().describe('Workspace root'),
    git_ref: z.string().optional().describe('Current git commit/branch'),
    git_dirty: z.boolean().optional().describe('Whether working tree has changes'),
    anvil_version: z.string().describe('Anvil CLI version'),
    command: z.string().describe('CLI command invoked (e.g., "anvil check")'),
    args: z.array(z.string()).describe('Command arguments'),
    environment: z
      .enum(['development', 'ci', 'production', 'unknown'])
      .describe('Execution context'),
  }),

  // Plan linkage (if this session is executing a plan)
  plan_id: z.string().optional().describe('Plan being executed (if any)'),
});

export type SessionStartObservation = z.infer<typeof SessionStartObservationSchema>;

/**
 * Session End: Closes the capsule with outcome
 */
export const SessionEndObservationSchema = z.object({
  kind: z.literal('session_end'),
  session_id: z.string().uuid().describe('Session being closed'),
  timestamp: z.string().datetime().describe('When session ended'),

  // Outcome
  outcome: z.enum(['success', 'failure', 'partial', 'cancelled']).describe('Session result'),
  exit_code: z.number().int().describe('Process exit code'),
  duration_ms: z.number().int().nonnegative().describe('Session duration in milliseconds'),

  // Summary counts (for quick reference)
  summary: z.object({
    gates_evaluated: z.number().int().nonnegative(),
    gates_passed: z.number().int().nonnegative(),
    gates_failed: z.number().int().nonnegative(),
    actions_executed: z.number().int().nonnegative(),
    errors_encountered: z.number().int().nonnegative(),
  }),
});

export type SessionEndObservation = z.infer<typeof SessionEndObservationSchema>;

// =============================================================================
// 2. PlanSpec Lifecycle Tracking
// =============================================================================

/**
 * Plan Created: New plan authored
 */
export const PlanCreatedObservationSchema = z.object({
  kind: z.literal('plan_created'),
  session_id: z.string().uuid(),
  timestamp: z.string().datetime(),

  plan_id: z.string().describe('Unique plan identifier'),
  plan_version: z.string().describe('Initial version (e.g., "1.0")'),
  plan_path: z.string().describe('Path to plan file'),
  plan_hash: z.string().describe('Content hash (for version tracking)'),

  // Context
  created_by: z.enum(['human', 'ai', 'system']).describe('Who created the plan'),
  source: z.string().optional().describe('Source context (e.g., "github-issue-123")'),
});

export type PlanCreatedObservation = z.infer<typeof PlanCreatedObservationSchema>;

/**
 * Plan Edited: Plan modified
 */
export const PlanEditedObservationSchema = z.object({
  kind: z.literal('plan_edited'),
  session_id: z.string().uuid(),
  timestamp: z.string().datetime(),

  plan_id: z.string(),
  previous_version: z.string(),
  new_version: z.string(),
  previous_hash: z.string(),
  new_hash: z.string(),

  edited_by: z.enum(['human', 'ai', 'system']),
  change_summary: z.string().optional().describe('Brief description of changes'),
});

export type PlanEditedObservation = z.infer<typeof PlanEditedObservationSchema>;

/**
 * Plan Approved: Human approval recorded
 */
export const PlanApprovedObservationSchema = z.object({
  kind: z.literal('plan_approved'),
  session_id: z.string().uuid(),
  timestamp: z.string().datetime(),

  plan_id: z.string(),
  plan_version: z.string(),
  approved_by: z.string().describe('Human identifier (e.g., username, email)'),
  approval_method: z.enum(['cli_confirm', 'explicit_flag', 'ci_gate']).describe('How approved'),
});

export type PlanApprovedObservation = z.infer<typeof PlanApprovedObservationSchema>;

/**
 * Plan Rejected: Human rejection recorded
 */
export const PlanRejectedObservationSchema = z.object({
  kind: z.literal('plan_rejected'),
  session_id: z.string().uuid(),
  timestamp: z.string().datetime(),

  plan_id: z.string(),
  plan_version: z.string(),
  rejected_by: z.string().describe('Human identifier'),
  rejection_reason: z.string().optional().describe('Why rejected (if provided)'),
});

export type PlanRejectedObservation = z.infer<typeof PlanRejectedObservationSchema>;

// =============================================================================
// 3. Action Provenance (What Actually Happened)
// =============================================================================

/**
 * Action Executed: Observable action taken
 */
export const ActionExecutedObservationSchema = z.object({
  kind: z.literal('action_executed'),
  session_id: z.string().uuid(),
  timestamp: z.string().datetime(),

  action_id: z.string().describe('Unique action identifier'),
  action_type: z
    .enum(['command', 'tool_invocation', 'file_write', 'file_delete', 'diff_apply'])
    .describe('Type of action'),

  // What happened (redacted for security)
  details: z.object({
    command: z.string().optional().describe('Command executed (redacted)'),
    tool_name: z.string().optional().describe('Tool invoked'),
    file_paths: z.array(z.string()).optional().describe('Files touched'),
    diff_summary: z
      .object({
        additions: z.number().int().nonnegative(),
        deletions: z.number().int().nonnegative(),
        files_changed: z.number().int().nonnegative(),
      })
      .optional()
      .describe('Summary of changes (NOT full diff)'),
    working_directory: z.string().describe('Where action executed'),
    environment_target: z.string().optional().describe('Environment (e.g., "dev", "staging")'),
  }),

  // Governance linkage
  governed_by_gate_id: z.string().optional().describe('Gate evaluation that allowed this'),
  governed_by_plan_id: z.string().optional().describe('Plan that authorized this'),

  // Outcome
  outcome: z.enum(['success', 'failure', 'partial']),
  exit_code: z.number().int().optional(),
  duration_ms: z.number().int().nonnegative(),
});

export type ActionExecutedObservation = z.infer<typeof ActionExecutedObservationSchema>;

// =============================================================================
// 4. Gate Evaluation Records
// =============================================================================

/**
 * Gate Evaluated: Structured gate check result
 */
export const GateEvaluatedObservationSchema = z.object({
  kind: z.literal('gate_evaluated'),
  session_id: z.string().uuid(),
  timestamp: z.string().datetime(),

  gate_eval_id: z.string().describe('Unique gate evaluation identifier'),
  gate_id: z.string().describe('Gate identifier (e.g., "architecture", "coverage")'),
  gate_version: z.string().optional().describe('Gate definition version'),

  // Inputs (sanitised)
  inputs: z
    .object({
      file_count: z.number().int().nonnegative().optional(),
      changed_files: z.array(z.string()).optional().describe('Files evaluated (paths only)'),
      baseline_hash: z.string().optional().describe('Architecture baseline used'),
    })
    .describe('What was evaluated (no sensitive data)'),

  // Outcome
  outcome: z.enum(['pass', 'fail', 'error', 'skipped']),

  // Reasons (rule IDs, not prose)
  rules_evaluated: z.array(z.string()).describe('Rule identifiers checked'),
  rules_violated: z.array(z.string()).optional().describe('Rule identifiers that failed'),

  // Enforcement
  enforcement: z.enum(['blocking', 'warning', 'informational']).describe('Action taken on failure'),

  // Metrics
  duration_ms: z.number().int().nonnegative(),
  violation_count: z.number().int().nonnegative().optional(),
  warning_count: z.number().int().nonnegative().optional(),
});

export type GateEvaluatedObservation = z.infer<typeof GateEvaluatedObservationSchema>;

// =============================================================================
// 5. Decision Constraints (Why Options Were Removed)
// =============================================================================

/**
 * Constraint Applied: When Anvil prevents an action
 */
export const ConstraintAppliedObservationSchema = z.object({
  kind: z.literal('constraint_applied'),
  session_id: z.string().uuid(),
  timestamp: z.string().datetime(),

  constraint_id: z.string().describe('Constraint identifier'),
  constraint_type: z
    .enum(['policy', 'rule', 'scope', 'environment', 'approval_required'])
    .describe('Type of constraint'),

  // What was prevented
  prevented_action: z.object({
    action_type: z.string().describe('What was attempted'),
    action_target: z.string().optional().describe('Target (e.g., file path, command)'),
  }),

  // Why prevented
  reason: z.string().describe('Rule ID or policy name that prevented action'),
  scope: z.string().optional().describe('Scope constraint (e.g., "src/ only")'),
  environment: z.string().optional().describe('Environment constraint (e.g., "not in production")'),

  // What was available vs what was allowed
  options_available: z.array(z.string()).optional().describe('All possible actions'),
  options_allowed: z.array(z.string()).optional().describe('Actions that passed constraints'),
});

export type ConstraintAppliedObservation = z.infer<typeof ConstraintAppliedObservationSchema>;

// =============================================================================
// 6. Human Inputs (First-Class Events)
// =============================================================================

/**
 * Human Input: User action recorded
 */
export const HumanInputObservationSchema = z.object({
  kind: z.literal('human_input'),
  session_id: z.string().uuid(),
  timestamp: z.string().datetime(),

  // Optional for backwards compatibility with observations stored before the
  // field existed; always populated by emitHumanInput (CIB-118).
  input_id: z.string().uuid().optional().describe('Unique input identifier for linking'),

  input_type: z
    .enum(['approval', 'override', 'rejection', 'manual_edit', 'confirmation', 'cancellation'])
    .describe('Type of human action'),

  // Context
  context: z.object({
    prompt: z.string().optional().describe('What user was asked to decide'),
    target: z.string().optional().describe('What the input was about (e.g., plan_id, gate_id)'),
  }),

  // Decision
  decision: z.string().describe('What user chose'),
  reason: z.string().optional().describe('User-provided reason (if any)'),

  // Identity (for accountability)
  user_identifier: z.string().describe('User identifier (username, email, etc.)'),
});

export type HumanInputObservation = z.infer<typeof HumanInputObservationSchema>;

// =============================================================================
// 7. Error and Interruption History
// =============================================================================

/**
 * Error: Failure recorded (not noise, data)
 */
export const ErrorObservationSchema = z.object({
  kind: z.literal('error'),
  session_id: z.string().uuid(),
  timestamp: z.string().datetime(),

  error_id: z.string().describe('Unique error identifier'),
  error_type: z
    .enum([
      'command_failure',
      'tool_error',
      'aborted_execution',
      'partial_state',
      'validation_failure',
    ])
    .describe('Error category'),

  // What failed
  context: z.object({
    component: z.string().describe('What was running (e.g., "gate:architecture")'),
    action_id: z.string().optional().describe('Action that failed (if applicable)'),
    gate_id: z.string().optional().describe('Gate that errored (if applicable)'),
  }),

  // Error details (sanitised)
  error_message: z.string().describe('Error message (redacted if sensitive)'),
  error_code: z.string().optional().describe('Error code (e.g., "ENOENT")'),
  exit_code: z.number().int().optional(),

  // State
  recoverable: z.boolean().describe('Whether error is recoverable'),
  partial_state_description: z
    .string()
    .optional()
    .describe('Description of partial state if interrupted'),
});

export type ErrorObservation = z.infer<typeof ErrorObservationSchema>;

// =============================================================================
// 9. Usage Analytics (USAGE-001)
// =============================================================================

/**
 * Argument shape (USAGE-001 privacy contract).
 *
 * Records an argument's NAME and the SHAPE of its value (coarse type,
 * length, presence) but never the value itself. For arguments whose
 * name matches the `SENSITIVE_FIELDS` deny-list, the shape is elided
 * and `redacted` carries the `<redacted>` marker instead — only the
 * name remains visible. Mirrors `anvil_observability::redaction::ArgShape`.
 */
export const ArgShapeSchema = z
  .object({
    name: z.string().describe('Argument name as typed (no leading dashes); never a value'),
    redacted: z
      .literal('<redacted>')
      .optional()
      .describe('Set for sensitive-named args; shape fields are then absent'),
    shape: z
      .enum(['integer', 'boolean', 'string', 'flag'])
      .optional()
      .describe('Coarse value type when not redacted'),
    length: z
      .enum(['empty', 'short', 'medium', 'long'])
      .optional()
      .describe('Coarse length bucket when not redacted and a value was supplied (never exact)'),
    present: z.boolean().optional().describe('Whether a value was supplied vs a bare flag'),
  })
  // Reject unknown keys so a producer that accidentally adds a raw
  // `value`/`value_len` field fails the contract instead of silently
  // stripping it — the privacy guardrail.
  .strict();

export type ArgShape = z.infer<typeof ArgShapeSchema>;

/**
 * One inline resolved feature-flag entry on a usage row (ADR-041).
 * USAGE-001 emits an empty `flag_set`; USAGE-002 populates it.
 */
export const FlagSetEntrySchema = z.object({
  key: z.string().describe('Canonical manifest key — the stable join key (ADR-041 D-2)'),
  variant: z.string().describe('Resolved variant for this invocation'),
  source: z.enum(['snapshot', 'override', 'default']).describe('Where the value came from'),
  gate_affecting: z.boolean().describe('Whether the flag is gate-affecting (ADR-019 boundary)'),
});

export type FlagSetEntry = z.infer<typeof FlagSetEntrySchema>;

/**
 * Command Invoked: one row per user-initiated CLI command or JSON-RPC
 * method call. Records THAT a command ran and the redacted shape of its
 * arguments — never argument values, results, or output. See the
 * privacy contract at `docs/observability/usage-analytics.md`.
 *
 * Mirrors `CommandInvokedObservation` in
 * `crates/anvil-intercept/src/kindling_observation.rs`.
 */
export const CommandInvokedObservationSchema = z
  .object({
    kind: z.literal('command.invoked'),
    session_id: z.string().uuid(),
    timestamp: z.string().datetime(),

    command: z.string().describe('Canonical command or method name (e.g. "check", "session.list")'),
    principal: z
      .string()
      .describe('Anonymised principal — one-way hash, or "anonymous"; never the raw identity'),
    args: z.array(ArgShapeSchema).describe('Redacted per-argument shapes (no values)'),
    flag_set: z
      .array(FlagSetEntrySchema)
      .describe('Inline resolved flag context (ADR-041); empty for USAGE-001, always present'),
    traceparent: z.string().optional().describe('W3C traceparent for cross-pipe correlation'),
  })
  // Reject unknown keys so a future producer that accidentally adds a
  // raw argv/value field fails validation instead of silently passing.
  .strict();

export type CommandInvokedObservation = z.infer<typeof CommandInvokedObservationSchema>;

// =============================================================================
// False-Positive Reported (OPSUP-007 / ADR-089)
// =============================================================================

export const FalsePositiveReportedObservationSchema = z
  .object({
    kind: z.literal('false_positive_reported'),
    session_id: z.string().uuid(),
    timestamp: z.string().datetime(),

    check_id: z.string().describe('Stable ANV-* check ID the false positive is reported against'),
    hashed_path: z
      .string()
      .describe('One-way hash of the file path; the plaintext path is never recorded'),
    line: z.number().int().min(1).describe('1-based line number the report points at'),
    principal: z
      .string()
      .describe('Anonymised principal — one-way hash, or "anonymous"; never the raw identity'),
    snippet: z
      .string()
      .optional()
      .describe('Opt-in source snippet; absent by default (fail-closed on anonymisation)'),
    traceparent: z.string().optional().describe('W3C traceparent for cross-pipe correlation'),
  })
  // Reject unknown keys so a producer that accidentally adds a raw path or
  // source field fails validation instead of silently passing.
  .strict();

export type FalsePositiveReportedObservation = z.infer<
  typeof FalsePositiveReportedObservationSchema
>;

// =============================================================================
// Observation (Union Type)
// =============================================================================

/**
 * All observation kinds (discriminated union by 'kind')
 */
export const ObservationSchema = z.discriminatedUnion('kind', [
  SessionStartObservationSchema,
  SessionEndObservationSchema,
  PlanCreatedObservationSchema,
  PlanEditedObservationSchema,
  PlanApprovedObservationSchema,
  PlanRejectedObservationSchema,
  ActionExecutedObservationSchema,
  GateEvaluatedObservationSchema,
  ConstraintAppliedObservationSchema,
  HumanInputObservationSchema,
  ErrorObservationSchema,
  CommandInvokedObservationSchema,
  FalsePositiveReportedObservationSchema,
]);

export type Observation =
  | SessionStartObservation
  | SessionEndObservation
  | PlanCreatedObservation
  | PlanEditedObservation
  | PlanApprovedObservation
  | PlanRejectedObservation
  | ActionExecutedObservation
  | GateEvaluatedObservation
  | ConstraintAppliedObservation
  | HumanInputObservation
  | ErrorObservation
  | CommandInvokedObservation
  | FalsePositiveReportedObservation;

// =============================================================================
// Observation Emission Contract
// =============================================================================

/**
 * What Anvil must emit to be "Kindling-complete"
 *
 * Every Anvil execution must:
 * 1. Emit SessionStartObservation when any command starts
 * 2. Emit SessionEndObservation when command completes (success or failure)
 * 3. Emit GateEvaluatedObservation for every gate check
 * 4. Emit ActionExecutedObservation for every observable action
 * 5. Emit ErrorObservation for every failure (even recoverable)
 * 6. Emit HumanInputObservation for every approval/override/rejection
 * 7. Emit ConstraintAppliedObservation when actions are prevented
 * 8. Emit Plan* observations for all plan lifecycle events
 *
 * Observations are:
 * - Immutable (write-once)
 * - Timestamped (ISO8601)
 * - Linked (session_id, plan_id, gate_id, action_id)
 * - Sanitised (no secrets, redacted commands)
 * - Facts only (no interpretation, no inference)
 */

// =============================================================================
// Integration Points (Where to Emit)
// =============================================================================

/**
 * Anvil codebase integration points:
 *
 * SessionStart/End:
 * - cli/src/commands/*.ts (every command entry/exit)
 * - cli/src/commands/watch.ts (each watch cycle)
 *
 * GateEvaluated:
 * - core/src/gate/gate-runner.ts (GateRunner.run completion)
 * - Each check implementation (architecture, coverage, secrets, etc.)
 *
 * ActionExecuted:
 * - Anywhere Anvil executes commands (via child_process)
 * - File write/delete operations
 * - Diff application
 *
 * Plan*:
 * - core/src/aps/ (plan parsing, validation, execution)
 * - cli/src/commands/plan.ts (plan management commands)
 *
 * HumanInput:
 * - cli/src/tui/ (TUI confirmation prompts)
 * - cli/src/commands/ (CLI --approve flags)
 *
 * ConstraintApplied:
 * - core/src/gate/ (when gate blocks action)
 * - Policy evaluation layers
 *
 * Error:
 * - All try/catch blocks that handle failures
 * - Process error handlers
 * - Validation failures
 */

// =============================================================================
// Validation Utilities
// =============================================================================

/**
 * Validate an observation before emission
 */
export function validateObservation(data: unknown): {
  success: boolean;
  data?: Observation;
  error?: string;
} {
  const result = ObservationSchema.safeParse(data);
  if (result.success) {
    return { success: true, data: result.data };
  }
  return { success: false, error: result.error.format()._errors.join(', ') };
}

/**
 * Check if observation contains sensitive data (should never pass validation)
 */
export function containsSensitiveData(obs: Observation): {
  hasSensitiveData: boolean;
  issues: string[];
} {
  const issues: string[] = [];

  // Check for common sensitive patterns
  const payloadStr = JSON.stringify(obs);

  // Passwords, tokens, keys
  if (/password|token|secret|api[_-]?key|private[_-]?key/i.test(payloadStr)) {
    issues.push('Possible password/token/key detected');
  }

  // AWS credentials
  if (/AKIA[0-9A-Z]{16}|aws_access_key_id|aws_secret_access_key/i.test(payloadStr)) {
    issues.push('Possible AWS credentials detected');
  }

  // Email addresses (may be sensitive depending on context)
  if (/[a-zA-Z0-9._%+-]{1,64}@[a-zA-Z0-9]+(?:[.-][a-zA-Z0-9]+)*\.[a-zA-Z]{2,}/g.test(payloadStr)) {
    issues.push('Email addresses detected (may be sensitive)');
  }

  return {
    hasSensitiveData: issues.length > 0,
    issues,
  };
}
