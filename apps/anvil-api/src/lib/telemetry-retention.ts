// FLEET-005 (ADR-107 §6): raw telemetry beacon rows are retained 90 days;
// daily aggregates are kept indefinitely. The retention window is
// configuration — this module is its single home; no SQL carries the value
// as a literal (the purge query takes it as a bound parameter).

/** ADR-107 §6 raw-row retention window. */
export const DEFAULT_TELEMETRY_RETENTION_DAYS = 90;
export const MAX_TELEMETRY_RETENTION_DAYS = 90;

/** Validate the privacy ceiling shared by cleanup and read-side queries. */
export function validateTelemetryRetentionDays(
  value: number,
  source = 'telemetry retention window'
): number {
  if (!Number.isInteger(value) || value < 1 || value > MAX_TELEMETRY_RETENTION_DAYS) {
    throw new Error(
      `${source} must be an integer from 1 to ${MAX_TELEMETRY_RETENTION_DAYS} days, got "${value}"`
    );
  }
  return value;
}

/**
 * Resolve the raw-beacon retention window in days.
 *
 * `TELEMETRY_RETENTION_DAYS` overrides the default. Invalid explicit
 * configuration throws rather than silently falling back: an operator who
 * set the variable meant something, and a typo must not quietly become a
 * different retention posture.
 */
export function getTelemetryRetentionDays(): number {
  const raw = process.env['TELEMETRY_RETENTION_DAYS'];
  if (raw === undefined || raw.trim() === '') {
    return DEFAULT_TELEMETRY_RETENTION_DAYS;
  }
  const parsed = Number(raw);
  return validateTelemetryRetentionDays(parsed, 'TELEMETRY_RETENTION_DAYS');
}
