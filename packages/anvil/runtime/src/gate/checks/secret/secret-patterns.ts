/**
 * Secret Patterns - Pattern definitions and matching for secret detection
 *
 * Contains regex patterns for detecting various types of secrets and sensitive data.
 */

import { createDebugger } from '@eddacraft/anvil-core';

const log = createDebugger('check');

/**
 * Secret pattern definition
 */
export interface SecretPattern {
  name: string;
  pattern: RegExp;
}

/**
 * Secret patterns for common API keys, tokens, and sensitive data
 */
export const SECRET_PATTERNS: SecretPattern[] = [
  // API Keys
  { name: 'API Key', pattern: /(?:api[_-]?key|apikey)\s*[:=]\s*['"]?[a-zA-Z0-9_-]{16,}['"]?/i },
  // JWT Tokens
  { name: 'JWT Token', pattern: /eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*/ },
  // AWS Keys
  { name: 'AWS Key', pattern: /AKIA[0-9A-Z]{16}/ },
  {
    name: 'AWS Secret Key',
    pattern: /(?:aws_secret|aws_secret_access_key)\s*[:=]\s*['"]?[A-Za-z0-9/+=]{40}['"]?/i,
  },
  // Private Keys
  { name: 'Private Key', pattern: /-----BEGIN\s+(?:RSA\s+)?PRIVATE\s+KEY-----/ },
  { name: 'PGP Private Key', pattern: /-----BEGIN PGP PRIVATE KEY BLOCK-----/ },
  // Database URLs
  { name: 'Database URL', pattern: /(?:postgres|mysql|mongodb|redis):\/\/[^:\s]+:[^@\s]+@/ },
  // Generic secrets
  {
    name: 'Generic Secret',
    pattern: /(?:secret|password|passwd|pwd)\s*[:=]\s*['"]?[^\s'"]{8,}['"]?/i,
  },
  // Credit Cards (basic pattern)
  { name: 'Credit Card', pattern: /\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b/ },
  // GitHub tokens
  { name: 'GitHub Token', pattern: /gh[pousr]_[A-Za-z0-9_]{36,}/ },
  // Slack tokens
  { name: 'Slack Token', pattern: /xox[baprs]-[0-9]{10,13}-[0-9]{10,13}-[a-zA-Z0-9]{24}/ },
  // Stripe keys
  { name: 'Stripe Key', pattern: /sk_live_[0-9a-zA-Z]{24}/ },
  { name: 'Stripe Test Key', pattern: /sk_test_[0-9a-zA-Z]{24}/ },
  // Google API keys
  { name: 'Google API Key', pattern: /AIza[0-9A-Za-z_-]{35}/ },
  // Heroku API key
  {
    name: 'Heroku API Key',
    pattern:
      /[h|H]eroku[a-zA-Z0-9_-]*[:=]\s*['"]?[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}['"]?/,
  },
  // SendGrid API key
  { name: 'SendGrid API Key', pattern: /SG\.[a-zA-Z0-9_-]{22}\.[a-zA-Z0-9_-]{43}/ },
  // Twilio API key
  { name: 'Twilio API Key', pattern: /SK[a-f0-9]{32}/ },
  // npm token
  { name: 'NPM Token', pattern: /npm_[A-Za-z0-9]{36}/ },
];

/**
 * Default patterns to allowlist (common false positives)
 */
export const DEFAULT_ALLOWLIST: RegExp[] = [
  /^[a-f0-9]{32}$/, // MD5 hashes
  /^[a-f0-9]{40}$/, // SHA1 hashes
  /^[a-f0-9]{64}$/, // SHA256 hashes
  /^0x[a-f0-9]+$/i, // Hex literals
  /^data:image\/[a-z]+;base64,/, // Base64 images
  /placeholder/i,
  /example/i,
  /test/i,
  /dummy/i,
  /sample/i,
  /lorem ipsum/i,
];

/**
 * Pattern matcher for secret detection
 */
export class PatternMatcher {
  /**
   * Check if a string is allowlisted
   */
  isAllowlisted(str: string, customAllowlist: string[]): boolean {
    // Check default allowlist
    for (const pattern of DEFAULT_ALLOWLIST) {
      if (pattern.test(str)) {
        log('secret-patterns: string allowlisted by default pattern');
        return true;
      }
    }

    // Check custom allowlist
    for (const pattern of customAllowlist) {
      try {
        if (new RegExp(pattern, 'i').test(str)) {
          log(`secret-patterns: string allowlisted by custom pattern: ${pattern}`);
          return true;
        }
      } catch {
        continue;
      }
    }

    return false;
  }

  /**
   * Check if a string looks like code (not a secret)
   */
  looksLikeCode(str: string): boolean {
    // Common code patterns that aren't secrets
    const codePatterns = [
      /^[a-z][a-zA-Z0-9]*\(/, // Function call
      /^[a-z][a-zA-Z0-9]*\.[a-z]/, // Method chain
      /^https?:\/\//, // URLs
      /^[a-z]+:\/\//, // Protocol URLs
      /\.(js|ts|css|html|json|md|txt)$/, // File extensions
      /^[A-Z][A-Z0-9_]+$/, // Constants (all caps)
      /\s+/, // Contains whitespace
      /^[a-z][a-z0-9]*[A-Z]/, // camelCase
      /^[A-Z][a-z]+[A-Z]/, // PascalCase
    ];

    return codePatterns.some((pattern) => pattern.test(str));
  }

  /**
   * Redact secret value for safe display
   */
  redactSecret(secret: string): string {
    if (secret.length <= 8) return '***';
    const prefix = secret.substring(0, 4);
    const suffix = secret.substring(secret.length - 4);
    return `${prefix}...${suffix}`;
  }

  /**
   * Redact potential secrets in a line for safe display
   */
  redactLine(line: string): string {
    let redacted = line;

    // Redact quoted strings that look like secrets
    redacted = redacted.replace(/(['"])[a-zA-Z0-9_/+=-]{16,}\1/g, '$1[REDACTED]$1');

    return redacted;
  }
}
