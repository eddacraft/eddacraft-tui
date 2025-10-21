/**
 * Command option type definitions for CLI commands
 * @module cli/types/command-options
 */

import type { AdapterOptions } from '@anvil/adapters';

/**
 * Options for the validate command
 */
export interface ValidateOptions {
  /** Enable verbose output with detailed validation results */
  verbose?: boolean;
  /** Validate hash integrity */
  validateHash?: boolean;
  /** Output format (cli, json, yaml) */
  format?: 'cli' | 'json' | 'yaml';
  /** Explicitly specify input format (bypasses auto-detection) */
  inputFormat?: string;
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
  inputFormat?: string;
  /** Skip format detection and treat as native APS */
  native?: boolean;
  /** Skip specific checks */
  skipChecks?: string[];
  /** Run only specific checks */
  onlyChecks?: string[];
  /** Fail fast on first error */
  failFast?: boolean;
  /** Adapter-specific options */
  adapterOptions?: AdapterOptions;
}

/**
 * Options for the export command
 */
export interface ExportOptions {
  /** Target format to export to */
  format: string;
  /** Output file path */
  output?: string;
  /** Pretty-print output */
  pretty?: boolean;
  /** Preserve comments and formatting from original */
  preserveFormatting?: boolean;
  /** Include evidence in exported document */
  includeEvidence?: boolean;
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
