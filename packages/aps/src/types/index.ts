/**
 * APS type definitions and schemas
 */

import { z } from 'zod';

/**
 * Task confidence levels
 */
export const ConfidenceSchema = z.enum(['low', 'medium', 'high']);
export type Confidence = z.infer<typeof ConfidenceSchema>;

/**
 * Task status values
 * Note: Status is managed externally in .anvil/state.json
 */
export const TaskStatusSchema = z.enum(['open', 'locked', 'completed', 'cancelled']);
export type TaskStatus = z.infer<typeof TaskStatusSchema>;

/**
 * Priority levels
 */
export const PrioritySchema = z.enum(['low', 'medium', 'high']);
export type Priority = z.infer<typeof PrioritySchema>;

/**
 * Parsed task from APS document
 */
/** Task ID regex: 1-10 uppercase alphanumeric scope, hyphen, 3-digit zero-padded number */
export const TASK_ID_REGEX = /^[A-Z0-9]{1,10}-\d{3}$/;

export const TaskSchema = z.object({
  /** Task ID in format SCOPE-NUMBER (e.g., AUTH-001, LLM2-007) */
  id: z.string().regex(TASK_ID_REGEX),

  /** Task title (after the ID and colon in H3 heading) */
  title: z.string(),

  /** What the task aims to achieve (required) */
  intent: z.string(),

  /** Success criteria (optional) */
  expectedOutcome: z.string().optional(),

  /** Certainty about approach (optional, defaults to 'medium') */
  confidence: ConfidenceSchema.default('medium'),

  /** What can be changed - LLM file access constraints (optional) */
  scopes: z.array(z.string()).optional(),

  /** Labels for filtering and search (optional) */
  tags: z.array(z.string()).optional(),

  /** Tasks that must complete first (optional) */
  dependencies: z.array(z.string()).optional(),

  /** Required inputs or context (optional) */
  inputs: z.array(z.string()).optional(),

  /** Current execution state (optional, managed externally) */
  status: TaskStatusSchema.optional(),

  /** Source file path where this task was parsed from */
  sourcePath: z.string().optional(),

  /** Line number in source file where this task starts */
  sourceLineNumber: z.number().optional(),
});

export type Task = z.infer<typeof TaskSchema>;

/**
 * Module metadata from index files or leaf spec headers
 */
export const ModuleMetadataSchema = z.object({
  /** Module identifier (e.g., 'auth', 'payments') */
  id: z.string().optional(),

  /** Module title from H1 heading */
  title: z.string().optional(),

  /** Path to leaf spec file (for index files) */
  path: z.string().optional(),

  /** Scope prefix for task IDs (e.g., AUTH, PAY) */
  scope: z.string().optional(),

  /** Person/team responsible */
  owner: z.string().optional(),

  /** Priority level */
  priority: PrioritySchema.optional(),

  /** Tags for filtering */
  tags: z.array(z.string()).optional(),

  /** Modules this depends on */
  dependencies: z.array(z.string()).optional(),
});

export type ModuleMetadata = z.infer<typeof ModuleMetadataSchema>;

/**
 * Parsed leaf spec document result
 * For index files, use ParsedIndex from the parser module
 */
export interface ParsedDocument {
  /** Document title from H1 */
  title: string;

  /** Module metadata from header line */
  metadata?: ModuleMetadata;

  /** List of tasks */
  tasks: Task[];

  /** Source file path */
  sourcePath?: string;
}

/**
 * Parser error with context
 */
export class ParseError extends Error {
  constructor(
    message: string,
    public readonly sourcePath?: string,
    public readonly lineNumber?: number,
    public readonly context?: string
  ) {
    super(message);
    this.name = 'ParseError';
  }
}

// ValidationResult is exported from validator/index.ts
