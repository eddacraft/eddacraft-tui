/**
 * Git Status Checker
 *
 * Utilities for checking git status of files to filter watch events
 * to only unstaged changes.
 */

import { exec } from 'child_process';
import { promisify } from 'util';
import { relative, resolve } from 'path';
import type { GitFileStatus } from './types.js';

const execAsync = promisify(exec);

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
      await execAsync('git rev-parse --git-dir', {
        cwd: this.workspaceRoot,
      });
      return true;
    } catch {
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
      const { stdout } = await execAsync(`git status --porcelain -- "${relativePath}"`, {
        cwd: this.workspaceRoot,
      });

      return this.parseStatusLine(stdout.trim(), relativePath);
    } catch {
      // Git command failed - treat as untracked
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
      const { stdout } = await execAsync('git status --porcelain', {
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
    } catch {
      return [];
    }
  }

  /**
   * Get all untracked files
   */
  async getUntrackedFiles(): Promise<string[]> {
    try {
      const { stdout } = await execAsync('git status --porcelain', {
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
    } catch {
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
