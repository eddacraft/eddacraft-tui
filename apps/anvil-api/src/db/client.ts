import { neon } from '@neondatabase/serverless';
import { createDebugger } from '../lib/debug.js';

const debug = createDebugger('api');

export type NeonClient = ReturnType<typeof neon>;

let _client: NeonClient | null = null;

export function getClient(): NeonClient {
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
