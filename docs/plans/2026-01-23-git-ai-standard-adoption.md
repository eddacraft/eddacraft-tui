# Git AI Standard v3.0.0 Adoption Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to
> implement this plan task-by-task.

**Goal:** Adopt the Git AI Standard v3.0.0 to track AI-generated code
contributions using Git Notes under `refs/notes/ai`, supplementing our existing
provenance system.

**Reference:**
[Git AI Standard v3.0.0](https://github.com/git-ai-project/git-ai/blob/main/specs/git_ai_standard_v3.0.0.md)

**Architecture:** The standard uses Git Notes to attach authorship logs to
commits. Each log has two sections:

1. **Attestation Section** - Maps files to AI session hashes and line ranges
2. **Metadata Section** - JSON with prompt records, agent details, and metrics

**Tech Stack:** TypeScript, Zod schemas, Git CLI integration

---

## Overview

### Why Adopt This Standard?

| Current State                               | With Git AI Standard                          |
| ------------------------------------------- | --------------------------------------------- |
| Provenance stored in `.anvil/history/` JSON | Provenance also in Git Notes (portable)       |
| AI tool detected but not per-line           | Per-line AI attribution with session tracking |
| No cross-repo portability                   | Standard format works across tools            |
| Manual audit trail                          | Native `git log --notes=ai` support           |

### Integration Points

```
┌─────────────────────────────────────────────────────────────────┐
│                    Anvil Provenance System                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Existing:                    New (Git AI Standard):            │
│  ┌──────────────────┐        ┌──────────────────────────────┐   │
│  │ ProvenanceRecord │───────►│ GitAIAuthorshipLog           │   │
│  │ - ai_tool        │        │ - Attestation (file→lines)   │   │
│  │ - git context    │        │ - Metadata (prompts, agents) │   │
│  │ - check results  │        └──────────────┬───────────────┘   │
│  └────────┬─────────┘                       │                   │
│           │                                 │                   │
│           ▼                                 ▼                   │
│  ┌──────────────────┐        ┌──────────────────────────────┐   │
│  │ .anvil/history/  │        │ refs/notes/ai                │   │
│  │ (JSON files)     │        │ (Git Notes)                  │   │
│  └──────────────────┘        └──────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Task 1: Define Git AI Standard TypeScript Schemas

**Files:**

- Create: `packages/anvil/core/src/provenance/git-ai-standard/types.ts`
- Create: `packages/anvil/core/src/provenance/git-ai-standard/index.ts`

**Step 1: Create directory structure**

```bash
mkdir -p packages/anvil/core/src/provenance/git-ai-standard
```

**Step 2: Define Zod schemas for Git AI Standard v3.0.0**

Create `packages/anvil/core/src/provenance/git-ai-standard/types.ts`:

```typescript
import { z } from 'zod';

/**
 * Git AI Standard v3.0.0 Type Definitions
 * @see https://github.com/git-ai-project/git-ai/blob/main/specs/git_ai_standard_v3.0.0.md
 */

export const SCHEMA_VERSION = 'authorship/3.0.0';

/**
 * Session hash - 16-character hex prefix of SHA-256({tool}:{conversation_id})
 */
export const SessionHashSchema = z
  .string()
  .regex(/^[a-f0-9]{16}$/, 'Session hash must be 16 hex characters')
  .describe('16-character session identifier');

/**
 * Line range specification (1-indexed)
 * Examples: "42", "19-222", "1,2,19-222,300"
 */
export const LineRangeSchema = z
  .string()
  .regex(/^(\d+(-\d+)?)(,\d+(-\d+)?)*$/, 'Invalid line range format')
  .describe('Line range specification (e.g., "1-10,15-20")');

/**
 * File attestation entry - maps session hash to line ranges
 */
export const FileAttestationSchema = z.object({
  sessionHash: SessionHashSchema,
  lineRanges: LineRangeSchema,
});

/**
 * Agent identifier
 */
export const AgentIdSchema = z.object({
  tool: z.string().describe('AI tool name (e.g., "claude-code", "cursor")'),
  id: z.string().describe('Unique conversation/session ID'),
  model: z.string().optional().describe('Model used (e.g., "claude-3-opus")'),
});

/**
 * Message in a prompt conversation
 */
export const MessageSchema = z.object({
  type: z.enum(['user', 'assistant', 'tool_use']).describe('Message type'),
  text: z.string().describe('Message content'),
  timestamp: z.string().datetime().optional().describe('ISO 8601 timestamp'),
});

/**
 * Prompt record for a session
 */
export const PromptRecordSchema = z.object({
  agent_id: AgentIdSchema,
  messages: z.array(MessageSchema).describe('Conversation turns'),
  total_additions: z.number().int().min(0).describe('Total lines added'),
  total_deletions: z.number().int().min(0).describe('Total lines deleted'),
  accepted_lines: z.number().int().min(0).describe('Lines accepted as-is'),
  overridden_lines: z.number().int().min(0).describe('Lines human-modified'),
  human_author: z
    .string()
    .optional()
    .describe('Author in "Name <email>" format'),
});

/**
 * Metadata section of authorship log
 */
export const AuthorshipMetadataSchema = z.object({
  schema_version: z.literal(SCHEMA_VERSION),
  base_commit_sha: z
    .string()
    .regex(/^[a-f0-9]{40}$/, 'Must be full 40-char SHA'),
  prompts: z.record(SessionHashSchema, PromptRecordSchema),
});

/**
 * Complete authorship log (attestation + metadata)
 */
export const AuthorshipLogSchema = z.object({
  attestations: z
    .record(z.string(), z.array(FileAttestationSchema))
    .describe('Map of file paths to attestation entries'),
  metadata: AuthorshipMetadataSchema,
});

// Type exports
export type SessionHash = z.infer<typeof SessionHashSchema>;
export type LineRange = z.infer<typeof LineRangeSchema>;
export type FileAttestation = z.infer<typeof FileAttestationSchema>;
export type AgentId = z.infer<typeof AgentIdSchema>;
export type Message = z.infer<typeof MessageSchema>;
export type PromptRecord = z.infer<typeof PromptRecordSchema>;
export type AuthorshipMetadata = z.infer<typeof AuthorshipMetadataSchema>;
export type AuthorshipLog = z.infer<typeof AuthorshipLogSchema>;
```

**Step 3: Create index export**

Create `packages/anvil/core/src/provenance/git-ai-standard/index.ts`:

```typescript
export * from './types.js';
```

**Step 4: Commit**

```bash
git add packages/anvil/core/src/provenance/git-ai-standard/
git commit -m "feat(provenance): add Git AI Standard v3.0.0 type definitions

Implements schema definitions for Git AI Standard:
- SessionHash (16-char hex identifier)
- FileAttestation (session→line range mapping)
- PromptRecord (agent, messages, metrics)
- AuthorshipLog (complete log structure)

Ref: https://github.com/git-ai-project/git-ai/blob/main/specs/git_ai_standard_v3.0.0.md"
```

---

## Task 2: Implement Authorship Log Serialization

**Files:**

- Create: `packages/anvil/core/src/provenance/git-ai-standard/serializer.ts`
- Create:
  `packages/anvil/core/src/provenance/git-ai-standard/__tests__/serializer.test.ts`

**Step 1: Write failing serialization test**

Create
`packages/anvil/core/src/provenance/git-ai-standard/__tests__/serializer.test.ts`:

```typescript
import { describe, it, expect } from 'vitest';
import { serializeAuthorshipLog, parseAuthorshipLog } from '../serializer.js';
import type { AuthorshipLog } from '../types.js';

describe('AuthorshipLog Serializer', () => {
  const sampleLog: AuthorshipLog = {
    attestations: {
      'src/auth/login.ts': [
        { sessionHash: 'a1b2c3d4e5f67890', lineRanges: '1-50,55-60' },
      ],
      'src/auth/jwt.ts': [
        { sessionHash: 'a1b2c3d4e5f67890', lineRanges: '1-30' },
      ],
    },
    metadata: {
      schema_version: 'authorship/3.0.0',
      base_commit_sha: 'abc123def456789012345678901234567890abcd',
      prompts: {
        a1b2c3d4e5f67890: {
          agent_id: {
            tool: 'claude-code',
            id: 'session-123',
            model: 'claude-3-opus',
          },
          messages: [
            { type: 'user', text: 'Implement login endpoint' },
            { type: 'assistant', text: 'Creating login.ts...' },
          ],
          total_additions: 80,
          total_deletions: 0,
          accepted_lines: 75,
          overridden_lines: 5,
          human_author: 'Alice <alice@example.com>',
        },
      },
    },
  };

  describe('serialize', () => {
    it('produces valid log format with attestation and metadata sections', () => {
      const output = serializeAuthorshipLog(sampleLog);

      // Should have attestation section
      expect(output).toContain('src/auth/login.ts');
      expect(output).toContain('a1b2c3d4e5f67890 1-50,55-60');

      // Should have separator
      expect(output).toContain('---');

      // Should have JSON metadata
      expect(output).toContain('"schema_version": "authorship/3.0.0"');
    });

    it('quotes file paths with special characters', () => {
      const logWithSpaces: AuthorshipLog = {
        attestations: {
          'src/My Component/index.ts': [
            { sessionHash: 'a1b2c3d4e5f67890', lineRanges: '1-10' },
          ],
        },
        metadata: sampleLog.metadata,
      };

      const output = serializeAuthorshipLog(logWithSpaces);
      expect(output).toContain('"src/My Component/index.ts"');
    });
  });

  describe('parse', () => {
    it('round-trips correctly', () => {
      const serialized = serializeAuthorshipLog(sampleLog);
      const parsed = parseAuthorshipLog(serialized);

      expect(parsed.attestations).toEqual(sampleLog.attestations);
      expect(parsed.metadata.schema_version).toBe('authorship/3.0.0');
      expect(parsed.metadata.prompts['a1b2c3d4e5f67890'].agent_id.tool).toBe(
        'claude-code'
      );
    });
  });
});
```

**Step 2: Implement serializer**

Create `packages/anvil/core/src/provenance/git-ai-standard/serializer.ts`:

````typescript
import type { AuthorshipLog, FileAttestation } from './types.js';
import { AuthorshipLogSchema, SCHEMA_VERSION } from './types.js';

/**
 * Serialize an AuthorshipLog to Git AI Standard format
 *
 * Format:
 * ```
 * file/path.ts
 *   a1b2c3d4e5f67890 1-50,55-60
 * another/file.ts
 *   b2c3d4e5f67890a1 1-100
 * ---
 * {"schema_version":"authorship/3.0.0",...}
 * ```
 */
export function serializeAuthorshipLog(log: AuthorshipLog): string {
  const lines: string[] = [];

  // Attestation section
  for (const [filePath, attestations] of Object.entries(log.attestations)) {
    // Quote paths with spaces or special characters
    const quotedPath = /[\s"']/.test(filePath) ? `"${filePath}"` : filePath;
    lines.push(quotedPath);

    for (const attestation of attestations) {
      lines.push(`  ${attestation.sessionHash} ${attestation.lineRanges}`);
    }
  }

  // Separator
  lines.push('---');

  // Metadata section (JSON)
  lines.push(JSON.stringify(log.metadata, null, 2));

  return lines.join('\n');
}

/**
 * Parse a Git AI Standard authorship log
 */
export function parseAuthorshipLog(content: string): AuthorshipLog {
  const separatorIndex = content.indexOf('\n---\n');
  if (separatorIndex === -1) {
    throw new Error('Invalid authorship log: missing --- separator');
  }

  const attestationSection = content.slice(0, separatorIndex);
  const metadataSection = content.slice(separatorIndex + 5); // Skip '\n---\n'

  // Parse attestations
  const attestations: Record<string, FileAttestation[]> = {};
  let currentFile: string | null = null;

  for (const line of attestationSection.split('\n')) {
    if (!line.trim()) continue;

    if (!line.startsWith(' ') && !line.startsWith('\t')) {
      // File path line
      currentFile = line.startsWith('"') ? line.slice(1, -1) : line;
      attestations[currentFile] = [];
    } else if (currentFile) {
      // Attestation entry line
      const match = line.trim().match(/^([a-f0-9]{16})\s+(.+)$/);
      if (match) {
        attestations[currentFile].push({
          sessionHash: match[1],
          lineRanges: match[2],
        });
      }
    }
  }

  // Parse metadata JSON
  const metadata = JSON.parse(metadataSection.trim());

  // Validate
  return AuthorshipLogSchema.parse({ attestations, metadata });
}

/**
 * Check if content looks like an authorship log
 */
export function isAuthorshipLog(content: string): boolean {
  return (
    content.includes('---\n') &&
    content.includes(`"schema_version": "${SCHEMA_VERSION}"`)
  );
}
````

**Step 3: Update index exports**

```typescript
export * from './types.js';
export * from './serializer.js';
```

**Step 4: Run tests**

```bash
pnpm --filter @eddacraft/anvil-core test -- --grep "AuthorshipLog"
```

**Step 5: Commit**

```bash
git add packages/anvil/core/src/provenance/git-ai-standard/
git commit -m "feat(provenance): implement Git AI Standard serialization

- serializeAuthorshipLog() - converts AuthorshipLog to standard format
- parseAuthorshipLog() - parses standard format back to TypeScript
- Handles quoted paths for files with spaces
- Full round-trip support"
```

---

## Task 3: Implement Git Notes Integration

**Files:**

- Create: `packages/anvil/core/src/provenance/git-ai-standard/git-notes.ts`
- Create:
  `packages/anvil/core/src/provenance/git-ai-standard/__tests__/git-notes.test.ts`

**Step 1: Implement Git Notes operations**

Create `packages/anvil/core/src/provenance/git-ai-standard/git-notes.ts`:

```typescript
import { exec } from 'child_process';
import { promisify } from 'util';
import { serializeAuthorshipLog, parseAuthorshipLog } from './serializer.js';
import type { AuthorshipLog } from './types.js';
import { createDebugger } from '../../utils/debug.js';

const debug = createDebugger('git-ai-notes');
const execAsync = promisify(exec);

const NOTES_REF = 'refs/notes/ai';

/**
 * Write an authorship log to Git Notes for a commit
 */
export async function writeAuthorshipNote(
  commitSha: string,
  log: AuthorshipLog,
  workspaceRoot: string
): Promise<void> {
  const content = serializeAuthorshipLog(log);

  // Use git notes add with --force to overwrite existing
  const escapedContent = content.replace(/'/g, "'\\''");

  try {
    await execAsync(
      `git notes --ref=${NOTES_REF} add -f -m '${escapedContent}' ${commitSha}`,
      { cwd: workspaceRoot }
    );
    debug('Wrote authorship note for commit %s', commitSha.slice(0, 8));
  } catch (error) {
    debug('Failed to write authorship note', error);
    throw new Error(`Failed to write authorship note: ${error}`);
  }
}

/**
 * Read an authorship log from Git Notes for a commit
 */
export async function readAuthorshipNote(
  commitSha: string,
  workspaceRoot: string
): Promise<AuthorshipLog | null> {
  try {
    const { stdout } = await execAsync(
      `git notes --ref=${NOTES_REF} show ${commitSha}`,
      { cwd: workspaceRoot }
    );

    return parseAuthorshipLog(stdout);
  } catch (error) {
    // Note doesn't exist
    debug('No authorship note found for commit %s', commitSha.slice(0, 8));
    return null;
  }
}

/**
 * List all commits with authorship notes
 */
export async function listAuthorshipNotes(
  workspaceRoot: string
): Promise<string[]> {
  try {
    const { stdout } = await execAsync(`git notes --ref=${NOTES_REF} list`, {
      cwd: workspaceRoot,
    });

    // Format: <note-sha> <commit-sha>
    return stdout
      .trim()
      .split('\n')
      .filter((line) => line)
      .map((line) => line.split(' ')[1]);
  } catch (error) {
    debug('Failed to list authorship notes', error);
    return [];
  }
}

/**
 * Remove an authorship note from a commit
 */
export async function removeAuthorshipNote(
  commitSha: string,
  workspaceRoot: string
): Promise<boolean> {
  try {
    await execAsync(`git notes --ref=${NOTES_REF} remove ${commitSha}`, {
      cwd: workspaceRoot,
    });
    return true;
  } catch (error) {
    debug('Failed to remove authorship note', error);
    return false;
  }
}

/**
 * Copy authorship notes when rebasing (from old SHA to new SHA)
 */
export async function copyAuthorshipNote(
  fromSha: string,
  toSha: string,
  workspaceRoot: string
): Promise<boolean> {
  try {
    const existingLog = await readAuthorshipNote(fromSha, workspaceRoot);
    if (!existingLog) return false;

    // Update base_commit_sha in metadata
    const updatedLog: AuthorshipLog = {
      ...existingLog,
      metadata: {
        ...existingLog.metadata,
        base_commit_sha: toSha.padEnd(40, '0').slice(0, 40),
      },
    };

    await writeAuthorshipNote(toSha, updatedLog, workspaceRoot);
    return true;
  } catch (error) {
    debug('Failed to copy authorship note from %s to %s', fromSha, toSha);
    return false;
  }
}

/**
 * Push authorship notes to remote
 */
export async function pushAuthorshipNotes(
  remote: string,
  workspaceRoot: string
): Promise<void> {
  try {
    await execAsync(`git push ${remote} ${NOTES_REF}`, { cwd: workspaceRoot });
    debug('Pushed authorship notes to %s', remote);
  } catch (error) {
    debug('Failed to push authorship notes', error);
    throw new Error(`Failed to push authorship notes: ${error}`);
  }
}

/**
 * Fetch authorship notes from remote
 */
export async function fetchAuthorshipNotes(
  remote: string,
  workspaceRoot: string
): Promise<void> {
  try {
    await execAsync(`git fetch ${remote} ${NOTES_REF}:${NOTES_REF}`, {
      cwd: workspaceRoot,
    });
    debug('Fetched authorship notes from %s', remote);
  } catch (error) {
    debug('Failed to fetch authorship notes', error);
    throw new Error(`Failed to fetch authorship notes: ${error}`);
  }
}
```

**Step 2: Update index exports**

```typescript
export * from './types.js';
export * from './serializer.js';
export * from './git-notes.js';
```

**Step 3: Commit**

```bash
git add packages/anvil/core/src/provenance/git-ai-standard/
git commit -m "feat(provenance): implement Git Notes integration for AI authorship

- writeAuthorshipNote() - attach authorship log to commit
- readAuthorshipNote() - retrieve authorship from commit
- copyAuthorshipNote() - support rebase operations
- push/fetchAuthorshipNotes() - remote sync support

Uses refs/notes/ai namespace per Git AI Standard v3.0.0"
```

---

## Task 4: Create Session Hash Generator

**Files:**

- Create: `packages/anvil/core/src/provenance/git-ai-standard/session.ts`

**Step 1: Implement session hash generation**

Create `packages/anvil/core/src/provenance/git-ai-standard/session.ts`:

```typescript
import { createHash } from 'crypto';
import type { SessionHash, AgentId } from './types.js';

/**
 * Generate a session hash from tool and conversation ID
 *
 * Per Git AI Standard: 16-character SHA-256 prefix of {tool}:{conversation_id}
 */
export function generateSessionHash(
  tool: string,
  conversationId: string
): SessionHash {
  const input = `${tool}:${conversationId}`;
  const fullHash = createHash('sha256').update(input).digest('hex');
  return fullHash.slice(0, 16) as SessionHash;
}

/**
 * Generate a session hash from an AgentId
 */
export function sessionHashFromAgentId(agentId: AgentId): SessionHash {
  return generateSessionHash(agentId.tool, agentId.id);
}

/**
 * Create an AgentId from environment and context
 */
export function createAgentId(options: {
  tool: string;
  conversationId?: string;
  model?: string;
}): AgentId {
  const { tool, model } = options;

  // Generate conversation ID if not provided
  const conversationId =
    options.conversationId ??
    `${Date.now()}-${Math.random().toString(36).slice(2)}`;

  return {
    tool,
    id: conversationId,
    model,
  };
}

/**
 * Detect current AI tool and create AgentId
 */
export function detectCurrentAgent(): AgentId | null {
  // Check environment for known AI tools
  if (process.env.CURSOR_SESSION) {
    return createAgentId({
      tool: 'cursor',
      conversationId: process.env.CURSOR_SESSION,
    });
  }

  if (process.env.CLAUDE_SESSION_ID) {
    return createAgentId({
      tool: 'claude-code',
      conversationId: process.env.CLAUDE_SESSION_ID,
      model: process.env.CLAUDE_MODEL,
    });
  }

  if (process.env.GITHUB_COPILOT_TOKEN) {
    return createAgentId({
      tool: 'copilot',
    });
  }

  return null;
}
```

**Step 2: Update index exports**

**Step 3: Commit**

```bash
git add packages/anvil/core/src/provenance/git-ai-standard/
git commit -m "feat(provenance): add session hash generation

- generateSessionHash() - creates 16-char hex from tool:id
- detectCurrentAgent() - auto-detect AI tool from environment
- Compatible with Git AI Standard v3.0.0 session semantics"
```

---

## Task 5: Integrate with ProvenanceCollector

**Files:**

- Modify: `packages/anvil/core/src/provenance/collector.ts`
- Modify: `packages/anvil/core/src/provenance/index.ts`

**Step 1: Add Git AI Standard support to collector**

Update `packages/anvil/core/src/provenance/collector.ts` to add:

```typescript
import {
  generateSessionHash,
  detectCurrentAgent,
  writeAuthorshipNote,
  type AuthorshipLog,
  type PromptRecord,
  SCHEMA_VERSION,
} from './git-ai-standard/index.js';

/**
 * Create an AuthorshipLog from a provenance record
 */
export function createAuthorshipLogFromProvenance(
  record: ProvenanceRecord,
  fileLineMap: Record<string, string>, // file → line ranges
  messages: Array<{ type: 'user' | 'assistant'; text: string }>
): AuthorshipLog | null {
  const agent = detectCurrentAgent();
  if (!agent || !record.git?.commit) return null;

  const sessionHash = generateSessionHash(agent.tool, agent.id);

  const attestations: Record<
    string,
    Array<{ sessionHash: string; lineRanges: string }>
  > = {};
  for (const [file, ranges] of Object.entries(fileLineMap)) {
    attestations[file] = [{ sessionHash, lineRanges: ranges }];
  }

  const promptRecord: PromptRecord = {
    agent_id: agent,
    messages: messages.map((m) => ({
      ...m,
      timestamp: new Date().toISOString(),
    })),
    total_additions: record.files_count,
    total_deletions: 0,
    accepted_lines: record.files_count,
    overridden_lines: 0,
    human_author: record.git.author,
  };

  return {
    attestations,
    metadata: {
      schema_version: SCHEMA_VERSION,
      base_commit_sha: record.git.commit.padEnd(40, '0').slice(0, 40),
      prompts: {
        [sessionHash]: promptRecord,
      },
    },
  };
}

/**
 * Attach authorship log to the current commit
 */
export async function attachAuthorshipToCommit(
  workspaceRoot: string,
  log: AuthorshipLog
): Promise<void> {
  const commitSha = log.metadata.base_commit_sha;
  await writeAuthorshipNote(commitSha, log, workspaceRoot);
}
```

**Step 2: Update provenance index exports**

Update `packages/anvil/core/src/provenance/index.ts`:

```typescript
export * from './types.js';
export * from './collector.js';
export * from './store.js';
export * from './git-ai-standard/index.js';
```

**Step 3: Commit**

```bash
git add packages/anvil/core/src/provenance/
git commit -m "feat(provenance): integrate Git AI Standard with ProvenanceCollector

- createAuthorshipLogFromProvenance() - convert existing records
- attachAuthorshipToCommit() - write to Git Notes
- Export Git AI Standard types from provenance module"
```

---

## Task 6: Add CLI Command for Viewing AI Authorship

**Files:**

- Create: `apps/anvil-cli/src/commands/authorship.ts`
- Modify: `apps/anvil-cli/src/index.ts`

**Step 1: Create authorship command**

Create `apps/anvil-cli/src/commands/authorship.ts`:

```typescript
import { Command } from 'commander';
import chalk from 'chalk';
import { readAuthorshipNote, listAuthorshipNotes } from '@eddacraft/anvil-core';

export function createAuthorshipCommand(): Command {
  const cmd = new Command('authorship').description(
    'View AI authorship information for commits'
  );

  cmd
    .command('show')
    .description('Show AI authorship for a commit')
    .argument('[commit]', 'Commit SHA (defaults to HEAD)')
    .action(async (commit = 'HEAD') => {
      const log = await readAuthorshipNote(commit, process.cwd());

      if (!log) {
        console.log(
          chalk.yellow('No AI authorship information found for this commit.')
        );
        return;
      }

      console.log(chalk.bold('AI Authorship Log'));
      console.log(chalk.dim('─'.repeat(50)));
      console.log();

      console.log(chalk.bold('Files with AI attribution:'));
      for (const [file, attestations] of Object.entries(log.attestations)) {
        console.log(`  ${chalk.cyan(file)}`);
        for (const a of attestations) {
          console.log(
            `    ${chalk.dim(a.sessionHash)} → lines ${a.lineRanges}`
          );
        }
      }
      console.log();

      console.log(chalk.bold('Sessions:'));
      for (const [hash, prompt] of Object.entries(log.metadata.prompts)) {
        console.log(`  ${chalk.magenta(hash)}`);
        console.log(`    Tool: ${prompt.agent_id.tool}`);
        if (prompt.agent_id.model) {
          console.log(`    Model: ${prompt.agent_id.model}`);
        }
        console.log(
          `    Lines: +${prompt.total_additions} -${prompt.total_deletions}`
        );
        console.log(
          `    Accepted: ${prompt.accepted_lines}, Modified: ${prompt.overridden_lines}`
        );
      }
    });

  cmd
    .command('list')
    .description('List commits with AI authorship')
    .option('-n, --limit <n>', 'Limit results', '10')
    .action(async (options) => {
      const commits = await listAuthorshipNotes(process.cwd());
      const limit = parseInt(options.limit);

      if (commits.length === 0) {
        console.log(chalk.yellow('No commits with AI authorship found.'));
        return;
      }

      console.log(
        chalk.bold(`Commits with AI authorship (${commits.length} total):`)
      );
      for (const sha of commits.slice(0, limit)) {
        console.log(`  ${chalk.cyan(sha.slice(0, 8))}`);
      }
    });

  return cmd;
}
```

**Step 2: Register command**

Update `apps/anvil-cli/src/index.ts` to add:

```typescript
import { createAuthorshipCommand } from './commands/authorship.js';

// ... existing code ...

program.addCommand(createAuthorshipCommand());
```

**Step 3: Commit**

```bash
git add apps/anvil-cli/src/
git commit -m "feat(cli): add anvil authorship command

- anvil authorship show [commit] - display AI authorship details
- anvil authorship list - list commits with AI attribution
- Reads from refs/notes/ai per Git AI Standard v3.0.0"
```

---

## Summary

| Task | Description                               | Status |
| ---- | ----------------------------------------- | ------ |
| 1    | Define Git AI Standard TypeScript schemas | Ready  |
| 2    | Implement authorship log serialization    | Ready  |
| 3    | Implement Git Notes integration           | Ready  |
| 4    | Create session hash generator             | Ready  |
| 5    | Integrate with ProvenanceCollector        | Ready  |
| 6    | Add CLI command for viewing authorship    | Ready  |

**Total: 6 tasks**

---

## Future Enhancements

- **History rewrite hooks**: Auto-copy notes on rebase/cherry-pick
- **Pre-commit hook**: Auto-attach authorship on AI-assisted commits
- **GitHub Action**: Validate AI authorship in CI
- **VS Code extension**: Show AI attribution inline

---

## References

- [Git AI Standard v3.0.0](https://github.com/git-ai-project/git-ai/blob/main/specs/git_ai_standard_v3.0.0.md)
- [Git Notes Documentation](https://git-scm.com/docs/git-notes)

_Last updated: January 2026_
