/**
 * KindlingService (KINDLING-001)
 *
 * Core service wrapper that mediates between Anvil and the Kindling store.
 * Handles validation, sensitive-data checks, and delegation to the store adapter.
 *
 * The service is built against the abstract `IKindlingStore` interface so that
 * it compiles without @kindling/core or @kindling/store-sqlite installed.
 * The actual storage backend is plugged in at runtime via the factory function.
 *
 * When no store is provided, the service operates in "disabled mode" using a
 * no-op store that silently discards all observations.
 */

import type { Observation } from './observation-contract.js';
import { validateObservation } from './observation-contract.js';
import type { QueryRequest, QueryResponse } from './query-contract.js';
import { QueryRequestSchema } from './query-contract.js';
import type { KindlingConfig } from './config.js';
import { DEFAULT_KINDLING_CONFIG, shouldCapture } from './config.js';
import { validateNoSensitiveData, redactSensitiveFields } from './sensitive-data-validator.js';
import { createDebugger } from './utils/debug.js';

const debug = createDebugger('kindling');

// =============================================================================
// Store Interface (Abstract Adapter)
// =============================================================================

/**
 * Abstract storage adapter interface.
 *
 * Implementations must provide emit (write), query (read), and close (cleanup).
 * This decouples the service layer from any concrete Kindling SDK dependency.
 */
export interface IKindlingStore {
  /**
   * Persist an observation to the store.
   * The observation has already been validated and redacted by the service layer.
   */
  emit(observation: Observation): Promise<void>;

  /**
   * Execute a bounded query against the store.
   * The request has already been validated by the service layer.
   */
  query(request: QueryRequest): Promise<QueryResponse>;

  /**
   * Release resources (close database connections, flush buffers, etc.)
   */
  close(): Promise<void>;
}

// =============================================================================
// No-Op Store (Disabled Mode)
// =============================================================================

/**
 * No-op store used when Kindling is disabled or no store is provided.
 * All operations succeed silently without side effects.
 */
export class NoOpKindlingStore implements IKindlingStore {
  async emit(_observation: Observation): Promise<void> {
    // Intentionally empty -- disabled mode
  }

  async query(_request: QueryRequest): Promise<QueryResponse> {
    return {
      metadata: {
        query_id: crypto.randomUUID(),
        executed_at: new Date().toISOString(),
        contract_version: '1.0.0',
        result_count: 0,
        truncated: false,
        truncation_reason: 'none',
      },
      observations: [],
    };
  }

  async close(): Promise<void> {
    // Intentionally empty -- nothing to close
  }
}

// =============================================================================
// Service Errors
// =============================================================================

/**
 * Error thrown when observation validation fails
 */
export class ObservationValidationError extends Error {
  constructor(
    message: string,
    public readonly issues: string[]
  ) {
    super(message);
    this.name = 'ObservationValidationError';
  }
}

/**
 * Error thrown when query validation fails
 */
export class QueryValidationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'QueryValidationError';
  }
}

// =============================================================================
// KindlingService
// =============================================================================

/**
 * Core Kindling service that wraps the store adapter with validation,
 * sensitive-data checks, and config-driven behavior.
 */
export class KindlingService {
  private readonly store: IKindlingStore;
  private readonly config: KindlingConfig;
  private closed = false;

  constructor(store: IKindlingStore, config: KindlingConfig) {
    this.store = store;
    this.config = config;
    debug('KindlingService created', { enabled: config.enabled });
  }

  /**
   * Whether the service is enabled (will actually emit observations)
   */
  get enabled(): boolean {
    return this.config.enabled;
  }

  /**
   * The active configuration
   */
  get configuration(): Readonly<KindlingConfig> {
    return this.config;
  }

  /**
   * Emit an observation to the Kindling store.
   *
   * This method is designed to be async and non-blocking. It:
   * 1. Checks if the service is enabled and the observation kind should be captured
   * 2. Validates the observation against the contract schema
   * 3. Checks for and redacts sensitive data
   * 4. Delegates to the store adapter
   *
   * Validation errors are thrown. Store errors are thrown (callers should catch
   * if they want fire-and-forget semantics -- see emitters).
   *
   * @param observation - The observation to emit
   * @throws ObservationValidationError if the observation is invalid
   */
  async emit(observation: Observation): Promise<void> {
    if (this.closed) {
      debug('emit skipped: service is closed');
      return;
    }

    // Check if this kind should be captured
    if (!shouldCapture(this.config, observation.kind)) {
      debug('emit skipped: kind not captured', observation.kind);
      return;
    }

    // Validate against the contract schema. The parsed result — not the
    // caller's original object — is what gets persisted, so the store only
    // ever receives exactly the payload the schema validated (CIB-118).
    const validation = validateObservation(observation);
    if (!validation.success || !validation.data) {
      debug('emit validation failed', validation.error);
      throw new ObservationValidationError(`Invalid observation: ${validation.error}`, [
        validation.error ?? 'Unknown validation error',
      ]);
    }

    // Check for sensitive data and redact if found
    let safeObservation = validation.data;
    const sensitiveCheck = validateNoSensitiveData(safeObservation);

    if (sensitiveCheck.hasSensitiveData) {
      debug('sensitive data detected, redacting', sensitiveCheck.issues);
      const redacted = redactSensitiveFields(safeObservation);

      // Re-validate the redacted payload against the same schema before it
      // is persisted: redaction must never smuggle a contract-breaking
      // payload past validation (CIB-118).
      const revalidation = validateObservation(redacted);
      if (!revalidation.success || !revalidation.data) {
        debug('redacted observation failed re-validation', revalidation.error);
        throw new ObservationValidationError(
          `Redacted observation no longer matches the contract schema: ${revalidation.error}`,
          [revalidation.error ?? 'Unknown validation error']
        );
      }
      safeObservation = revalidation.data;
    }

    // Delegate to store (async, non-blocking from caller's perspective)
    debug('emitting observation', { kind: observation.kind, session_id: observation.session_id });
    await this.store.emit(safeObservation);
  }

  /**
   * Execute a query against the Kindling store.
   *
   * Validates the query request against the contract schema and enforces
   * configured query limits before delegating to the store.
   *
   * @param request - The query request
   * @returns Query response with observations
   * @throws QueryValidationError if the request is invalid
   */
  async query(request: QueryRequest): Promise<QueryResponse> {
    if (this.closed) {
      debug('query rejected: service is closed');
      throw new QueryValidationError('Service is closed');
    }

    // Validate the request
    const validation = QueryRequestSchema.safeParse(request);
    if (!validation.success) {
      debug('query validation failed', validation.error.format());
      throw new QueryValidationError(
        `Invalid query request: ${validation.error.format()._errors.join(', ')}`
      );
    }

    debug('executing query', { scope: request.scope });

    // Enforce configured query limits (use config defaults if request has higher values)
    const limitedRequest = {
      ...validation.data,
      max_results: Math.min(validation.data.max_results, this.config.query_limits.max_results),
      max_payload_bytes: Math.min(
        validation.data.max_payload_bytes,
        this.config.query_limits.max_payload_bytes
      ),
    } as QueryRequest;

    return this.store.query(limitedRequest);
  }

  /**
   * Close the service and release underlying store resources.
   * After calling close(), emit() becomes a no-op and query() throws.
   */
  async close(): Promise<void> {
    if (this.closed) {
      return;
    }
    debug('closing KindlingService');
    this.closed = true;
    await this.store.close();
  }
}

// =============================================================================
// Factory
// =============================================================================

/**
 * Create a KindlingService instance.
 *
 * If no store is provided, the service operates in disabled mode with a no-op store.
 * This allows code to unconditionally call emit/query without checking for null.
 *
 * @param config - Kindling configuration (defaults to disabled config)
 * @param store - Optional store adapter (defaults to NoOpKindlingStore)
 * @returns Configured KindlingService instance
 */
export function createKindlingService(
  config: KindlingConfig = DEFAULT_KINDLING_CONFIG,
  store?: IKindlingStore
): KindlingService {
  const effectiveStore = store ?? new NoOpKindlingStore();
  debug('creating KindlingService', { enabled: config.enabled, hasStore: !!store });
  return new KindlingService(effectiveStore, config);
}
