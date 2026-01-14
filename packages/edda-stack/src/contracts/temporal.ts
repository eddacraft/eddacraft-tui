/**
 * Timestamp and Temporal Schemas (STACK-002)
 *
 * Defines shared timestamp conventions used across all Edda Stack layers.
 * All timestamps are ISO8601 strings in UTC for consistency and portability.
 *
 * @module @anvil/edda-stack/contracts/temporal
 */

import { z } from 'zod';

// =============================================================================
// Base Timestamp Schema
// =============================================================================

/**
 * ISO8601 timestamp string (always UTC)
 * Example: "2024-01-15T14:30:00.000Z"
 */
export const TimestampSchema = z
  .string()
  .datetime({ message: 'Must be a valid ISO8601 datetime string' })
  .describe('ISO8601 UTC timestamp');

export type Timestamp = z.infer<typeof TimestampSchema>;

// =============================================================================
// Duration Schemas
// =============================================================================

/**
 * Duration in milliseconds (non-negative integer)
 */
export const DurationMsSchema = z.number().int().nonnegative().describe('Duration in milliseconds');

export type DurationMs = z.infer<typeof DurationMsSchema>;

/**
 * Duration in seconds (non-negative integer)
 */
export const DurationSecondsSchema = z.number().int().nonnegative().describe('Duration in seconds');

export type DurationSeconds = z.infer<typeof DurationSecondsSchema>;

/**
 * Duration in days (for TTL/expiry calculations)
 */
export const DurationDaysSchema = z.number().int().positive().describe('Duration in days');

export type DurationDays = z.infer<typeof DurationDaysSchema>;

// =============================================================================
// Time Range Schema
// =============================================================================

/**
 * Time range with start and optional end
 */
export const TimeRangeSchema = z
  .object({
    start: TimestampSchema.describe('Range start (inclusive)'),
    end: TimestampSchema.optional().describe('Range end (exclusive)'),
  })
  .refine(
    (data) => {
      if (!data.end) return true;
      return new Date(data.start) < new Date(data.end);
    },
    { message: 'Start must be before end' }
  );

export type TimeRange = z.infer<typeof TimeRangeSchema>;

// =============================================================================
// TTL (Time-To-Live) Schema
// =============================================================================

/**
 * TTL configuration for decay semantics (Ember proposals)
 */
export const TtlConfigSchema = z.object({
  default_ttl_days: DurationDaysSchema.default(30).describe('Default TTL for proposals'),
  min_ttl_days: DurationDaysSchema.default(7).describe('Minimum allowed TTL'),
  max_ttl_days: DurationDaysSchema.default(90).describe('Maximum allowed TTL'),
});

export type TtlConfig = z.infer<typeof TtlConfigSchema>;

/**
 * Expiry information for a record with TTL
 */
export const ExpiryInfoSchema = z.object({
  created_at: TimestampSchema.describe('When the record was created'),
  expires_at: TimestampSchema.describe('When the record expires'),
  ttl_days: DurationDaysSchema.describe('Original TTL in days'),
});

export type ExpiryInfo = z.infer<typeof ExpiryInfoSchema>;

// =============================================================================
// Temporal Utilities
// =============================================================================

/**
 * Create a timestamp for the current moment
 */
export function now(): Timestamp {
  return new Date().toISOString() as Timestamp;
}

/**
 * Parse a timestamp string and validate it
 */
export function parseTimestamp(value: string): Timestamp {
  return TimestampSchema.parse(value);
}

/**
 * Check if a timestamp is valid ISO8601
 */
export function isValidTimestamp(value: string): boolean {
  return TimestampSchema.safeParse(value).success;
}

/**
 * Calculate expiry timestamp from creation time and TTL
 */
export function calculateExpiry(createdAt: Timestamp, ttlDays: number): Timestamp {
  const date = new Date(createdAt);
  date.setDate(date.getDate() + ttlDays);
  return date.toISOString() as Timestamp;
}

/**
 * Check if a timestamp is expired (past current time)
 */
export function isExpired(expiresAt: Timestamp): boolean {
  return new Date(expiresAt) < new Date();
}

/**
 * Calculate remaining time until expiry in milliseconds
 * Returns 0 if already expired
 */
export function remainingTtlMs(expiresAt: Timestamp): DurationMs {
  const remaining = new Date(expiresAt).getTime() - Date.now();
  return Math.max(0, remaining) as DurationMs;
}

/**
 * Calculate duration between two timestamps in milliseconds
 */
export function durationBetween(start: Timestamp, end: Timestamp): DurationMs {
  const startMs = new Date(start).getTime();
  const endMs = new Date(end).getTime();
  return Math.abs(endMs - startMs) as DurationMs;
}

/**
 * Create a time range from start to now
 */
export function rangeFromStart(start: Timestamp): TimeRange {
  return { start, end: now() };
}

/**
 * Create a time range for the last N days
 */
export function lastNDays(days: number): TimeRange {
  const end = now();
  const startDate = new Date();
  startDate.setDate(startDate.getDate() - days);
  return {
    start: startDate.toISOString() as Timestamp,
    end,
  };
}

/**
 * Create expiry info from creation timestamp and TTL
 */
export function createExpiryInfo(createdAt: Timestamp, ttlDays: DurationDays): ExpiryInfo {
  return {
    created_at: createdAt,
    expires_at: calculateExpiry(createdAt, ttlDays),
    ttl_days: ttlDays,
  };
}
