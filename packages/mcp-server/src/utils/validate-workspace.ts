import { resolve } from 'node:path';
import { existsSync, statSync } from 'node:fs';

/**
 * Validate that a workspaceRoot parameter points to an existing directory
 * and is an absolute path. Returns the resolved path or throws.
 */
export function validateWorkspaceRoot(workspaceRoot: string): string {
  const resolved = resolve(workspaceRoot);

  if (resolved !== workspaceRoot) {
    throw new Error(`workspaceRoot must be an absolute path, got: ${workspaceRoot}`);
  }

  if (!existsSync(resolved)) {
    throw new Error(`workspaceRoot does not exist: ${resolved}`);
  }

  const stat = statSync(resolved);
  if (!stat.isDirectory()) {
    throw new Error(`workspaceRoot is not a directory: ${resolved}`);
  }

  return resolved;
}
