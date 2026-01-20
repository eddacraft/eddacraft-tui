/**
 * Kindling Port Interface (STACK-007, STACK-009)
 *
 * Defines the interface for Kindling observation storage adapters.
 * Implementations can use SQLite, PostgreSQL, or in-memory storage.
 *
 * @module @eddacraft/anvil-edda-stack/contracts/ports/kindling
 */

import type { ObservationId, SessionId, PlanId } from '../identifiers.js';
import type { TimeRange, Timestamp } from '../temporal.js';
import type { KindlingRef } from '../provenance.js';

// =============================================================================
// Observation Types
// =============================================================================

/**
 * Observation kinds that Kindling can record
 */
export type ObservationKind =
  | 'gate_evaluated'
  | 'action_executed'
  | 'action_failed'
  | 'plan_started'
  | 'plan_completed'
  | 'constraint_applied'
  | 'error_recorded'
  | 'metric_recorded'
  | 'custom';

/**
 * A single observation record in Kindling
 */
export interface Observation {
  /** Unique observation identifier */
  id: ObservationId;

  /** Session this observation belongs to */
  session_id: SessionId;

  /** Type of observation */
  kind: ObservationKind;

  /** When the observation was recorded */
  timestamp: Timestamp;

  /** Human-readable summary */
  summary: string;

  /** Observation-specific data */
  data: Record<string, unknown>;

  /** Optional tags for categorization */
  tags?: string[];
}

/**
 * Input for creating a new observation
 */
export interface CreateObservationInput {
  session_id: SessionId;
  kind: ObservationKind;
  summary: string;
  data: Record<string, unknown>;
  tags?: string[];
}

/**
 * Query options for observations
 */
export interface ObservationQuery {
  /** Filter by session ID */
  session_id?: SessionId;

  /** Filter by observation kinds */
  kinds?: ObservationKind[];

  /** Filter by time range */
  time_range?: TimeRange;

  /** Filter by tags (any match) */
  tags?: string[];

  /** Maximum number of results */
  limit?: number;

  /** Offset for pagination */
  offset?: number;
}

/**
 * Result of an observation query
 */
export interface ObservationQueryResult {
  observations: Observation[];
  total: number;
  has_more: boolean;
}

// =============================================================================
// Session Query Types (STACK-007)
// =============================================================================

/**
 * Options for querying a session
 */
export interface SessionQueryOptions {
  /** Filter by observation kinds */
  kinds?: ObservationKind[];

  /** Filter by time range within session */
  time_range?: TimeRange;

  /** Filter by tags (any match) */
  tags?: string[];

  /** Include observation data/payloads (default: true) */
  include_payloads?: boolean;

  /** Limit number of observations returned */
  limit?: number;

  /** Offset for pagination */
  offset?: number;

  /** Sort order */
  sort_order?: 'asc' | 'desc';
}

/**
 * Result of a session query
 */
export interface SessionQueryResult {
  /** The session ID queried */
  session_id: SessionId;

  /** Observations matching the query */
  observations: Observation[];

  /** Total observations in session (before limit) */
  total: number;

  /** Whether more observations exist */
  has_more: boolean;

  /** Session metadata (if available) */
  session_metadata?: {
    started_at?: Timestamp;
    ended_at?: Timestamp;
    plan_id?: PlanId;
  };
}

// =============================================================================
// Plan Query Types (STACK-007)
// =============================================================================

/**
 * Options for querying by plan
 */
export interface PlanQueryOptions {
  /** Filter by observation kinds */
  kinds?: ObservationKind[];

  /** Filter sessions by time range */
  session_time_range?: TimeRange;

  /** Include observations from all sessions */
  include_observations?: boolean;

  /** Limit number of sessions returned */
  limit?: number;

  /** Offset for pagination */
  offset?: number;
}

/**
 * Summary of a session (used in plan queries)
 */
export interface SessionSummary {
  session_id: SessionId;
  started_at: Timestamp;
  ended_at?: Timestamp;
  observation_count: number;
  kinds_observed: ObservationKind[];
}

/**
 * Result of a plan query
 */
export interface PlanQueryResult {
  /** The plan ID queried */
  plan_id: PlanId;

  /** Sessions that executed this plan */
  sessions: SessionSummary[];

  /** Total sessions (before limit) */
  total_sessions: number;

  /** Whether more sessions exist */
  has_more: boolean;

  /** Observations (if include_observations was true) */
  observations?: Observation[];
}

// =============================================================================
// Kindling Port Interface
// =============================================================================

/**
 * Port interface for Kindling observation storage
 *
 * This is the primary abstraction for reading/writing observations.
 * Implementations should be stateless and thread-safe.
 */
export interface IKindlingPort {
  // ─────────────────────────────────────────────────────────────────────────
  // Write Operations
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Record a new observation
   */
  createObservation(input: CreateObservationInput): Promise<Observation>;

  /**
   * Record multiple observations in a batch
   */
  createObservationBatch(inputs: CreateObservationInput[]): Promise<Observation[]>;

  // ─────────────────────────────────────────────────────────────────────────
  // Read Operations
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Get a single observation by ID
   */
  getObservation(id: ObservationId): Promise<Observation | null>;

  /**
   * Query observations with filters
   */
  queryObservations(query: ObservationQuery): Promise<ObservationQueryResult>;

  /**
   * Get all observations for a session
   */
  getSessionObservations(sessionId: SessionId): Promise<Observation[]>;

  /**
   * Check if an observation exists
   */
  observationExists(id: ObservationId): Promise<boolean>;

  // ─────────────────────────────────────────────────────────────────────────
  // Session & Plan Queries (STACK-007)
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Query observations for a specific session with options
   *
   * @param sessionId - The session to query
   * @param options - Query options (filters, pagination)
   * @returns Session query result with observations
   */
  querySession(sessionId: SessionId, options?: SessionQueryOptions): Promise<SessionQueryResult>;

  /**
   * Query sessions and observations for a specific plan
   *
   * @param planId - The plan to query
   * @param options - Query options (filters, pagination)
   * @returns Plan query result with sessions
   */
  queryByPlan(planId: PlanId, options?: PlanQueryOptions): Promise<PlanQueryResult>;

  /**
   * Get all observations for a session (convenience method)
   *
   * @param sessionId - The session to query
   * @returns All observations for the session
   */
  getObservationsBySession(sessionId: SessionId): Promise<Observation[]>;

  /**
   * Get observations within a time range across all sessions
   *
   * @param range - Time range to query
   * @returns Observations within the time range
   */
  getObservationsByTimeRange(range: TimeRange): Promise<Observation[]>;

  // ─────────────────────────────────────────────────────────────────────────
  // Provenance Helpers
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Get observations as KindlingRef array (for provenance chains)
   */
  getObservationsAsRefs(ids: ObservationId[]): Promise<KindlingRef[]>;

  // ─────────────────────────────────────────────────────────────────────────
  // Maintenance & Status
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Check if the Kindling store is available and operational
   *
   * @returns True if the store is available
   */
  isAvailable(): Promise<boolean>;

  /**
   * Get total observation count (optionally filtered by session)
   */
  countObservations(sessionId?: SessionId): Promise<number>;

  /**
   * Delete observations older than a given timestamp
   * Returns the number of deleted observations
   */
  pruneObservations(olderThan: Timestamp): Promise<number>;
}
