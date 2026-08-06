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

/**
 * Field names whose value is credential-shaped wherever it appears. Matched as
 * substrings of the normalised key, so `pollToken`, `access_token`, and
 * `refreshTokenHash` all resolve through the same rule.
 */
const CREDENTIAL_KEY_SUBSTRINGS = [
  'token',
  'secret',
  'password',
  'passwd',
  'credential',
  'apikey',
  'privatekey',
  'authorization',
  'cookie',
  'signature',
  'bearer',
  'licence',
  'license',
];

/**
 * Field names that identify a credential or a person only as a whole word.
 * These deliberately do NOT match as substrings: `deliveryCode` is an email
 * delivery outcome, `githubDeviceSessions` is a row count, and `authMethod` is
 * operational taxonomy — all three stay readable so ordinary debug context
 * remains useful.
 */
const CREDENTIAL_KEY_EXACT = new Set([
  'code',
  'usercode',
  'devicecode',
  'otpcode',
  'authcode',
  'verificationcode',
  'email',
  'emailaddress',
  'ip',
  'ipaddress',
  'clientip',
  'remoteip',
  'actor',
  'session',
  'sessionid',
]);

function isCredentialKey(key: string): boolean {
  const normalised = key.toLowerCase().replace(/[^a-z0-9]/g, '');
  if (CREDENTIAL_KEY_EXACT.has(normalised)) {
    return true;
  }
  return CREDENTIAL_KEY_SUBSTRINGS.some((needle) => normalised.includes(needle));
}

/**
 * Recursively redact a structured debug payload.
 *
 * Scalar debug arguments already pass through `sanitizeForLog`; a structured
 * argument used to reach `console.debug` untouched, so a nested device code,
 * email, or token was printed verbatim. This walk applies the same value filter
 * to every nested string, and additionally drops values whose *key* is
 * credential-shaped — a device code or an email address is not recognisable
 * from its value alone, so key awareness is what makes the guarantee hold.
 */
/**
 * Deepest structure rendered before the walk stops. Comfortably above any
 * hand-written debug context in this service, and low enough that a nested
 * upstream JSON body cannot exhaust the stack.
 */
const MAX_REDACTION_DEPTH = 8;

function redactStructured(value: unknown, seen: Set<object> = new Set(), depth = 0): unknown {
  if (typeof value === 'string') {
    return sanitizeForLog(value);
  }

  if (value === null || typeof value !== 'object') {
    return value;
  }

  // A debug call must never be able to crash the request that made it, so both
  // a self-referential context and a pathologically deep one degrade to a
  // marker rather than recursing without bound.
  if (seen.has(value)) {
    return '[CIRCULAR]';
  }
  if (depth >= MAX_REDACTION_DEPTH) {
    return '[TRUNCATED]';
  }
  seen.add(value);

  // Error, Date, Map, and Set carry their content on the prototype or in
  // internal slots, so an own-property walk would render them as `{}` and throw
  // away the very detail the operator is reading the log for.
  if (value instanceof Error) {
    seen.delete(value);
    return { name: value.name, message: sanitizeForLog(value.message) };
  }

  if (value instanceof Date) {
    seen.delete(value);
    return Number.isNaN(value.getTime()) ? 'Invalid Date' : value.toISOString();
  }

  if (value instanceof Map) {
    const entries: Record<string, unknown> = {};
    for (const [key, nested] of value) {
      const name = String(key);
      entries[name] = isCredentialKey(name)
        ? '[REDACTED]'
        : redactStructured(nested, seen, depth + 1);
    }
    seen.delete(value);
    return entries;
  }

  if (value instanceof Set) {
    const items = [...value].map((item) => redactStructured(item, seen, depth + 1));
    seen.delete(value);
    return items;
  }

  if (Array.isArray(value)) {
    const items = value.map((item) => redactStructured(item, seen, depth + 1));
    seen.delete(value);
    return items;
  }

  const result: Record<string, unknown> = {};
  for (const [key, nested] of Object.entries(value as Record<string, unknown>)) {
    result[key] = isCredentialKey(key) ? '[REDACTED]' : redactStructured(nested, seen, depth + 1);
  }
  seen.delete(value);
  return result;
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
      console.debug(`${prefix} ${sanitizedMessage}:`, redactStructured(data));
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
 * passed through the same redaction filter as the debug path, and any
 * credential-shaped field name is dropped outright, as a defence in depth — but
 * the primary guarantee is the caller's discipline.
 */
function infoLog(namespace: DebugNamespace, event: string, fields?: InfoFields): void {
  const entry: Record<string, unknown> = {};
  if (fields) {
    for (const [key, value] of Object.entries(fields)) {
      if (value === undefined) {
        continue;
      }
      if (isCredentialKey(key)) {
        entry[key] = '[REDACTED]';
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
