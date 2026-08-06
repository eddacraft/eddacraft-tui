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
  let sanitized = value.replace(/\b(sk-|ghp_|ghu_|gho_|ghs_|ghr_)[A-Za-z0-9_-]+/g, '[REDACTED]');
  sanitized = sanitized.replace(/Bearer\s+[A-Za-z0-9_.+/=-]+/g, 'Bearer [REDACTED]');
  sanitized = sanitized.replace(/\b[0-9a-fA-F]{40,}\b/g, '[REDACTED]');
  // Device user codes as minted by `generateUserCode`, and email addresses.
  // Both are named by CIB-214 and neither is recognisable to the generic
  // high-entropy rules below, so they need shape rules of their own.
  sanitized = sanitized.replace(/\bANVIL-[0-9A-F]{8}\b/g, '[REDACTED]');
  sanitized = sanitized.replace(
    /\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b/g,
    '[REDACTED]'
  );
  sanitized = sanitized.replace(/\b[A-Za-z0-9+/]{20,}={0,3}\b/g, '[REDACTED]');
  return sanitized;
}

/**
 * Shapes that identify a person or credential precisely enough to be applied to
 * a *key* name. Deliberately excludes the generic high-entropy rules used on
 * values: an ordinary camelCase identifier such as `githubDeviceSessions` is 20
 * characters of alphanumerics and would trip the base64 heuristic, redacting a
 * field name that carries no secret at all.
 */
const IDENTIFYING_KEY_SHAPES = [
  /[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/,
  /\bANVIL-[0-9A-F]{8}\b/,
  /\b(sk-|ghp_|ghu_|gho_|ghs_|ghr_)[A-Za-z0-9_-]+/,
  /\b\d{1,3}(\.\d{1,3}){3}\b/, // IPv4 — rate-limit buckets are keyed by client IP
  /^[0-9a-fA-F:]{15,}$/, // IPv6-ish
  /\b[0-9a-fA-F]{40,}\b/,
];

/**
 * Field names that mark a credential when they carry *text*. Matched as
 * substrings of the normalised key, so `pollToken`, `access_token`, and
 * `refreshTokenHash` all resolve through the same rule.
 *
 * Everything under such a name is dropped except numbers, booleans, and absent
 * values: those are counters and flags, never the credential itself —
 * `refreshTokens: 87` is a purge row count, and `tokenOnly` / `hasToken` are
 * exactly the presence flags this module asks call sites to log instead of a
 * secret. Redacting those would strip real operational context and make the
 * safe pattern look unsafe. Strings, objects, arrays, `Map`, and `Set` are all
 * dropped — a credential nested one level down is still a credential.
 */
const CREDENTIAL_TEXT_KEY_SUBSTRINGS = [
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
 * Field names that identify a credential or a person as a whole word, whatever
 * type they carry — a numeric OTP is still an OTP. These deliberately do NOT
 * match as substrings: `deliveryCode` is an email delivery outcome,
 * `githubDeviceSessions` is a row count, and `authMethod` is operational
 * taxonomy — all three stay readable so ordinary debug context remains useful.
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

/**
 * Whether `value` must be dropped because of the name it is filed under.
 * Type-aware: see `CREDENTIAL_TEXT_KEY_SUBSTRINGS` for why a boolean or a count
 * under a token-ish name is kept.
 */
function isCredentialKey(key: string, value: unknown): boolean {
  const normalised = key.toLowerCase().replace(/[^a-z0-9]/g, '');
  if (CREDENTIAL_KEY_EXACT.has(normalised)) {
    return true;
  }
  // Allow-list the safe types rather than deny-list the unsafe ones. Written
  // the other way round, this also exempted objects, arrays, Map, and Set —
  // so `{ credentials: { pass: '…' } }` printed verbatim.
  if (typeof value === 'number' || typeof value === 'boolean') {
    return false;
  }
  // A null or absent field carries no secret, and "present but empty" is
  // itself useful signal.
  if (value === null || value === undefined) {
    return false;
  }
  return CREDENTIAL_TEXT_KEY_SUBSTRINGS.some((needle) => normalised.includes(needle));
}

/**
 * Render a key, dropping it when the key *text* is itself identifying. The
 * rate-limit and waitlist-throttle buckets are keyed by client IP and email, so
 * the PII sits in the key in exactly the structures most likely to be dumped.
 */
function redactKey(key: string): string {
  return IDENTIFYING_KEY_SHAPES.some((shape) => shape.test(key)) ? '[REDACTED]' : key;
}

/**
 * Deepest structure rendered before the walk stops. Comfortably above any
 * hand-written debug context in this service, and low enough that a nested
 * upstream JSON body cannot exhaust the stack.
 */
const MAX_REDACTION_DEPTH = 8;

/**
 * Recursively redact a structured debug payload.
 *
 * Scalar debug arguments already pass through `sanitizeForLog`; a structured
 * argument used to reach `console.debug` untouched, so a nested device code,
 * email, or token was printed verbatim. This walk applies the same value filter
 * to every nested string and key, and additionally drops values filed under a
 * credential-shaped name.
 *
 * The guarantee is a deny-list, not a proof: `sanitizeForLog` recognises
 * provider token prefixes, bearer headers, long hex and base64 runs, device
 * user codes, and email addresses. A novel secret shape under an unlisted key
 * name would still print, which is why the call sites are expected to pass
 * operational metadata rather than payloads.
 */
function redactStructured(value: unknown, seen: Set<object> = new Set(), depth = 0): unknown {
  // Contain failures per node, not per line. `instanceof` consults prototype
  // traps and `.message` / `.getTime()` / `.valueOf()` are all caller-supplied
  // code, so any single node can throw — but one hostile field must not cost
  // the operator every other field on the line.
  try {
    return redactNode(value, seen, depth);
  } catch {
    if (value !== null && typeof value === 'object') {
      seen.delete(value);
    }
    return '[UNREADABLE]';
  }
}

function redactNode(value: unknown, seen: Set<object>, depth: number): unknown {
  if (typeof value === 'string') {
    return sanitizeForLog(value);
  }

  if (typeof value === 'function') {
    return '[Function]';
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
    return { name: sanitizeForLog(value.name), message: sanitizeForLog(value.message) };
  }

  if (value instanceof Date) {
    seen.delete(value);
    return Number.isNaN(value.getTime()) ? 'Invalid Date' : value.toISOString();
  }

  // Binary carries no operator-readable context and an index walk would both
  // spill its bytes and bury the rest of the line.
  if (ArrayBuffer.isView(value) || value instanceof ArrayBuffer) {
    seen.delete(value);
    return `[Binary ${value.byteLength} bytes]`;
  }

  // Boxed primitives: unwrap so `new String(code)` is filtered like the literal.
  if (value instanceof String || value instanceof Number || value instanceof Boolean) {
    seen.delete(value);
    return redactStructured(value.valueOf(), seen, depth + 1);
  }

  if (value instanceof Map) {
    const entries: Record<string, unknown> = {};
    for (const [key, nested] of value) {
      const name = String(key);
      const rendered = isCredentialKey(name, nested)
        ? '[REDACTED]'
        : redactStructured(nested, seen, depth + 1);
      assignUnique(entries, redactKey(name), rendered);
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

  // Read through property *descriptors* rather than Object.entries: entries
  // invokes accessors, and a lazily-computed property on a driver or upstream
  // error object is free to throw. Logging must never be able to fail the
  // request that produced it.
  const result: Record<string, unknown> = {};
  let descriptors: Record<string, PropertyDescriptor>;
  try {
    descriptors = Object.getOwnPropertyDescriptors(value);
  } catch {
    seen.delete(value);
    return '[UNREADABLE]';
  }

  for (const [key, descriptor] of Object.entries(descriptors)) {
    if (!descriptor.enumerable) {
      continue;
    }
    if (!('value' in descriptor)) {
      assignUnique(result, redactKey(key), '[GETTER]');
      continue;
    }
    const nested = descriptor.value;
    const rendered = isCredentialKey(key, nested)
      ? '[REDACTED]'
      : redactStructured(nested, seen, depth + 1);
    assignUnique(result, redactKey(key), rendered);
  }
  seen.delete(value);
  return result;
}

/**
 * Assign into `target` without letting two redacted keys silently collide.
 * Three throttle buckets keyed by email must not render as one entry holding
 * whichever value happened to come last.
 */
function assignUnique(target: Record<string, unknown>, key: string, value: unknown): void {
  if (!(key in target)) {
    target[key] = value;
    return;
  }
  let suffix = 2;
  while (`${key}:${suffix}` in target) {
    suffix += 1;
  }
  target[`${key}:${suffix}`] = value;
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
      if (isCredentialKey(key, value)) {
        assignUnique(entry, redactKey(key), '[REDACTED]');
        continue;
      }
      assignUnique(
        entry,
        redactKey(key),
        typeof value === 'string' ? sanitizeForLog(value) : value
      );
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
