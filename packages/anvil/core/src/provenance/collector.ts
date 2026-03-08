import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { randomUUID } from 'node:crypto';
import type { AITool, Environment, GitContext, CheckSummary, ProvenanceRecord } from './types.js';
import type { GateRunResult } from '../contracts/index.js';
import { createDebugger } from '../utils/debug.js';
import {
  generateSessionHash,
  detectCurrentAgent,
  writeAuthorshipNote,
  SCHEMA_VERSION,
  type AuthorshipLog,
  type PromptRecord,
  type Message,
} from './git-ai-standard/index.js';

const debug = createDebugger('provenance');

const execFileAsync = promisify(execFile);

function generateProvenanceId(): string {
  return `prov-${randomUUID()}`;
}

/**
 * Collects environment information
 */
export async function collectEnvironment(workspaceRoot: string): Promise<Environment> {
  let anvilVersion = '0.0.0';

  // Try to get Anvil version from package.json
  try {
    const pkgPath = join(workspaceRoot, 'node_modules', '@anvil', 'cli', 'package.json');
    if (existsSync(pkgPath)) {
      const pkg = JSON.parse(readFileSync(pkgPath, 'utf-8'));
      anvilVersion = pkg.version || '0.0.0';
    }
  } catch (error) {
    debug('Failed to read Anvil CLI package.json for version', error);
  }

  return {
    os: `${process.platform}-${process.arch}`,
    node_version: process.version,
    anvil_version: anvilVersion,
    cwd: workspaceRoot,
    ci: detectCIEnvironment(),
    ci_provider: detectCIProvider(),
  };
}

/**
 * Detects if running in a CI environment
 */
function detectCIEnvironment(): boolean {
  return !!(
    process.env.CI ||
    process.env.CONTINUOUS_INTEGRATION ||
    process.env.BUILD_NUMBER ||
    process.env.GITHUB_ACTIONS ||
    process.env.GITLAB_CI ||
    process.env.CIRCLECI ||
    process.env.JENKINS_URL ||
    process.env.TRAVIS
  );
}

/**
 * Detects the CI provider
 */
function detectCIProvider(): string | undefined {
  if (process.env.GITHUB_ACTIONS) return 'github-actions';
  if (process.env.GITLAB_CI) return 'gitlab-ci';
  if (process.env.CIRCLECI) return 'circleci';
  if (process.env.JENKINS_URL) return 'jenkins';
  if (process.env.TRAVIS) return 'travis-ci';
  if (process.env.BUILDKITE) return 'buildkite';
  if (process.env.AZURE_PIPELINES) return 'azure-pipelines';
  if (process.env.BITBUCKET_PIPELINE) return 'bitbucket-pipelines';
  return undefined;
}

/**
 * Collects git context information
 */
export async function collectGitContext(workspaceRoot: string): Promise<GitContext | undefined> {
  // Check if this is a git repository
  if (!existsSync(join(workspaceRoot, '.git'))) {
    return undefined;
  }

  try {
    const [branch, commit, commitMessage, author, status, stagedFiles] = await Promise.all([
      execFileAsync('git', ['rev-parse', '--abbrev-ref', 'HEAD'], {
        cwd: workspaceRoot,
        timeout: 30_000,
      })
        .then((r) => r.stdout.trim())
        .catch(() => undefined),
      execFileAsync('git', ['rev-parse', 'HEAD'], { cwd: workspaceRoot, timeout: 30_000 })
        .then((r) => r.stdout.trim())
        .catch(() => undefined),
      execFileAsync('git', ['log', '-1', '--format=%s'], { cwd: workspaceRoot, timeout: 30_000 })
        .then((r) => r.stdout.trim())
        .catch(() => undefined),
      execFileAsync('git', ['log', '-1', '--format=%an <%ae>'], {
        cwd: workspaceRoot,
        timeout: 30_000,
      })
        .then((r) => r.stdout.trim())
        .catch(() => undefined),
      execFileAsync('git', ['status', '--porcelain'], { cwd: workspaceRoot, timeout: 30_000 })
        .then((r) => r.stdout.trim())
        .catch(() => ''),
      execFileAsync('git', ['diff', '--name-only', '--cached'], {
        cwd: workspaceRoot,
        timeout: 30_000,
      })
        .then((r) =>
          r.stdout
            .trim()
            .split('\n')
            .filter((f) => f)
        )
        .catch(() => []),
    ]);

    // Parse modified files from git status --porcelain
    // Format is "XY filename" where X and Y are status chars (may be spaces)
    // Use regex to extract filename after the 2-char status + space
    const modifiedFiles = status
      .split('\n')
      .filter((line) => line.trim())
      .map((line) => line.replace(/^.{0,2}\s/, '').trim())
      .filter((f) => f);

    // Try to get repository URL
    let repository: string | undefined;
    try {
      const { stdout } = await execFileAsync('git', ['remote', 'get-url', 'origin'], {
        cwd: workspaceRoot,
        timeout: 30_000,
      });
      repository = stdout.trim();
    } catch (error) {
      debug('No git remote configured or failed to get remote URL', error);
    }

    return {
      repository,
      branch,
      commit,
      commit_message: commitMessage,
      author,
      dirty: status.length > 0,
      staged_files: stagedFiles.length > 0 ? stagedFiles : undefined,
      modified_files: modifiedFiles.length > 0 ? modifiedFiles : undefined,
    };
  } catch (error) {
    debug('Failed to collect git context', error);
    return undefined;
  }
}

/**
 * Detects which AI coding tool might have been used
 */
export async function detectAITool(workspaceRoot: string): Promise<AITool | undefined> {
  const indicators: string[] = [];
  let name: AITool['name'] = 'unknown';
  let confidence: AITool['confidence'] = 'low';

  // Check for Cursor
  if (existsSync(join(workspaceRoot, '.cursor'))) {
    indicators.push('.cursor directory present');
    name = 'cursor';
    confidence = 'high';
  }

  // Check for Copilot
  if (existsSync(join(workspaceRoot, '.github', 'copilot'))) {
    indicators.push('.github/copilot directory present');
    name = 'copilot';
    confidence = 'medium';
  }

  // Check VS Code settings for AI tools
  const vscodePath = join(workspaceRoot, '.vscode', 'settings.json');
  if (existsSync(vscodePath)) {
    try {
      const settings = readFileSync(vscodePath, 'utf-8');
      if (settings.includes('github.copilot')) {
        indicators.push('VS Code Copilot settings found');
        if (name === 'unknown') {
          name = 'copilot';
          confidence = 'medium';
        }
      }
      if (settings.includes('cursor')) {
        indicators.push('VS Code Cursor settings found');
        if (name === 'unknown') {
          name = 'cursor';
          confidence = 'medium';
        }
      }
    } catch (error) {
      debug('Failed to parse VS Code settings.json for AI tool detection', error);
    }
  }

  // Check for Claude Code (CLAUDE.md)
  if (existsSync(join(workspaceRoot, 'CLAUDE.md'))) {
    indicators.push('CLAUDE.md present');
    if (name === 'unknown') {
      name = 'claude-code';
      confidence = 'high';
    }
  }

  // Check environment variables
  if (process.env.CURSOR_SESSION) {
    indicators.push('CURSOR_SESSION env var present');
    name = 'cursor';
    confidence = 'high';
  }

  if (process.env.GITHUB_COPILOT_TOKEN) {
    indicators.push('GITHUB_COPILOT_TOKEN env var present');
    name = 'copilot';
    confidence = 'high';
  }

  // Check recent git commits for AI indicators
  try {
    const { stdout } = await execFileAsync('git', ['log', '-5', '--format=%s'], {
      cwd: workspaceRoot,
      timeout: 30_000,
    });
    const messages = stdout.toLowerCase();

    if (messages.includes('generated by cursor') || messages.includes('cursor:')) {
      indicators.push('Recent commit mentions Cursor');
      name = 'cursor';
      confidence = 'medium';
    }
    if (messages.includes('copilot') || messages.includes('generated by github')) {
      indicators.push('Recent commit mentions Copilot');
      name = 'copilot';
      confidence = 'medium';
    }
    if (messages.includes('claude') || messages.includes('anthropic')) {
      indicators.push('Recent commit mentions Claude');
      name = 'claude-code';
      confidence = 'medium';
    }
  } catch (error) {
    debug('Git log command failed while detecting AI tool from commits', error);
  }

  // If no indicators found, return undefined
  if (indicators.length === 0) {
    return undefined;
  }

  return {
    name,
    confidence,
    indicators,
  };
}

/**
 * Converts gate results to check summaries
 */
export function summariseChecks(results: GateRunResult): CheckSummary[] {
  return results.checks.map((check) => ({
    name: check.check,
    passed: check.passed,
    score: check.score,
    issues_count: check.details?.findings
      ? (check.details.findings as unknown[]).length
      : undefined,
  }));
}

/**
 * Creates a full provenance record from a gate run
 */
export async function createProvenanceRecord(params: {
  workspaceRoot: string;
  filesChecked: string[];
  scope: ProvenanceRecord['scope'];
  results: GateRunResult;
  trigger: ProvenanceRecord['trigger'];
  startTime: number;
  planId?: string;
  parentId?: string;
}): Promise<ProvenanceRecord> {
  const { workspaceRoot, filesChecked, scope, results, trigger, startTime, planId, parentId } =
    params;

  const endTime = Date.now();

  // Collect all context in parallel
  const [environment, gitContext, aiTool] = await Promise.all([
    collectEnvironment(workspaceRoot),
    collectGitContext(workspaceRoot),
    detectAITool(workspaceRoot),
  ]);

  // Get current user
  let user: string | undefined;
  try {
    const { stdout } = await execFileAsync('git', ['config', 'user.name'], {
      cwd: workspaceRoot,
      timeout: 30_000,
    });
    user = stdout.trim() || undefined;
  } catch (error) {
    debug('Failed to get git user.name, falling back to env vars', error);
    user = process.env.USER || process.env.USERNAME;
  }

  return {
    id: generateProvenanceId(),
    timestamp: new Date().toISOString(),

    scope,
    files_checked: filesChecked.map((f) => f.replace(workspaceRoot, '').replace(/^\//, '')),
    files_count: filesChecked.length,

    overall_passed: results.overall,
    overall_score: results.score,
    checks: summariseChecks(results),

    environment,
    git: gitContext,
    ai_tool: aiTool,

    plan_id: planId,
    parent_id: parentId,

    trigger,
    duration_ms: endTime - startTime,
    user,
  };
}

/**
 * Formats a provenance record for display
 */
export function formatProvenanceRecord(record: ProvenanceRecord): string {
  const lines: string[] = [];

  lines.push(`Provenance Record: ${record.id}`);
  lines.push(`${'─'.repeat(50)}`);
  lines.push(`Timestamp: ${record.timestamp}`);
  lines.push(`Duration: ${record.duration_ms}ms`);
  lines.push(`Trigger: ${record.trigger}`);
  lines.push('');

  lines.push('Scope:');
  lines.push(`  Type: ${record.scope}`);
  lines.push(`  Files: ${record.files_count}`);
  lines.push('');

  lines.push('Results:');
  lines.push(`  Overall: ${record.overall_passed ? '✓ PASSED' : '✗ FAILED'}`);
  lines.push(`  Score: ${record.overall_score}/100`);
  lines.push('  Checks:');
  for (const check of record.checks) {
    const status = check.passed ? '✓' : '✗';
    const score = check.score !== undefined ? ` (${check.score})` : '';
    lines.push(`    ${status} ${check.name}${score}`);
  }
  lines.push('');

  if (record.git) {
    lines.push('Git Context:');
    if (record.git.branch) lines.push(`  Branch: ${record.git.branch}`);
    if (record.git.commit) lines.push(`  Commit: ${record.git.commit.substring(0, 8)}`);
    if (record.git.dirty) lines.push(`  Status: dirty (uncommitted changes)`);
    lines.push('');
  }

  if (record.ai_tool) {
    lines.push('AI Tool Detected:');
    lines.push(`  Tool: ${record.ai_tool.name}`);
    lines.push(`  Confidence: ${record.ai_tool.confidence}`);
    if (record.ai_tool.indicators) {
      lines.push(`  Indicators: ${record.ai_tool.indicators.join(', ')}`);
    }
    lines.push('');
  }

  lines.push('Environment:');
  lines.push(`  OS: ${record.environment.os}`);
  lines.push(`  Node: ${record.environment.node_version}`);
  lines.push(`  Anvil: ${record.environment.anvil_version}`);
  if (record.environment.ci) {
    lines.push(`  CI: ${record.environment.ci_provider || 'yes'}`);
  }

  return lines.join('\n');
}

/**
 * Create an AuthorshipLog from provenance context
 *
 * This bridges the Anvil provenance system with the Git AI Standard v3.0.0,
 * enabling AI-generated code to be tracked in Git Notes.
 *
 * @param params - Parameters for creating the authorship log
 * @param params.commitSha - Full 40-character commit SHA (use `git rev-parse` to resolve short refs)
 * @returns AuthorshipLog if AI tool is detected, null otherwise
 * @throws Error if commitSha is not a valid 40-character hex SHA
 */
export function createAuthorshipLog(params: {
  commitSha: string;
  fileLineMap: Record<string, string>; // file path → line ranges (e.g., "1-50,55-60")
  messages: Array<{ type: 'user' | 'assistant'; text: string }>;
  humanAuthor?: string;
  totalAdditions?: number;
  totalDeletions?: number;
}): AuthorshipLog | null {
  const {
    commitSha,
    fileLineMap,
    messages,
    humanAuthor,
    totalAdditions = 0,
    totalDeletions = 0,
  } = params;

  // Validate commit SHA is a full 40-character hex string
  if (!/^[a-f0-9]{40}$/.test(commitSha)) {
    throw new Error(
      `commitSha must be a full 40-character hex SHA, got: "${commitSha}". ` +
        'Use `git rev-parse <ref>` to resolve short refs or branch names.'
    );
  }

  // Try to detect the current AI agent
  const agent = detectCurrentAgent();
  if (!agent) {
    debug('No AI agent detected, skipping authorship log creation');
    return null;
  }

  const sessionHash = generateSessionHash(agent.tool, agent.id);

  // Build attestations from file→line map
  const attestations: AuthorshipLog['attestations'] = {};
  for (const [file, ranges] of Object.entries(fileLineMap)) {
    attestations[file] = [{ sessionHash, lineRanges: ranges }];
  }

  // Build prompt record
  const promptRecord: PromptRecord = {
    agent_id: agent,
    messages: messages.map((m) => ({
      type: m.type,
      text: m.text,
      timestamp: new Date().toISOString(),
    })) as Message[],
    total_additions: totalAdditions,
    total_deletions: totalDeletions,
    accepted_lines: totalAdditions, // Assume all lines accepted initially
    overridden_lines: 0,
    human_author: humanAuthor,
  };

  return {
    attestations,
    metadata: {
      schema_version: SCHEMA_VERSION,
      base_commit_sha: commitSha,
      prompts: {
        [sessionHash]: promptRecord,
      },
    },
  };
}

/**
 * Attach an AuthorshipLog to a commit via Git Notes
 *
 * @param log - The authorship log to attach
 * @param workspaceRoot - The repository root directory
 */
export async function attachAuthorshipToCommit(
  log: AuthorshipLog,
  workspaceRoot: string
): Promise<void> {
  const commitSha = log.metadata.base_commit_sha;
  await writeAuthorshipNote(commitSha, log, workspaceRoot);
  debug(`Attached authorship log to commit ${commitSha.slice(0, 8)}`);
}

/**
 * Create and attach an AuthorshipLog in one operation
 *
 * Convenience function that combines createAuthorshipLog and attachAuthorshipToCommit.
 *
 * @param params - Parameters for creating the authorship log
 * @param workspaceRoot - The repository root directory
 * @returns true if log was created and attached, false if no AI agent detected
 */
export async function recordAIAuthorship(
  params: {
    commitSha: string;
    fileLineMap: Record<string, string>;
    messages: Array<{ type: 'user' | 'assistant'; text: string }>;
    humanAuthor?: string;
    totalAdditions?: number;
    totalDeletions?: number;
  },
  workspaceRoot: string
): Promise<boolean> {
  const log = createAuthorshipLog(params);
  if (!log) return false;

  await attachAuthorshipToCommit(log, workspaceRoot);
  return true;
}
