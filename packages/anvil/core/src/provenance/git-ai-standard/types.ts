import { z } from 'zod';

/**
 * Git AI Standard v3.0.0 Type Definitions
 *
 * This module implements the schema definitions for tracking AI-generated code
 * contributions using Git Notes under the refs/notes/ai namespace.
 *
 * @see https://github.com/git-ai-project/git-ai/blob/main/specs/git_ai_standard_v3.0.0.md
 */

export const SCHEMA_VERSION = 'authorship/3.0.0' as const;

/**
 * Session hash - 16-character hex prefix of SHA-256({tool}:{conversation_id})
 *
 * Session hashes must:
 * - Remain stable across commits for the same session
 * - Correspond to keys in the prompts object
 * - Use only hexadecimal characters
 *
 * Backward compatibility allows 7-character hashes.
 */
export const SessionHashSchema = z
  .string()
  .regex(/^[a-f0-9]{7,16}$/, 'Session hash must be 7-16 hex characters')
  .describe('Session identifier (16-char SHA-256 prefix)');

/**
 * Line range specification (1-indexed)
 *
 * Formats:
 * - Single lines: "42"
 * - Ranges: "19-222"
 * - Multiple: "1,2,19-222,300"
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
 * Agent identifier - identifies the AI tool and session
 */
export const AgentIdSchema = z.object({
  tool: z.string().describe('AI tool name (e.g., "claude-code", "cursor", "copilot")'),
  id: z.string().describe('Unique conversation/session ID'),
  model: z.string().optional().describe('Model used (e.g., "claude-3-opus", "gpt-4")'),
});

/**
 * Message types in a prompt conversation
 *
 * Note: Tool responses are explicitly excluded per the spec to avoid bloat.
 */
export const MessageTypeSchema = z.enum(['user', 'assistant', 'tool_use']);

/**
 * Message in a prompt conversation
 */
export const MessageSchema = z.object({
  type: MessageTypeSchema.describe('Message type'),
  text: z.string().describe('Message content'),
  timestamp: z.string().datetime().optional().describe('ISO 8601 timestamp'),
});

/**
 * Prompt record for a session
 *
 * Contains the full conversation and metrics for a single AI session
 * that contributed to the commit.
 */
export const PromptRecordSchema = z.object({
  agent_id: AgentIdSchema.describe('AI tool identifier'),
  messages: z.array(MessageSchema).describe('Conversation turns'),
  total_additions: z.number().int().min(0).describe('Total lines added by AI'),
  total_deletions: z.number().int().min(0).describe('Total lines deleted by AI'),
  accepted_lines: z.number().int().min(0).describe('Lines accepted as-is from AI'),
  overriden_lines: z.number().int().min(0).describe('Lines human-modified after AI generation'),
  human_author: z.string().optional().describe('Human author in "Name <email>" format'),
});

/**
 * Metadata section of authorship log
 *
 * The JSON object containing prompt records and versioning information.
 */
export const AuthorshipMetadataSchema = z.object({
  schema_version: z.literal(SCHEMA_VERSION).describe('Must be "authorship/3.0.0"'),
  base_commit_sha: z
    .string()
    .regex(/^[a-f0-9]{40}$/, 'Must be full 40-char SHA')
    .describe('The commit SHA this log is attached to'),
  prompts: z
    .record(SessionHashSchema, PromptRecordSchema)
    .describe('Map of session hashes to prompt records'),
});

/**
 * Complete authorship log (attestation + metadata)
 *
 * The full structure stored in Git Notes under refs/notes/ai.
 * Consists of two sections separated by "---":
 * 1. Attestation section - maps files to AI sessions and line numbers
 * 2. Metadata section - JSON object with prompt records and versioning
 */
export const AuthorshipLogSchema = z.object({
  attestations: z
    .record(z.string(), z.array(FileAttestationSchema))
    .describe('Map of file paths to attestation entries'),
  metadata: AuthorshipMetadataSchema.describe('Metadata with prompts and versioning'),
});

// Type exports
export type SessionHash = z.infer<typeof SessionHashSchema>;
export type LineRange = z.infer<typeof LineRangeSchema>;
export type FileAttestation = z.infer<typeof FileAttestationSchema>;
export type AgentId = z.infer<typeof AgentIdSchema>;
export type MessageType = z.infer<typeof MessageTypeSchema>;
export type Message = z.infer<typeof MessageSchema>;
export type PromptRecord = z.infer<typeof PromptRecordSchema>;
export type AuthorshipMetadata = z.infer<typeof AuthorshipMetadataSchema>;
export type AuthorshipLog = z.infer<typeof AuthorshipLogSchema>;
