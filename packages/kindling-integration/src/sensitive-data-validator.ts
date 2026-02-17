/**
 * Sensitive Data Validation (KINDLING-015)
 *
 * Validates and redacts sensitive data from observations before they
 * are persisted to Kindling. This is a defense-in-depth layer on top of
 * the `containsSensitiveData` check in observation-contract.ts.
 *
 * Patterns detected:
 * - API keys (sk-*, ghp_*, AKIA*)
 * - Long hex tokens (40+ characters)
 * - Email addresses
 * - Password-like values
 *
 * @see observation-contract.ts for the base containsSensitiveData utility
 */

import { containsSensitiveData, type Observation } from './observation-contract.js';
import { createDebugger } from './utils/debug.js';

const debug = createDebugger('kindling');

// =============================================================================
// Sensitive Patterns
// =============================================================================

/**
 * Known sensitive value patterns and their replacement strings
 */
const SENSITIVE_PATTERNS: ReadonlyArray<{
  name: string;
  pattern: RegExp;
  replacement: string;
}> = [
  {
    name: 'openai-api-key',
    pattern: /sk-[a-zA-Z0-9]{20,}/g,
    replacement: '[REDACTED:api-key]',
  },
  {
    name: 'github-pat',
    pattern: /ghp_[a-zA-Z0-9]{36,}/g,
    replacement: '[REDACTED:github-token]',
  },
  {
    name: 'github-fine-grained-pat',
    pattern: /github_pat_[a-zA-Z0-9_]{36,}/g,
    replacement: '[REDACTED:github-token]',
  },
  {
    name: 'aws-access-key',
    pattern: /AKIA[0-9A-Z]{16}/g,
    replacement: '[REDACTED:aws-key]',
  },
  {
    name: 'aws-secret-key',
    pattern: /(?<=aws_secret_access_key\s*[=:]\s*)[^\s"',]+/gi,
    replacement: '[REDACTED:aws-secret]',
  },
  {
    name: 'hex-token',
    pattern: /\b[0-9a-fA-F]{40,}\b/g,
    replacement: '[REDACTED:token]',
  },
  {
    name: 'email',
    pattern: /[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}/g,
    replacement: '[REDACTED:email]',
  },
  {
    name: 'password-value',
    pattern: /(?<=(?:password|passwd|pwd)\s*[=:]\s*["']?)[^\s"',]+/gi,
    replacement: '[REDACTED:password]',
  },
  {
    name: 'bearer-token',
    pattern: /(?<=Bearer\s+)[a-zA-Z0-9._~+/=-]{20,}/gi,
    replacement: '[REDACTED:bearer-token]',
  },
  {
    name: 'npm-token',
    pattern: /npm_[a-zA-Z0-9]{36,}/g,
    replacement: '[REDACTED:npm-token]',
  },
];

// =============================================================================
// Validation
// =============================================================================

/**
 * Result of sensitive data validation
 */
export interface SensitiveDataValidationResult {
  /** Whether any sensitive data was detected */
  hasSensitiveData: boolean;
  /** Detailed issues found */
  issues: string[];
}

/**
 * Validate that an observation does not contain sensitive data.
 *
 * Uses the contract-level `containsSensitiveData` check plus additional
 * pattern matching for known credential formats.
 *
 * @param observation - The observation to validate
 * @returns Validation result with issues if sensitive data found
 */
export function validateNoSensitiveData(observation: Observation): SensitiveDataValidationResult {
  const issues: string[] = [];

  // Run the contract-level check first
  const contractCheck = containsSensitiveData(observation);
  if (contractCheck.hasSensitiveData) {
    issues.push(...contractCheck.issues);
  }

  // Run additional pattern checks against the serialized payload
  const serialized = JSON.stringify(observation);

  for (const { name, pattern } of SENSITIVE_PATTERNS) {
    // Reset lastIndex for global regex patterns
    pattern.lastIndex = 0;
    if (pattern.test(serialized)) {
      issues.push(`Sensitive pattern detected: ${name}`);
    }
  }

  if (issues.length > 0) {
    debug('sensitive data detected in observation', {
      issueCount: issues.length,
      patterns: issues,
    });
  }

  return {
    hasSensitiveData: issues.length > 0,
    issues,
  };
}

// =============================================================================
// Redaction
// =============================================================================

/**
 * Deep-clone an observation and redact all known sensitive patterns from
 * string values. This operates on the JSON serialization to catch values
 * regardless of nesting depth.
 *
 * The returned observation is safe to persist.
 *
 * @param observation - The observation to redact
 * @returns A new observation with sensitive values replaced
 */
export function redactSensitiveFields(observation: Observation): Observation {
  debug('redacting sensitive fields from observation', { kind: observation.kind });
  let serialized = JSON.stringify(observation);

  for (const { pattern, replacement } of SENSITIVE_PATTERNS) {
    // Reset lastIndex for global regex patterns
    pattern.lastIndex = 0;
    serialized = serialized.replace(pattern, replacement);
  }

  return JSON.parse(serialized) as Observation;
}
