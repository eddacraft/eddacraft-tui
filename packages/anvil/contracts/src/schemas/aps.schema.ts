import { z } from 'zod';

/**
 * Schema version - using literal type for strict versioning
 */
const SCHEMA_VERSION = '0.1.0' as const;

const FORBIDDEN_KEYS = new Set(['__proto__', 'constructor', 'prototype']);
const safeKey = z.string().refine((k) => !FORBIDDEN_KEYS.has(k), {
  message: 'Forbidden key — potential prototype pollution',
});

/**
 * Build a safe z.record() that rejects __proto__ keys *before* Zod's own
 * record parsing can strip them (Zod v4 silently drops __proto__ during
 * internal object construction, so the safeKey refinement alone cannot
 * catch it).  The superRefine step runs on the raw input, then pipes into
 * the normal record schema that handles constructor/prototype via safeKey.
 */
function safeRecord<V extends z.ZodTypeAny>(valueSchema: V) {
  return z
    .unknown()
    .superRefine((val, ctx) => {
      if (
        val !== null &&
        typeof val === 'object' &&
        Object.prototype.hasOwnProperty.call(val, '__proto__')
      ) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'Forbidden key — potential prototype pollution',
          path: ['__proto__'],
        });
      }
    })
    .pipe(z.record(safeKey, valueSchema));
}


/**
 * Change type enumeration for proposed changes
 */
export const ChangeTypeSchema = z.enum([
  'file_create',
  'file_update',
  'file_delete',
  'config_update',
  'dependency_add',
  'dependency_remove',
  'dependency_update',
  'script_execute',
]);

/**
 * Individual change object schema
 */
export const ChangeSchema = z.object({
  type: ChangeTypeSchema,
  path: z.string().describe('File or resource path affected by the change'),
  description: z.string().describe('Human-readable description of the change'),
  content: z.string().optional().describe('New content for file changes'),
  diff: z.string().optional().describe('Unified diff for updates'),
  metadata: safeRecord(z.unknown()).optional().describe('Additional change-specific metadata'),
});

/**
 * Provenance information for audit trail
 */
export const ProvenanceSchema = z.object({
  timestamp: z.string().datetime().describe('ISO 8601 timestamp of plan creation'),
  author: z.string().optional().describe('User or system that created the plan'),
  source: z.enum(['cli', 'api', 'automation', 'manual']).describe('Origin of the plan'),
  version: z.string().describe('Version of the tool that created the plan'),
  repository: z.string().optional().describe('Repository URL or identifier'),
  branch: z.string().optional().describe('Git branch name'),
  commit: z.string().optional().describe('Git commit hash'),
});

/**
 * Validation requirements and checks
 */
export const ValidationSchema = z.object({
  required_checks: z.array(z.string()).default([]).describe('List of required validation checks'),
  policy_version: z.string().optional().describe('Version of the policy bundle to use'),
  skip_checks: z.array(z.string()).default([]).describe('Checks to skip for this plan'),
  custom_rules: safeRecord(z.unknown()).optional().describe('Custom validation rules'),
});

/**
 * Evidence entry for gate results
 */
export const EvidenceEntrySchema = z.object({
  check: z.string().describe('Name of the check performed'),
  status: z.enum(['passed', 'failed', 'skipped', 'warning']).describe('Result status'),
  timestamp: z.string().datetime().describe('When the check was performed'),
  details: safeRecord(z.unknown()).optional().describe('Detailed check results'),
  message: z.string().optional().describe('Human-readable result message'),
});

/**
 * Evidence bundle containing all gate results
 */
export const EvidenceSchema = z.object({
  gate_version: z.string().describe('Version of the gate that ran'),
  timestamp: z.string().datetime().describe('When the gate was executed'),
  overall_status: z.enum(['passed', 'failed', 'partial']).describe('Overall gate result'),
  checks: z.array(EvidenceEntrySchema).describe('Individual check results'),
  summary: z.string().optional().describe('Summary of gate execution'),
  artifacts: safeRecord(z.string()).optional().describe('Links or references to artifacts'),
});

/**
 * Approval information
 */
export const ApprovalSchema = z.object({
  approved: z.boolean().describe('Whether the plan is approved'),
  approved_by: z.string().optional().describe('User who approved the plan'),
  approved_at: z.string().datetime().optional().describe('When the plan was approved'),
  approval_notes: z.string().optional().describe('Notes or comments on approval'),
});

/**
 * Execution result for apply/rollback operations
 */
export const ExecutionResultSchema = z.object({
  operation: z.enum(['apply', 'rollback', 'dry-run']).describe('Type of operation'),
  status: z.enum(['success', 'failed', 'partial']).describe('Execution status'),
  timestamp: z.string().datetime().describe('When the operation was performed'),
  executed_by: z.string().optional().describe('User who executed the operation'),
  changes_applied: z.array(z.string()).optional().describe('List of successfully applied changes'),
  changes_failed: z.array(z.string()).optional().describe('List of failed changes'),
  rollback_point: z.string().optional().describe('Snapshot ID for rollback'),
  logs: z.array(z.string()).optional().describe('Execution logs'),
});

/**
 * Main APS Plan Schema with branding for type safety
 */
export const APSPlanSchema = z
  .object({
    // Identification
    id: z
      .string()
      .regex(/^aps-[a-f0-9]{8}(?:[a-f0-9]{8})?$/)
      .describe('Unique plan identifier'),
    hash: z
      .string()
      .regex(/^[a-f0-9]{64}$/)
      .describe('SHA-256 hash of the plan content'),

    // Core content
    intent: z
      .string()
      .min(10, 'Intent must be at least 10 characters')
      .max(500, 'Intent must not exceed 500 characters')
      .describe('Human-readable description of what this plan intends to achieve'),

    // Schema version using literal type
    schema_version: z.literal(SCHEMA_VERSION).describe('Version of the APS schema'),

    // Plan details
    proposed_changes: z.array(ChangeSchema).describe('List of changes this plan will make'),

    // Metadata
    provenance: ProvenanceSchema.describe('Information about plan creation'),
    validations: ValidationSchema.describe('Validation requirements for this plan'),

    // Optional fields that get added during lifecycle
    evidence: z
      .array(EvidenceSchema)
      .optional()
      .describe('Evidence from gate executions (immutable, append-only)'),
    approval: ApprovalSchema.optional().describe('Approval information'),
    executions: z.array(ExecutionResultSchema).optional().describe('History of plan executions'),

    // Additional metadata
    tags: z.array(z.string()).optional().describe('Tags for categorization and filtering'),
    metadata: safeRecord(z.unknown()).optional().describe('Additional plan-specific metadata'),
  })
  .strict(); // Reject unknown properties
// .brand<'APSPlan'>(); // Brand for nominal typing - disabled for easier testing

/**
 * Type exports - automatically inferred from Zod schemas
 */
export type ChangeType = z.infer<typeof ChangeTypeSchema>;
export type Change = z.infer<typeof ChangeSchema>;
export type Provenance = z.infer<typeof ProvenanceSchema>;
export type Validation = z.infer<typeof ValidationSchema>;
export type EvidenceEntry = z.infer<typeof EvidenceEntrySchema>;
export type Evidence = z.infer<typeof EvidenceSchema>;
export type Approval = z.infer<typeof ApprovalSchema>;
export type ExecutionResult = z.infer<typeof ExecutionResultSchema>;
export type APSPlan = z.infer<typeof APSPlanSchema>;

/**
 * Validation result type for consistent error handling
 */
export interface SchemaValidationResult {
  success: boolean;
  data?: APSPlan;
  errors?: Array<{
    path: string;
    message: string;
    code?: string;
  }>;
}

/**
 * Parse and validate a plan with user-friendly error formatting
 */
export function validatePlan(data: unknown): SchemaValidationResult {
  const result = APSPlanSchema.safeParse(data);

  if (result.success) {
    return {
      success: true,
      data: result.data,
    };
  }

  // Format Zod errors for better user experience
  const errors = result.error.issues.map((issue) => ({
    path: issue.path.join('.'),
    message: issue.message,
    code: issue.code,
  }));

  return {
    success: false,
    errors,
  };
}

/**
 * Create a new plan with defaults
 */
export function createPlan(params: {
  id: string;
  intent: string;
  provenance: Provenance;
  changes?: Change[];
  validations?: Validation;
}): Omit<APSPlan, 'hash'> {
  return {
    id: params.id,
    intent: params.intent,
    schema_version: SCHEMA_VERSION,
    proposed_changes: params.changes || [],
    provenance: params.provenance,
    validations: params.validations || {
      required_checks: ['lint', 'test', 'coverage', 'secrets'],
      skip_checks: [],
    },
  } as Omit<APSPlan, 'hash'>;
}

/**
 * Schema version for external reference
 */
export const APS_SCHEMA_VERSION = SCHEMA_VERSION;
