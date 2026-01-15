import { z } from 'zod';

export const ArchViolationSeveritySchema = z.enum(['error', 'warn', 'info', 'ignore']);
export type ArchViolationSeverity = z.infer<typeof ArchViolationSeveritySchema>;

export const ArchViolationSchema = z.object({
  from: z.string(),
  to: z.string(),
  rule: z.string(),
  severity: ArchViolationSeveritySchema,
  is_circular: z.boolean(),
  cycle: z.array(z.string()).optional(),
  is_new: z.boolean(),
  from_layer: z.string().nullable(),
  to_layer: z.string().nullable(),
});
export type ArchViolation = z.infer<typeof ArchViolationSchema>;

export const ModuleInfoSchema = z.object({
  path: z.string(),
  layer: z.string().nullable(),
  confidence: z.enum(['high', 'medium', 'low']).optional(),
  matched_pattern: z.string().optional(),
  dependency_count: z.number().int().nonnegative(),
  dependent_count: z.number().int().nonnegative(),
  is_orphan: z.boolean(),
});
export type ModuleInfo = z.infer<typeof ModuleInfoSchema>;

export const LayerStatsSchema = z.object({
  name: z.string(),
  module_count: z.number().int().nonnegative(),
  violations_from: z.number().int().nonnegative(),
  violations_to: z.number().int().nonnegative(),
  depends_on: z.array(z.string()),
  patterns: z.array(z.string()),
});
export type LayerStats = z.infer<typeof LayerStatsSchema>;

export const ArchitectureContextSchema = z.object({
  timestamp: z.string(),
  summary: z.object({
    total_modules: z.number().int().nonnegative(),
    total_violations: z.number().int().nonnegative(),
    new_violations: z.number().int().nonnegative(),
    error_count: z.number().int().nonnegative(),
    warn_count: z.number().int().nonnegative(),
    info_count: z.number().int().nonnegative(),
    circular_count: z.number().int().nonnegative(),
    orphan_count: z.number().int().nonnegative(),
    layer_violation_count: z.number().int().nonnegative(),
    baseline_loaded: z.boolean(),
  }),
  violations: z.array(ArchViolationSchema),
  layers: z.record(z.string(), LayerStatsSchema),
  modules: z.array(ModuleInfoSchema).optional(),
  dependencies: z.record(z.string(), z.array(z.string())).optional(),
  config: z
    .object({
      config_file: z.string().optional(),
      scope: z.enum(['affected', 'full']).optional(),
      severity_threshold: z.enum(['error', 'warn', 'info']).optional(),
    })
    .optional(),
});
export type ArchitectureContext = z.infer<typeof ArchitectureContextSchema>;

export function createEmptyContext(): ArchitectureContext {
  return {
    timestamp: new Date().toISOString(),
    summary: {
      total_modules: 0,
      total_violations: 0,
      new_violations: 0,
      error_count: 0,
      warn_count: 0,
      info_count: 0,
      circular_count: 0,
      orphan_count: 0,
      layer_violation_count: 0,
      baseline_loaded: false,
    },
    violations: [],
    layers: {},
  };
}

export class ArchitectureContextBuilder {
  private context: ArchitectureContext;

  constructor() {
    this.context = createEmptyContext();
  }

  setSummary(summary: Partial<ArchitectureContext['summary']>): this {
    this.context.summary = { ...this.context.summary, ...summary };
    return this;
  }

  addViolation(violation: ArchViolation): this {
    this.context.violations.push(violation);
    return this;
  }

  setViolations(violations: ArchViolation[]): this {
    this.context.violations = violations;
    return this;
  }

  addLayerStats(name: string, stats: Omit<LayerStats, 'name'>): this {
    this.context.layers[name] = { name, ...stats };
    return this;
  }

  setModules(modules: ModuleInfo[]): this {
    this.context.modules = modules;
    return this;
  }

  setDependencies(deps: Record<string, string[]>): this {
    this.context.dependencies = deps;
    return this;
  }

  setConfig(config: ArchitectureContext['config']): this {
    this.context.config = config;
    return this;
  }

  build(): ArchitectureContext {
    this.context.timestamp = new Date().toISOString();
    return this.context;
  }
}

export function createContextBuilder(): ArchitectureContextBuilder {
  return new ArchitectureContextBuilder();
}
