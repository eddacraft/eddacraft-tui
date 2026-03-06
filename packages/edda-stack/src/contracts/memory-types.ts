import { z } from 'zod';
import {
  MemoryObjectSchema,
  DecisionMemoryMetadataSchema,
  PatternMemoryMetadataSchema,
  ConstraintMemoryMetadataSchema,
  WarningMemoryMetadataSchema,
  DoctrineMemoryMetadataSchema,
  LessonMemoryMetadataSchema,
  type MemoryType,
} from './edda-memory.js';

const BaseTypedMemorySchema = MemoryObjectSchema.omit({
  type: true,
  metadata: true,
});

export const DecisionMemorySchema = BaseTypedMemorySchema.extend({
  type: z.literal('decision'),
  metadata: DecisionMemoryMetadataSchema,
});

export const PatternMemorySchema = BaseTypedMemorySchema.extend({
  type: z.literal('pattern'),
  metadata: PatternMemoryMetadataSchema,
});

export const ConstraintMemorySchema = BaseTypedMemorySchema.extend({
  type: z.literal('constraint'),
  metadata: ConstraintMemoryMetadataSchema,
});

export const WarningMemorySchema = BaseTypedMemorySchema.extend({
  type: z.literal('warning'),
  metadata: WarningMemoryMetadataSchema,
});

export const DoctrineMemorySchema = BaseTypedMemorySchema.extend({
  type: z.literal('doctrine'),
  metadata: DoctrineMemoryMetadataSchema,
});

export const LessonMemorySchema = BaseTypedMemorySchema.extend({
  type: z.literal('lesson'),
  metadata: LessonMemoryMetadataSchema,
});

export const TypedMemorySchema = z.discriminatedUnion('type', [
  DecisionMemorySchema,
  PatternMemorySchema,
  ConstraintMemorySchema,
  WarningMemorySchema,
  DoctrineMemorySchema,
  LessonMemorySchema,
]);

export type DecisionMemory = z.infer<typeof DecisionMemorySchema>;
export type PatternMemory = z.infer<typeof PatternMemorySchema>;
export type ConstraintMemory = z.infer<typeof ConstraintMemorySchema>;
export type WarningMemory = z.infer<typeof WarningMemorySchema>;
export type DoctrineMemory = z.infer<typeof DoctrineMemorySchema>;
export type LessonMemory = z.infer<typeof LessonMemorySchema>;
export type TypedMemory = z.infer<typeof TypedMemorySchema>;

export interface MemoryMetadataByType {
  decision: z.infer<typeof DecisionMemoryMetadataSchema>;
  pattern: z.infer<typeof PatternMemoryMetadataSchema>;
  constraint: z.infer<typeof ConstraintMemoryMetadataSchema>;
  warning: z.infer<typeof WarningMemoryMetadataSchema>;
  doctrine: z.infer<typeof DoctrineMemoryMetadataSchema>;
  lesson: z.infer<typeof LessonMemoryMetadataSchema>;
}

const MemoryMetadataSchemaMap = {
  decision: DecisionMemoryMetadataSchema,
  pattern: PatternMemoryMetadataSchema,
  constraint: ConstraintMemoryMetadataSchema,
  warning: WarningMemoryMetadataSchema,
  doctrine: DoctrineMemoryMetadataSchema,
  lesson: LessonMemoryMetadataSchema,
} satisfies Record<MemoryType, z.ZodTypeAny>;

const TypedMemorySchemaMap = {
  decision: DecisionMemorySchema,
  pattern: PatternMemorySchema,
  constraint: ConstraintMemorySchema,
  warning: WarningMemorySchema,
  doctrine: DoctrineMemorySchema,
  lesson: LessonMemorySchema,
} satisfies Record<MemoryType, z.ZodTypeAny>;

export function validateMemoryMetadata<T extends MemoryType>(memory: {
  type: T;
  metadata: unknown;
}): MemoryMetadataByType[T] | null {
  const result = MemoryMetadataSchemaMap[memory.type].safeParse(memory.metadata);
  return result.success ? (result.data as MemoryMetadataByType[T]) : null;
}

export function createTypedMemory(
  memory: {
    type: MemoryType;
    metadata: unknown;
  } & Omit<z.input<typeof MemoryObjectSchema>, 'type' | 'metadata'>
): TypedMemory {
  return TypedMemorySchemaMap[memory.type].parse(memory) as TypedMemory;
}

export function parseTypedMemory(input: unknown): TypedMemory {
  return TypedMemorySchema.parse(input);
}
