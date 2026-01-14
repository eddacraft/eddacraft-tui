/**
 * Watch Mode Types and Schemas
 *
 * Defines configuration and event types for file watching functionality.
 */

import { z } from 'zod';

/**
 * Git filter configuration
 */
export const WatchGitConfigSchema = z.object({
  /** Only watch files with unstaged changes (default: true) */
  unstagedOnly: z.boolean().default(true),
  /** Include untracked files in watch (default: true) */
  includeUntracked: z.boolean().default(true),
});

export type WatchGitConfig = z.infer<typeof WatchGitConfigSchema>;

/**
 * Watch configuration schema for .anvilrc
 */
export const WatchConfigSchema = z.object({
  /** Enable watch mode in config (default: false) */
  enabled: z.boolean().default(false),
  /** Glob patterns to watch (default: planning document patterns) */
  patterns: z.array(z.string()).default(['**/*.md']),
  /** Glob patterns to exclude */
  exclude: z
    .array(z.string())
    .default(['node_modules/**', 'dist/**', 'build/**', '.git/**', 'coverage/**']),
  /** Action to run on change: validate, gate, or check */
  action: z.enum(['validate', 'gate', 'check']).default('validate'),
  /** Debounce interval in milliseconds (default: 300) */
  debounceMs: z.number().min(50).max(5000).default(300),
  /** Git filter configuration */
  git: WatchGitConfigSchema.default({ unstagedOnly: true, includeUntracked: true }),
  /** Gate profile to use when action is 'gate' */
  gateProfile: z.enum(['dev', 'ci', 'production']).optional(),
});

export type WatchConfig = z.infer<typeof WatchConfigSchema>;

/**
 * Git file status from git status --porcelain
 */
export interface GitFileStatus {
  /** File path relative to workspace root */
  path: string;
  /** Whether file is tracked by git */
  isTracked: boolean;
  /** Whether file has staged changes */
  isStaged: boolean;
  /** Whether file has unstaged changes */
  isUnstaged: boolean;
  /** Whether file is untracked */
  isUntracked: boolean;
  /** Raw status code from git (e.g., ' M', 'M ', '??') */
  statusCode: string;
}

/**
 * File change event from watcher
 */
export interface WatchChangeEvent {
  /** Event type */
  type: 'add' | 'change' | 'unlink';
  /** Absolute file path */
  path: string;
  /** Timestamp of change */
  timestamp: Date;
}

/**
 * Debounced batch of changes
 */
export interface DebouncedChanges {
  /** Files that changed */
  files: string[];
  /** Timestamp when batch was flushed */
  timestamp: Date;
}

/**
 * Watch status event types
 */
export type WatchStatusEventType =
  | 'ready'
  | 'change'
  | 'action:start'
  | 'action:complete'
  | 'action:error'
  | 'stopped';

/**
 * Action result from validate, gate, or check
 */
export interface WatchActionResult {
  /** Whether the action succeeded */
  success: boolean;
  /** Action type that was run */
  action: 'validate' | 'gate' | 'check';
  /** Files that were processed */
  files: string[];
  /** Execution time in ms */
  executionTimeMs: number;
  /** Detailed results (validation errors, gate results, warnings, etc.) */
  details?: unknown;
  /** Error message if action failed */
  error?: string;
}

/**
 * Status events emitted by watch orchestrator
 */
export type WatchStatusEvent =
  | { type: 'ready'; patterns: string[]; gitFilter: boolean }
  | { type: 'change'; files: string[]; filtered: string[] }
  | { type: 'action:start'; action: 'validate' | 'gate' | 'check'; files: string[] }
  | { type: 'action:complete'; result: WatchActionResult }
  | { type: 'action:error'; error: Error; files: string[] }
  | { type: 'stopped' };

/**
 * Watch orchestrator options
 */
export interface WatchOrchestratorOptions {
  /** Workspace root directory */
  workspaceRoot: string;
  /** Watch configuration */
  config: WatchConfig;
  /** Status event callback */
  onEvent?: (event: WatchStatusEvent) => void;
  /** Verbose logging */
  verbose?: boolean;
}

/**
 * Default watch patterns (reused from file-discovery)
 */
export const DEFAULT_WATCH_PATTERNS = [
  '**/*.md',
  '**/prd.*',
  '**/plan.*',
  '**/todo.*',
  '**/spec.*',
  '**/requirements.*',
  '**/rfc-*',
  '**/adr-*',
];

/**
 * Default exclude patterns
 */
export const DEFAULT_EXCLUDE_PATTERNS = [
  'node_modules/**',
  'dist/**',
  'build/**',
  '.git/**',
  'coverage/**',
  '.next/**',
  '.nuxt/**',
  'out/**',
  'target/**',
  'vendor/**',
];

/**
 * Parse and validate watch config from raw object
 */
export function parseWatchConfig(raw: unknown): WatchConfig {
  return WatchConfigSchema.parse(raw);
}

/**
 * Get default watch config
 */
export function getDefaultWatchConfig(): WatchConfig {
  return WatchConfigSchema.parse({});
}
