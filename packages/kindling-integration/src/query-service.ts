/**
 * Kindling Query Service (KINDLING-009)
 *
 * High-level query interface that wraps KindlingService.query() with
 * convenience methods for each query scope. Applies query limit enforcement
 * before returning results.
 *
 * This is the primary read API for Anvil CLI commands and tooling.
 */

import type { KindlingService } from './kindling-service.js';
import type {
  QueryResponse,
  SessionQuery,
  PlanQuery,
  GateQuery,
  ActionQuery,
  ResultShape,
  OutputFormat,
} from './query-contract.js';
import { enforceQueryLimits, limitsFromConfig } from './query-limits.js';

// =============================================================================
// Query Options
// =============================================================================

/**
 * Common options for all query methods
 */
export interface QueryOptions {
  /** Result structure (default: 'list') */
  shape?: ResultShape;
  /** Output format (default: 'json') */
  format?: OutputFormat;
  /** Include observations after this time */
  time_after?: string;
  /** Include observations before this time */
  time_before?: string;
  /** Maximum observations to return (capped by config) */
  max_results?: number;
  /** Maximum total payload size in bytes (capped by config) */
  max_payload_bytes?: number;
}

/**
 * Options specific to session queries
 */
export interface SessionQueryOptions extends QueryOptions {
  /** Filter to specific phases */
  include_phases?: Array<'plan' | 'gate' | 'action' | 'outcome' | 'error'>;
}

/**
 * Options specific to plan queries
 */
export interface PlanQueryOptions extends QueryOptions {
  /** Include linked execution run IDs (default: true) */
  include_executions?: boolean;
  /** Include plan version history (default: true) */
  include_versions?: boolean;
}

/**
 * Options specific to action queries
 */
export interface ActionQueryOptions extends QueryOptions {
  /** Include approval chain (default: true) */
  include_approval_chain?: boolean;
}

// =============================================================================
// Query Service
// =============================================================================

/**
 * High-level query service for Kindling observations.
 *
 * Provides typed convenience methods for each query scope and enforces
 * query limits from the service configuration.
 */
export class KindlingQueryService {
  private readonly service: KindlingService;

  constructor(service: KindlingService) {
    this.service = service;
  }

  /**
   * Query: "What happened in this run?"
   *
   * Returns ordered observations for a specific session, optionally
   * filtered by phase.
   *
   * @param sessionId - Session/run ID
   * @param options - Query options
   * @returns Query response with session observations
   */
  async querySession(sessionId: string, options: SessionQueryOptions = {}): Promise<QueryResponse> {
    const request: SessionQuery = {
      scope: 'session',
      session_id: sessionId,
      shape: options.shape ?? 'timeline',
      format: options.format ?? 'json',
      time_after: options.time_after,
      time_before: options.time_before,
      max_results: options.max_results ?? this.service.configuration.query_limits.max_results,
      max_payload_bytes:
        options.max_payload_bytes ?? this.service.configuration.query_limits.max_payload_bytes,
      include_phases: options.include_phases,
    };

    const response = await this.service.query(request);
    return this.applyLimits(response);
  }

  /**
   * Query: "What happened because of this plan?"
   *
   * Returns plan metadata, versions, and linked executions.
   * This is the only cross-session read allowed.
   *
   * @param planId - Plan ID
   * @param options - Query options
   * @returns Query response with plan observations
   */
  async queryPlan(planId: string, options: PlanQueryOptions = {}): Promise<QueryResponse> {
    const request: PlanQuery = {
      scope: 'plan',
      plan_id: planId,
      shape: options.shape ?? 'entity',
      format: options.format ?? 'json',
      time_after: options.time_after,
      time_before: options.time_before,
      max_results: options.max_results ?? this.service.configuration.query_limits.max_results,
      max_payload_bytes:
        options.max_payload_bytes ?? this.service.configuration.query_limits.max_payload_bytes,
      include_executions: options.include_executions ?? true,
      include_versions: options.include_versions ?? true,
    };

    const response = await this.service.query(request);
    return this.applyLimits(response);
  }

  /**
   * Query: "Why did this gate pass/fail?"
   *
   * Returns gate evaluation details with rule IDs, inputs, and outcomes.
   *
   * @param gateEvalId - Gate evaluation ID
   * @param options - Query options
   * @returns Query response with gate observations
   */
  async queryGate(gateEvalId: string, options: QueryOptions = {}): Promise<QueryResponse> {
    const request: GateQuery = {
      scope: 'gate',
      gate_eval_id: gateEvalId,
      shape: options.shape ?? 'entity',
      format: options.format ?? 'json',
      time_after: options.time_after,
      time_before: options.time_before,
      max_results: options.max_results ?? this.service.configuration.query_limits.max_results,
      max_payload_bytes:
        options.max_payload_bytes ?? this.service.configuration.query_limits.max_payload_bytes,
    };

    const response = await this.service.query(request);
    return this.applyLimits(response);
  }

  /**
   * Query: "What exactly did this action do?"
   *
   * Returns action execution details with redacted command, environment,
   * and linked governance.
   *
   * @param actionId - Action ID
   * @param options - Query options
   * @returns Query response with action observations
   */
  async queryAction(actionId: string, options: ActionQueryOptions = {}): Promise<QueryResponse> {
    const request: ActionQuery = {
      scope: 'action',
      action_id: actionId,
      shape: options.shape ?? 'entity',
      format: options.format ?? 'json',
      time_after: options.time_after,
      time_before: options.time_before,
      max_results: options.max_results ?? this.service.configuration.query_limits.max_results,
      max_payload_bytes:
        options.max_payload_bytes ?? this.service.configuration.query_limits.max_payload_bytes,
      include_approval_chain: options.include_approval_chain ?? true,
    };

    const response = await this.service.query(request);
    return this.applyLimits(response);
  }

  // ===========================================================================
  // Private
  // ===========================================================================

  /**
   * Apply query limits from the service configuration as a defense-in-depth layer.
   */
  private applyLimits(response: QueryResponse): QueryResponse {
    const limits = limitsFromConfig(this.service.configuration.query_limits);
    return enforceQueryLimits(response, limits);
  }
}
