/**
 * Minimal debug logging utility for anvil-api
 *
 * Self-contained to avoid dependency on @eddacraft/anvil-core
 */

type DebugNamespace = 'api' | 'auth-device' | 'auth-github-device' | 'auth-session';

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

/**
 * A flat field bag for a structured info log. Values are scalars only —
 * operational metadata (latency, outcome, error class, HTTP status), never
 * payloads. The caller is responsible for never passing a secret here; see
 * `infoLog` for the contract.
 */
export type InfoFields = Record<string, string | number | boolean | null | undefined>;

/**
 * Emit a single ungated structured operational log line via `console.info`.
 *
 * Unlike `debugLog` (gated behind ANVIL_DEBUG/DEBUG and meant for local
 * diagnosis), `infoLog` always runs so production can observe upstream-call
 * outcomes — latency, outcome, error class, HTTP status — without enabling
 * debug. It is the operator-facing signal for the device-flow login.
 *
 * SECURITY CONTRACT: `fields` carries operational metadata ONLY. Never pass a
 * secret value — no `access_token`, `device_code`, `poll_token`, licence
 * payload, user email, or `Authorization` header. Log presence, latency, and
 * class, never the value. Field values are scalars; a string value is still
 * passed through the same redaction filter as the debug path as a defence in
 * depth, but the primary guarantee is the caller's discipline.
 */
function infoLog(namespace: DebugNamespace, event: string, fields?: InfoFields): void {
  const entry: Record<string, unknown> = {};
  if (fields) {
    for (const [key, value] of Object.entries(fields)) {
      if (value === undefined) {
        continue;
      }
      entry[key] = typeof value === 'string' ? sanitizeForLog(value) : value;
    }
  }
  // Reserved keys are written last so a caller-supplied field can never
  // override them; the event name is sanitised and clamped in case a caller
  // ever passes a dynamic value despite the static-literal convention.
  entry.ts = new Date().toISOString();
  entry.ns = `anvil:${namespace}`;
  entry.event = sanitizeForLog(event).slice(0, 64);

  /* eslint-disable-next-line no-console -- ungated operational log */
  console.info(JSON.stringify(entry));
}

/**
 * Build an ungated structured info logger bound to a namespace. Use for
 * operational upstream-call outcomes that must be visible in production
 * without ANVIL_DEBUG. See `infoLog` for the no-secrets contract.
 */
export function createInfoLogger(
  namespace: DebugNamespace
): (event: string, fields?: InfoFields) => void {
  return (event: string, fields?: InfoFields) => infoLog(namespace, event, fields);
}
