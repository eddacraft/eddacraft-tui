/**
 * Memory Fixtures (STACK-010)
 *
 * Factory functions for creating valid MemoryObject test fixtures.
 * All fixtures pass Zod validation.
 *
 * @module @anvil/edda-stack/testing/fixtures/memories
 */

import { v4 as uuidv4 } from 'uuid';
import type {
  MemoryObject,
  MemoryType,
  MemoryStatus,
  MemoryContext,
  Evolution,
} from '../../contracts/edda-memory.js';
import { MemoryObjectSchema, MEMORY_SCHEMA_VERSION } from '../../contracts/edda-memory.js';
import type { MemoryId, Timestamp } from '../../contracts/index.js';
import type { EddaConfidenceLevel } from '../../contracts/confidence.js';
import type { ProvenanceChain, Attribution } from '../../contracts/provenance.js';
import { createMemoryId, createSessionId } from '../../contracts/identifiers.js';
import { now } from '../../contracts/temporal.js';

// =============================================================================
// Types
// =============================================================================

/**
 * Override options for memory fixtures
 */
export interface MemoryFixtureOverrides {
  id?: MemoryId;
  type?: MemoryType;
  status?: MemoryStatus;
  schema_version?: number;
  statement?: string;
  context?: Partial<MemoryContext>;
  confidence?: EddaConfidenceLevel;
  confidence_rationale?: string;
  provenance?: Partial<ProvenanceChain>;
  attribution?: Partial<Attribution>;
  evolution?: Partial<Evolution>;
  created_at?: Timestamp;
  updated_at?: Timestamp;
  metadata?: Record<string, unknown>;
}

// =============================================================================
// Generic Factory
// =============================================================================

/**
 * Create a valid memory fixture
 */
export function createMemoryFixture(
  type: MemoryType,
  overrides: MemoryFixtureOverrides = {}
): MemoryObject {
  const id = overrides.id ?? createMemoryId(uuidv4());
  const sessionId = createSessionId(uuidv4());
  const observationId = uuidv4();
  const createdAt = overrides.created_at ?? now();

  const baseMemory = {
    id,
    type: overrides.type ?? type,
    status: overrides.status ?? 'active',
    schema_version: overrides.schema_version ?? MEMORY_SCHEMA_VERSION,
    statement: overrides.statement ?? getDefaultStatement(type),
    context: {
      when: getDefaultWhen(type),
      why: getDefaultWhy(type),
      conditions: [],
      tags: getDefaultTags(type),
      ...overrides.context,
    },
    confidence: overrides.confidence ?? 'medium',
    confidence_rationale: overrides.confidence_rationale,
    provenance: {
      kindling_sources: [
        {
          observation_id: observationId,
          session_id: sessionId,
          kind: 'gate_evaluated',
          timestamp: createdAt,
        },
      ],
      source_sessions: [sessionId],
      ...overrides.provenance,
    },
    attribution: {
      actor: 'test-user@example.com',
      timestamp: createdAt,
      method: 'cli_command' as const,
      reason: 'Test fixture creation',
      ...overrides.attribution,
    },
    evolution: overrides.evolution ?? {},
    created_at: createdAt,
    ...(overrides.updated_at && { updated_at: overrides.updated_at }),
    ...(overrides.metadata && { metadata: overrides.metadata }),
  };

  // Validate and return
  return MemoryObjectSchema.parse(baseMemory);
}

// =============================================================================
// Type-Specific Factories
// =============================================================================

/**
 * Create a valid decision memory
 */
export function createValidDecisionMemory(overrides: MemoryFixtureOverrides = {}): MemoryObject {
  return createMemoryFixture('decision', {
    statement: 'We use pnpm as our package manager for all projects',
    context: {
      when: '2024-01-15',
      why: 'pnpm offers better disk space efficiency and faster installs',
      conditions: ['All Node.js projects', 'Monorepo development'],
      scope: 'Organization-wide',
      tags: ['tooling', 'pnpm', 'package-manager'],
    },
    confidence: 'high',
    confidence_rationale: 'Explicit team decision with documented benefits',
    metadata: {
      decision_point: 'Package manager selection',
      alternatives_considered: ['npm', 'yarn', 'pnpm'],
      outcome: 'Successful migration to pnpm',
      reversible: true,
    },
    ...overrides,
  });
}

/**
 * Create a valid pattern memory
 */
export function createValidPatternMemory(overrides: MemoryFixtureOverrides = {}): MemoryObject {
  return createMemoryFixture('pattern', {
    statement: 'Use the Repository pattern for all data access operations',
    context: {
      when: '2024-01-10',
      why: 'Provides consistent abstraction for database operations',
      conditions: ['Data access layers', 'Service implementations'],
      tags: ['pattern', 'repository', 'data-access'],
    },
    confidence: 'medium',
    confidence_rationale: 'Observed consistently across multiple services',
    metadata: {
      pattern_name: 'Repository Pattern',
      applies_to: ['services', 'data-layer'],
      anti_pattern: false,
    },
    ...overrides,
  });
}

/**
 * Create a valid constraint memory
 */
export function createValidConstraintMemory(overrides: MemoryFixtureOverrides = {}): MemoryObject {
  return createMemoryFixture('constraint', {
    statement: 'API response payloads must not exceed 1MB',
    context: {
      when: '2024-01-05',
      why: 'Gateway has a 1MB limit; larger payloads cause 413 errors',
      conditions: ['All API endpoints', 'REST and GraphQL'],
      scope: 'API Gateway',
      tags: ['constraint', 'api', 'size-limit'],
    },
    confidence: 'high',
    confidence_rationale: 'Hard limit enforced by infrastructure',
    metadata: {
      constraint_type: 'technical',
      enforcement: 'hard',
      workaround: 'Use pagination or streaming for large datasets',
    },
    ...overrides,
  });
}

/**
 * Create a valid warning memory
 */
export function createValidWarningMemory(overrides: MemoryFixtureOverrides = {}): MemoryObject {
  return createMemoryFixture('warning', {
    statement: 'Avoid using setTimeout for polling; use proper event-driven patterns',
    context: {
      when: '2024-01-08',
      why: 'setTimeout-based polling causes memory leaks and race conditions',
      conditions: ['Async operations', 'Event handling'],
      tags: ['warning', 'async', 'polling'],
    },
    confidence: 'high',
    confidence_rationale: 'Multiple production incidents traced to this pattern',
    metadata: {
      severity: 'high',
      affected_areas: ['frontend', 'background-jobs'],
      mitigation: 'Use event emitters or message queues instead',
    },
    ...overrides,
  });
}

/**
 * Create a valid doctrine memory
 */
export function createValidDoctrineMemory(overrides: MemoryFixtureOverrides = {}): MemoryObject {
  return createMemoryFixture('doctrine', {
    statement: 'Code review is mandatory for all changes to production code',
    context: {
      when: '2024-01-01',
      why: 'Quality assurance and knowledge sharing across the team',
      conditions: ['All production code changes', 'All team members'],
      scope: 'Organization-wide',
      tags: ['doctrine', 'code-review', 'process'],
    },
    confidence: 'high',
    confidence_rationale: 'Established organizational policy',
    metadata: {
      principle: 'All code must be reviewed by at least one other developer',
      source: 'Engineering handbook',
      exceptions: ['Emergency hotfixes (require post-hoc review)'],
    },
    ...overrides,
  });
}

/**
 * Create a valid lesson memory
 */
export function createValidLessonMemory(overrides: MemoryFixtureOverrides = {}): MemoryObject {
  return createMemoryFixture('lesson', {
    statement: 'Always add database indexes before deploying queries on large tables',
    context: {
      when: '2024-01-12',
      why: 'Query performance degraded significantly without proper indexes',
      conditions: ['Database queries', 'Large tables (>1M rows)'],
      tags: ['lesson', 'database', 'performance'],
    },
    confidence: 'high',
    confidence_rationale: 'Learned from production incident',
    metadata: {
      lesson_type: 'failure',
      applicable_to: ['database migrations', 'query optimization'],
      key_takeaway: 'Test query performance with production-like data volumes',
    },
    ...overrides,
  });
}

// =============================================================================
// Status Variant Factories
// =============================================================================

/**
 * Create an active memory (default)
 */
export function createActiveMemory(
  type: MemoryType = 'decision',
  overrides: MemoryFixtureOverrides = {}
): MemoryObject {
  return createMemoryFixture(type, {
    status: 'active',
    ...overrides,
  });
}

/**
 * Create a superseded memory
 */
export function createSupersededMemory(
  type: MemoryType = 'decision',
  supersededById?: MemoryId,
  overrides: MemoryFixtureOverrides = {}
): MemoryObject {
  const retiredAt = now();
  const newMemoryId = supersededById ?? createMemoryId(uuidv4());

  return createMemoryFixture(type, {
    status: 'superseded',
    evolution: {
      superseded_by: newMemoryId,
      retired_at: retiredAt,
      retired_reason: 'Replaced with updated guidance',
      retired_by: 'user@example.com',
    },
    ...overrides,
  });
}

/**
 * Create a retired memory
 */
export function createRetiredMemory(
  type: MemoryType = 'decision',
  overrides: MemoryFixtureOverrides = {}
): MemoryObject {
  const retiredAt = now();

  return createMemoryFixture(type, {
    status: 'retired',
    evolution: {
      retired_at: retiredAt,
      retired_reason: 'No longer applicable',
      retired_by: 'user@example.com',
    },
    ...overrides,
  });
}

// =============================================================================
// Evolution Chain Factories
// =============================================================================

/**
 * Create a memory that supersedes another
 */
export function createSupersedesMemory(
  oldMemoryId: MemoryId,
  overrides: MemoryFixtureOverrides = {}
): MemoryObject {
  return createMemoryFixture('decision', {
    evolution: {
      supersedes: [oldMemoryId],
    },
    ...overrides,
  });
}

/**
 * Create a complete evolution chain (old -> new)
 */
export function createEvolutionChain(type: MemoryType = 'decision'): {
  oldMemory: MemoryObject;
  newMemory: MemoryObject;
} {
  const oldMemoryId = createMemoryId(uuidv4());
  const newMemoryId = createMemoryId(uuidv4());
  const baseTimestamp = new Date('2024-01-01T10:00:00.000Z');

  const oldMemory = createMemoryFixture(type, {
    id: oldMemoryId,
    status: 'superseded',
    statement: 'Original guidance (now superseded)',
    created_at: baseTimestamp.toISOString() as Timestamp,
    evolution: {
      superseded_by: newMemoryId,
      retired_at: new Date(baseTimestamp.getTime() + 2592000000).toISOString() as Timestamp,
      retired_reason: 'Updated with new information',
      retired_by: 'user@example.com',
    },
  });

  const newMemory = createMemoryFixture(type, {
    id: newMemoryId,
    status: 'active',
    statement: 'Updated guidance (supersedes original)',
    created_at: new Date(baseTimestamp.getTime() + 2592000000).toISOString() as Timestamp,
    evolution: {
      supersedes: [oldMemoryId],
    },
  });

  return { oldMemory, newMemory };
}

/**
 * Create a multi-level evolution chain (v1 -> v2 -> v3)
 */
export function createMultiLevelEvolutionChain(): MemoryObject[] {
  const v1Id = createMemoryId(uuidv4());
  const v2Id = createMemoryId(uuidv4());
  const v3Id = createMemoryId(uuidv4());
  const baseTimestamp = new Date('2024-01-01T10:00:00.000Z');

  const v1 = createMemoryFixture('decision', {
    id: v1Id,
    status: 'superseded',
    statement: 'Version 1: Use npm',
    created_at: baseTimestamp.toISOString() as Timestamp,
    evolution: {
      superseded_by: v2Id,
      retired_at: new Date(baseTimestamp.getTime() + 2592000000).toISOString() as Timestamp,
      retired_reason: 'Migrating to yarn',
      retired_by: 'user@example.com',
    },
  });

  const v2 = createMemoryFixture('decision', {
    id: v2Id,
    status: 'superseded',
    statement: 'Version 2: Use yarn',
    created_at: new Date(baseTimestamp.getTime() + 2592000000).toISOString() as Timestamp,
    evolution: {
      supersedes: [v1Id],
      superseded_by: v3Id,
      retired_at: new Date(baseTimestamp.getTime() + 5184000000).toISOString() as Timestamp,
      retired_reason: 'Migrating to pnpm',
      retired_by: 'user@example.com',
    },
  });

  const v3 = createMemoryFixture('decision', {
    id: v3Id,
    status: 'active',
    statement: 'Version 3: Use pnpm',
    created_at: new Date(baseTimestamp.getTime() + 5184000000).toISOString() as Timestamp,
    evolution: {
      supersedes: [v2Id],
    },
  });

  return [v1, v2, v3];
}

// =============================================================================
// Batch Factories
// =============================================================================

/**
 * Create a set of memories of all types
 */
export function createMemoriesOfAllTypes(): MemoryObject[] {
  return [
    createValidDecisionMemory(),
    createValidPatternMemory(),
    createValidConstraintMemory(),
    createValidWarningMemory(),
    createValidDoctrineMemory(),
    createValidLessonMemory(),
  ];
}

/**
 * Create a set of memories with all statuses
 */
export function createMemoriesOfAllStatuses(): MemoryObject[] {
  return [
    createActiveMemory('decision'),
    createSupersededMemory('pattern'),
    createRetiredMemory('warning'),
  ];
}

// =============================================================================
// Helper Functions
// =============================================================================

function getDefaultStatement(type: MemoryType): string {
  const statements: Record<MemoryType, string> = {
    decision: 'A decision was made regarding project direction',
    pattern: 'A recurring pattern should be followed',
    constraint: 'A constraint or limitation must be respected',
    warning: 'A warning about potential issues',
    doctrine: 'An organizational principle to follow',
    lesson: 'A lesson learned from experience',
  };
  return statements[type];
}

function getDefaultWhen(_type: MemoryType): string {
  return '2024-01-15';
}

function getDefaultWhy(type: MemoryType): string {
  const whys: Record<MemoryType, string> = {
    decision: 'This decision impacts project architecture and should be documented',
    pattern: 'This pattern improves code consistency and maintainability',
    constraint: 'This constraint affects what can and cannot be done',
    warning: 'This warning helps avoid known pitfalls',
    doctrine: 'This doctrine guides organizational behaviour',
    lesson: 'This lesson helps avoid repeating past mistakes',
  };
  return whys[type];
}

function getDefaultTags(type: MemoryType): string[] {
  const tags: Record<MemoryType, string[]> = {
    decision: ['decision', 'architecture'],
    pattern: ['pattern', 'code'],
    constraint: ['constraint', 'limitation'],
    warning: ['warning', 'caution'],
    doctrine: ['doctrine', 'policy'],
    lesson: ['lesson', 'learning'],
  };
  return tags[type];
}
