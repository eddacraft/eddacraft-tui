import { z } from 'zod';

export const ARCHITECTURE_DEFINITION_VERSION = '0.1.0' as const;

export const ArchitectureTemplateSchema = z.enum([
  'starter',
  'layered',
  'hexagonal',
  'clean',
  'ddd',
  'monorepo',
  'serverless',
  'nx-workspace',
  'custom',
]);
export type ArchitectureTemplate = z.infer<typeof ArchitectureTemplateSchema>;

export const LayerDefinitionSchema = z.object({
  patterns: z.array(z.string()).min(1),
  depends_on: z.array(z.string()).default([]),
  description: z.string().optional(),
});
export type LayerDefinition = z.infer<typeof LayerDefinitionSchema>;

export const BoundedContextSchema = z.object({
  layers: z.record(z.string(), LayerDefinitionSchema).optional(),
  allowed_dependencies: z.array(z.string()).default([]),
  description: z.string().optional(),
});
export type BoundedContext = z.infer<typeof BoundedContextSchema>;

export const RuleSeveritySchema = z.enum(['error', 'warn', 'info', 'ignore']);
export type RuleSeverity = z.infer<typeof RuleSeveritySchema>;

export const ArchitectureRuleSchema = z.object({
  name: z.string(),
  from: z.string(),
  to: z.string(),
  severity: RuleSeveritySchema.default('error'),
  allowed: z.boolean().default(false),
  message: z.string().optional(),
});
export type ArchitectureRule = z.infer<typeof ArchitectureRuleSchema>;

export const ArchitectureOptionsSchema = z.object({
  detect_orphans: z.boolean().default(true),
  detect_circular: z.boolean().default(true),
  default_severity: RuleSeveritySchema.default('error'),
  exclude_patterns: z
    .array(z.string())
    .default([
      '**/*.test.ts',
      '**/*.spec.ts',
      '**/__tests__/**',
      '**/__fixtures__/**',
      '**/node_modules/**',
    ]),
});
export type ArchitectureOptions = z.infer<typeof ArchitectureOptionsSchema>;

export const ArchitectureDefinitionSchema = z.object({
  schema_version: z
    .literal(ARCHITECTURE_DEFINITION_VERSION)
    .default(ARCHITECTURE_DEFINITION_VERSION),
  template: ArchitectureTemplateSchema.default('custom'),
  layers: z.record(z.string(), LayerDefinitionSchema).default({}),
  bounded_contexts: z.record(z.string(), BoundedContextSchema).optional(),
  rules: z.array(ArchitectureRuleSchema).default([]),
  options: ArchitectureOptionsSchema.optional(),
});
export type ArchitectureDefinition = z.infer<typeof ArchitectureDefinitionSchema>;

export const AVAILABLE_TEMPLATES: ArchitectureTemplate[] = [
  'starter',
  'layered',
  'hexagonal',
  'clean',
  'ddd',
  'monorepo',
  'serverless',
  'nx-workspace',
  'custom',
];

export function getAvailableTemplates(): ArchitectureTemplate[] {
  return [...AVAILABLE_TEMPLATES];
}

export function isValidTemplate(template: string): template is ArchitectureTemplate {
  return AVAILABLE_TEMPLATES.includes(template as ArchitectureTemplate);
}

export function validateArchitectureDefinition(
  data: unknown
): { success: true; data: ArchitectureDefinition } | { success: false; error: z.ZodError } {
  const result = ArchitectureDefinitionSchema.safeParse(data);
  if (result.success) {
    return { success: true, data: result.data };
  }
  return { success: false, error: result.error };
}

export function getDefaultOptions(): ArchitectureOptions {
  return {
    detect_orphans: true,
    detect_circular: true,
    default_severity: 'error',
    exclude_patterns: [
      '**/*.test.ts',
      '**/*.spec.ts',
      '**/__tests__/**',
      '**/__fixtures__/**',
      '**/node_modules/**',
    ],
  };
}
