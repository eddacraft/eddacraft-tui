/**
 * Git Agent Identification
 *
 * Utilities for identifying agents via git commit metadata.
 * Supports reading and writing agent identification through commit trailers.
 */

import { execSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { createAgentInfo } from './agent.js';
import type { AgentType, AgentInfo } from './types.js';
import { createDebugger } from '@eddacraft/anvil-core';

const debug = createDebugger('git-agent');

// ============================================================================
// Trailer Constants
// ============================================================================

/**
 * Git trailer keys used for agent identification
 */
export const GIT_TRAILERS = {
  /** Agent ID trailer */
  AGENT_ID: 'Anvil-Agent-ID',

  /** Agent type trailer */
  AGENT_TYPE: 'Anvil-Agent-Type',

  /** Session ID trailer */
  SESSION_ID: 'Anvil-Session-ID',

  /** Agent name trailer */
  AGENT_NAME: 'Anvil-Agent-Name',

  /** Co-authored-by for agent attribution */
  CO_AUTHORED_BY: 'Co-authored-by',
} as const;

// ============================================================================
// Commit Trailer Utilities
// ============================================================================

/**
 * Agent info extracted from a commit
 */
export interface CommitAgentInfo {
  /** Agent ID from trailer */
  agentId?: string;

  /** Agent type from trailer */
  agentType?: AgentType;

  /** Session ID from trailer */
  sessionId?: string;

  /** Agent name from trailer */
  agentName?: string;

  /** Whether commit was made by an AI agent */
  isAiGenerated: boolean;

  /** Co-authors (may include AI attribution) */
  coAuthors: string[];

  /** Raw trailers from commit */
  trailers: Record<string, string>;
}

/**
 * Parse trailers from commit message
 */
export function parseCommitTrailers(commitMessage: string): Record<string, string> {
  const trailers: Record<string, string> = {};
  const lines = commitMessage.split('\n');

  // Trailers are at the end, after a blank line
  let inTrailers = false;

  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i].trim();

    if (line === '') {
      if (inTrailers) break;
      inTrailers = true;
      continue;
    }

    if (inTrailers) {
      const match = line.match(/^([A-Za-z][A-Za-z0-9-]*)\s*:\s*(.+)$/);
      if (match) {
        trailers[match[1]] = match[2];
      }
    }
  }

  return trailers;
}

/**
 * Extract agent info from commit trailers
 */
export function extractAgentInfo(trailers: Record<string, string>): CommitAgentInfo {
  const agentId = trailers[GIT_TRAILERS.AGENT_ID];
  const agentTypeStr = trailers[GIT_TRAILERS.AGENT_TYPE];
  const sessionId = trailers[GIT_TRAILERS.SESSION_ID];
  const agentName = trailers[GIT_TRAILERS.AGENT_NAME];

  // Check for AI indicators
  const isAiGenerated =
    !!agentId ||
    !!agentTypeStr ||
    Object.values(trailers).some(
      (v) =>
        v.toLowerCase().includes('ai') ||
        v.toLowerCase().includes('claude') ||
        v.toLowerCase().includes('copilot') ||
        v.toLowerCase().includes('cursor')
    );

  // Extract co-authors
  const coAuthors: string[] = [];
  for (const [key, value] of Object.entries(trailers)) {
    if (key.toLowerCase() === 'co-authored-by') {
      coAuthors.push(value);
    }
  }

  return {
    agentId,
    agentType: agentTypeStr as AgentType | undefined,
    sessionId,
    agentName,
    isAiGenerated,
    coAuthors,
    trailers,
  };
}

/**
 * Get agent info from a specific commit
 */
export function getCommitAgentInfo(commitRef: string, cwd?: string): CommitAgentInfo | null {
  try {
    const message = execSync(`git log -1 --format=%B ${commitRef}`, {
      cwd,
      encoding: 'utf-8',
      stdio: ['pipe', 'pipe', 'pipe'],
    });

    const trailers = parseCommitTrailers(message);
    return extractAgentInfo(trailers);
  } catch (error) {
    debug(`Failed to get commit agent info for ${commitRef}:`, error);
    return null;
  }
}

/**
 * Get agent info from recent commits
 */
export function getRecentCommitsAgentInfo(
  count: number = 10,
  cwd?: string
): Array<{ hash: string; info: CommitAgentInfo }> {
  const results: Array<{ hash: string; info: CommitAgentInfo }> = [];

  try {
    const hashes = execSync(`git log -${count} --format=%H`, {
      cwd,
      encoding: 'utf-8',
      stdio: ['pipe', 'pipe', 'pipe'],
    })
      .trim()
      .split('\n');

    for (const hash of hashes) {
      const info = getCommitAgentInfo(hash, cwd);
      if (info) {
        results.push({ hash, info });
      }
    }
  } catch (error) {
    debug('Failed to get recent commits:', error);
  }

  return results;
}

// ============================================================================
// Commit Message Formatting
// ============================================================================

/**
 * Options for formatting a commit message with agent info
 */
export interface FormatCommitOptions {
  /** Original commit message */
  message: string;

  /** Agent info (auto-detected if not provided) */
  agent?: AgentInfo;

  /** Include co-authored-by trailer */
  includeCoAuthor?: boolean;

  /** Additional trailers to include */
  additionalTrailers?: Record<string, string>;
}

/**
 * Format a commit message with agent identification trailers
 */
export function formatCommitWithAgent(options: FormatCommitOptions): string {
  const {
    message,
    agent = createAgentInfo(),
    includeCoAuthor = true,
    additionalTrailers = {},
  } = options;

  const trailers: string[] = [];

  // Add agent trailers
  trailers.push(`${GIT_TRAILERS.AGENT_ID}: ${agent.id}`);
  trailers.push(`${GIT_TRAILERS.AGENT_TYPE}: ${agent.type}`);

  if (agent.sessionId) {
    trailers.push(`${GIT_TRAILERS.SESSION_ID}: ${agent.sessionId}`);
  }

  if (agent.name) {
    trailers.push(`${GIT_TRAILERS.AGENT_NAME}: ${agent.name}`);
  }

  // Add co-authored-by for AI agents
  if (includeCoAuthor && agent.type !== 'human') {
    const coAuthorName = getAgentCoAuthorName(agent);
    trailers.push(`${GIT_TRAILERS.CO_AUTHORED_BY}: ${coAuthorName}`);
  }

  // Add additional trailers
  for (const [key, value] of Object.entries(additionalTrailers)) {
    trailers.push(`${key}: ${value}`);
  }

  // Ensure message has proper spacing before trailers
  const trimmedMessage = message.trimEnd();
  const trailersBlock = trailers.join('\n');

  // Check if message already has trailers
  const existingTrailers = parseCommitTrailers(trimmedMessage);
  const hasExistingTrailers = Object.keys(existingTrailers).length > 0;

  if (hasExistingTrailers) {
    // Append to existing trailers
    return `${trimmedMessage}\n${trailersBlock}`;
  }

  // Add blank line before trailers
  return `${trimmedMessage}\n\n${trailersBlock}`;
}

/**
 * Get co-author name for an agent
 */
function getAgentCoAuthorName(agent: AgentInfo): string {
  switch (agent.type) {
    case 'claude':
      return 'Claude <claude@anthropic.com>';
    case 'cursor':
      return 'Cursor AI <ai@cursor.sh>';
    case 'copilot':
      return 'GitHub Copilot <copilot@github.com>';
    case 'aider':
      return 'Aider <aider@aider.chat>';
    case 'continue':
      return 'Continue <ai@continue.dev>';
    case 'codeium':
      return 'Codeium <ai@codeium.com>';
    default:
      return `AI Agent (${agent.id}) <noreply@example.com>`;
  }
}

// ============================================================================
// Git Hook Helpers
// ============================================================================

/**
 * Prepare commit message hook helper
 *
 * Automatically adds agent trailers to commit messages.
 * Can be used in a prepare-commit-msg hook.
 */
export function prepareCommitMsgHook(
  commitMsgFile: string,
  commitSource?: string,
  _sha1?: string
): void {
  // Only modify non-merge, non-squash commits
  if (commitSource === 'merge' || commitSource === 'squash') {
    return;
  }

  // Read original message
  const originalMessage = readFileSync(commitMsgFile, 'utf-8');

  // Check if already has agent trailers
  const trailers = parseCommitTrailers(originalMessage);
  if (trailers[GIT_TRAILERS.AGENT_ID]) {
    return; // Already has agent info
  }

  // Add agent info
  const agent = createAgentInfo();

  // Only add for AI agents
  if (agent.type === 'human' || agent.type === 'unknown') {
    return;
  }

  const modifiedMessage = formatCommitWithAgent({
    message: originalMessage,
    agent,
  });

  writeFileSync(commitMsgFile, modifiedMessage);
}

/**
 * Generate a git config command to set up the prepare-commit-msg hook
 */
export function getHookSetupCommand(): string {
  return `
# Add to .git/hooks/prepare-commit-msg:
#!/bin/sh
COMMIT_MSG_FILE=$1
COMMIT_SOURCE=$2
SHA1=$3

# Check if anvil is available
if command -v anvil &> /dev/null; then
  anvil hooks prepare-commit-msg "$COMMIT_MSG_FILE" "$COMMIT_SOURCE" "$SHA1"
fi
`.trim();
}

// ============================================================================
// Authorship Analysis
// ============================================================================

/**
 * Summary of agent contributions
 */
export interface AgentContributionSummary {
  /** Agent ID */
  agentId: string;

  /** Agent type */
  agentType: AgentType;

  /** Number of commits */
  commitCount: number;

  /** First commit timestamp */
  firstCommit: string;

  /** Last commit timestamp */
  lastCommit: string;

  /** Files touched (if available) */
  filesTouched?: number;
}

/**
 * Get contribution summary by agent
 */
export function getAgentContributions(
  sinceRef?: string,
  cwd?: string
): Map<string, AgentContributionSummary> {
  const contributions = new Map<string, AgentContributionSummary>();

  try {
    const sinceArg = sinceRef ? `${sinceRef}..HEAD` : '';
    const format = '%H|%aI'; // hash|timestamp

    const output = execSync(`git log ${sinceArg} --format="${format}"`, {
      cwd,
      encoding: 'utf-8',
      stdio: ['pipe', 'pipe', 'pipe'],
    }).trim();

    if (!output) return contributions;

    const lines = output.split('\n');

    for (const line of lines) {
      const [hash, timestamp] = line.split('|');
      const info = getCommitAgentInfo(hash, cwd);

      if (!info?.agentId) continue;

      const existing = contributions.get(info.agentId);

      if (existing) {
        existing.commitCount++;
        if (timestamp < existing.firstCommit) {
          existing.firstCommit = timestamp;
        }
        if (timestamp > existing.lastCommit) {
          existing.lastCommit = timestamp;
        }
      } else {
        contributions.set(info.agentId, {
          agentId: info.agentId,
          agentType: info.agentType ?? 'unknown',
          commitCount: 1,
          firstCommit: timestamp,
          lastCommit: timestamp,
        });
      }
    }
  } catch (error) {
    debug('Failed to get agent contributions:', error);
  }

  return contributions;
}

/**
 * Get percentage of AI-generated commits in a range
 */
export function getAiCommitPercentage(sinceRef?: string, cwd?: string): number {
  try {
    const sinceArg = sinceRef ? `${sinceRef}..HEAD` : '';

    const totalCount = parseInt(
      execSync(`git rev-list --count ${sinceArg || 'HEAD'}`, {
        cwd,
        encoding: 'utf-8',
        stdio: ['pipe', 'pipe', 'pipe'],
      }).trim(),
      10
    );

    if (totalCount === 0) return 0;

    const commits = getRecentCommitsAgentInfo(totalCount, cwd);
    const aiCount = commits.filter((c) => c.info.isAiGenerated).length;

    return (aiCount / totalCount) * 100;
  } catch (error) {
    debug('Failed to calculate AI commit percentage:', error);
    return 0;
  }
}
