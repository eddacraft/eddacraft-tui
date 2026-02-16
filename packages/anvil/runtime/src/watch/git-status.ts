/**
 * Git Status Checker
 *
 * Utilities for checking git status of files to filter watch events
 * to only unstaged changes.
 */

import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { relative, resolve } from 'node:path';
import type { GitFileStatus } from './types.js';
import { createDebugger } from '@eddacraft/anvil-core';

const debug = createDebugger('gate');

const execFileAsync = promisify(execFile);

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
      await execFileAsync('git', ['rev-parse', '--git-dir'], {
        cwd: this.workspaceRoot,
      });
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
      const { stdout } = await execFileAsync('git', ['status', '--porcelain', '--', relativePath], {
        cwd: this.workspaceRoot,
      });

      return this.parseStatusLine(stdout.trim(), relativePath);
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
    try {
      const { stdout } = await execFileAsync('git', ['status', '--porcelain'], {
        cwd: this.workspaceRoot,
      });

      const files: string[] = [];
      const lines = stdout.trim().split('\n').filter(Boolean);

      for (const line of lines) {
        const status = this.parseStatusLine(line, '');
        if (status.isUnstaged && status.path) {
          files.push(resolve(this.workspaceRoot, status.path));
        }
      }

      return files;
    } catch (error) {
      debug('Failed to get unstaged files from git status', error);
      return [];
    }
  }

  /**
   * Get all untracked files
   */
  async getUntrackedFiles(): Promise<string[]> {
    try {
      const { stdout } = await execFileAsync('git', ['status', '--porcelain'], {
        cwd: this.workspaceRoot,
      });

      const files: string[] = [];
      const lines = stdout.trim().split('\n').filter(Boolean);

      for (const line of lines) {
        const status = this.parseStatusLine(line, '');
        if (status.isUntracked && status.path) {
          files.push(resolve(this.workspaceRoot, status.path));
        }
      }

      return files;
    } catch (error) {
      debug('Failed to get untracked files from git status', error);
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

    for (const filePath of filePaths) {
      const status = await this.getFileStatus(filePath);

      if (status.isUnstaged) {
        results.push(filePath);
      } else if (includeUntracked && status.isUntracked) {
        results.push(filePath);
      }
    }

    return results;
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
    const path = line.substring(3).trim() || defaultPath;

    const indexStatus = statusCode[0];
    const workTreeStatus = statusCode[1];

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
      const { stdout } = await execFileAsync('git', ['diff', '--name-only', since], {
        cwd: workspaceRoot,
      });
      const diffFiles = stdout.trim().split('\n').filter(Boolean);
      for (const file of diffFiles) {
        files.add(resolve(workspaceRoot, file));
      }
    } else {
      const { stdout } = await execFileAsync('git', ['status', '--porcelain'], {
        cwd: workspaceRoot,
      });

      const lines = stdout.trim().split('\n').filter(Boolean);

      for (const line of lines) {
        if (line.length < 3) continue;

        const statusCode = line.substring(0, 2);
        const filePath = line.substring(3).trim();
        const indexStatus = statusCode[0];
        const workTreeStatus = statusCode[1];

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
