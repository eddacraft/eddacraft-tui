import { describe, expect, it, vi } from 'vitest';
import {
  generateUserCode,
  isUniqueViolation,
  isUserCodeCollision,
  withUserCodeRetry,
} from '../device-code.js';

function pgError(fields: {
  code?: string;
  constraint?: string;
  detail?: string;
  message?: string;
}): Error {
  const err = new Error(fields.message ?? 'duplicate key value');
  return Object.assign(err, fields);
}

describe('generateUserCode', () => {
  it('returns codes with the ANVIL- prefix and 8 hex characters', () => {
    const code = generateUserCode();
    expect(code).toMatch(/^ANVIL-[0-9A-F]{8}$/);
  });
});

describe('isUniqueViolation', () => {
  it('matches Postgres SQLSTATE 23505', () => {
    expect(isUniqueViolation(pgError({ code: '23505' }))).toBe(true);
  });

  it('rejects other SQLSTATEs', () => {
    expect(isUniqueViolation(pgError({ code: '23503' }))).toBe(false);
  });

  it('rejects non-errors', () => {
    expect(isUniqueViolation(null)).toBe(false);
    expect(isUniqueViolation('23505')).toBe(false);
    expect(isUniqueViolation({ code: '23505' })).toBe(false);
  });
});

describe('isUserCodeCollision', () => {
  it('matches by constraint name', () => {
    expect(
      isUserCodeCollision(pgError({ code: '23505', constraint: 'device_codes_user_code_key' }))
    ).toBe(true);
  });

  it('matches by detail when constraint is missing', () => {
    expect(
      isUserCodeCollision(
        pgError({
          code: '23505',
          detail: 'Key (user_code)=(ANVIL-ABCD1234) already exists.',
        })
      )
    ).toBe(true);
  });

  it('rejects other unique constraint collisions', () => {
    expect(
      isUserCodeCollision(pgError({ code: '23505', constraint: 'access_tokens_token_hash_key' }))
    ).toBe(false);
    expect(
      isUserCodeCollision(
        pgError({
          code: '23505',
          detail: 'Key (token_hash)=(abc123) already exists.',
        })
      )
    ).toBe(false);
  });

  it('rejects non-unique-violation errors even if they mention user_code', () => {
    expect(
      isUserCodeCollision(
        pgError({
          code: '23503',
          detail: 'Key (user_code)=(ANVIL-ABCD1234) already exists.',
        })
      )
    ).toBe(false);
  });
});

describe('withUserCodeRetry', () => {
  it('returns the first successful attempt', async () => {
    const attempt = vi.fn(async (code: string) => ({ code }));
    const result = await withUserCodeRetry(attempt);
    expect(attempt).toHaveBeenCalledTimes(1);
    expect(result.code).toMatch(/^ANVIL-/);
  });

  it('retries on user_code collisions and eventually succeeds', async () => {
    const collision = pgError({ code: '23505', constraint: 'device_codes_user_code_key' });
    const attempt = vi
      .fn()
      .mockRejectedValueOnce(collision)
      .mockRejectedValueOnce(collision)
      .mockResolvedValueOnce('ok');
    const result = await withUserCodeRetry(attempt, 3);
    expect(attempt).toHaveBeenCalledTimes(3);
    expect(result).toBe('ok');
  });

  it('rethrows non-user_code unique violations without retrying', async () => {
    const other = pgError({ code: '23505', constraint: 'access_tokens_token_hash_key' });
    const attempt = vi.fn().mockRejectedValue(other);
    await expect(withUserCodeRetry(attempt, 3)).rejects.toBe(other);
    expect(attempt).toHaveBeenCalledTimes(1);
  });

  it('rethrows non-unique-violation errors without retrying', async () => {
    const boom = new Error('boom');
    const attempt = vi.fn().mockRejectedValue(boom);
    await expect(withUserCodeRetry(attempt, 3)).rejects.toBe(boom);
    expect(attempt).toHaveBeenCalledTimes(1);
  });

  it('throws the last collision error once retries are exhausted', async () => {
    const collision = pgError({ code: '23505', constraint: 'device_codes_user_code_key' });
    const attempt = vi.fn().mockRejectedValue(collision);
    await expect(withUserCodeRetry(attempt, 2)).rejects.toBe(collision);
    expect(attempt).toHaveBeenCalledTimes(2);
  });

  it('rejects maxRetries < 1 with a RangeError', async () => {
    const attempt = vi.fn();
    await expect(withUserCodeRetry(attempt, 0)).rejects.toBeInstanceOf(RangeError);
    expect(attempt).not.toHaveBeenCalled();
  });
});
