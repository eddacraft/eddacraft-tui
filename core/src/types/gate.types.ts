// Import APSPlan type from schema
import type { APSPlan } from '../schema/index.js';
import type { WatchConfig } from '../watch/types.js';
import type { Warning, WarningResult } from '../antipattern/types.js';
import { existsSync } from 'fs';
import { join, isAbsolute } from 'path';

export interface GateCheck {
  name: string;
  description: string;
  enabled: boolean;
  config?: Record<string, unknown>;
}

/**
 * Extended details that can include warnings from anti-pattern/boundary checks
 */
export interface GateResultDetails {
  /** Warnings from anti-pattern or boundary detection */
  warnings?: WarningResult;
  /** Any other check-specific details */
  [key: string]: unknown;
}

export interface GateResult {
  check: string;
  passed: boolean;
  score?: number;
  message: string;
  details?: GateResultDetails;
  error?: string;
  skipped?: boolean;
}

/**
 * Helper to extract warnings from a GateResult
 */
export function getWarningsFromResult(result: GateResult): Warning[] {
  return result.details?.warnings?.warnings ?? [];
}

/**
 * Helper to check if a GateResult has blocking warnings
 */
export function hasBlockingWarnings(result: GateResult): boolean {
  const warnings = getWarningsFromResult(result);
  return warnings.some((w) => w.severity === 'error' && !w.suppressed);
}

export interface GateRunResult {
  overall: boolean;
  score: number;
  checks: GateResult[];
  summary: {
    total: number;
    passed: number;
    failed: number;
    skipped: number;
  };
}

export interface GateConfig {
  version: number;
  checks: GateCheck[];
  thresholds: {
    overall_score: number;
    [key: string]: number;
  };
  global_config?: Record<string, unknown>;
  /** Watch mode configuration */
  watch?: WatchConfig;
}

// Use APSPlan directly instead of a separate interface
export type PlanData = APSPlan;

export interface CheckContext {
  /** Plan data - optional for planless mode */
  plan?: PlanData;
  workspace_root: string;
  config: GateConfig;
  check_config: Record<string, unknown>;
  /** Full codebase scan mode (no plan-based scoping) */
  fullScan?: boolean;
  /** Explicit file list for planless mode (workspace-relative or absolute paths) */
  targetFiles?: string[];
}

// =============================================================================
// Path Normalisation Helpers
// =============================================================================

/**
 * Options for normalising target files
 */
export interface NormaliseFilesOptions {
  /** Filter function to check if file should be included */
  filter?: (filePath: string) => boolean;
  /** Whether to check file existence (default: true) */
  checkExists?: boolean;
}

/**
 * Normalise target files to absolute paths and filter non-existent files.
 *
 * This ensures consistent behaviour between plan-based and planless modes:
 * - Converts workspace-relative paths to absolute paths
 * - Filters out non-existent files (unless checkExists is false)
 * - Applies optional filter function
 *
 * @param targetFiles - Array of file paths (relative or absolute)
 * @param workspaceRoot - Workspace root directory
 * @param options - Normalisation options
 * @returns Array of absolute paths to existing files
 */
export function normaliseTargetFiles(
  targetFiles: string[],
  workspaceRoot: string,
  options: NormaliseFilesOptions = {}
): string[] {
  const { filter, checkExists = true } = options;

  return targetFiles
    .map((filePath) => {
      // Convert to absolute path if relative
      if (isAbsolute(filePath)) {
        return filePath;
      }
      return join(workspaceRoot, filePath);
    })
    .filter((absolutePath) => {
      // Check existence if required
      if (checkExists && !existsSync(absolutePath)) {
        return false;
      }
      // Apply custom filter if provided
      if (filter && !filter(absolutePath)) {
        return false;
      }
      return true;
    });
}

/**
 * Get files from CheckContext, handling both plan-based and planless modes consistently.
 *
 * For planless mode (targetFiles provided):
 * - Normalises paths to absolute
 * - Filters non-existent files
 * - Applies optional filter function
 *
 * For plan-based mode:
 * - Extracts file paths from proposed_changes
 * - Joins with workspace_root
 * - Filters non-existent files (except for file_delete)
 * - Applies optional filter function
 *
 * @param context - Check context
 * @param options - Options including filter function
 * @returns Array of absolute paths to files
 */
export function getFilesFromContext(
  context: CheckContext,
  options: NormaliseFilesOptions = {}
): string[] {
  const { filter, checkExists = true } = options;

  // Planless mode: use targetFiles
  if (context.targetFiles && context.targetFiles.length > 0) {
    return normaliseTargetFiles(context.targetFiles, context.workspace_root, options);
  }

  // Plan-based mode: extract from proposed_changes
  const files: string[] = [];

  if (context.plan) {
    for (const change of context.plan.proposed_changes) {
      const isFileChange =
        change.type === 'file_create' ||
        change.type === 'file_update' ||
        change.type === 'file_delete';

      if (!isFileChange || !change.path) {
        continue;
      }

      // Apply filter if provided
      if (filter && !filter(change.path)) {
        continue;
      }

      const fullPath = join(context.workspace_root, change.path);

      // For file_delete, don't check existence
      // For other changes, check existence if required
      if (change.type === 'file_delete' || !checkExists || existsSync(fullPath)) {
        files.push(fullPath);
      }
    }
  }

  return files;
}
