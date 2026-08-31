import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { getClient, setClient, wrapNeonClient, type NeonClient } from '../db/client.js';

function connectFailure(message = 'Error connecting to database: TypeError: fetch failed'): Error {
  const cause = Object.assign(new Error('fetch failed'), { code: 'ETIMEDOUT' });
  return Object.assign(new Error(message), { cause });
}

function constraintFailure(): Error {
  return Object.assign(
    new Error('duplicate key value violates unique constraint "beta_users_email_key"'),
    {
      code: '23505',
    }
  );
}

function taggedClient(): NeonClient {
  const inner = vi.fn() as unknown as NeonClient;
  return inner;
}

describe('wrapNeonClient (APGOV-008)', () => {
  it('retries once on a connect-class tagged-template failure and returns the retry result', async () => {
    const inner = taggedClient();
    vi.mocked(inner)
      .mockRejectedValueOnce(connectFailure())
      .mockResolvedValueOnce([{ ok: 1 }]);

    const sql = wrapNeonClient(inner);
    const result = await sql`SELECT 1`;

    expect(result).toEqual([{ ok: 1 }]);
    expect(inner).toHaveBeenCalledTimes(2);
  });

  it('retries once on ETIMEDOUT without a wrapping Neon message', async () => {
    const inner = taggedClient();
    const timeout = Object.assign(new Error('connect timed out'), { code: 'ETIMEDOUT' });
    vi.mocked(inner)
      .mockRejectedValueOnce(timeout)
      .mockResolvedValueOnce([{ ok: 1 }]);

    const sql = wrapNeonClient(inner);
    await expect(sql`SELECT 1`).resolves.toEqual([{ ok: 1 }]);
    expect(inner).toHaveBeenCalledTimes(2);
  });

  it('does not retry SQL constraint errors', async () => {
    const inner = taggedClient();
    const err = constraintFailure();
    vi.mocked(inner).mockRejectedValueOnce(err);

    const sql = wrapNeonClient(inner);
    await expect(sql`INSERT INTO beta_users`).rejects.toBe(err);
    expect(inner).toHaveBeenCalledTimes(1);
  });

  it('surfaces the second connect-class failure without a third attempt', async () => {
    const inner = taggedClient();
    const first = connectFailure();
    const second = connectFailure('Error connecting to database: TypeError: fetch failed');
    vi.mocked(inner).mockRejectedValueOnce(first).mockRejectedValueOnce(second);

    const sql = wrapNeonClient(inner);
    await expect(sql`SELECT 1`).rejects.toBe(second);
    expect(inner).toHaveBeenCalledTimes(2);
  });

  it('retries once on a connect-class transaction failure', async () => {
    const inner = taggedClient();
    inner.transaction = vi
      .fn()
      .mockRejectedValueOnce(connectFailure())
      .mockResolvedValueOnce([[{ ok: 1 }]]);

    const sql = wrapNeonClient(inner);
    const result = await sql.transaction([sql`SELECT 1`]);

    expect(result).toEqual([[{ ok: 1 }]]);
    expect(inner.transaction).toHaveBeenCalledTimes(2);
  });

  it('retries once on a connect-class query() failure', async () => {
    const inner = taggedClient();
    inner.query = vi
      .fn()
      .mockRejectedValueOnce(connectFailure())
      .mockResolvedValueOnce([{ ok: 1 }]);

    const sql = wrapNeonClient(inner);
    const result = await sql.query('SELECT 1');

    expect(result).toEqual([{ ok: 1 }]);
    expect(inner.query).toHaveBeenCalledTimes(2);
  });
});

describe('getClient / setClient wrap the shared Neon client', () => {
  const previousNodeEnv = process.env.NODE_ENV;

  beforeEach(() => {
    process.env.NODE_ENV = 'test';
  });

  afterEach(() => {
    process.env.NODE_ENV = previousNodeEnv;
  });

  it('retries through setClient so callers of getClient inherit one connect retry', async () => {
    const inner = taggedClient();
    vi.mocked(inner)
      .mockRejectedValueOnce(connectFailure())
      .mockResolvedValueOnce([{ ok: 1 }]);
    setClient(inner);

    const sql = getClient();
    await expect(sql`SELECT 1`).resolves.toEqual([{ ok: 1 }]);
    expect(inner).toHaveBeenCalledTimes(2);
  });
});
