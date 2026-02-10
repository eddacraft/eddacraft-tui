import { resolve, isAbsolute } from 'node:path';
import { existsSync, statSync } from 'node:fs';

/**
 * Validate that a workspaceRoot parameter points to an existing directory
 * and is an absolute path. Returns the resolved path or throws.
 */
export function validateWorkspaceRoot(workspaceRoot: string): string {
  if (!isAbsolute(workspaceRoot)) {
    throw new Error(`workspaceRoot must be an absolute path, got: ${workspaceRoot}`);
  }

  const resolved = resolve(workspaceRoot);

  if (!existsSync(resolved)) {
    throw new Error(`workspaceRoot does not exist: ${resolved}`);
  }

  const stat = statSync(resolved);
  if (!stat.isDirectory()) {
    throw new Error(`workspaceRoot is not a directory: ${resolved}`);
  }

  return resolved;
}
