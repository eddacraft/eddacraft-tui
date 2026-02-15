/**
 * Query Limits Enforcement (KINDLING-010)
 *
 * Provides post-query truncation and metadata flagging.
 * Used by KindlingQueryService before returning results to callers,
 * as a defense-in-depth layer on top of store-level limits.
 */

import type { QueryResponse, QueryResponseMetadata } from './query-contract.js';
import type { QueryLimitConfig } from './config.js';

// =============================================================================
// Types
// =============================================================================

/**
 * Limits to enforce on a query response
 */
export interface QueryLimits {
  /** Maximum number of observations to return */
  max_results: number;
  /** Maximum total payload size in bytes */
  max_payload_bytes: number;
}

// =============================================================================
// Enforcement
// =============================================================================

/**
 * Enforce query limits on a response by truncating results if necessary.
 *
 * This is applied after the store returns results, as a safety net.
 * If the store already respects limits, this is a no-op.
 *
 * Sets the `truncated` and `truncation_reason` metadata flags when
 * results are truncated.
 *
 * @param response - The query response from the store
 * @param limits - The limits to enforce
 * @returns A new QueryResponse with limits enforced
 */
export function enforceQueryLimits(response: QueryResponse, limits: QueryLimits): QueryResponse {
  let observations = response.observations;
  let truncated = response.metadata.truncated;
  let truncationReason = response.metadata.truncation_reason;

  // Check max_results
  if (observations.length > limits.max_results) {
    observations = observations.slice(0, limits.max_results);
    truncated = true;
    truncationReason = 'max_results';
  }

  // Check max_payload_bytes
  const serialized = JSON.stringify(observations);
  const payloadBytes = new TextEncoder().encode(serialized).byteLength;

  if (payloadBytes > limits.max_payload_bytes) {
    // Binary search for the maximum number of observations that fit
    let lo = 0;
    let hi = observations.length;

    while (lo < hi) {
      const mid = Math.floor((lo + hi + 1) / 2);
      const slice = observations.slice(0, mid);
      const sliceBytes = new TextEncoder().encode(JSON.stringify(slice)).byteLength;

      if (sliceBytes <= limits.max_payload_bytes) {
        lo = mid;
      } else {
        hi = mid - 1;
      }
    }

    observations = observations.slice(0, lo);
    truncated = true;
    // Only override reason if not already set (max_results takes precedence)
    truncationReason = truncationReason === 'max_results' ? 'max_results' : 'max_payload_bytes';
  }

  const metadata: QueryResponseMetadata = {
    ...response.metadata,
    result_count: observations.length,
    truncated,
    truncation_reason: truncated ? truncationReason : 'none',
  };

  return {
    metadata,
    observations,
  };
}

/**
 * Create QueryLimits from a QueryLimitConfig.
 *
 * @param config - Query limit configuration
 * @returns QueryLimits object
 */
export function limitsFromConfig(config: QueryLimitConfig): QueryLimits {
  return {
    max_results: config.max_results,
    max_payload_bytes: config.max_payload_bytes,
  };
}
