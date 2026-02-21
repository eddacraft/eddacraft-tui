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
 */
export function getApiUrl(): string {
  return getEnv('ANVIL_API_URL', DEFAULT_API_URL);
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

  const res = await fetch(url, {
    method: options.method,
    headers,
    body: options.body !== undefined ? JSON.stringify(options.body) : undefined,
  });

  if (!res.ok) {
    const body = await res.text();
    log(`apiRequest: FAILED ${options.operationName} status=${res.status}`);
    throw new Error(`${options.operationName} failed: ${res.status} ${body}`);
  }

  log(`apiRequest: OK ${options.operationName} status=${res.status}`);
  return (await res.json()) as T;
}
