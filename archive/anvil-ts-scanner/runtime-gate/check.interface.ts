import { CheckContext, GateResult } from '../types/gate.types.js';

export interface Check {
  name: string;
  description: string;
  run(context: CheckContext): Promise<GateResult>;
}

export abstract class BaseCheck implements Check {
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
