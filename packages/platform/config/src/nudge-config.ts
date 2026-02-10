/**
 * Nudge configuration
 *
 * Controls coaching nudge behaviour across CLI, MCP, and IDE surfaces.
 */

/**
 * Severity threshold for nudge display.
 * Only warnings at or above this severity will show nudges.
 * Ordered: error > warning > info
 */
export type NudgeSeverityThreshold = 'error' | 'warning' | 'info';

/**
 * Nudge configuration options
 */
export interface NudgeConfig {
  /** Whether nudges are enabled (default: true) */
  enabled: boolean;
  /** Whether interactive mode is on by default in CLI (default: false) */
  interactive: boolean;
  /** Minimum severity to show nudges for (default: 'warning') */
  severityThreshold: NudgeSeverityThreshold;
}

/**
 * Default nudge configuration
 */
export const DEFAULT_NUDGE_CONFIG: Readonly<NudgeConfig> = {
  enabled: true,
  interactive: false,
  severityThreshold: 'warning',
};

/**
 * Severity ordering for threshold comparison.
 * Higher number = higher severity.
 */
const SEVERITY_ORDER: Record<NudgeSeverityThreshold, number> = {
  info: 0,
  warning: 1,
  error: 2,
};

/**
 * Check whether a warning severity meets the nudge threshold.
 *
 * @param warningSeverity - The severity of the warning
 * @param threshold - The minimum severity threshold
 * @returns true if the warning severity is at or above the threshold
 */
export function meetsNudgeThreshold(
  warningSeverity: string,
  threshold: NudgeSeverityThreshold
): boolean {
  const warningLevel = SEVERITY_ORDER[warningSeverity as NudgeSeverityThreshold] ?? -1;
  const thresholdLevel = SEVERITY_ORDER[threshold];
  return warningLevel >= thresholdLevel;
}
