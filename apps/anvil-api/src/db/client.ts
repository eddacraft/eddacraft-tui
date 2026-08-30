import { setDefaultResultOrder } from 'node:dns';
import { neon } from '@neondatabase/serverless';
import { createDebugger } from '../lib/debug.js';

const debug = createDebugger('api');

export type NeonClient = ReturnType<typeof neon>;

let _client: NeonClient | null = null;

export function getClient(): NeonClient {
  // Vercel + Node Happy Eyeballs can black-hole Neon IPv6 and fail the
  // HTTP fetch at ~750ms (`ETIMEDOUT` / `internalConnectMultiple`). Prefer
  // A records so the serverless driver reaches Neon over IPv4.
  setDefaultResultOrder('ipv4first');
  if (!_client) {
    const url = process.env['DATABASE_URL'];
    if (!url) {
      throw new Error('DATABASE_URL environment variable is required');
    }
    debug('creating Neon database client');
    _client = neon(url);
  }
  return _client;
}

/** Override the client (for testing). */
export function setClient(client: NeonClient): void {
  if (process.env.NODE_ENV !== 'test') {
    throw new Error('setClient() is only available in test environment');
  }
  _client = client;
}
