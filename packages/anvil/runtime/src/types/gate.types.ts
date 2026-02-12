/**
 * Gate Types for @eddacraft/anvil-runtime
 *
 * Re-exports types from @eddacraft/anvil-contracts and provides runtime utility functions.
 */

import { existsSync } from 'node:fs';
import { join, isAbsolute } from 'node:path';
import type { CheckContext, NormaliseFilesOptions } from '@eddacraft/anvil-core';

// Re-export only contract types (not full core surface) to keep runtime API stable
export type {
  APSPlan,
  Change,
  ChangeType,
  Provenance,
  Validation,
  EvidenceEntry,
  Evidence,
  Approval,
  ExecutionResult,
  SchemaValidationResult,
  GateConfig,
  GateCheck,
  GateResult,
  GateResultDetails,
  GateRunResult,
  CheckContext,
  PlanData,
  WatchConfig,
  PolicyConfig,
  PolicyBundleConfig,
  PolicyVerificationConfig,
  SignatureAlgorithm,
  StackConfig,
  StackLayerConfig,
  StackValidationConfig,
  ArchitectureContextBase,
  NormaliseFilesOptions,
} from '@eddacraft/anvil-core';

export {
  APSPlanSchema,
  APS_SCHEMA_VERSION,
  ChangeTypeSchema,
  ChangeSchema,
  ProvenanceSchema,
  ValidationSchema,
  EvidenceEntrySchema,
  EvidenceSchema,
  ApprovalSchema,
  ExecutionResultSchema,
  validatePlan,
  createPlan,
  generateJSONSchema,
  getWarningsFromResult,
  hasBlockingWarnings,
} from '@eddacraft/anvil-core';

// =============================================================================
// Path Normalisation Helpers (runtime-specific with fs operations)
// =============================================================================

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
