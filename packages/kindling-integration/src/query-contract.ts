/**
 * Kindling Query Contract (v1)
 *
 * Defines the read-only, bounded query surface for Kindling observations.
 * This is a system of record, not a reasoning engine.
 *
 * GOVERNING RULE:
 * Queries may retrieve facts; interpretation is the caller's responsibility.
 * User-supplied AI may read, but may not mutate, infer, or generalise via Kindling.
 *
 * @see plans/modules/kindling-integration.aps.md for integration plan
 */

import { z } from 'zod';

// =============================================================================
// Schema Version
// =============================================================================

export const KINDLING_QUERY_CONTRACT_VERSION = '1.0.0';

// =============================================================================
// Query Scopes (Mandatory Boundary)
// =============================================================================

/**
 * Every query must specify exactly one scope.
 * No free-text search. No global scans. No cross-project reads.
 */
export const QueryScopeSchema = z.enum([
  'session', // "What happened in this run?"
  'plan', // "What happened because of this plan?"
  'gate', // "Why did this gate pass/fail?"
  'action', // "What exactly did this action do?"
]);

export type QueryScope = z.infer<typeof QueryScopeSchema>;

// =============================================================================
// Result Shape
// =============================================================================

/**
 * How results should be structured
 */
export const ResultShapeSchema = z.enum([
  'timeline', // Ordered observations grouped by phase
  'list', // Flat list of observations
  'entity', // Single entity with metadata
]);

export type ResultShape = z.infer<typeof ResultShapeSchema>;

// =============================================================================
// Output Format
// =============================================================================

/**
 * Result serialisation format
 */
export const OutputFormatSchema = z.enum([
  'json', // Machine-readable
  'text', // Human-readable (for CLI)
]);

export type OutputFormat = z.infer<typeof OutputFormatSchema>;

// =============================================================================
// Query Request (Base)
// =============================================================================

/**
 * Base query request with mandatory constraints
 */
export const QueryRequestBaseSchema = z.object({
  scope: QueryScopeSchema.describe('Query scope (mandatory)'),
  shape: ResultShapeSchema.describe('Result structure (mandatory)'),
  format: OutputFormatSchema.default('json').describe('Output format'),

  // Time bounds (optional but encouraged)
  time_after: z.string().datetime().optional().describe('Include observations after this time'),
  time_before: z.string().datetime().optional().describe('Include observations before this time'),

  // Result limits (anti-vacuum-cleaner)
  max_results: z
    .number()
    .int()
    .positive()
    .max(1000)
    .default(100)
    .describe('Maximum observations to return'),
  max_payload_bytes: z
    .number()
    .int()
    .positive()
    .max(10 * 1024 * 1024) // 10MB
    .default(1024 * 1024) // 1MB
    .describe('Maximum total payload size'),
});

export type QueryRequestBase = z.infer<typeof QueryRequestBaseSchema>;

// =============================================================================
// Session Query
// =============================================================================

/**
 * Query: "What happened in this run?"
 *
 * Returns ordered observations grouped by phase (plan / gate / action / outcome).
 * Raw payloads only, no summaries.
 */
export const SessionQuerySchema = QueryRequestBaseSchema.extend({
  scope: z.literal('session'),
  session_id: z.string().uuid().describe('Session/run ID (mandatory)'),
  include_phases: z
    .array(z.enum(['plan', 'gate', 'action', 'outcome', 'error']))
    .optional()
    .describe('Filter to specific phases'),
});

export type SessionQuery = z.infer<typeof SessionQuerySchema>;

// =============================================================================
// Plan Query
// =============================================================================

/**
 * Query: "What happened because of this plan?"
 *
 * Returns plan metadata, versions, and linked executions.
 * This is the ONLY cross-session read allowed, via explicit plan_id.
 */
export const PlanQuerySchema = QueryRequestBaseSchema.extend({
  scope: z.literal('plan'),
  plan_id: z.string().describe('Plan ID (mandatory)'),
  include_executions: z.boolean().default(true).describe('Include linked execution run IDs'),
  include_versions: z.boolean().default(true).describe('Include plan version history'),
});

export type PlanQuery = z.infer<typeof PlanQuerySchema>;

// =============================================================================
// Gate Query
// =============================================================================

/**
 * Query: "Why did this gate pass/fail?"
 *
 * Returns gate evaluation details with rule IDs, inputs (sanitised), and outcomes.
 * No prose. No explanation layer.
 */
export const GateQuerySchema = QueryRequestBaseSchema.extend({
  scope: z.literal('gate'),
  gate_eval_id: z.string().describe('Gate evaluation ID (mandatory)'),
});

export type GateQuery = z.infer<typeof GateQuerySchema>;

// =============================================================================
// Action Query
// =============================================================================

/**
 * Query: "What exactly did this action do?"
 *
 * Returns action details with redacted command, environment, and linked governance.
 * This is the atomic unit of accountability.
 */
export const ActionQuerySchema = QueryRequestBaseSchema.extend({
  scope: z.literal('action'),
  action_id: z.string().describe('Action ID (mandatory)'),
  include_approval_chain: z
    .boolean()
    .default(true)
    .describe('Include approval requirements and state'),
});

export type ActionQuery = z.infer<typeof ActionQuerySchema>;

// =============================================================================
// Query Request (Union)
// =============================================================================

/**
 * All query types (discriminated union)
 */
export const QueryRequestSchema = z.discriminatedUnion('scope', [
  SessionQuerySchema,
  PlanQuerySchema,
  GateQuerySchema,
  ActionQuerySchema,
]);

export type QueryRequest = z.infer<typeof QueryRequestSchema>;

// =============================================================================
// Query Response (Output Guarantees)
// =============================================================================

/**
 * Standard metadata for all responses
 */
export const QueryResponseMetadataSchema = z.object({
  query_id: z.string().uuid().describe('Unique query identifier (for debugging)'),
  executed_at: z.string().datetime().describe('When query was executed'),
  contract_version: z.string().describe('Query contract version used'),
  result_count: z.number().int().nonnegative().describe('Number of observations returned'),
  truncated: z.boolean().describe('Whether results were truncated (hit limits)'),
  truncation_reason: z
    .enum(['max_results', 'max_payload_bytes', 'none'])
    .optional()
    .describe('Why truncation occurred'),
});

export type QueryResponseMetadata = z.infer<typeof QueryResponseMetadataSchema>;

/**
 * Standard provenance link
 */
export const ProvenanceLinkSchema = z.object({
  type: z.enum(['caused_by', 'governed_by', 'approved_by', 'linked_to']).describe('Link type'),
  entity_type: z
    .enum(['session', 'plan', 'gate', 'action', 'human'])
    .describe('Target entity type'),
  entity_id: z.string().describe('Target entity ID'),
  timestamp: z.string().datetime().describe('When link was created'),
});

export type ProvenanceLink = z.infer<typeof ProvenanceLinkSchema>;

/**
 * Observation (base type for all returned data)
 */
export const ObservationSchema = z.object({
  id: z.string().uuid().describe('Observation ID'),
  kind: z
    .enum([
      'session_start',
      'session_end',
      'plan_created',
      'plan_edited',
      'plan_approved',
      'plan_rejected',
      'gate_evaluated',
      'action_executed',
      'constraint_applied',
      'human_input',
      'error',
    ])
    .describe('Observation kind'),
  timestamp: z.string().datetime().describe('When observation was recorded'),
  session_id: z.string().uuid().describe('Session this observation belongs to'),

  // Provenance (explicit links)
  provenance: z.array(ProvenanceLinkSchema).describe('Explicit links to other entities'),

  // Payload (fact data, no inference)
  payload: z.record(z.unknown()).describe('Observation-specific data (raw facts only)'),
});

export type Observation = z.infer<typeof ObservationSchema>;

/**
 * Query response
 */
export const QueryResponseSchema = z.object({
  metadata: QueryResponseMetadataSchema.describe('Query execution metadata'),
  observations: z.array(ObservationSchema).describe('Ordered observations (facts only)'),
});

export type QueryResponse = z.infer<typeof QueryResponseSchema>;

// =============================================================================
// Output Guarantees (Documented Requirements)
// =============================================================================

/**
 * Every Kindling response guarantees:
 *
 * 1. Stable field names - No field names change between queries
 * 2. Explicit timestamps - Every observation has ISO8601 timestamp
 * 3. Explicit links - Provenance via typed links (caused_by, governed_by, approved_by)
 * 4. No hidden inference - Payload contains only raw facts
 * 5. No reordered history - Observations returned in recorded order
 *
 * This makes Kindling LLM-safe by construction.
 *
 * AI can:
 * - Narrate events
 * - Summarise outcomes
 * - Explain facts
 *
 * But AI will always be explaining facts, not ghosts.
 */

// =============================================================================
// Read-Only Enforcement (Anti-Pattern Markers)
// =============================================================================

/**
 * Operations that MUST NOT exist in the query API:
 *
 * ❌ write()
 * ❌ update()
 * ❌ delete()
 * ❌ annotate()
 * ❌ tag()
 * ❌ learn()
 * ❌ embed()
 * ❌ infer()
 *
 * If user AI wants memory, it must bring its own store.
 */

// =============================================================================
// Explicit Non-Goals (v1 Boundary)
// =============================================================================

/**
 * The following are explicitly OUT OF SCOPE for v1:
 *
 * ❌ Semantic search
 * ❌ Similarity queries
 * ❌ Embeddings
 * ❌ Cross-plan discovery
 * ❌ Learned relevance
 * ❌ Auto-summaries (stored in Kindling)
 * ❌ AI-generated annotations stored in Kindling
 *
 * These belong to Edda / Ember, not Kindling v1.
 */

// =============================================================================
// Validation Utilities
// =============================================================================

/**
 * Validate a query request
 */
export function validateQueryRequest(data: unknown): {
  success: boolean;
  data?: QueryRequest;
  error?: string;
} {
  const result = QueryRequestSchema.safeParse(data);
  if (result.success) {
    return { success: true, data: result.data };
  }
  return { success: false, error: result.error.format()._errors.join(', ') };
}

/**
 * Validate a query response
 */
export function validateQueryResponse(data: unknown): {
  success: boolean;
  data?: QueryResponse;
  error?: string;
} {
  const result = QueryResponseSchema.safeParse(data);
  if (result.success) {
    return { success: true, data: result.data };
  }
  return { success: false, error: result.error.format()._errors.join(', ') };
}

// =============================================================================
// CLI Command Mapping (Human-First, AI-Compatible)
// =============================================================================

/**
 * All queries have CLI equivalents:
 *
 * anvil run show <run_id> --json
 * → SessionQuery { scope: 'session', session_id: run_id, shape: 'timeline' }
 *
 * anvil plan trace <plan_id> --json
 * → PlanQuery { scope: 'plan', plan_id: plan_id, shape: 'entity' }
 *
 * anvil gate show <gate_eval_id> --json
 * → GateQuery { scope: 'gate', gate_eval_id: gate_eval_id, shape: 'entity' }
 *
 * anvil action show <action_id> --json
 * → ActionQuery { scope: 'action', action_id: action_id, shape: 'entity' }
 *
 * The CLI is a thin wrapper over this query surface.
 * That symmetry is intentional.
 */
