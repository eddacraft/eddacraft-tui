/**
 * Severity parsing utilities
 *
 * Shared utilities for parsing and normalizing severity levels across checks.
 */

/**
 * Standard severity levels used across Anvil
 */
export type Severity = 'error' | 'warning' | 'info';

/**
 * Parse a severity string into a normalized Severity type.
 *
 * Accepts common variations:
 * - 'error' → 'error'
 * - 'warning' | 'warn' → 'warning'
 * - 'info' → 'info'
 * - Any other value → returns defaultValue
 *
 * When called with a single argument, defaults to 'info'.
 *
 * @param value - The value to parse (typically from configuration)
 * @param defaultValue - The default severity to return if parsing fails (default: 'info')
 * @returns The normalized severity level or defaultValue
 *
 * @example
 * ```typescript
 * parseSeverity('ERROR') // 'error'
 * parseSeverity('warn') // 'warning'
 * parseSeverity('invalid') // 'info' (default)
 * parseSeverity('invalid', 'error') // 'error'
 * parseSeverity(123) // 'info'
 * parseSeverity(123, undefined) // undefined
 * ```
 */
export function parseSeverity(value: unknown, defaultValue?: Severity): Severity;
export function parseSeverity(value: unknown, defaultValue: undefined): Severity | undefined;
export function parseSeverity(
  value: unknown,
  defaultValue: Severity | undefined = 'info'
): Severity | undefined {
  if (typeof value !== 'string') {
    return defaultValue;
  }

  const lower = value.toLowerCase();

  switch (lower) {
    case 'error':
      return 'error';
    case 'warn':
    case 'warning':
      return 'warning';
    case 'info':
      return 'info';
    default:
      return defaultValue;
  }
}
