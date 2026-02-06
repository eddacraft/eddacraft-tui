/**
 * Multi-Agent Concurrency Types
 *
 * Type definitions for agent registration, locking, and queuing systems
 * to handle concurrent AI agent operations across multiple processes.
 */

import { z } from 'zod';

// ============================================================================
// Agent Identification
// ============================================================================

/**
 * Agent type enumeration
 */
export const AgentTypeSchema = z.enum([
  'claude', // Claude Code / Anthropic Claude
  'cursor', // Cursor AI
  'copilot', // GitHub Copilot
  'aider', // Aider
  'continue', // Continue.dev
  'codeium', // Codeium
  'human', // Human developer
  'ci', // CI/CD system
  'unknown', // Unknown/unidentified
]);

export type AgentType = z.infer<typeof AgentTypeSchema>;

/**
 * Agent information schema
 */
export const AgentInfoSchema = z.object({
  /** Unique agent ID (UUID or custom identifier) */
  id: z.string().min(1),

  /** Type of agent */
  type: AgentTypeSchema.default('unknown'),

  /** Process ID (if available) */
  pid: z.number().optional(),

  /** Human-readable name/label */
  name: z.string().optional(),

  /** Session ID (for tracking related operations) */
  sessionId: z.string().optional(),

  /** Parent agent ID (for hierarchical agent setups) */
  parentAgentId: z.string().optional(),

  /** Custom metadata */
  metadata: z.record(z.string(), z.unknown()).optional(),
});

export type AgentInfo = z.infer<typeof AgentInfoSchema>;

/**
 * Registered agent record in the registry
 */
export const AgentRegistrationSchema = z.object({
  /** Agent info */
  agent: AgentInfoSchema,

  /** ISO timestamp when registered */
  registeredAt: z.string(),

  /** ISO timestamp of last heartbeat */
  lastHeartbeat: z.string(),

  /** Number of heartbeats received */
  heartbeatCount: z.number().default(0),

  /** Current state */
  state: z.enum(['active', 'idle', 'stale', 'terminated']).default('active'),

  /** Current operation (if any) */
  currentOperation: z.string().optional(),

  /** Workspace being operated on */
  workspaceRoot: z.string().optional(),
});

export type AgentRegistration = z.infer<typeof AgentRegistrationSchema>;

/**
 * Agent registry file schema (.anvil/agents/registry.json)
 */
export const AgentRegistrySchema = z.object({
  /** Schema version */
  version: z.string().default('1.0.0'),

  /** Last updated timestamp */
  updatedAt: z.string(),

  /** Registered agents by ID */
  agents: z.record(z.string(), AgentRegistrationSchema),
});

export type AgentRegistry = z.infer<typeof AgentRegistrySchema>;

// ============================================================================
// Lock Management
// ============================================================================

/**
 * Lock type enumeration
 */
export const LockTypeSchema = z.enum([
  'watch', // Watch mode lock (only one watcher per workspace)
  'action', // Action execution lock (gate, validate, check)
  'cache', // Cache write lock
  'state', // State file lock
  'task', // Task execution lock (specific task)
]);

export type LockType = z.infer<typeof LockTypeSchema>;

/**
 * Lock record schema
 */
export const LockRecordSchema = z.object({
  /** Lock type */
  type: LockTypeSchema,

  /** Resource being locked (e.g., file path, task ID) */
  resource: z.string(),

  /** Agent holding the lock */
  agentId: z.string(),

  /** Agent type (for display) */
  agentType: AgentTypeSchema.optional(),

  /** Process ID holding the lock */
  pid: z.number().optional(),

  /** ISO timestamp when lock was acquired */
  acquiredAt: z.string(),

  /** ISO timestamp when lock expires (for auto-release) */
  expiresAt: z.string(),

  /** Lock reason/description */
  reason: z.string().optional(),

  /** Number of times lock was renewed */
  renewCount: z.number().default(0),
});

export type LockRecord = z.infer<typeof LockRecordSchema>;

/**
 * Lock file schema (.anvil/locks/{type}-{resource-hash}.lock)
 */
export const LockFileSchema = z.object({
  /** Schema version */
  version: z.string().default('1.0.0'),

  /** Lock record */
  lock: LockRecordSchema,

  /** Lock acquisition history (for debugging) */
  history: z
    .array(
      z.object({
        agentId: z.string(),
        acquiredAt: z.string(),
        releasedAt: z.string().optional(),
        reason: z.string().optional(),
      })
    )
    .default([]),
});

export type LockFile = z.infer<typeof LockFileSchema>;

/**
 * Lock acquisition result
 */
export interface LockAcquisitionResult {
  /** Whether lock was acquired */
  acquired: boolean;

  /** Lock record if acquired */
  lock?: LockRecord;

  /** Error message if not acquired */
  error?: string;

  /** Existing lock holder info (if lock is held by another) */
  heldBy?: {
    agentId: string;
    agentType?: AgentType;
    acquiredAt: string;
    expiresAt: string;
    pid?: number;
  };

  /** Position in queue (if queued) */
  queuePosition?: number;
}

/**
 * Lock release result
 */
export interface LockReleaseResult {
  /** Whether lock was released */
  released: boolean;

  /** Error message if not released */
  error?: string;

  /** Whether lock was held by a different agent */
  wasHeldByOther?: boolean;
}

// ============================================================================
// Queue Management
// ============================================================================

/**
 * Queue entry schema
 */
export const QueueEntrySchema = z.object({
  /** Unique entry ID */
  id: z.string(),

  /** Agent requesting the resource */
  agentId: z.string(),

  /** Agent type */
  agentType: AgentTypeSchema.optional(),

  /** Lock type being requested */
  lockType: LockTypeSchema,

  /** Resource being requested */
  resource: z.string(),

  /** ISO timestamp when queued */
  queuedAt: z.string(),

  /** Priority (lower = higher priority, default: 100) */
  priority: z.number().default(100),

  /** Timeout for queue entry (auto-remove after this) */
  timeoutAt: z.string(),

  /** Reason for request */
  reason: z.string().optional(),

  /** Callback URL/path for notification (optional) */
  callbackPath: z.string().optional(),
});

export type QueueEntry = z.infer<typeof QueueEntrySchema>;

/**
 * Queue file schema (.anvil/queue/{resource-hash}.json)
 */
export const QueueFileSchema = z.object({
  /** Schema version */
  version: z.string().default('1.0.0'),

  /** Resource identifier */
  resource: z.string(),

  /** Lock type */
  lockType: LockTypeSchema,

  /** Last updated timestamp */
  updatedAt: z.string(),

  /** Queued entries (ordered by priority then queuedAt) */
  entries: z.array(QueueEntrySchema),
});

export type QueueFile = z.infer<typeof QueueFileSchema>;

/**
 * Queue join result
 */
export interface QueueJoinResult {
  /** Entry ID */
  entryId: string;

  /** Position in queue (1-based) */
  position: number;

  /** Estimated wait time in ms (if calculable) */
  estimatedWaitMs?: number;

  /** Whether already in queue (entry updated) */
  alreadyQueued: boolean;
}

/**
 * Queue status result
 */
export interface QueueStatusResult {
  /** Total entries in queue */
  totalEntries: number;

  /** Your position (if in queue, 1-based) */
  yourPosition?: number;

  /** Your entry (if in queue) */
  yourEntry?: QueueEntry;

  /** Current lock holder */
  currentHolder?: {
    agentId: string;
    agentType?: AgentType;
    acquiredAt: string;
    expiresAt: string;
  };
}

// ============================================================================
// Coordination Events
// ============================================================================

/**
 * Coordination event types
 */
export type CoordinationEventType =
  | 'agent:registered'
  | 'agent:heartbeat'
  | 'agent:terminated'
  | 'agent:stale'
  | 'lock:acquired'
  | 'lock:released'
  | 'lock:expired'
  | 'lock:denied'
  | 'queue:joined'
  | 'queue:advanced'
  | 'queue:timeout'
  | 'queue:removed';

/**
 * Coordination event
 */
export interface CoordinationEvent {
  /** Event type */
  type: CoordinationEventType;

  /** Timestamp */
  timestamp: string;

  /** Agent involved */
  agentId: string;

  /** Resource involved (if applicable) */
  resource?: string;

  /** Additional details */
  details?: Record<string, unknown>;
}

// ============================================================================
// Configuration
// ============================================================================

/**
 * Concurrency configuration schema
 */
export const ConcurrencyConfigSchema = z.object({
  /** Enable multi-agent mode */
  enabled: z.boolean().default(true),

  /** Lock timeout in ms (default: 5 minutes) */
  lockTimeoutMs: z.number().min(1000).max(3600000).default(300000),

  /** Heartbeat interval in ms (default: 10 seconds) */
  heartbeatIntervalMs: z.number().min(1000).max(60000).default(10000),

  /** Stale agent threshold in ms (3x heartbeat) */
  staleThresholdMs: z.number().min(3000).max(180000).default(30000),

  /** Queue timeout in ms (default: 10 minutes) */
  queueTimeoutMs: z.number().min(5000).max(3600000).default(600000),

  /** Maximum queue size per resource */
  maxQueueSize: z.number().min(1).max(100).default(20),

  /** Whether to auto-acquire lock from stale agents */
  autoAcquireFromStale: z.boolean().default(true),

  /** Lock directory path (relative to workspace) */
  lockDir: z.string().default('.anvil/locks'),

  /** Queue directory path (relative to workspace) */
  queueDir: z.string().default('.anvil/queue'),

  /** Agent registry path (relative to workspace) */
  registryPath: z.string().default('.anvil/agents/registry.json'),
});

export type ConcurrencyConfig = z.infer<typeof ConcurrencyConfigSchema>;

/**
 * Get default concurrency config
 */
export function getDefaultConcurrencyConfig(): ConcurrencyConfig {
  return ConcurrencyConfigSchema.parse({});
}
