/**
 * Git Scanner - Scan git history for secrets in recent commits
 *
 * Scans git commit diffs to find secrets that may have been committed in the past.
 */

import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { SECRET_PATTERNS, PatternMatcher } from './secret-patterns.js';

const execFileAsync = promisify(execFile);

/**
 * Secret finding from git history
 */
export interface GitHistoryFinding {
  file: string;
  line: number;
  type: string;
  match: string;
  context: string;
  source: 'git-history';
}

/**
 * Configuration for git scanning
 */
export interface GitScannerConfig {
  /** Number of commits to scan in git history (default: 10) */
  git_history_depth: number;
  /** Patterns to allowlist (reduce false positives) */
  allowlist: string[];
}

/**
 * Git scanner for finding secrets in repository history
 */
export class GitScanner {
  private matcher = new PatternMatcher();

  /**
   * Scan git history for secrets in recent commits
   */
  async scanGitHistory(
    workspaceRoot: string,
    config: GitScannerConfig
  ): Promise<GitHistoryFinding[]> {
    const findings: GitHistoryFinding[] = [];
    // Clamp depth to a sane range (1–1000)
    const depth = Math.max(1, Math.min(1000, Math.floor(config.git_history_depth)));

    try {
      // Check if we're in a git repository (handles worktrees where .git is a file)
      try {
        await execFileAsync('git', ['rev-parse', '--git-dir'], { cwd: workspaceRoot });
      } catch {
        return findings;
      }

      // Get recent commit diffs
      const { stdout } = await execFileAsync(
        'git',
        [
          'log',
          '-p',
          `-${depth}`,
          '--all',
          '--diff-filter=A',
          '--',
          '*.ts',
          '*.js',
          '*.json',
          '*.env*',
          '*.yaml',
          '*.yml',
        ],
        {
          cwd: workspaceRoot,
          maxBuffer: 10 * 1024 * 1024,
          timeout: 60_000,
        }
      );

      // Parse git diff output
      const commitBlocks = stdout.split(/^commit /m).slice(1);

      for (const block of commitBlocks) {
        const commitMatch = block.match(/^([a-f0-9]+)/);
        const commitHash = commitMatch ? commitMatch[1].substring(0, 8) : 'unknown';

        // Find added lines (starting with +)
        const addedLines = block
          .split('\n')
          .filter((line) => line.startsWith('+') && !line.startsWith('+++'));

        for (const addedLine of addedLines) {
          const lineContent = addedLine.substring(1); // Remove the + prefix

          // Check for pattern matches
          for (const pattern of SECRET_PATTERNS) {
            const matches = lineContent.match(pattern.pattern);
            if (matches && !this.matcher.isAllowlisted(matches[0], config.allowlist)) {
              findings.push({
                file: `git-history:${commitHash}`,
                line: 0,
                type: `${pattern.name} (in git history)`,
                match: this.matcher.redactSecret(matches[0]),
                context: this.matcher.redactLine(lineContent.trim()),
                source: 'git-history',
              });
            }
          }
        }
      }
    } catch {
      // Git command failed, likely not a git repo or git not available
      // Silently skip git history scanning
    }

    return findings;
  }
}
