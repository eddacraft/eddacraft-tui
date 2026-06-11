/**
 * Minimal debug logging utility for anvil-api
 *
 * Self-contained to avoid dependency on @eddacraft/anvil-core
 */

type DebugNamespace =
  | 'api'
  | 'auth-device'
  | 'auth-github-device'
  | 'auth-session'
  | 'require-auth';

function isDebugEnabled(): boolean {
  const anvilDebug = process.env.ANVIL_DEBUG;
  const debug = process.env.DEBUG;

  if (anvilDebug === '1' || anvilDebug === 'true') {
    return true;
  }

  if (debug) {
    if (debug.includes('anvil:*') || debug.includes('anvil:api')) {
      return true;
    }
  }

  return false;
}

function sanitizeForLog(value: string): string {
  let sanitized = value.replace(/\b(sk-|ghp_|ghu_)[A-Za-z0-9_-]+/g, '[REDACTED]');
  sanitized = sanitized.replace(/Bearer\s+[A-Za-z0-9_.+/=-]+/g, 'Bearer [REDACTED]');
  sanitized = sanitized.replace(/\b[0-9a-fA-F]{40,}\b/g, '[REDACTED]');
  sanitized = sanitized.replace(/\b[A-Za-z0-9+/]{20,}={0,3}\b/g, '[REDACTED]');
  return sanitized;
}

function debugLog(namespace: DebugNamespace, message: string, data?: unknown): void {
  if (!isDebugEnabled()) {
    return;
  }

  const timestamp = new Date().toISOString();
  const prefix = `[${timestamp}] [anvil:${namespace}]`;
  const sanitizedMessage = sanitizeForLog(message);

  /* eslint-disable no-console -- debug utility */
  if (data !== undefined) {
    if (data instanceof Error) {
      console.debug(`${prefix} ${sanitizedMessage}:`, sanitizeForLog(data.message));
      if (data.stack) {
        console.debug(`${prefix} Stack:`, sanitizeForLog(data.stack));
      }
    } else if (typeof data === 'string') {
      console.debug(`${prefix} ${sanitizedMessage}:`, sanitizeForLog(data));
    } else {
      console.debug(`${prefix} ${sanitizedMessage}:`, data);
    }
  } else {
    console.debug(`${prefix} ${sanitizedMessage}`);
  }
  /* eslint-enable no-console */
}

export function createDebugger(
  namespace: DebugNamespace
): (message: string, data?: unknown) => void {
  return (message: string, data?: unknown) => debugLog(namespace, message, data);
}
