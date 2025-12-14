// Import APSPlan type from schema
import type { APSPlan } from '../schema/index.js';
import type { WatchConfig } from '../watch/types.js';

export interface GateCheck {
  name: string;
  description: string;
  enabled: boolean;
  config?: Record<string, unknown>;
}

export interface GateResult {
  check: string;
  passed: boolean;
  score?: number;
  message: string;
  details?: Record<string, unknown>;
  error?: string;
  skipped?: boolean;
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
  plan: PlanData;
  workspace_root: string;
  config: GateConfig;
  check_config: Record<string, unknown>;
  /** Full codebase scan mode (no plan-based scoping) */
  fullScan?: boolean;
}
