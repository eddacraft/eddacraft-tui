/**
 * Types for the `anvil doctor` diagnostics TUI
 */

/** Status of a diagnostic check */
export type DiagnosticStatus = 'pass' | 'warn' | 'fail' | 'skip';

/** Result of running a diagnostic check */
export interface DiagnosticResult {
  /** Check identifier */
  checkId: string;
  /** Human-readable check name */
  name: string;
  /** Result status */
  status: DiagnosticStatus;
  /** Result message */
  message: string;
  /** Whether this issue can be auto-fixed */
  fixable: boolean;
  /** Detailed information (shown in verbose mode) */
  details?: string;
  /** Suggested fix command or action */
  suggestion?: string;
}

/** Interface for diagnostic checks */
export interface DiagnosticCheck {
  /** Unique check identifier */
  readonly id: string;
  /** Human-readable name */
  readonly name: string;
  /** Check description */
  readonly description: string;
  /** Run the diagnostic check */
  run(context: DiagnosticContext): Promise<DiagnosticResult>;
  /** Apply auto-fix (if fixable) */
  fix?(context: DiagnosticContext): Promise<FixResult>;
}

/** Context passed to diagnostic checks */
export interface DiagnosticContext {
  /** Project root directory */
  projectRoot: string;
  /** Path to .anvilrc (if exists) */
  configPath?: string;
  /** Verbose mode */
  verbose: boolean;
}

/** Result of applying a fix */
export interface FixResult {
  /** Whether the fix was successful */
  success: boolean;
  /** Description of what was fixed */
  message: string;
  /** Files that were modified */
  filesModified?: string[];
  /** Commands that were executed */
  commandsRun?: string[];
}

/** Overall diagnostics summary */
export interface DiagnosticsSummary {
  /** Total number of checks run */
  total: number;
  /** Number of checks passed */
  passed: number;
  /** Number of warnings */
  warnings: number;
  /** Number of failures */
  failed: number;
  /** Number of skipped checks */
  skipped: number;
  /** Number of fixable issues */
  fixable: number;
  /** Overall health status */
  healthy: boolean;
}

/** Complete diagnostics data */
export interface DiagnosticsData {
  /** Project root path */
  projectRoot: string;
  /** Individual check results */
  results: DiagnosticResult[];
  /** Summary statistics */
  summary: DiagnosticsSummary;
  /** Timestamp when diagnostics were run */
  ranAt: Date;
}

/** Check categories for grouping */
export type CheckCategory = 'system' | 'config' | 'hooks' | 'permissions';

/** Check metadata for registration */
export interface CheckMetadata {
  /** Check instance */
  check: DiagnosticCheck;
  /** Category for grouping */
  category: CheckCategory;
  /** Run order priority (lower = earlier) */
  priority: number;
}

/** All check categories in display order */
export const CHECK_CATEGORIES: CheckCategory[] = ['system', 'config', 'hooks', 'permissions'];

/** Category display names */
export const CATEGORY_NAMES: Record<CheckCategory, string> = {
  system: 'System Requirements',
  config: 'Configuration',
  hooks: 'Git Hooks',
  permissions: 'File Permissions',
};

/** Calculate summary from results */
export function calculateSummary(results: DiagnosticResult[]): DiagnosticsSummary {
  const passed = results.filter((r) => r.status === 'pass').length;
  const warnings = results.filter((r) => r.status === 'warn').length;
  const failed = results.filter((r) => r.status === 'fail').length;
  const skipped = results.filter((r) => r.status === 'skip').length;
  const fixable = results.filter((r) => r.fixable && r.status !== 'pass').length;

  return {
    total: results.length,
    passed,
    warnings,
    failed,
    skipped,
    fixable,
    healthy: failed === 0,
  };
}
