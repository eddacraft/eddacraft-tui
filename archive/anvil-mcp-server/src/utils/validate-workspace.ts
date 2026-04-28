import { resolve, relative, isAbsolute } from 'node:path';
import { existsSync, statSync, realpathSync } from 'node:fs';

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

/**
 * Validate that a client-supplied workspaceRoot is within the server's
 * configured allowed root. When a server root is configured, the client
 * must request a path that is the server root itself or a subdirectory.
 *
 * For stdio transport (no configured root), this is a no-op.
 */
export function validateWorkspaceRootAgainstServer(
  clientRoot: string,
  serverRoot: string | undefined
): string {
  const resolved = validateWorkspaceRoot(clientRoot);

  if (!serverRoot) {
    return resolved;
  }

  const resolvedServer = resolve(serverRoot);

  // Resolve symlinks for both to prevent escaping via symlinks
  let realClient: string;
  let realServer: string;
  try {
    realClient = realpathSync(resolved);
    realServer = realpathSync(resolvedServer);
  } catch {
    throw new Error(`workspaceRoot "${clientRoot}" could not be validated against server root`);
  }

  const rel = relative(realServer, realClient);
  if (realClient !== realServer && (rel.startsWith('..') || isAbsolute(rel))) {
    throw new Error(
      `workspaceRoot "${clientRoot}" is outside the server's allowed root. ` +
        `The server is configured for: ${serverRoot}`
    );
  }

  return resolved;
}
