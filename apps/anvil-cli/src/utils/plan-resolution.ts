/**
 * Plan resolution utilities
 *
 * Shared utilities for resolving plan IDs to file paths.
 */

import { existsSync } from 'fs';
import { resolve } from 'path';
import { findPlanById, getWorkspaceRoot } from './file-io.js';

/**
 * Resolved plan information
 */
export interface ResolvedPlan {
  /** The absolute path to the plan file */
  path: string;
  /** Whether the input was a plan ID (vs. a file path) */
  wasId: boolean;
}

/**
 * Resolve a plan path or ID to an absolute file path.
 *
 * Accepts either:
 * - A plan ID (format: 'aps-XXXXXXXX') - will be resolved to the corresponding file in .anvil/plans
 * - A file path (relative or absolute) - will be validated for existence and converted to absolute
 *
 * @param planPathOrId - The plan ID or file path
 * @param workspaceRoot - Optional workspace root (defaults to auto-detected)
 * @returns Resolved plan information with absolute path
 * @throws {Error} If plan ID is not found or file path doesn't exist
 *
 * @example
 * ```typescript
 * // Resolve plan ID
 * const plan = resolvePlanPathOrId('aps-abc12345');
 * // Returns: { path: '/path/to/.anvil/plans/aps-abc12345.json', wasId: true }
 *
 * // Resolve file path
 * const plan = resolvePlanPathOrId('./my-plan.json');
 * // Returns: { path: '/absolute/path/to/my-plan.json', wasId: false }
 * ```
 */
export function resolvePlanPathOrId(planPathOrId: string, workspaceRoot?: string): ResolvedPlan {
  // Check if it's a plan ID (starts with 'aps-')
  if (planPathOrId.startsWith('aps-')) {
    const root = workspaceRoot || getWorkspaceRoot();
    const resolvedPath = findPlanById(planPathOrId, root);

    if (!resolvedPath) {
      throw new Error(`Plan with ID '${planPathOrId}' not found`);
    }

    return {
      path: resolvedPath,
      wasId: true,
    };
  }

  // It's a file path - validate it exists
  if (!existsSync(planPathOrId)) {
    throw new Error(`Plan file not found: ${planPathOrId}`);
  }

  // Convert to absolute path
  return {
    path: resolve(planPathOrId),
    wasId: false,
  };
}
