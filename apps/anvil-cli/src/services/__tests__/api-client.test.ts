import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { apiRequest, getApiUrl, getAdminKey } from '../api-client.js';

describe('api-client', () => {
  const originalEnv = { ...process.env };

  afterEach(() => {
    process.env = { ...originalEnv };
    vi.restoreAllMocks();
  });

  describe('getApiUrl', () => {
    it('should return the default URL when env var is not set', () => {
      delete process.env.ANVIL_API_URL;

      expect(getApiUrl()).toBe('https://api.eddacraft.com');
    });

    it('should return the custom URL from env var', () => {
      process.env.ANVIL_API_URL = 'https://custom-api.example.com';

      expect(getApiUrl()).toBe('https://custom-api.example.com');
    });
  });

  describe('getAdminKey', () => {
    it('should return the key when env var is set', () => {
      process.env.ANVIL_ADMIN_KEY = 'test-key-123';

      expect(getAdminKey()).toBe('test-key-123');
    });

    it('should throw when env var is not set', () => {
      delete process.env.ANVIL_ADMIN_KEY;

      expect(() => getAdminKey()).toThrow('ANVIL_ADMIN_KEY environment variable is required');
    });
  });

  describe('apiRequest', () => {
    beforeEach(() => {
      delete process.env.ANVIL_API_URL;
    });

    it('should make a GET request and return parsed JSON', async () => {
      const mockResponse = { data: 'test' };
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockResponse),
      });

      const result = await apiRequest<{ data: string }>({
        method: 'GET',
        path: '/api/v1/test',
        operationName: 'Test get',
      });

      expect(result).toEqual({ data: 'test' });
      expect(fetch).toHaveBeenCalledWith('https://api.eddacraft.com/api/v1/test', {
        method: 'GET',
        headers: {},
        body: undefined,
      });
    });

    it('should make a POST request with JSON body', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({ success: true }),
      });

      await apiRequest({
        method: 'POST',
        path: '/api/v1/action',
        body: { email: 'test@example.com' },
        operationName: 'Test post',
      });

      expect(fetch).toHaveBeenCalledWith('https://api.eddacraft.com/api/v1/action', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: '{"email":"test@example.com"}',
      });
    });

    it('should include authorization header when token is provided', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({}),
      });

      await apiRequest({
        method: 'GET',
        path: '/api/v1/secure',
        token: 'bearer-token-123',
        operationName: 'Test auth',
      });

      expect(fetch).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({
          headers: { Authorization: 'Bearer bearer-token-123' },
        })
      );
    });

    it('should throw with status and body on non-ok response', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 403,
        text: () => Promise.resolve('Forbidden: invalid key'),
      });

      await expect(
        apiRequest({
          method: 'POST',
          path: '/api/v1/admin/invite',
          body: { email: 'test@example.com' },
          token: 'bad-key',
          operationName: 'Admin invite',
        })
      ).rejects.toThrow('Admin invite failed: 403 Forbidden: invalid key');
    });

    it('should use custom API URL from env var', async () => {
      process.env.ANVIL_API_URL = 'https://staging.example.com';
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({}),
      });

      await apiRequest({
        method: 'GET',
        path: '/api/v1/test',
        operationName: 'Test',
      });

      expect(fetch).toHaveBeenCalledWith(
        'https://staging.example.com/api/v1/test',
        expect.any(Object)
      );
    });
  });
});
