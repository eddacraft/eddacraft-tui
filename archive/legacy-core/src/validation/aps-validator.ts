/**
 * APS Validator
 *
 * Main validation class for Anvil Plan Specification documents.
 * Provides comprehensive validation including schema and hash verification.
 */

import { APSPlan, APSPlanSchema } from '../schema/index.js';
import {
  SchemaValidationError,
  HashValidationError,
  ValidationIssue,
  formatZodErrors,
  formatValidationErrors,
  createValidationSummary,
} from './errors.js';

export interface ValidationResult {
  valid: boolean;
  data?: APSPlan;
  issues?: ValidationIssue[];
  summary: string;
  formattedErrors?: string;
  hashValidated?: boolean;
}

/**
 * Validation options
 */
export interface ValidationOptions {
  validateHash?: boolean;
  strict?: boolean;
  format?: 'cli' | 'json';
}

/**
 * APS Validator class
 *
 * Provides comprehensive validation for APS plans including:
 * - Schema validation using Zod
 * - Hash integrity verification (when crypto module is available)
 * - User-friendly error formatting
 */
export class APSValidator {
  private hashValidator?: (data: unknown) => string;

  constructor() {
    // Hash validator will be injected when crypto module is ready
    this.hashValidator = undefined;
  }

  /**
   * Set the hash validator function (to be called when crypto module is ready)
   */
  setHashValidator(validator: (data: unknown) => string): void {
    this.hashValidator = validator;
  }

  /**
   * Validate an APS plan
   */
  async validate(plan: unknown, options: ValidationOptions = {}): Promise<ValidationResult> {
    const { validateHash = false, strict = true, format = 'cli' } = options;

    const issues: ValidationIssue[] = [];

    // Step 1: Schema validation
    const schemaResult = await this.validateSchema(plan);
    if (!schemaResult.valid) {
      issues.push(...(schemaResult.issues || []));
    }

    // If schema validation failed and we're in strict mode, stop here
    if (strict && issues.length > 0) {
      return this.createResult(false, undefined, issues, format);
    }

    // Step 2: Hash validation (if requested and schema passed)
    let hashValidationInfo: string | undefined;
    if (validateHash && schemaResult.valid && schemaResult.data) {
      const hashResult = await this.validateHash(schemaResult.data);
      if (!hashResult.valid) {
        issues.push(...(hashResult.issues || []));
      }
      // Store hash validation summary for inclusion in final result
      hashValidationInfo = hashResult.summary;
    }

    // Create final result
    const isValid = issues.length === 0;
    const result = this.createResult(
      isValid,
      isValid ? schemaResult.data : undefined,
      issues,
      format
    );

    // If hash validation was performed and everything passed, include hash validation info
    if (isValid && hashValidationInfo) {
      result.summary = `${result.summary} - ${hashValidationInfo}`;
    }

    return result;
  }

  /**
   * Validate plan against schema
   */
  async validateSchema(plan: unknown): Promise<ValidationResult> {
    try {
      const result = APSPlanSchema.safeParse(plan);

      if (result.success) {
        return {
          valid: true,
          data: result.data,
          summary: '✅ Schema validation passed',
        };
      }

      // Format Zod errors
      const issues = formatZodErrors(result.error.issues);

      return {
        valid: false,
        issues,
        summary: createValidationSummary(false, issues),
        formattedErrors: formatValidationErrors(issues),
      };
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      throw new SchemaValidationError(`Schema validation failed: ${errorMessage}`, []);
    }
  }

  async validateHash(plan: APSPlan): Promise<ValidationResult> {
    if (!this.hashValidator) {
      return {
        valid: true,
        hashValidated: false,
        issues: [
          {
            path: 'hash',
            message: 'Hash validation skipped (crypto module not available)',
            code: 'HASH_VALIDATION_SKIPPED',
            severity: 'warning',
          },
        ],
        summary: '⚠️ Hash validation skipped (crypto module not available)',
      };
    }

    try {
      // Create a copy of the plan without the hash field for calculation
      const { hash: expectedHash, ...planWithoutHash } = plan;

      // Calculate the actual hash
      const actualHash = this.hashValidator(planWithoutHash);

      if (actualHash === expectedHash) {
        return {
          valid: true,
          hashValidated: true,
          summary: '✅ Hash validation passed',
        };
      }

      // Hash mismatch
      const issue: ValidationIssue = {
        path: 'hash',
        message: `Hash mismatch: expected ${expectedHash}, got ${actualHash}`,
        code: 'HASH_MISMATCH',
        severity: 'error',
      };

      return {
        valid: false,
        issues: [issue],
        summary: createValidationSummary(false, [issue]),
        formattedErrors: formatValidationErrors([issue]),
      };
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      throw new HashValidationError(
        `Hash validation failed: ${errorMessage}`,
        plan.hash,
        'calculation_failed'
      );
    }
  }

  /**
   * Quick check if a plan's schema is valid
   */
  isSchemaValid(plan: unknown): boolean {
    return APSPlanSchema.safeParse(plan).success;
  }

  /**
   * Create a validation result
   */
  private createResult(
    valid: boolean,
    data: APSPlan | undefined,
    issues: ValidationIssue[],
    format: 'cli' | 'json'
  ): ValidationResult {
    const result: ValidationResult = {
      valid,
      data,
      issues: issues.length > 0 ? issues : undefined,
      summary: createValidationSummary(valid, issues),
    };

    if (format === 'cli' && issues.length > 0) {
      result.formattedErrors = formatValidationErrors(issues);
    }

    return result;
  }
}

/**
 * Singleton instance for convenience
 */
export const validator = new APSValidator();

/**
 * Convenience function for quick validation
 */
export async function validateAPSPlan(
  plan: unknown,
  options?: ValidationOptions
): Promise<ValidationResult> {
  return validator.validate(plan, options);
}
