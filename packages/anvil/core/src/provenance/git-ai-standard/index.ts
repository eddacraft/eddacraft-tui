/**
 * Git AI Standard v3.0.0 Implementation
 *
 * This module provides support for tracking AI-generated code contributions
 * using Git Notes under the refs/notes/ai namespace.
 *
 * @see https://github.com/git-ai-project/git-ai/blob/main/specs/git_ai_standard_v3.0.0.md
 *
 * @example
 * ```typescript
 * import {
 *   writeAuthorshipNote,
 *   readAuthorshipNote,
 *   generateSessionHash,
 *   SCHEMA_VERSION,
 *   type AuthorshipLog,
 * } from '@eddacraft/anvil-core';
 *
 * // Create an authorship log
 * const log: AuthorshipLog = {
 *   attestations: {
 *     'src/feature.ts': [
 *       { sessionHash: generateSessionHash('claude-code', 'session-123'), lineRanges: '1-50' }
 *     ]
 *   },
 *   metadata: {
 *     schema_version: SCHEMA_VERSION,
 *     base_commit_sha: 'abc123...',
 *     prompts: { ... }
 *   }
 * };
 *
 * // Attach to commit
 * await writeAuthorshipNote(commitSha, log, workspaceRoot);
 *
 * // Read back
 * const retrieved = await readAuthorshipNote(commitSha, workspaceRoot);
 * ```
 */

// Types
export {
  SCHEMA_VERSION,
  SessionHashSchema,
  LineRangeSchema,
  FileAttestationSchema,
  AgentIdSchema,
  MessageTypeSchema,
  MessageSchema,
  PromptRecordSchema,
  AuthorshipMetadataSchema,
  AuthorshipLogSchema,
  type SessionHash,
  type LineRange,
  type FileAttestation,
  type AgentId,
  type MessageType,
  type Message,
  type PromptRecord,
  type AuthorshipMetadata,
  type AuthorshipLog,
} from './types.js';

// Serialization
export {
  serializeAuthorshipLog,
  parseAuthorshipLog,
  isAuthorshipLog,
  expandLineRanges,
  compactLineRanges,
} from './serializer.js';

// Session management
export {
  generateSessionHash,
  sessionHashFromAgentId,
  createAgentId,
  detectCurrentAgent,
  createExplicitAgent,
  formatAgentId,
} from './session.js';

// Git Notes operations
export {
  NOTES_REF,
  writeAuthorshipNote,
  readAuthorshipNote,
  listAuthorshipNotes,
  removeAuthorshipNote,
  copyAuthorshipNote,
  pushAuthorshipNotes,
  fetchAuthorshipNotes,
  hasAuthorshipNote,
  getAuthorshipStats,
} from './git-notes.js';
