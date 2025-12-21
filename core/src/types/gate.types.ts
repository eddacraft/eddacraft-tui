// Import APSPlan type from schema
import type { APSPlan } from '../schema/index.js';
import type { WatchConfig } from '../watch/types.js';
import type { Warning, WarningResult } from '../antipattern/types.js';

export interface GateCheck {
  name: string;
  description: string;
  enabled: boolean;
  config?: Record<string, unknown>;
}

/**
 * Extended details that can include warnings from anti-pattern/boundary checks
 */
export interface GateResultDetails {
  /** Warnings from anti-pattern or boundary detection */
  warnings?: WarningResult;
  /** Any other check-specific details */
  [key: string]: unknown;
}

export interface GateResult {
  check: string;
  passed: boolean;
  score?: number;
  message: string;
  details?: GateResultDetails;
  error?: string;
  skipped?: boolean;
}

/**
 * Helper to extract warnings from a GateResult
 */
export function getWarningsFromResult(result: GateResult): Warning[] {
  return result.details?.warnings?.warnings ?? [];
}

/**
 * Helper to check if a GateResult has blocking warnings
 */
export function hasBlockingWarnings(result: GateResult): boolean {
  const warnings = getWarningsFromResult(result);
  return warnings.some((w) => w.severity === 'error' && !w.suppressed);
}

export interface GateRunResult {
  overall: boolean;
  score: number;
  checks: GateResult[];
  summary: {
    total: number;
    passed: number;
    failed: number;
    skipped: number;
  };
}

export interface GateConfig {
  version: number;
  checks: GateCheck[];
  thresholds: {
    overall_score: number;
    [key: string]: number;
  };
  global_config?: Record<string, unknown>;
  /** Watch mode configuration */
  watch?: WatchConfig;
}

// Use APSPlan directly instead of a separate interface
export type PlanData = APSPlan;

export interface CheckContext {
  /** Plan data - optional for planless mode */
  plan?: PlanData;
  workspace_root: string;
  config: GateConfig;
  check_config: Record<string, unknown>;
  /** Full codebase scan mode (no plan-based scoping) */
  fullScan?: boolean;
  /** Explicit file list for planless mode */
  targetFiles?: string[];
}
