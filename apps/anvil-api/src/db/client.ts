import { neon } from '@neondatabase/serverless';

export type NeonClient = ReturnType<typeof neon>;

let _client: NeonClient | null = null;

export function getClient(): NeonClient {
  if (!_client) {
    const url = process.env['DATABASE_URL'];
    if (!url) {
      throw new Error('DATABASE_URL environment variable is required');
    }
    _client = neon(url);
  }
  return _client;
}

/** Override the client (for testing). */
export function setClient(client: NeonClient): void {
  _client = client;
}
