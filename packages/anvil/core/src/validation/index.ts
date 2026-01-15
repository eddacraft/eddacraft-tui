/**
 * Validation Module Public API
 *
 * Exports validation utilities for APS plans.
 */

import { generateHash } from '../crypto/index.js';
import { validator as defaultValidator } from './aps-validator.js';

// Main validator
export {
  APSValidator,
  validator,
  validateAPSPlan,
  type ValidationResult,
  type ValidationOptions,
} from './aps-validator.js';

// Error types and formatters
export {
  ValidationError,
  SchemaValidationError,
  HashValidationError,
  type ValidationIssue,
  formatValidationErrors,
  formatZodErrors,
  createValidationSummary,
} from './errors.js';

// Initialize the validator with the hash function
defaultValidator.setHashValidator(generateHash);
