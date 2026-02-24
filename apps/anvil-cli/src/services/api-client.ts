/**
 * Shared HTTP client utilities for Anvil API requests.
 *
 * Consolidates the common fetch + error handling pattern used across
 * admin-client.ts and auth-client.ts.
 */

import { createDebugger } from '@eddacraft/anvil-core';
import { requireEnv, getEnv } from '../utils/env.js';

const log = createDebugger('service');

const DEFAULT_API_URL = 'https://eddacraft-api.vercel.app';

/**
 * Get the configured API base URL.
 * Rejects non-HTTPS URLs to prevent credential leakage.
 */
export function getApiUrl(): string {
  const url = getEnv('ANVIL_API_URL', DEFAULT_API_URL);
  if (!url.startsWith('https://')) {
    throw new Error(`ANVIL_API_URL must use HTTPS (got ${url})`);
  }
  return url;
}

/**
 * Get the admin API key from environment, or throw if missing.
 */
export function getAdminKey(): string {
  return requireEnv('ANVIL_ADMIN_KEY', 'admin commands');
}

interface ApiRequestOptions {
  /** HTTP method */
  method: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH';
  /** URL path (appended to API base URL) */
  path: string;
  /** Request body (will be JSON-serialized) */
  body?: unknown;
  /** Authorization bearer token */
  token?: string;
  /** Operation name for error messages */
  operationName: string;
}

/**
 * Make an authenticated API request with standard error handling.
 *
 * @throws {Error} If the response is not ok, including status and response body.
 */
export async function apiRequest<T>(options: ApiRequestOptions): Promise<T> {
  const url = `${getApiUrl()}${options.path}`;
  log(`apiRequest: ${options.method} ${url} operation=${options.operationName}`);
  const headers: Record<string, string> = {};

  if (options.body !== undefined) {
    headers['Content-Type'] = 'application/json';
  }

  if (options.token) {
    headers['Authorization'] = `Bearer ${options.token}`;
  }

  const body = options.body !== undefined ? JSON.stringify(options.body) : undefined;

  let res: Response;
  try {
    res = await fetch(url, {
      method: options.method,
      headers,
      body,
      signal: AbortSignal.timeout(30_000),
    });
  } catch (err) {
    // AbortSignal.timeout() rejects with a DOMException (TimeoutError)
    if (err instanceof DOMException && err.name === 'TimeoutError') {
      log(`apiRequest: TIMEOUT ${options.operationName}`);
      throw new Error(
        `Request timed out for ${options.operationName}. Check your internet connection and try again.`
      );
    }
    if (!(err instanceof TypeError)) throw err;
    let cause = '';
    if (err.cause != null) {
      const causeValue = err.cause as unknown;
      const causeMessage = causeValue instanceof Error ? causeValue.message : String(causeValue);
      cause = ` (${causeMessage})`;
    }
    log(`apiRequest: NETWORK ERROR ${options.operationName}${cause}`);
    throw new Error(
      `Could not connect to ${getApiUrl()}${cause}. Check your internet connection and try again.`
    );
  }

  if (!res.ok) {
    const body = await res.text();
    log(`apiRequest: FAILED ${options.operationName} status=${res.status}`);

    if (res.status === 401 || res.status === 403) {
      const isAdminRoute = options.path.includes('/admin/');
      if (isAdminRoute) {
        const truncated = body.length > 200 ? body.slice(0, 200) + '...' : body;
        throw new Error(`${options.operationName} failed: ${res.status} ${truncated}`);
      }
      // User-token flow: clear stale credentials and prompt re-login
      const { clearAuth } = await import('./auth-store.js');
      clearAuth();
      throw new Error('Authentication expired — run `anvil login` to re-authenticate.');
    }

    const truncated = body.length > 200 ? body.slice(0, 200) + '...' : body;
    throw new Error(`${options.operationName} failed: ${res.status} ${truncated}`);
  }

  log(`apiRequest: OK ${options.operationName} status=${res.status}`);
  return (await res.json()) as T;
}
