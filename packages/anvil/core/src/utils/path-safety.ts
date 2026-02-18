import * as path from 'node:path';

/**
 * Sanitize an identifier (e.g. snapshot name, record ID) to prevent path traversal.
 * Extracts only the basename and rejects directory separators, null bytes, and dot-only names.
 *
 * @throws Error if the identifier contains directory separators or is otherwise unsafe
 */
export function sanitizeIdentifier(identifier: string): string {
  const basename = path.basename(identifier);

  if (basename !== identifier) {
    throw new Error(`Invalid identifier: contains path separators: ${identifier}`);
  }

  if (!basename || basename === '.' || basename === '..' || basename.includes('\0')) {
    throw new Error(`Invalid identifier: ${identifier}`);
  }

  return basename;
}

/**
 * Validate that a resolved target path is within the expected root directory.
 * Resolves both paths to absolute form before comparing.
 *
 * @throws Error if the target path escapes the root directory
 */
export function validatePathWithinRoot(targetPath: string, rootDir: string): string {
  const resolvedRoot = path.resolve(rootDir);
  const resolvedTarget = path.resolve(rootDir, targetPath);

  if (resolvedTarget !== resolvedRoot && !resolvedTarget.startsWith(resolvedRoot + path.sep)) {
    throw new Error(`Path escapes root directory: ${targetPath}`);
  }

  return resolvedTarget;
}

/**
 * Validate that a path is relative and does not escape upward via `../` sequences or absolute prefixes.
 *
 * @throws Error if the path is absolute or contains `..` segments
 */
export function validateRelativePath(relPath: string): string {
  if (path.isAbsolute(relPath)) {
    throw new Error(`Expected relative path, got absolute: ${relPath}`);
  }

  if (relPath.includes('\0')) {
    throw new Error(`Path contains null byte: ${relPath}`);
  }

  const normalized = path.normalize(relPath);
  if (normalized.startsWith('..') || normalized.startsWith(path.sep)) {
    throw new Error(`Path escapes parent directory: ${relPath}`);
  }

  return normalized.replaceAll('\\', '/');
}
