/**
 * Git Scanner - Scan git history for secrets in recent commits
 *
 * Scans git commit diffs to find secrets that may have been committed in the past.
 */

import { exec } from 'child_process';
import { promisify } from 'util';
import { existsSync } from 'fs';
import { join } from 'path';
import { SECRET_PATTERNS, PatternMatcher } from './secret-patterns.js';

const execAsync = promisify(exec);

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
    const depth = config.git_history_depth;

    try {
      // Check if we're in a git repository
      const isGitRepo = existsSync(join(workspaceRoot, '.git'));
      if (!isGitRepo) {
        return findings;
      }

      // Get recent commit diffs
      const { stdout } = await execAsync(
        `git log -p -${depth} --all --diff-filter=A -- '*.ts' '*.js' '*.json' '*.env*' '*.yaml' '*.yml'`,
        {
          cwd: workspaceRoot,
          maxBuffer: 10 * 1024 * 1024,
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
