import { setDefaultResultOrder } from 'node:dns';
import { neon } from '@neondatabase/serverless';
import { createDebugger } from '../lib/debug.js';

const debug = createDebugger('api');

export type NeonClient = ReturnType<typeof neon>;

let _client: NeonClient | null = null;

const CONNECT_CLASS =
  /ETIMEDOUT|ENOTFOUND|EAI_AGAIN|ECONNRESET|ECONNREFUSED|UND_ERR_CONNECT_TIMEOUT|fetch failed|Error connecting to database|internalConnectMultiple/i;

// Vercel + Node Happy Eyeballs can black-hole Neon IPv6 and fail the
// HTTP fetch at ~750ms (`ETIMEDOUT` / `internalConnectMultiple`). Prefer
// A records so the serverless driver reaches Neon over IPv4. Process-global;
// set once at module load, not on every getClient() call.
setDefaultResultOrder('ipv4first');

function collectErrorText(error: unknown): string {
  const parts: string[] = [];
  const seen = new Set<unknown>();
  let current: unknown = error;
  while (current && typeof current === 'object' && !seen.has(current)) {
    seen.add(current);
    if (current instanceof Error) {
      parts.push(current.name, current.message);
    } else if ('message' in current && typeof current.message === 'string') {
      parts.push(current.message);
    }
    if ('code' in current && current.code != null) {
      parts.push(String(current.code));
    }
    const next =
      ('cause' in current ? (current as { cause?: unknown }).cause : undefined) ??
      ('sourceError' in current ? (current as { sourceError?: unknown }).sourceError : undefined);
    current = next;
  }
  return parts.join('\n');
}

function isConnectClassFailure(error: unknown): boolean {
  if (error && typeof error === 'object' && 'code' in error) {
    const code = String((error as { code?: unknown }).code ?? '');
    // Postgres SQLSTATE is five alphanumeric chars (e.g. 23505). Never retry
    // query/constraint errors even if the message mentions a network word.
    if (/^[0-9A-Z]{5}$/.test(code) && !CONNECT_CLASS.test(code)) {
      return false;
    }
  }
  return CONNECT_CLASS.test(collectErrorText(error));
}

async function invokeWithOneConnectRetry<T>(op: () => T | PromiseLike<T>): Promise<T> {
  try {
    return await op();
  } catch (error) {
    if (!isConnectClassFailure(error)) {
      throw error;
    }
    debug('retrying Neon query after connect-class failure');
    return await op();
  }
}

type QueryDescriptor = PromiseLike<unknown> & {
  queryData: unknown;
  then: (...args: unknown[]) => unknown;
  catch?: (...args: unknown[]) => unknown;
};

function isQueryDescriptor(value: unknown): value is QueryDescriptor {
  return (
    !!value &&
    typeof value === 'object' &&
    'queryData' in value &&
    typeof (value as { then?: unknown }).then === 'function'
  );
}

function withConnectRetry<T>(make: () => T): T {
  const first = make();
  if (isQueryDescriptor(first)) {
    const originalThen = first.then.bind(first);
    first.then = ((onFulfilled?: unknown, onRejected?: unknown) =>
      originalThen(onFulfilled, (error: unknown) => {
        if (!isConnectClassFailure(error)) {
          if (typeof onRejected === 'function') {
            return (onRejected as (reason: unknown) => unknown)(error);
          }
          return Promise.reject(error);
        }
        debug('retrying Neon query after connect-class failure');
        return Promise.resolve(make()).then(
          onFulfilled as ((value: unknown) => unknown) | undefined,
          onRejected as ((reason: unknown) => unknown) | undefined
        );
      })) as typeof first.then;
    first.catch = ((onRejected?: unknown) => first.then(undefined, onRejected)) as NonNullable<
      QueryDescriptor['catch']
    >;
    return first as T;
  }
  return Promise.resolve(first).catch((error: unknown) => {
    if (!isConnectClassFailure(error)) {
      throw error;
    }
    debug('retrying Neon query after connect-class failure');
    return make();
  }) as T;
}

/** Wrap a Neon HTTP client so one connect-class failure is retried (APGOV-008). */
export function wrapNeonClient(client: NeonClient): NeonClient {
  const wrapped = ((strings: TemplateStringsArray, ...params: unknown[]) =>
    withConnectRetry(() => client(strings, ...params))) as NeonClient;

  if (typeof client.query === 'function') {
    wrapped.query = ((...args: Parameters<NeonClient['query']>) =>
      withConnectRetry(() => client.query(...args))) as NeonClient['query'];
  }

  if (typeof client.unsafe === 'function') {
    wrapped.unsafe = client.unsafe.bind(client);
  }

  if (typeof client.transaction === 'function') {
    wrapped.transaction = ((...args: Parameters<NeonClient['transaction']>) =>
      invokeWithOneConnectRetry(() => client.transaction(...args))) as NeonClient['transaction'];
  }

  return wrapped;
}

export function getClient(): NeonClient {
  if (!_client) {
    const url = process.env['DATABASE_URL'];
    if (!url) {
      throw new Error('DATABASE_URL environment variable is required');
    }
    debug('creating Neon database client');
    _client = wrapNeonClient(neon(url));
  }
  return _client;
}

/** Override the client (for testing). */
export function setClient(client: NeonClient): void {
  if (process.env.NODE_ENV !== 'test') {
    throw new Error('setClient() is only available in test environment');
  }
  _client = wrapNeonClient(client);
}
