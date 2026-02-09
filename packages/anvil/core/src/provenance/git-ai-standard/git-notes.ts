import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { tmpdir } from 'node:os';
import { serializeAuthorshipLog, parseAuthorshipLog } from './serializer.js';
import type { AuthorshipLog } from './types.js';
import { createDebugger } from '../../utils/debug.js';

const debug = createDebugger('git-ai-notes');
const execFileAsync = promisify(execFile);

/**
 * Validate a git commit SHA or ref to prevent shell injection
 * Allows: hex characters (SHA), alphanumeric, dash, underscore, slash, tilde, caret, @, dot
 */
function isValidGitRef(ref: string): boolean {
  return /^[a-zA-Z0-9_.~^@/-]+$/.test(ref) && ref.length > 0 && ref.length <= 256;
}

/**
 * Validate a git remote name to prevent shell injection
 * Allows: alphanumeric, dash, underscore, dot
 */
function isValidRemoteName(remote: string): boolean {
  return /^[a-zA-Z0-9_.-]+$/.test(remote) && remote.length > 0 && remote.length <= 128;
}

/**
 * Validate a git revision range to prevent shell injection
 * Allows: hex characters, alphanumeric, dash, underscore, dot, tilde, caret, @, colon
 */
function isValidRevisionRange(range: string): boolean {
  return /^[a-zA-Z0-9_.~^@:-]+$/.test(range) && range.length > 0 && range.length <= 512;
}

/**
 * Git Notes namespace for AI authorship logs
 * Per Git AI Standard v3.0.0
 */
export const NOTES_REF = 'refs/notes/ai';

/**
 * Write an authorship log to Git Notes for a commit
 *
 * @param commitSha - The commit SHA to attach the note to
 * @param log - The authorship log to write
 * @param workspaceRoot - The repository root directory
 */
export async function writeAuthorshipNote(
  commitSha: string,
  log: AuthorshipLog,
  workspaceRoot: string
): Promise<void> {
  if (!isValidGitRef(commitSha)) {
    throw new Error(`Invalid commit SHA: ${commitSha}`);
  }

  const content = serializeAuthorshipLog(log);

  // Write content to a temp file in OS temp directory to avoid issues with
  // worktrees/submodules where .git may be a file, not a directory
  const { writeFile, unlink } = await import('node:fs/promises');
  const { join } = await import('node:path');
  const { randomUUID } = await import('node:crypto');

  const tempFile = join(tmpdir(), `anvil-note-${randomUUID()}.tmp`);

  try {
    await writeFile(tempFile, content, 'utf-8');

    await execFileAsync(
      'git',
      ['notes', `--ref=${NOTES_REF}`, 'add', '-f', '-F', tempFile, '--', commitSha],
      {
        cwd: workspaceRoot,
      }
    );

    debug(`Wrote authorship note for commit ${commitSha.slice(0, 8)}`);
  } catch (error) {
    debug('Failed to write authorship note', error);
    throw new Error(`Failed to write authorship note: ${error}`);
  } finally {
    // Clean up temp file
    try {
      await unlink(tempFile);
    } catch (cleanupError) {
      debug('Failed to clean up temp file', { tempFile, error: cleanupError });
    }
  }
}

/**
 * Read an authorship log from Git Notes for a commit
 *
 * @param commitSha - The commit SHA to read the note from (or 'HEAD')
 * @param workspaceRoot - The repository root directory
 * @returns The parsed AuthorshipLog, or null if no note exists
 */
export async function readAuthorshipNote(
  commitSha: string,
  workspaceRoot: string
): Promise<AuthorshipLog | null> {
  if (!isValidGitRef(commitSha)) {
    throw new Error(`Invalid commit SHA: ${commitSha}`);
  }

  try {
    const { stdout } = await execFileAsync(
      'git',
      ['notes', `--ref=${NOTES_REF}`, 'show', '--', commitSha],
      {
        cwd: workspaceRoot,
      }
    );

    return parseAuthorshipLog(stdout);
  } catch (error) {
    // Note doesn't exist or other error
    debug(`No authorship note found for commit ${commitSha.slice(0, 8)}`, error);
    return null;
  }
}

/**
 * List all commits with authorship notes
 *
 * @param workspaceRoot - The repository root directory
 * @returns Array of commit SHAs that have authorship notes
 */
export async function listAuthorshipNotes(workspaceRoot: string): Promise<string[]> {
  try {
    const { stdout } = await execFileAsync('git', ['notes', `--ref=${NOTES_REF}`, 'list'], {
      cwd: workspaceRoot,
    });

    // Format: <note-sha> <commit-sha>
    return stdout
      .trim()
      .split('\n')
      .filter((line) => line)
      .map((line) => line.split(' ')[1])
      .filter((sha): sha is string => !!sha);
  } catch (error) {
    debug('Failed to list authorship notes', error);
    return [];
  }
}

/**
 * Remove an authorship note from a commit
 *
 * @param commitSha - The commit SHA to remove the note from
 * @param workspaceRoot - The repository root directory
 * @returns true if the note was removed, false if it didn't exist
 */
export async function removeAuthorshipNote(
  commitSha: string,
  workspaceRoot: string
): Promise<boolean> {
  if (!isValidGitRef(commitSha)) {
    throw new Error(`Invalid commit SHA: ${commitSha}`);
  }

  try {
    await execFileAsync('git', ['notes', `--ref=${NOTES_REF}`, 'remove', '--', commitSha], {
      cwd: workspaceRoot,
    });
    debug(`Removed authorship note for commit ${commitSha.slice(0, 8)}`);
    return true;
  } catch (error) {
    debug('Failed to remove authorship note (may not exist)', error);
    return false;
  }
}

/**
 * Copy authorship note when rebasing (from old SHA to new SHA)
 *
 * Updates the base_commit_sha in metadata to point to the new commit.
 *
 * @param fromSha - The original commit SHA
 * @param toSha - The new commit SHA after rebase
 * @param workspaceRoot - The repository root directory
 * @returns true if the note was copied, false if source note didn't exist
 */
export async function copyAuthorshipNote(
  fromSha: string,
  toSha: string,
  workspaceRoot: string
): Promise<boolean> {
  if (!isValidGitRef(fromSha) || !isValidGitRef(toSha)) {
    throw new Error(`Invalid commit SHA: fromSha=${fromSha}, toSha=${toSha}`);
  }

  try {
    const existingLog = await readAuthorshipNote(fromSha, workspaceRoot);
    if (!existingLog) return false;

    // Resolve toSha to full 40-character SHA
    const { stdout } = await execFileAsync('git', ['rev-parse', '--', toSha], {
      cwd: workspaceRoot,
    });
    const resolvedSha = stdout.trim();

    // Update base_commit_sha in metadata to point to new commit
    const updatedLog: AuthorshipLog = {
      ...existingLog,
      metadata: {
        ...existingLog.metadata,
        base_commit_sha: resolvedSha,
      },
    };

    await writeAuthorshipNote(toSha, updatedLog, workspaceRoot);
    debug(`Copied authorship note from ${fromSha.slice(0, 8)} to ${toSha.slice(0, 8)}`);
    return true;
  } catch (error) {
    debug(`Failed to copy authorship note from ${fromSha} to ${toSha}`, error);
    return false;
  }
}

/**
 * Push authorship notes to remote
 *
 * @param remote - Remote name (e.g., 'origin')
 * @param workspaceRoot - The repository root directory
 */
export async function pushAuthorshipNotes(remote: string, workspaceRoot: string): Promise<void> {
  if (!isValidRemoteName(remote)) {
    throw new Error(`Invalid remote name: ${remote}`);
  }

  try {
    await execFileAsync('git', ['push', remote, NOTES_REF], {
      cwd: workspaceRoot,
    });
    debug(`Pushed authorship notes to ${remote}`);
  } catch (error) {
    debug('Failed to push authorship notes', error);
    throw new Error(`Failed to push authorship notes: ${error}`);
  }
}

/**
 * Fetch authorship notes from remote
 *
 * @param remote - Remote name (e.g., 'origin')
 * @param workspaceRoot - The repository root directory
 */
export async function fetchAuthorshipNotes(remote: string, workspaceRoot: string): Promise<void> {
  if (!isValidRemoteName(remote)) {
    throw new Error(`Invalid remote name: ${remote}`);
  }

  try {
    await execFileAsync('git', ['fetch', remote, `${NOTES_REF}:${NOTES_REF}`], {
      cwd: workspaceRoot,
    });
    debug(`Fetched authorship notes from ${remote}`);
  } catch (error) {
    debug('Failed to fetch authorship notes', error);
    throw new Error(`Failed to fetch authorship notes: ${error}`);
  }
}

/**
 * Check if a commit has an authorship note
 *
 * @param commitSha - The commit SHA to check
 * @param workspaceRoot - The repository root directory
 * @returns true if the commit has an authorship note
 */
export async function hasAuthorshipNote(
  commitSha: string,
  workspaceRoot: string
): Promise<boolean> {
  if (!isValidGitRef(commitSha)) {
    throw new Error(`Invalid commit SHA: ${commitSha}`);
  }

  try {
    await execFileAsync('git', ['notes', `--ref=${NOTES_REF}`, 'show', '--', commitSha], {
      cwd: workspaceRoot,
    });
    return true;
  } catch {
    return false;
  }
}

/**
 * Get summary statistics of AI authorship in a range of commits
 *
 * @param range - Git revision range (e.g., 'main..HEAD', 'HEAD~10..HEAD')
 * @param workspaceRoot - The repository root directory
 * @returns Summary object with counts
 */
export async function getAuthorshipStats(
  range: string,
  workspaceRoot: string
): Promise<{
  totalCommits: number;
  commitsWithAI: number;
  totalAdditions: number;
  totalDeletions: number;
  tools: Record<string, number>;
}> {
  if (!isValidRevisionRange(range)) {
    throw new Error(`Invalid revision range: ${range}`);
  }

  const stats = {
    totalCommits: 0,
    commitsWithAI: 0,
    totalAdditions: 0,
    totalDeletions: 0,
    tools: {} as Record<string, number>,
  };

  try {
    // Get list of commits in range
    const { stdout } = await execFileAsync('git', ['rev-list', '--', range], {
      cwd: workspaceRoot,
    });

    const commits = stdout.trim().split('\n').filter(Boolean);
    stats.totalCommits = commits.length;

    // Check each commit for authorship notes
    for (const commit of commits) {
      const log = await readAuthorshipNote(commit, workspaceRoot);
      if (log) {
        stats.commitsWithAI++;

        for (const prompt of Object.values(log.metadata.prompts)) {
          stats.totalAdditions += prompt.total_additions;
          stats.totalDeletions += prompt.total_deletions;

          const tool = prompt.agent_id.tool;
          stats.tools[tool] = (stats.tools[tool] || 0) + 1;
        }
      }
    }
  } catch (error) {
    debug('Failed to get authorship stats', error);
  }

  return stats;
}
