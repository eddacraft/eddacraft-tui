/**
 * Check interface definitions
 *
 * Defines the contract for gate checks.
 */

import type { APSPlan, Warning, WarningResult } from '@anvil/contracts';

/**
 * Extended details that can include warnings from anti-pattern/boundary checks
 */
export interface GateResultDetails {
  /** Warnings from anti-pattern or boundary detection */
  warnings?: WarningResult;
  /** Any other check-specific details */
  [key: string]: unknown;
}

/**
 * Result of a gate check execution
 */
export interface GateResult {
  check: string;
  passed: boolean;
  score?: number;
  message: string;
  details?: GateResultDetails;
  error?: string;
}

/**
 * Context provided to checks during execution
 */
export interface CheckContext {
  /** The plan being validated (optional for planless checks) */
  plan?: APSPlan;
  /** Root directory of the workspace */
  workspaceRoot: string;
  /** Check-specific configuration */
  config?: Record<string, unknown>;
  /** Whether to run in verbose mode */
  verbose?: boolean;
}

/**
 * Check interface - implemented by all gate checks
 */
export interface ICheck {
  /** Unique name of the check */
  name: string;
  /** Human-readable description */
  description: string;
  /** Execute the check */
  run(context: CheckContext): Promise<GateResult>;
}

/**
 * Abstract base class for implementing checks
 */
export abstract class BaseCheck implements ICheck {
  abstract name: string;
  abstract description: string;

  abstract run(context: CheckContext): Promise<GateResult>;

  protected createResult(
    passed: boolean,
    message: string,
    score?: number,
    details?: Record<string, unknown>,
    error?: string
  ): GateResult {
    return {
      check: this.name,
      passed,
      score,
      message,
      details,
      error,
    };
  }

  protected createSuccess(
    message: string,
    score?: number,
    details?: Record<string, unknown>
  ): GateResult {
    return this.createResult(true, message, score, details);
  }

  protected createFailure(
    message: string,
    error?: string,
    details?: Record<string, unknown>
  ): GateResult {
    return this.createResult(false, message, undefined, details, error);
  }
}
