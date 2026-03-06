import { z } from 'zod';
import {
  CandidateProposalSchema,
  DecisionMetadataSchema,
  PatternMetadataSchema,
  WarningMetadataSchema,
  LessonMetadataSchema,
  AnomalyMetadataSchema,
  ConstraintMetadataSchema,
  type ProposalType,
} from './ember-proposal.js';

const BaseTypedProposalSchema = CandidateProposalSchema.omit({
  type: true,
  metadata: true,
});

export const DecisionProposalSchema = BaseTypedProposalSchema.extend({
  type: z.literal('decision'),
  metadata: DecisionMetadataSchema,
});

export const PatternProposalSchema = BaseTypedProposalSchema.extend({
  type: z.literal('pattern'),
  metadata: PatternMetadataSchema,
});

export const WarningProposalSchema = BaseTypedProposalSchema.extend({
  type: z.literal('warning'),
  metadata: WarningMetadataSchema,
});

export const LessonProposalSchema = BaseTypedProposalSchema.extend({
  type: z.literal('lesson'),
  metadata: LessonMetadataSchema,
});

export const AnomalyProposalSchema = BaseTypedProposalSchema.extend({
  type: z.literal('anomaly'),
  metadata: AnomalyMetadataSchema,
});

export const ConstraintProposalSchema = BaseTypedProposalSchema.extend({
  type: z.literal('constraint'),
  metadata: ConstraintMetadataSchema,
});

export const TypedProposalSchema = z.discriminatedUnion('type', [
  DecisionProposalSchema,
  PatternProposalSchema,
  WarningProposalSchema,
  LessonProposalSchema,
  AnomalyProposalSchema,
  ConstraintProposalSchema,
]);

export type DecisionProposal = z.infer<typeof DecisionProposalSchema>;
export type PatternProposal = z.infer<typeof PatternProposalSchema>;
export type WarningProposal = z.infer<typeof WarningProposalSchema>;
export type LessonProposal = z.infer<typeof LessonProposalSchema>;
export type AnomalyProposal = z.infer<typeof AnomalyProposalSchema>;
export type ConstraintProposal = z.infer<typeof ConstraintProposalSchema>;
export type TypedProposal = z.infer<typeof TypedProposalSchema>;

export interface ProposalMetadataByType {
  decision: z.infer<typeof DecisionMetadataSchema>;
  pattern: z.infer<typeof PatternMetadataSchema>;
  warning: z.infer<typeof WarningMetadataSchema>;
  lesson: z.infer<typeof LessonMetadataSchema>;
  anomaly: z.infer<typeof AnomalyMetadataSchema>;
  constraint: z.infer<typeof ConstraintMetadataSchema>;
}

const ProposalMetadataSchemaMap = {
  decision: DecisionMetadataSchema,
  pattern: PatternMetadataSchema,
  warning: WarningMetadataSchema,
  lesson: LessonMetadataSchema,
  anomaly: AnomalyMetadataSchema,
  constraint: ConstraintMetadataSchema,
} satisfies Record<ProposalType, z.ZodTypeAny>;

export function validateProposalMetadata<T extends ProposalType>(
  type: T,
  metadata: unknown
): ProposalMetadataByType[T] | null {
  const result = ProposalMetadataSchemaMap[type].safeParse(metadata);
  return result.success ? (result.data as ProposalMetadataByType[T]) : null;
}

export function createTypedProposal<T extends ProposalType>(
  type: T,
  metadata: unknown
): { type: T; metadata: ProposalMetadataByType[T] } {
  const validatedMetadata = ProposalMetadataSchemaMap[type].parse(
    metadata
  ) as ProposalMetadataByType[T];
  return {
    type,
    metadata: validatedMetadata,
  };
}

export function parseTypedProposal(proposal: unknown): TypedProposal {
  return TypedProposalSchema.parse(proposal);
}
