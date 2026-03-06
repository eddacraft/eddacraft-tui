import { parse, stringify } from 'yaml';
import { z } from 'zod';
import type { MemoryObject } from '../contracts/edda-memory.js';
import {
  MemoryObjectSchema,
  MemoryStatusSchema,
  MemoryTypeSchema,
} from '../contracts/edda-memory.js';
import { MemoryIdSchema, ProposalIdSchema } from '../contracts/identifiers.js';
import { EddaConfidenceLevelSchema, TimestampSchema } from '../contracts/index.js';

export const MemoryIndexEntrySchema = z.object({
  id: MemoryIdSchema,
  type: MemoryTypeSchema,
  status: MemoryStatusSchema,
  path: z.string().min(1),
  statement: z.string().optional(),
  confidence: EddaConfidenceLevelSchema.optional(),
  tags: z.array(z.string()).optional(),
  created_at: TimestampSchema.optional(),
  proposal_id: ProposalIdSchema.optional(),
});

export const MemoryIndexSchema = z.object({
  memories: z.array(MemoryIndexEntrySchema),
  updated_at: TimestampSchema,
});

export type MemoryIndexEntry = z.infer<typeof MemoryIndexEntrySchema>;
export type MemoryIndex = z.infer<typeof MemoryIndexSchema>;

export function serialiseMemory(memory: MemoryObject): string {
  return stringify(memory);
}

export function deserialiseMemory(yaml: string): MemoryObject {
  return parseAndValidate(yaml, MemoryObjectSchema, 'memory');
}

export function serialiseIndex(index: MemoryIndex): string {
  return stringify(index);
}

export function deserialiseIndex(yaml: string): MemoryIndex {
  return parseAndValidate(yaml, MemoryIndexSchema, 'memory index');
}

function parseAndValidate<T>(yaml: string, schema: z.ZodSchema<T>, target: string): T {
  let parsed: unknown;
  try {
    parsed = parse(yaml);
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Unknown YAML parse error';
    throw new Error(`Failed to parse ${target} YAML: ${message}`, {
      cause: error,
    });
  }

  const result = schema.safeParse(parsed);
  if (!result.success) {
    throw new Error(`Invalid ${target} payload: ${formatZodIssues(result.error.issues)}`);
  }

  return result.data;
}

function formatZodIssues(
  issues: ReadonlyArray<{ path: ReadonlyArray<PropertyKey>; message: string }>
): string {
  return issues
    .map((issue) => {
      const path = issue.path.length > 0 ? issue.path.join('.') : 'root';
      return `${path}: ${issue.message}`;
    })
    .join('; ');
}
