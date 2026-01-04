import { z } from 'zod';

export const ExplanationSectionSchema = z.object({
  title: z.string().describe('Section title (e.g., "WHY THIS WARNING EXISTS")'),
  content: z.string().describe('Section content, may contain placeholders'),
});

export type ExplanationSection = z.infer<typeof ExplanationSectionSchema>;

export const WarningExplanationSchema = z.object({
  ruleId: z.string().describe('Rule ID (e.g., AP-003, ARCH-001)'),
  title: z.string().describe('Warning title'),
  summary: z.string().describe('One-line summary of the warning'),
  whyItMatters: ExplanationSectionSchema.describe('Why this warning exists'),
  howToAddress: ExplanationSectionSchema.describe('How to fix or address'),
  whenToSuppress: ExplanationSectionSchema.describe('When suppression is appropriate'),
  related: z
    .object({
      documentation: z.string().optional(),
      ruleDefinition: z.string().optional(),
      similarWarnings: z.number().int().nonnegative().optional(),
    })
    .optional(),
});

export type WarningExplanation = z.infer<typeof WarningExplanationSchema>;

export const ExplanationContextSchema = z.object({
  file: z.string().describe('File path where warning occurred'),
  line: z.number().int().positive().describe('Line number'),
  code: z.string().optional().describe('Source code snippet at the location'),
  fromFile: z.string().optional().describe('Source file (for boundary violations)'),
  toFile: z.string().optional().describe('Target file (for boundary violations)'),
  fromLayer: z.string().optional().describe('Source layer (for boundary violations)'),
  toLayer: z.string().optional().describe('Target layer (for boundary violations)'),
  patternName: z.string().optional().describe('Pattern name (for anti-patterns)'),
  similarCount: z.number().int().nonnegative().optional(),
});

export type ExplanationContext = z.infer<typeof ExplanationContextSchema>;

export interface ExplanationTemplate {
  ruleId: string;
  render(context: ExplanationContext): WarningExplanation;
}
