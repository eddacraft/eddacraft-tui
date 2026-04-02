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

  // Redact strings starting with common secret prefixes
  sanitized = sanitized.replace(/\b(sk-|ghp_|ghu_)[A-Za-z0-9_-]+/g, '[REDACTED]');

  // Redact "Bearer <token>" patterns
  sanitized = sanitized.replace(/Bearer\s+[A-Za-z0-9_.+/=-]+/g, 'Bearer [REDACTED]');

  // Redact hex tokens (40+ hex chars, typical of SHA1/SHA256 tokens)
  sanitized = sanitized.replace(/\b[0-9a-fA-F]{40,}\b/g, '[REDACTED]');

  // Redact base64 tokens (20+ chars of base64 alphabet, ending with optional padding)
  sanitized = sanitized.replace(/\b[A-Za-z0-9+/]{20,}={0,3}\b/g, '[REDACTED]');

  return sanitized;
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
      console.debug('%s %s:', prefix, sanitizedMessage, data);
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
