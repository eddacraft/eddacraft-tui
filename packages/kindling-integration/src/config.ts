/**
 * Kindling Configuration Schema (KINDLING-002)
 *
 * Defines configuration for the Kindling integration layer.
 * Configuration is read from `.anvilrc` or `anvil.config.json` in the project root.
 *
 * @see kindling-service.ts for how config is consumed
 */

import { z } from 'zod';
import { readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';

// =============================================================================
// Configuration Schema
// =============================================================================

/**
 * Capture flags: which observation kinds to record
 */
export const CaptureConfigSchema = z.object({
  sessions: z.boolean().default(true).describe('Record session start/end'),
  gates: z.boolean().default(true).describe('Record gate evaluations'),
  actions: z.boolean().default(true).describe('Record action executions'),
  plans: z.boolean().default(true).describe('Record plan lifecycle events'),
  human_inputs: z.boolean().default(true).describe('Record human inputs'),
  constraints: z.boolean().default(true).describe('Record constraint applications'),
  errors: z.boolean().default(true).describe('Record errors'),
});

export type CaptureConfig = z.infer<typeof CaptureConfigSchema>;

/**
 * Retention policy
 */
export const RetentionConfigSchema = z.object({
  days: z.number().int().positive().default(90).describe('Days to retain observations'),
  auto_prune: z.boolean().default(false).describe('Automatically prune on session start'),
});

export type RetentionConfig = z.infer<typeof RetentionConfigSchema>;

/**
 * Query limit defaults
 */
export const QueryLimitConfigSchema = z.object({
  max_results: z
    .number()
    .int()
    .positive()
    .max(1000)
    .default(100)
    .describe('Default max results per query'),
  max_payload_bytes: z
    .number()
    .int()
    .positive()
    .max(10 * 1024 * 1024)
    .default(1024 * 1024)
    .describe('Default max payload size in bytes'),
});

export type QueryLimitConfig = z.infer<typeof QueryLimitConfigSchema>;

/**
 * Full Kindling configuration
 */
export const KindlingConfigSchema = z.object({
  enabled: z.boolean().default(false).describe('Whether Kindling recording is active'),
  database_path: z
    .string()
    .default('.anvil/kindling.db')
    .describe('Path to the SQLite database (relative to project root)'),
  retention: RetentionConfigSchema.default(() => ({
    days: 90,
    auto_prune: false,
  })),
  capture: CaptureConfigSchema.default(() => ({
    sessions: true,
    gates: true,
    actions: true,
    plans: true,
    human_inputs: true,
    constraints: true,
    errors: true,
  })),
  query_limits: QueryLimitConfigSchema.default(() => ({
    max_results: 100,
    max_payload_bytes: 1024 * 1024,
  })),
});

export type KindlingConfig = z.infer<typeof KindlingConfigSchema>;

// =============================================================================
// Default Configuration
// =============================================================================

/**
 * Default configuration: disabled, sensible defaults for everything else
 */
export const DEFAULT_KINDLING_CONFIG: KindlingConfig = KindlingConfigSchema.parse({});

// =============================================================================
// Configuration Loading
// =============================================================================

/**
 * Known configuration file names, checked in order
 */
const CONFIG_FILE_NAMES = ['.anvilrc', 'anvil.config.json'] as const;

/**
 * Load Kindling configuration from the project root.
 *
 * Searches for `.anvilrc` or `anvil.config.json` in the given directory.
 * Expects a JSON file with an optional `kindling` key containing the config.
 *
 * Returns the default (disabled) config if no file is found or if the
 * `kindling` key is absent.
 *
 * @param projectRoot - Absolute path to the project root directory
 * @returns Parsed and validated KindlingConfig
 */
export function loadKindlingConfig(projectRoot: string): KindlingConfig {
  for (const fileName of CONFIG_FILE_NAMES) {
    const filePath = join(projectRoot, fileName);

    if (!existsSync(filePath)) {
      continue;
    }

    try {
      const raw = readFileSync(filePath, 'utf-8');
      const parsed: unknown = JSON.parse(raw);

      if (parsed === null || typeof parsed !== 'object') {
        continue;
      }

      const record = parsed as Record<string, unknown>;

      if (!('kindling' in record) || record['kindling'] === undefined) {
        // Config file exists but has no kindling section
        return DEFAULT_KINDLING_CONFIG;
      }

      const result = KindlingConfigSchema.safeParse(record['kindling']);

      if (result.success) {
        return result.data;
      }

      // Invalid config shape -- fall back to defaults rather than crashing
      // The caller (service layer) will operate in disabled mode
      return DEFAULT_KINDLING_CONFIG;
    } catch {
      // JSON parse error or read error -- fall back to defaults
      return DEFAULT_KINDLING_CONFIG;
    }
  }

  // No config file found
  return DEFAULT_KINDLING_CONFIG;
}

/**
 * Check whether a specific observation kind should be captured based on config.
 *
 * @param config - Kindling configuration
 * @param kind - Observation kind string
 * @returns true if the observation should be captured
 */
export function shouldCapture(config: KindlingConfig, kind: string): boolean {
  if (!config.enabled) {
    return false;
  }

  switch (kind) {
    case 'session_start':
    case 'session_end':
      return config.capture.sessions;
    case 'gate_evaluated':
      return config.capture.gates;
    case 'action_executed':
      return config.capture.actions;
    case 'plan_created':
    case 'plan_edited':
    case 'plan_approved':
    case 'plan_rejected':
      return config.capture.plans;
    case 'human_input':
      return config.capture.human_inputs;
    case 'constraint_applied':
      return config.capture.constraints;
    case 'error':
      return config.capture.errors;
    default:
      // Unknown kinds are captured by default when enabled
      return true;
  }
}
