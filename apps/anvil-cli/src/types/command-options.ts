/**
 * Command option type definitions for CLI commands
 * @module cli/types/command-options
 */

import type { AdapterOptions } from '@anvil/adapters';

/**
 * Available gate profiles
 */
export type GateProfile = 'dev' | 'ci' | 'production';

/**
 * Options for the validate command
 */
export interface ValidateOptions {
  /** Enable verbose output with detailed validation results */
  verbose?: boolean;
  /** Validate hash integrity */
  validateHash?: boolean;
  /** Explicitly specify input format (bypasses auto-detection) */
  format?: string;
  /** Adapter-specific options */
  adapterOptions?: AdapterOptions;
  /** Skip format detection and treat as native APS */
  native?: boolean;
}

/**
 * Options for the gate command
 */
export interface GateOptions {
  /** Custom config file path */
  config?: string;
  /** Enable verbose output */
  verbose?: boolean;
  /** Inject evidence back into source document */
  inject?: boolean;
  /** Explicitly specify input format */
  format?: string;
  /** Skip format detection and treat as native APS */
  native?: boolean;
  /** Skip specific checks (comma-separated string) */
  skipChecks?: string;
  /** Run only specific checks (comma-separated string) */
  onlyChecks?: string;
  /** Fail fast on first error */
  failFast?: boolean;
  /** Adapter-specific options */
  adapterOptions?: AdapterOptions;
  /** Use predefined gate profile */
  profile?: GateProfile;
  /** List available gate profiles */
  listProfiles?: boolean;
  /** Disable caching */
  noCache?: boolean;
  /** Limit parallel execution (0 = sequential) */
  parallel?: string;
  /** Output format (human, json) */
  output?: 'human' | 'json';
  /** Show real-time progress for checks */
  progress?: boolean;
  /** Force TUI explorer mode after gate execution */
  tui?: boolean;
  /** Force plain text mode (no TUI) */
  noTui?: boolean;
  /** Skip command safety check (convenience flag) */
  skipCommandSafety?: boolean;
}

/**
 * Options for the export command
 */
export interface ExportOptions {
  /** Target format to export to */
  to: string;
  /** Output file path or directory */
  output?: string;
  /** Source format (auto-detected if not specified) */
  from?: string;
  /** Compact JSON output (no pretty-printing) */
  compact?: boolean;
  /** Adapter-specific options */
  adapterOptions?: AdapterOptions;
}

/**
 * Options for the import command
 */
export interface ImportOptions {
  /** Source format (auto-detected if not specified) */
  format?: string;
  /** Output file path for APS plan */
  output?: string;
  /** Output format (json, yaml) */
  outputFormat?: 'json' | 'yaml';
  /** Include provenance metadata */
  includeProvenance?: boolean;
  /** Adapter-specific options */
  adapterOptions?: AdapterOptions;
}
