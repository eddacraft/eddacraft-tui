/**
 * Command result type definitions for CLI commands
 * @module cli/types/command-results
 */

import type { APSPlan, ValidationResult, GateRunResult, Evidence } from '@eddacraft/anvil-core';
import type { AdapterError, AdapterWarning } from '@eddacraft/anvil-adapters';
import type { EvidenceInjectionResult, SourceFormatInfo } from './services.js';

/**
 * Result of validate command execution
 */
export interface ValidateCommandResult {
  /** Whether validation passed */
  success: boolean;
  /** Validated APS plan */
  plan?: APSPlan;
  /** Validation result details */
  validation: ValidationResult;
  /** Source format information */
  sourceFormat?: SourceFormatInfo;
  /** Warnings from parsing/conversion */
  warnings?: AdapterWarning[];
}

/**
 * Result of gate command execution
 */
export interface GateCommandResult {
  /** Whether all gates passed */
  success: boolean;
  /** Gate execution results */
  gateResult: GateRunResult;
  /** APS plan that was gated */
  plan: APSPlan;
  /** Evidence bundle */
  evidence: Evidence;
  /** Evidence injection result (if requested) */
  injection?: EvidenceInjectionResult;
  /** Source format information */
  sourceFormat?: SourceFormatInfo;
}

/**
 * Result of export command execution
 */
export interface ExportCommandResult {
  /** Whether export succeeded */
  success: boolean;
  /** Path to exported file */
  outputPath?: string;
  /** Exported content */
  content?: string;
  /** Target format */
  targetFormat: string;
  /** Errors encountered */
  errors?: AdapterError[];
  /** Warnings encountered */
  warnings?: AdapterWarning[];
}

/**
 * Result of import command execution
 */
export interface ImportCommandResult {
  /** Whether import succeeded */
  success: boolean;
  /** Imported APS plan */
  plan?: APSPlan;
  /** Path to saved APS file */
  outputPath?: string;
  /** Source format information */
  sourceFormat: SourceFormatInfo;
  /** Conversion warnings */
  warnings?: AdapterWarning[];
}
