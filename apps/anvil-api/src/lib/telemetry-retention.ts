// FLEET-005 (ADR-107 §6): raw telemetry beacon rows are retained 90 days;
// daily aggregates are kept indefinitely. The retention window is
// configuration — this module is its single home; no SQL carries the value
// as a literal (the purge query takes it as a bound parameter).

/** ADR-107 §6 raw-row retention window. */
export const DEFAULT_TELEMETRY_RETENTION_DAYS = 90;

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
  if (!Number.isInteger(parsed) || parsed < 1) {
    throw new Error(
      `TELEMETRY_RETENTION_DAYS must be a positive integer number of days, got "${raw}"`
    );
  }
  return parsed;
}
