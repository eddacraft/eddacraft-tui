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
 * @param value - The value to parse (typically from configuration)
 * @param defaultValue - The default severity to return if parsing fails (default: 'info')
 * @returns The normalized severity level or defaultValue
 *
 * @example
 * ```typescript
 * parseSeverity('ERROR') // 'error'
 * parseSeverity('warn') // 'warning'
 * parseSeverity('invalid') // 'info'
 * parseSeverity('invalid', 'error') // 'error'
 * parseSeverity('invalid', undefined) // undefined
 * parseSeverity(123) // 'info'
 * ```
 */
export function parseSeverity(
  value: unknown,
  defaultValue?: Severity | undefined
): Severity | undefined {
  // When defaultValue is not provided, use 'info'
  const fallback =
    defaultValue !== undefined ? defaultValue : arguments.length === 1 ? 'info' : undefined;

  if (typeof value !== 'string') {
    return fallback;
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
      return fallback;
  }
}
