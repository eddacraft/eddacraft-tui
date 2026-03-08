/**
 * Git Status Checker
 *
 * Utilities for checking git status of files to filter watch events
 * to only unstaged changes.
 */

import { relative, resolve } from 'node:path';
import type { GitFileStatus } from './types.js';
import { createDebugger, gitExec } from '@eddacraft/anvil-core';

const debug = createDebugger('watch');

/**
 * Unescape a git-quoted path. Git wraps paths in double-quotes and
 * uses backslash escapes (e.g. `\t`, `\n`, `\\`, `\"`, `\NNN` for octal bytes).
 */
function unescapeGitQuotedPath(value: string): string {
  if (!value.startsWith('"') || !value.endsWith('"')) {
    return value;
  }

  const inner = value.slice(1, -1);
  let result = '';
  let i = 0;

  while (i < inner.length) {
    const ch = inner[i];

    if (ch === '\\' && i + 1 < inner.length) {
      const next = inner[i + 1];

      switch (next) {
        case '"':
          result += '"';
          i += 2;
          continue;
        case '\\':
          result += '\\';
          i += 2;
          continue;
        case 't':
          result += '\t';
          i += 2;
          continue;
        case 'n':
          result += '\n';
          i += 2;
          continue;
      }

      // Octal escapes: \NNN (1–3 octal digits)
      if (next >= '0' && next <= '7') {
        let octal = '';
        let j = i + 1;
        while (j < inner.length && j - (i + 1) < 3 && inner[j] >= '0' && inner[j] <= '7') {
          octal += inner[j];
          j += 1;
        }
        result += String.fromCharCode(parseInt(octal, 8));
        i = j;
        continue;
      }

      // Fallback: keep the character after backslash as-is
      result += next;
      i += 2;
      continue;
    }

    result += ch;
    i += 1;
  }

  return result;
}

/**
 * Extract the destination path from a rename/copy status line.
 * Handles quoted source paths that may contain ' -> '.
 */
function extractRenamePath(raw: string): string {
  // If the source path is quoted, find the true closing quote (skip escaped quotes)
  if (raw.startsWith('"')) {
    let i = 1;
    while (i < raw.length) {
      if (raw[i] === '\\') {
        i += 2; // skip escaped character
        continue;
      }
      if (raw[i] === '"') {
        break;
      }
      i++;
    }
    if (i < raw.length) {
      const arrow = raw.indexOf(' -> ', i);
      if (arrow !== -1) {
        return raw.substring(arrow + 4);
      }
    }
  }

  // Unquoted: split at first ' -> '
  const arrow = raw.indexOf(' -> ');
  if (arrow !== -1) {
    return raw.substring(arrow + 4);
  }

  return raw;
}

/**
 * Git status checker for filtering watched files
 */
export class GitStatusChecker {
  constructor(private workspaceRoot: string) {}

  /**
   * Check if directory is a git repository
   */
  async isGitRepository(): Promise<boolean> {
    try {
      await gitExec(['rev-parse', '--git-dir'], { cwd: this.workspaceRoot });
      return true;
    } catch (error) {
      debug('Failed to check if directory is a git repository', error);
      return false;
    }
  }

  /**
   * Get git status for a specific file
   *
   * @param filePath - Absolute or relative file path
   * @returns Git file status
   */
  async getFileStatus(filePath: string): Promise<GitFileStatus> {
    const relativePath = this.toRelativePath(filePath);

    try {
      const { stdout } = await gitExec(['status', '--porcelain', '--', relativePath], {
        cwd: this.workspaceRoot,
      });

      return this.parseStatusLine(stdout, relativePath);
    } catch (error) {
      debug('Git status command failed, treating file as untracked', error);
      return {
        path: relativePath,
        isTracked: false,
        isStaged: false,
        isUnstaged: false,
        isUntracked: true,
        statusCode: '??',
      };
    }
  }

  /**
   * Check if file has unstaged changes
   */
  async isUnstaged(filePath: string): Promise<boolean> {
    const status = await this.getFileStatus(filePath);
    return status.isUnstaged;
  }

  /**
   * Check if file is untracked
   */
  async isUntracked(filePath: string): Promise<boolean> {
    const status = await this.getFileStatus(filePath);
    return status.isUntracked;
  }

  /**
   * Get all files with unstaged changes
   */
  async getUnstagedFiles(): Promise<string[]> {
    return this.getFilesByStatusFilter(
      (status) => status.isUnstaged,
      'Failed to get unstaged files from git status'
    );
  }

  /**
   * Get all untracked files
   */
  async getUntrackedFiles(): Promise<string[]> {
    return this.getFilesByStatusFilter(
      (status) => status.isUntracked,
      'Failed to get untracked files from git status'
    );
  }

  /**
   * Get files matching a given git status predicate
   *
   * @param predicate - Function to test each file status
   * @param errorMessage - Message to log if the git command fails
   */
  private async getFilesByStatusFilter(
    predicate: (status: GitFileStatus) => boolean,
    errorMessage: string
  ): Promise<string[]> {
    try {
      const { stdout } = await gitExec(['status', '--porcelain'], {
        cwd: this.workspaceRoot,
      });

      const files: string[] = [];
      const lines = stdout.split('\n').filter((l: string) => l.length >= 3);

      for (const line of lines) {
        const status = this.parseStatusLine(line, '');
        if (predicate(status) && status.path) {
          files.push(resolve(this.workspaceRoot, status.path));
        }
      }

      return files;
    } catch (error) {
      debug(errorMessage, error);
      return [];
    }
  }

  /**
   * Filter file paths to only those with unstaged changes
   *
   * @param filePaths - Array of file paths to filter
   * @param includeUntracked - Whether to include untracked files
   * @returns Filtered array of file paths
   */
  async filterUnstaged(filePaths: string[], includeUntracked = false): Promise<string[]> {
    const results: string[] = [];

    if (filePaths.length === 0) {
      return results;
    }

    // Map the input file paths to a set of absolute paths for fast lookup
    const inputPathsSet = new Set(
      filePaths.map((p) => resolve(this.workspaceRoot, this.toRelativePath(p)))
    );

    try {
      const { stdout } = await gitExec(['status', '--porcelain'], {
        cwd: this.workspaceRoot,
      });

      const lines = stdout.split('\n').filter((l: string) => l.length >= 3);

      for (const line of lines) {
        const status = this.parseStatusLine(line, '');

        if (!status.path) {
          continue;
        }

        const absPath = resolve(this.workspaceRoot, status.path);

        if (!inputPathsSet.has(absPath)) {
          continue;
        }

        if (status.isUnstaged) {
          results.push(absPath);
        } else if (includeUntracked && status.isUntracked) {
          results.push(absPath);
        }
      }

      return results;
    } catch (error) {
      debug('Failed to filter unstaged files from git status', error);
      return [];
    }
  }

  /**
   * Parse git status --porcelain output line
   *
   * Format: XY filename
   * X = status in staging area
   * Y = status in working tree
   *
   * Common codes:
   * ' M' = modified in working tree (unstaged)
   * 'M ' = modified in index (staged)
   * 'MM' = modified in both (staged + unstaged changes)
   * '??' = untracked
   * 'A ' = added in index (staged new file)
   * ' D' = deleted in working tree
   * 'D ' = deleted in index
   */
  private parseStatusLine(line: string, defaultPath: string): GitFileStatus {
    if (!line || line.length < 3) {
      return {
        path: defaultPath,
        isTracked: true,
        isStaged: false,
        isUnstaged: false,
        isUntracked: false,
        statusCode: '',
      };
    }

    const statusCode = line.substring(0, 2);
    let path = line.substring(3).trim() || defaultPath;
    const indexStatus = statusCode[0];
    const workTreeStatus = statusCode[1];

    // Handle rename/copy entries: "R  old.ts -> new.ts" — use the destination
    const isRenameOrCopy =
      indexStatus === 'R' ||
      indexStatus === 'C' ||
      workTreeStatus === 'R' ||
      workTreeStatus === 'C';
    if (isRenameOrCopy && path.includes(' -> ')) {
      path = extractRenamePath(path);
    }

    // Handle git-quoted paths
    path = unescapeGitQuotedPath(path);

    // Check if untracked
    if (statusCode === '??') {
      return {
        path,
        isTracked: false,
        isStaged: false,
        isUnstaged: false,
        isUntracked: true,
        statusCode,
      };
    }

    // Check staging area (index) status
    const isStaged = indexStatus !== ' ' && indexStatus !== '?';

    // Check working tree status
    const isUnstaged = workTreeStatus !== ' ' && workTreeStatus !== '?';

    return {
      path,
      isTracked: true,
      isStaged,
      isUnstaged,
      isUntracked: false,
      statusCode,
    };
  }

  /**
   * Convert absolute path to relative path from workspace root
   */
  private toRelativePath(filePath: string): string {
    if (filePath.startsWith(this.workspaceRoot)) {
      return relative(this.workspaceRoot, filePath);
    }
    return filePath;
  }
}

/**
 * Create a git status checker for the workspace
 */
export function createGitStatusChecker(workspaceRoot: string): GitStatusChecker {
  return new GitStatusChecker(workspaceRoot);
}

/**
 * Options for getChangedFiles
 */
export interface GetChangedFilesOptions {
  /** Include staged files (default: true) */
  staged?: boolean;
  /** Include unstaged files (default: true) */
  unstaged?: boolean;
  /** Include untracked files (default: false) */
  untracked?: boolean;
  /** Compare against git ref (e.g., 'main', 'HEAD~3') */
  since?: string;
  /** Filter to specific extensions (e.g., ['.ts', '.tsx']) */
  extensions?: string[];
}

/**
 * Get changed files from git with flexible filtering
 *
 * @param workspaceRoot - Root directory of the workspace
 * @param options - Options for filtering changed files
 * @returns Array of absolute file paths
 */
export async function getChangedFiles(
  workspaceRoot: string,
  options: GetChangedFilesOptions = {}
): Promise<string[]> {
  const { staged = true, unstaged = true, untracked = false, since, extensions } = options;

  const files = new Set<string>();

  try {
    if (since) {
      const { stdout } = await gitExec(['diff', '--name-only', since], {
        cwd: workspaceRoot,
      });
      const diffFiles = stdout.split('\n').filter(Boolean);
      for (const file of diffFiles) {
        files.add(resolve(workspaceRoot, file));
      }
    } else {
      const { stdout } = await gitExec(['status', '--porcelain'], {
        cwd: workspaceRoot,
      });

      const lines = stdout.split('\n').filter((l: string) => l.length >= 3);

      for (const line of lines) {
        const statusCode = line.substring(0, 2);
        let filePath = line.substring(3).trim();
        const indexStatus = statusCode[0];
        const workTreeStatus = statusCode[1];

        // Handle rename/copy and quoted paths
        const isRenameOrCopy =
          indexStatus === 'R' ||
          indexStatus === 'C' ||
          workTreeStatus === 'R' ||
          workTreeStatus === 'C';
        if (isRenameOrCopy && filePath.includes(' -> ')) {
          filePath = extractRenamePath(filePath);
        }
        filePath = unescapeGitQuotedPath(filePath);

        const isFileStaged = indexStatus !== ' ' && indexStatus !== '?';
        const isFileUnstaged = workTreeStatus !== ' ' && workTreeStatus !== '?';
        const isFileUntracked = statusCode === '??';

        const shouldInclude =
          (staged && isFileStaged) ||
          (unstaged && isFileUnstaged) ||
          (untracked && isFileUntracked);

        if (shouldInclude) {
          files.add(resolve(workspaceRoot, filePath));
        }
      }
    }
  } catch (error) {
    debug('Failed to get changed files from git', error);
    return [];
  }

  let result = Array.from(files);

  if (extensions && extensions.length > 0) {
    result = result.filter((file) => extensions.some((ext) => file.endsWith(ext)));
  }

  return result.sort();
}
