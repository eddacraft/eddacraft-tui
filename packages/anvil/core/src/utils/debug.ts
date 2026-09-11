/**
 * Debug logging utility for Anvil
 *
 * Enables debug output when ANVIL_DEBUG or DEBUG environment variable is set.
 * This provides visibility into error handling without cluttering production output.
 *
 * Usage:
 *   import { debug } from './utils/debug.js';
 *   debug('provenance', 'Failed to parse index', error);
 *
 * Enable with:
 *   ANVIL_DEBUG=1 anvil gate plan.md
 *   DEBUG=anvil:* anvil gate plan.md
 */

import { redactCredentialShapedText } from './credential-redaction.js';

type DebugNamespace =
  | 'provenance'
  | 'cache'
  | 'gate'
  | 'validation'
  | 'adapter'
  | 'architecture'
  | 'edge-detector'
  | 'entry-detector'
  | 'drift'
  | 'policy'
  | 'git-ai-notes'
  | 'agent'
  | 'atomic'
  | 'git-agent'
  | 'lock'
  | 'queue'
  | 'check'
  | 'watch'
  | 'cli'
  | 'kindling'
  | 'api'
  | 'service'
  | 'export'
  | 'explain'
  | 'suppression'
  | 'config'
  | 'secret'
  | 'compiler';

/**
 * Check if debug logging is enabled
 */
export function isDebugEnabled(namespace?: DebugNamespace): boolean {
  const anvilDebug = process.env.ANVIL_DEBUG;
  const debug = process.env.DEBUG;

  // ANVIL_DEBUG=1 enables all debug output
  if (anvilDebug === '1' || anvilDebug === 'true') {
    return true;
  }

  // DEBUG=anvil:* enables all, DEBUG=anvil:provenance enables specific
  if (debug) {
    if (debug.includes('anvil:*')) {
      return true;
    }
    if (namespace && debug.includes(`anvil:${namespace}`)) {
      return true;
    }
  }

  return false;
}

/**
 * Log a debug message if debug mode is enabled
 *
 * @param namespace - The component namespace (e.g., 'provenance', 'gate')
 * @param message - The debug message
 * @param data - Optional additional data to log
 */

/**
 * Sanitise a string for safe log output.
 *
 * Performs three layers of sanitisation:
 * 1. Strips CR/LF (log injection) — replaced with visual ⏎ (U+23CE)
 * 2. Strips ANSI escape sequences (CSI and OSC) to prevent log forging
 * 3. Redacts values that look like tokens, keys, or secrets
 *
 * Secret patterns redacted:
 * - Hex tokens (40+ hex characters, e.g. SHA tokens, API keys)
 * - Base64 tokens (20+ chars of base64 alphabet)
 * - Common secret prefixes: sk-, ghp_, ghu_, Bearer
 *
 * @param value - The string to sanitize
 * @returns The sanitized string with control chars stripped and secrets replaced by [REDACTED]
 */
export function sanitizeForLog(value: string): string {
  // Strip control characters that enable log injection (newlines, carriage returns)
  let sanitized = value.replace(/[\r\n]/g, '\u23CE');

  // Strip ANSI escape sequences (CSI and OSC) that could forge coloured output
  // eslint-disable-next-line no-control-regex -- intentional ESC match for ANSI stripping
  sanitized = sanitized.replace(/\x1B\[[0-?]*[ -/]*[@-~]|\x1B\][^\x07\x1B]*(?:\x07|\x1B\\)/g, '');

  sanitized = redactCredentialShapedText(sanitized);

  // Redact base64 tokens while preserving exact Git SHA-1/SHA-256 object ids.
  sanitized = sanitized.replace(/\b[A-Za-z0-9+/]{20,}={0,3}\b/g, (candidate) =>
    /^(?:[0-9a-f]{40}|[0-9a-f]{64})$/i.test(candidate) ? candidate : '[REDACTED]'
  );

  return sanitized;
}

const MAX_STRUCTURED_DEPTH = 8;
const CREDENTIAL_WORDS = new Set([
  'token',
  'tokens',
  'secret',
  'secrets',
  'password',
  'passwords',
  'passwd',
  'credential',
  'credentials',
  'authorization',
  'cookie',
  'bearer',
]);
function isCredentialField(key: string, value: unknown): boolean {
  if (
    value === null ||
    value === undefined ||
    typeof value === 'number' ||
    typeof value === 'boolean'
  ) {
    return false;
  }
  const words = key
    .replace(/([a-z0-9])([A-Z])/g, '$1_$2')
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter(Boolean);
  const last = words.at(-1);
  if (last !== undefined && CREDENTIAL_WORDS.has(last)) return true;
  for (let index = 0; index < words.length - 1; index += 1) {
    const isKeyPair =
      (words[index] === 'api' || words[index] === 'private') && words[index + 1] === 'key';
    if (isKeyPair) return true;
  }
  const credentialIndex = words.findIndex((word) => CREDENTIAL_WORDS.has(word));
  if (credentialIndex >= 0 && words[credentialIndex] !== 'cookie') return true;
  if (credentialIndex >= 0 && words[credentialIndex] === 'cookie') {
    return words.length !== credentialIndex + 2 || words[credentialIndex + 1] !== 'policy';
  }
  return false;
}

function sanitizeStructuredData(value: unknown, seen: Set<object> = new Set(), depth = 0): unknown {
  try {
    if (typeof value === 'string') return sanitizeForLog(value);
    if (typeof value === 'function') return '[Function]';
    if (value === null || typeof value !== 'object') return value;
    if (seen.has(value)) return '[CIRCULAR]';
    if (depth >= MAX_STRUCTURED_DEPTH) return '[TRUNCATED]';

    seen.add(value);
    if (Array.isArray(value)) {
      const result = value.map((item) => sanitizeStructuredData(item, seen, depth + 1));
      seen.delete(value);
      return result;
    }
    if (value instanceof Date) {
      seen.delete(value);
      return Number.isNaN(value.getTime()) ? 'Invalid Date' : value.toISOString();
    }
    if (value instanceof Map) {
      const result = [...value].map(([key, item]) => {
        const keyText = typeof key === 'string' ? key : null;
        return [
          sanitizeStructuredData(key, seen, depth + 1),
          keyText !== null && isCredentialField(keyText, item)
            ? '[redacted]'
            : sanitizeStructuredData(item, seen, depth + 1),
        ];
      });
      seen.delete(value);
      return result;
    }
    if (value instanceof Set) {
      const result = [...value].map((item) => sanitizeStructuredData(item, seen, depth + 1));
      seen.delete(value);
      return result;
    }

    const result: Record<string, unknown> = {};
    for (const [key, descriptor] of Object.entries(Object.getOwnPropertyDescriptors(value))) {
      if (!descriptor.enumerable) continue;
      Object.defineProperty(result, sanitizeForLog(key), {
        value:
          'value' in descriptor
            ? isCredentialField(key, descriptor.value)
              ? '[redacted]'
              : sanitizeStructuredData(descriptor.value, seen, depth + 1)
            : '[GETTER]',
        enumerable: true,
        writable: true,
        configurable: true,
      });
    }
    seen.delete(value);
    return result;
  } catch {
    if (value !== null && typeof value === 'object') seen.delete(value);
    return '[UNREADABLE]';
  }
}

export function debug(namespace: DebugNamespace, message: string, data?: unknown): void {
  if (!isDebugEnabled(namespace)) {
    return;
  }

  const timestamp = new Date().toISOString();
  const prefix = `[${timestamp}] [anvil:${namespace}]`;
  const sanitizedMessage = sanitizeForLog(message);

  /* eslint-disable no-console -- debug utility; independently verified by codex 20260205 */
  if (data !== undefined) {
    if (data instanceof Error) {
      console.debug('%s %s: %s', prefix, sanitizedMessage, sanitizeForLog(data.message));
      if (data.stack) {
        console.debug('%s Stack: %s', prefix, sanitizeForLog(data.stack));
      }
    } else if (typeof data === 'string') {
      console.debug('%s %s: %s', prefix, sanitizedMessage, sanitizeForLog(data));
    } else {
      console.debug('%s %s:', prefix, sanitizedMessage, sanitizeStructuredData(data));
    }
  } else {
    console.debug('%s %s', prefix, sanitizedMessage);
  }
  /* eslint-enable no-console */
}

/**
 * Create a namespaced debug logger
 *
 * @param namespace - The component namespace
 * @returns A debug function bound to the namespace
 */
export function createDebugger(
  namespace: DebugNamespace
): (message: string, data?: unknown) => void {
  return (message: string, data?: unknown) => debug(namespace, message, data);
}
