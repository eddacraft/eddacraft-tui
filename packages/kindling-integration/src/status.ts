/**
 * Kindling Status Utility (KINDLING-014)
 *
 * Provides a decoupled status summary for Kindling integration.
 * Returns observation counts, database size, and retention config
 * without coupling to any specific CLI framework.
 *
 * Usage:
 *   const status = await getKindlingStatus(config, store);
 *   // => { enabled: true, observationCount: 42, dbSizeBytes: 81920, ... }
 */

// =============================================================================
// Status Types
// =============================================================================

/**
 * Kindling status summary returned by getKindlingStatus()
 */
export interface KindlingStatus {
  /** Whether Kindling is enabled in config */
  enabled: boolean;

  /** Total number of observations stored (undefined if disabled) */
  observationCount?: number;

  /** Database file size in bytes (undefined if disabled or no DB) */
  dbSizeBytes?: number;

  /** Configured retention period in days (undefined if disabled) */
  retentionDays?: number;

  /** ISO8601 timestamp of the most recent observation (undefined if none) */
  lastObservationAt?: string;
}

// =============================================================================
// Configuration Interface (minimal, no coupling to config.ts)
// =============================================================================

/**
 * Minimal config shape needed for status checks.
 * This avoids importing from config.ts (built by other agent).
 */
export interface KindlingStatusConfig {
  enabled: boolean;
  database?: string;
  retention?: {
    days?: number;
  };
}

// =============================================================================
// Store Interface (minimal, no coupling to kindling-service.ts)
// =============================================================================

/**
 * Minimal store interface for status queries.
 * Implementations can be KindlingService, a direct SQLite handle, or a mock.
 */
export interface KindlingStatusStore {
  /**
   * Count all observations in the store.
   * Returns 0 if the store is empty.
   */
  countObservations(): Promise<number>;

  /**
   * Get the database file size in bytes.
   * Returns 0 if the database does not exist or cannot be read.
   */
  getDatabaseSizeBytes(): Promise<number>;

  /**
   * Get the timestamp of the most recent observation.
   * Returns undefined if no observations exist.
   */
  getLastObservationTimestamp(): Promise<string | undefined>;
}

// =============================================================================
// Status Function
// =============================================================================

/**
 * Get Kindling integration status.
 *
 * If store is not provided or config is disabled, returns a minimal
 * disabled status. This function never throws -- it handles errors
 * gracefully and returns partial information.
 *
 * @param config - Kindling configuration (at minimum, { enabled: boolean })
 * @param store - Optional store interface for querying observation data
 * @returns KindlingStatus summary
 *
 * @example
 * ```typescript
 * import { getKindlingStatus } from './status.js';
 *
 * // Disabled (no store needed)
 * const status = await getKindlingStatus({ enabled: false });
 * // => { enabled: false }
 *
 * // Enabled with store
 * const status = await getKindlingStatus(
 *   { enabled: true, retention: { days: 90 } },
 *   myStore,
 * );
 * // => { enabled: true, observationCount: 42, dbSizeBytes: 81920,
 * //      retentionDays: 90, lastObservationAt: '2026-02-15T...' }
 * ```
 */
export async function getKindlingStatus(
  config: KindlingStatusConfig,
  store?: KindlingStatusStore
): Promise<KindlingStatus> {
  // Fast path: disabled
  if (!config.enabled) {
    return { enabled: false };
  }

  // Enabled but no store available
  if (!store) {
    return {
      enabled: true,
      retentionDays: config.retention?.days,
    };
  }

  // Enabled with store -- query for status data
  const [observationCount, dbSizeBytes, lastObservationAt] = await Promise.all([
    safeCall(() => store.countObservations(), 0),
    safeCall(() => store.getDatabaseSizeBytes(), 0),
    safeCall(() => store.getLastObservationTimestamp(), undefined),
  ]);

  return {
    enabled: true,
    observationCount,
    dbSizeBytes,
    retentionDays: config.retention?.days,
    lastObservationAt,
  };
}

// =============================================================================
// Formatting Utilities
// =============================================================================

/**
 * Format a KindlingStatus as a human-readable string.
 * Useful for CLI output without coupling to a specific formatter.
 *
 * @example
 * ```typescript
 * const status = await getKindlingStatus(config, store);
 * console.log(formatKindlingStatus(status));
 * // Kindling: enabled
 * // Observations: 42
 * // Database size: 80 KB
 * // Retention: 90 days
 * // Last observation: 2026-02-15T10:30:00.000Z
 * ```
 */
export function formatKindlingStatus(status: KindlingStatus): string {
  const lines: string[] = [];

  lines.push(`Kindling: ${status.enabled ? 'enabled' : 'disabled'}`);

  if (!status.enabled) {
    return lines.join('\n');
  }

  if (status.observationCount !== undefined) {
    lines.push(`Observations: ${status.observationCount.toLocaleString()}`);
  }

  if (status.dbSizeBytes !== undefined) {
    lines.push(`Database size: ${formatBytes(status.dbSizeBytes)}`);
  }

  if (status.retentionDays !== undefined) {
    lines.push(`Retention: ${status.retentionDays} days`);
  }

  if (status.lastObservationAt) {
    lines.push(`Last observation: ${status.lastObservationAt}`);
  } else if (status.observationCount === 0) {
    lines.push('Last observation: none');
  }

  return lines.join('\n');
}

// =============================================================================
// Internal Helpers
// =============================================================================

/**
 * Call an async function and return a fallback value on error.
 * Ensures getKindlingStatus never throws.
 */
async function safeCall<T>(fn: () => Promise<T>, fallback: T): Promise<T> {
  try {
    return await fn();
  } catch {
    return fallback;
  }
}

/**
 * Format bytes into a human-readable string.
 */
function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const k = 1024;
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), units.length - 1);
  const value = bytes / Math.pow(k, i);
  return `${value < 10 ? value.toFixed(1) : Math.round(value)} ${units[i]}`;
}
