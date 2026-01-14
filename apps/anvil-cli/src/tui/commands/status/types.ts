/**
 * Types for the `anvil status` dashboard TUI
 */

/** Status of an individual git hook */
export type HookState = 'active' | 'disabled' | 'missing';

/** Information about a single git hook */
export interface HookInfo {
  /** Hook name (e.g., 'pre-commit', 'commit-msg') */
  name: string;
  /** Current state of the hook */
  state: HookState;
  /** Path to the hook file (if exists) */
  path?: string;
  /** Last execution time (if available) */
  lastRun?: Date;
  /** Whether it's an Anvil-managed hook */
  isAnvilManaged: boolean;
}

/** Collection of git hooks status */
export interface HooksStatus {
  /** Whether Husky is installed */
  huskyInstalled: boolean;
  /** Path to hooks directory */
  hooksDir: string;
  /** Individual hook statuses */
  hooks: HookInfo[];
}

/** Information about a configured quality check */
export interface CheckConfig {
  /** Check name (e.g., 'eslint', 'test', 'coverage') */
  name: string;
  /** Whether the check is enabled */
  enabled: boolean;
  /** Check-specific configuration */
  options?: Record<string, unknown>;
}

/** Repository profile from .anvilrc */
export interface RepoProfile {
  /** Whether .anvilrc exists */
  hasConfig: boolean;
  /** Path to .anvilrc */
  configPath: string;
  /** Planning directory */
  planningDir?: string;
  /** Detected/configured format */
  format?: string;
  /** Configured quality checks */
  checks: CheckConfig[];
  /** Coverage threshold (if configured) */
  coverageThreshold?: number;
  /** APS schema version */
  schemaVersion?: string;
}

/** Result of a single validation run */
export interface ValidationResult {
  /** Unique ID for this result */
  id: string;
  /** When the validation was run */
  timestamp: Date;
  /** Path to the validated plan */
  planPath: string;
  /** Overall pass/fail status */
  passed: boolean;
  /** Number of checks that passed */
  passedChecks: number;
  /** Total number of checks run */
  totalChecks: number;
  /** Brief summary message */
  summary?: string;
}

/** Recent validation results */
export interface RecentResults {
  /** Whether cache exists */
  hasCache: boolean;
  /** Path to cache directory */
  cacheDir: string;
  /** Last N validation results */
  results: ValidationResult[];
}

/** Complete status data for the dashboard */
export interface StatusData {
  /** Project root path */
  projectRoot: string;
  /** Project name (from package.json) */
  projectName?: string;
  /** Git hooks status */
  hooks: HooksStatus;
  /** Repository profile */
  profile: RepoProfile;
  /** Recent validation results */
  recent: RecentResults;
  /** Timestamp when status was gathered */
  gatheredAt: Date;
}

/** Panel identifier for keyboard navigation */
export type PanelId = 'hooks' | 'profile' | 'results';

/** All panels in order */
export const PANELS: PanelId[] = ['hooks', 'profile', 'results'];

/** Get next panel (wraps around) */
export function getNextPanel(current: PanelId): PanelId {
  const idx = PANELS.indexOf(current);
  return PANELS[(idx + 1) % PANELS.length];
}

/** Get previous panel (wraps around) */
export function getPreviousPanel(current: PanelId): PanelId {
  const idx = PANELS.indexOf(current);
  return PANELS[(idx - 1 + PANELS.length) % PANELS.length];
}
