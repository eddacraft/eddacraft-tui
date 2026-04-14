/**
 * OpenAPI 3.1 Spec Generator (KINDLING-019)
 *
 * Generates an OpenAPI 3.1 specification from the Kindling query contract.
 * Uses manual schema extraction (no external dependencies like zod-to-json-schema).
 *
 * Run: npx tsx scripts/generate-openapi.ts
 * Output: openapi.json
 *
 * Includes all 4 query endpoints:
 *   GET /sessions/{id}
 *   GET /plans/{id}
 *   GET /gates/{id}
 *   GET /actions/{id}
 *
 * @see src/query-contract.ts for the source Zod schemas
 */

import { writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

// =============================================================================
// Output Path
// =============================================================================

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const OUTPUT_PATH = resolve(__dirname, '..', 'openapi.json');

// =============================================================================
// Shared Schema Definitions
// =============================================================================

const observationKindEnum = [
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
];

const provenanceLinkSchema = {
  type: 'object' as const,
  required: ['type', 'entity_type', 'entity_id', 'timestamp'],
  properties: {
    type: {
      type: 'string' as const,
      enum: ['caused_by', 'governed_by', 'approved_by', 'linked_to'],
      description: 'Link type',
    },
    entity_type: {
      type: 'string' as const,
      enum: ['session', 'plan', 'gate', 'action', 'human'],
      description: 'Target entity type',
    },
    entity_id: {
      type: 'string' as const,
      description: 'Target entity ID',
    },
    timestamp: {
      type: 'string' as const,
      format: 'date-time' as const,
      description: 'When link was created',
    },
  },
};

const observationSchema = {
  type: 'object' as const,
  required: ['id', 'kind', 'timestamp', 'session_id', 'provenance', 'payload'],
  properties: {
    id: {
      type: 'string' as const,
      format: 'uuid' as const,
      description: 'Observation ID',
    },
    kind: {
      type: 'string' as const,
      enum: observationKindEnum,
      description: 'Observation kind',
    },
    timestamp: {
      type: 'string' as const,
      format: 'date-time' as const,
      description: 'When observation was recorded',
    },
    session_id: {
      type: 'string' as const,
      format: 'uuid' as const,
      description: 'Session this observation belongs to',
    },
    provenance: {
      type: 'array' as const,
      items: { $ref: '#/components/schemas/ProvenanceLink' },
      description: 'Explicit links to other entities',
    },
    payload: {
      type: 'object' as const,
      additionalProperties: true,
      description: 'Observation-specific data (raw facts only)',
    },
  },
};

const queryResponseMetadataSchema = {
  type: 'object' as const,
  required: ['query_id', 'executed_at', 'contract_version', 'result_count', 'truncated'],
  properties: {
    query_id: {
      type: 'string' as const,
      format: 'uuid' as const,
      description: 'Unique query identifier (for debugging)',
    },
    executed_at: {
      type: 'string' as const,
      format: 'date-time' as const,
      description: 'When query was executed',
    },
    contract_version: {
      type: 'string' as const,
      description: 'Query contract version used',
    },
    result_count: {
      type: 'integer' as const,
      minimum: 0,
      description: 'Number of observations returned',
    },
    truncated: {
      type: 'boolean' as const,
      description: 'Whether results were truncated (hit limits)',
    },
    truncation_reason: {
      type: 'string' as const,
      enum: ['max_results', 'max_payload_bytes', 'none'],
      description: 'Why truncation occurred',
    },
  },
};

const queryResponseSchema = {
  type: 'object' as const,
  required: ['metadata', 'observations'],
  properties: {
    metadata: { $ref: '#/components/schemas/QueryResponseMetadata' },
    observations: {
      type: 'array' as const,
      items: { $ref: '#/components/schemas/Observation' },
      description: 'Ordered observations (facts only)',
    },
  },
};

const errorResponseSchema = {
  type: 'object' as const,
  required: ['error', 'message'],
  properties: {
    error: {
      type: 'string' as const,
      description: 'Error code',
    },
    message: {
      type: 'string' as const,
      description: 'Human-readable error message',
    },
  },
};

// =============================================================================
// Shared Query Parameters
// =============================================================================

const commonQueryParams = [
  {
    name: 'shape',
    in: 'query' as const,
    required: false,
    description: 'Result structure',
    schema: {
      type: 'string' as const,
      enum: ['timeline', 'list', 'entity'],
      default: 'entity',
    },
  },
  {
    name: 'format',
    in: 'query' as const,
    required: false,
    description: 'Output format',
    schema: {
      type: 'string' as const,
      enum: ['json', 'text'],
      default: 'json',
    },
  },
  {
    name: 'max_results',
    in: 'query' as const,
    required: false,
    description: 'Maximum observations to return (anti-vacuum-cleaner)',
    schema: {
      type: 'integer' as const,
      minimum: 1,
      maximum: 1000,
      default: 100,
    },
  },
  {
    name: 'max_payload_bytes',
    in: 'query' as const,
    required: false,
    description: 'Maximum total payload size in bytes',
    schema: {
      type: 'integer' as const,
      minimum: 1,
      maximum: 10485760,
      default: 1048576,
    },
  },
  {
    name: 'time_after',
    in: 'query' as const,
    required: false,
    description: 'Include observations after this time (ISO8601)',
    schema: {
      type: 'string' as const,
      format: 'date-time' as const,
    },
  },
  {
    name: 'time_before',
    in: 'query' as const,
    required: false,
    description: 'Include observations before this time (ISO8601)',
    schema: {
      type: 'string' as const,
      format: 'date-time' as const,
    },
  },
];

// =============================================================================
// OpenAPI Specification
// =============================================================================

const openApiSpec = {
  openapi: '3.1.0',
  info: {
    title: 'Kindling Query API',
    version: '1.0.0',
    description: [
      'Read-only, bounded query API for Kindling observations.',
      '',
      '**Governing Rule:** Kindling is a system of record, not a reasoning engine.',
      "Queries may retrieve facts; interpretation is the caller's responsibility.",
      '',
      '**Read-Only Enforcement:** No write, update, delete, annotate, tag, learn,',
      'embed, or infer operations exist. If user AI wants memory, it must bring its own store.',
      '',
      '**Anti-Vacuum-Cleaner:** All queries require explicit scope + ID.',
      'No free-text search. No global scans. No cross-project reads.',
      'Results are bounded by max_results (default 100, max 1000) and',
      'max_payload_bytes (default 1MB, max 10MB).',
    ].join('\n'),
    contact: {
      name: 'eddacraft',
    },
    license: {
      name: 'Proprietary',
    },
  },
  servers: [
    {
      url: 'http://localhost:3000/api/v1/kindling',
      description: 'Local development (Kindling runs locally)',
    },
  ],
  tags: [
    {
      name: 'Sessions',
      description: 'Session queries - "What happened in this run?"',
    },
    {
      name: 'Plans',
      description: 'Plan queries - "What happened because of this plan?" (only cross-session read)',
    },
    {
      name: 'Gates',
      description: 'Gate queries - "Why did this gate pass/fail?"',
    },
    {
      name: 'Actions',
      description: 'Action queries - "What exactly did this action do?"',
    },
  ],
  paths: {
    '/sessions/{session_id}': {
      get: {
        operationId: 'getSession',
        summary: 'Query session observations',
        description: [
          'Returns ordered observations grouped by phase (plan / gate / action / outcome).',
          'Raw payloads only, no summaries.',
          '',
          'CLI equivalent: `anvil run show <run_id> --json`',
        ].join('\n'),
        tags: ['Sessions'],
        parameters: [
          {
            name: 'session_id',
            in: 'path' as const,
            required: true,
            description: 'Session/run ID (UUID)',
            schema: {
              type: 'string' as const,
              format: 'uuid' as const,
            },
          },
          {
            name: 'include_phases',
            in: 'query' as const,
            required: false,
            description: 'Filter to specific phases',
            schema: {
              type: 'array' as const,
              items: {
                type: 'string' as const,
                enum: ['plan', 'gate', 'action', 'outcome', 'error'],
              },
            },
          },
          ...commonQueryParams,
        ],
        responses: {
          '200': {
            description: 'Session observations returned successfully',
            content: {
              'application/json': {
                schema: { $ref: '#/components/schemas/QueryResponse' },
              },
            },
          },
          '400': {
            description: 'Invalid query parameters',
            content: {
              'application/json': {
                schema: { $ref: '#/components/schemas/ErrorResponse' },
              },
            },
          },
          '404': {
            description: 'Session not found',
            content: {
              'application/json': {
                schema: { $ref: '#/components/schemas/ErrorResponse' },
              },
            },
          },
        },
      },
    },
    '/plans/{plan_id}': {
      get: {
        operationId: 'getPlan',
        summary: 'Query plan observations',
        description: [
          'Returns plan metadata, versions, and linked executions.',
          'This is the ONLY cross-session read allowed, via explicit plan_id.',
          '',
          'CLI equivalent: `anvil plan trace <plan_id> --json`',
        ].join('\n'),
        tags: ['Plans'],
        parameters: [
          {
            name: 'plan_id',
            in: 'path' as const,
            required: true,
            description: 'Plan ID',
            schema: {
              type: 'string' as const,
            },
          },
          {
            name: 'include_executions',
            in: 'query' as const,
            required: false,
            description: 'Include linked execution run IDs',
            schema: {
              type: 'boolean' as const,
              default: true,
            },
          },
          {
            name: 'include_versions',
            in: 'query' as const,
            required: false,
            description: 'Include plan version history',
            schema: {
              type: 'boolean' as const,
              default: true,
            },
          },
          ...commonQueryParams,
        ],
        responses: {
          '200': {
            description: 'Plan observations returned successfully',
            content: {
              'application/json': {
                schema: { $ref: '#/components/schemas/QueryResponse' },
              },
            },
          },
          '400': {
            description: 'Invalid query parameters',
            content: {
              'application/json': {
                schema: { $ref: '#/components/schemas/ErrorResponse' },
              },
            },
          },
          '404': {
            description: 'Plan not found',
            content: {
              'application/json': {
                schema: { $ref: '#/components/schemas/ErrorResponse' },
              },
            },
          },
        },
      },
    },
    '/gates/{gate_eval_id}': {
      get: {
        operationId: 'getGate',
        summary: 'Query gate evaluation',
        description: [
          'Returns gate evaluation details with rule IDs, inputs (sanitised), and outcomes.',
          'No prose. No explanation layer.',
          '',
          'CLI equivalent: `anvil gate show <gate_eval_id> --json`',
        ].join('\n'),
        tags: ['Gates'],
        parameters: [
          {
            name: 'gate_eval_id',
            in: 'path' as const,
            required: true,
            description: 'Gate evaluation ID',
            schema: {
              type: 'string' as const,
            },
          },
          ...commonQueryParams,
        ],
        responses: {
          '200': {
            description: 'Gate evaluation returned successfully',
            content: {
              'application/json': {
                schema: { $ref: '#/components/schemas/QueryResponse' },
              },
            },
          },
          '400': {
            description: 'Invalid query parameters',
            content: {
              'application/json': {
                schema: { $ref: '#/components/schemas/ErrorResponse' },
              },
            },
          },
          '404': {
            description: 'Gate evaluation not found',
            content: {
              'application/json': {
                schema: { $ref: '#/components/schemas/ErrorResponse' },
              },
            },
          },
        },
      },
    },
    '/actions/{action_id}': {
      get: {
        operationId: 'getAction',
        summary: 'Query action details',
        description: [
          'Returns action details with redacted command, environment, and linked governance.',
          'This is the atomic unit of accountability.',
          '',
          'CLI equivalent: `anvil action show <action_id> --json`',
        ].join('\n'),
        tags: ['Actions'],
        parameters: [
          {
            name: 'action_id',
            in: 'path' as const,
            required: true,
            description: 'Action ID',
            schema: {
              type: 'string' as const,
            },
          },
          {
            name: 'include_approval_chain',
            in: 'query' as const,
            required: false,
            description: 'Include approval requirements and state',
            schema: {
              type: 'boolean' as const,
              default: true,
            },
          },
          ...commonQueryParams,
        ],
        responses: {
          '200': {
            description: 'Action details returned successfully',
            content: {
              'application/json': {
                schema: { $ref: '#/components/schemas/QueryResponse' },
              },
            },
          },
          '400': {
            description: 'Invalid query parameters',
            content: {
              'application/json': {
                schema: { $ref: '#/components/schemas/ErrorResponse' },
              },
            },
          },
          '404': {
            description: 'Action not found',
            content: {
              'application/json': {
                schema: { $ref: '#/components/schemas/ErrorResponse' },
              },
            },
          },
        },
      },
    },
  },
  components: {
    schemas: {
      ProvenanceLink: provenanceLinkSchema,
      Observation: observationSchema,
      QueryResponseMetadata: queryResponseMetadataSchema,
      QueryResponse: queryResponseSchema,
      ErrorResponse: errorResponseSchema,
    },
  },
};

// =============================================================================
// Generate
// =============================================================================

function generate(): void {
  const json = JSON.stringify(openApiSpec, null, 2);

  writeFileSync(OUTPUT_PATH, json + '\n', 'utf-8');
  console.log(`OpenAPI 3.1 spec written to: ${OUTPUT_PATH}`);
  console.log(`  Endpoints: ${Object.keys(openApiSpec.paths).length}`);
  console.log(`  Schemas: ${Object.keys(openApiSpec.components.schemas).length}`);
}

generate();
