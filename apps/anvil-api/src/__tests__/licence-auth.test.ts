import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { authenticateLicence } from '../middleware/licence-auth.js';

const mocks = vi.hoisted(() => ({
  verify: vi.fn(),
  find: vi.fn(),
  getClient: vi.fn(),
  sql: vi.fn(),
}));
vi.mock('../lib/licence.js', () => ({ verifyLicence: mocks.verify }));
vi.mock('../db/queries.js', () => ({ findUserById: mocks.find }));
vi.mock('../db/client.js', () => ({ getClient: mocks.getClient }));

describe('shared licence authentication (SEC-013)', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  beforeEach(() => {
    vi.resetAllMocks();
    mocks.getClient.mockReturnValue(mocks.sql);
    mocks.verify.mockResolvedValue({ sub: 'signed-subject', email: 'alice@example.com' });
    mocks.find.mockResolvedValue({ id: 'signed-subject', status: 'active', plan: 'beta' });
  });

  it('returns identity only after verification and a fresh active-account lookup', async () => {
    const result = await authenticateLicence('signed-token');
    expect(result).toMatchObject({
      status: 'active',
      claims: { sub: 'signed-subject' },
      user: { plan: 'beta' },
    });
    expect(mocks.verify).toHaveBeenCalledWith('signed-token');
    expect(mocks.find).toHaveBeenCalledWith(mocks.sql, 'signed-subject');
    mocks.find.mockResolvedValue({ status: 'suspended' });
    expect(await authenticateLicence('signed-token')).toEqual({ status: 'inactive' });
    expect(mocks.find).toHaveBeenCalledTimes(2);
  });

  it.each([
    null,
    { status: 'pending' },
    { status: 'suspended' },
    { status: 'banned' },
    {},
    { status: 'unknown' },
  ])('fails closed for missing/non-active accounts %j', async (user) => {
    mocks.find.mockResolvedValue(user);
    expect(await authenticateLicence('signed-token')).toEqual({ status: 'inactive' });
  });

  it('rejects invalid credentials before validation or database access', async () => {
    mocks.verify.mockResolvedValue(null);
    const validate = vi.fn();
    expect(await authenticateLicence('bad-token', validate)).toEqual({ status: 'invalid' });
    expect(validate).not.toHaveBeenCalled();
    expect(mocks.getClient).not.toHaveBeenCalled();
  });

  it('preserves payload rejection before database access without exposing claims', async () => {
    const response = new Response('invalid payload', { status: 400 });
    const result = await authenticateLicence('signed-token', async () => response);
    expect(result).toEqual({ status: 'rejected', response });
    expect(mocks.getClient).not.toHaveBeenCalled();
  });

  it.each(['verify', 'getClient', 'find'] as const)(
    'fails closed when %s is unavailable',
    async (stage) => {
      const error = new Error('private configuration detail');
      mocks[stage].mockImplementation(() => {
        throw error;
      });
      expect(await authenticateLicence('signed-token')).toEqual({ status: 'unavailable' });
    }
  );
});
