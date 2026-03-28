import { describe, it, expect, vi, afterEach, beforeAll, afterAll } from 'vitest';
import { mkdtempSync, existsSync, writeFileSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { setAuthDir } from '../auth-store.js';
import { saveLicence, loadLicence } from '../licence-store.js';
import {
  scheduleRefresh,
  getLastRefreshAttempt,
  _setRefreshStatePath,
} from '../licence-refresh.js';
import { safeCleanup } from '../../../../../tools/test-utils/safe-cleanup.js';

let tempDir: string;

beforeAll(() => {
  tempDir = mkdtempSync(join(tmpdir(), 'anvil-refresh-test-'));
  setAuthDir(tempDir);
  _setRefreshStatePath(join(tempDir, 'refresh-state'));
});

afterAll(async () => {
  setAuthDir(null);
  _setRefreshStatePath(null);
  if (tempDir && existsSync(tempDir)) {
    await safeCleanup(tempDir);
  }
});

afterEach(() => {
  for (const f of ['license', 'refresh-state', 'auth.json']) {
    const p = join(tempDir, f);
    if (existsSync(p)) rmSync(p);
  }
  vi.restoreAllMocks();
});

describe('licence-refresh', () => {
  describe('getLastRefreshAttempt', () => {
    it('returns 0 when no refresh-state file exists', () => {
      expect(getLastRefreshAttempt()).toBe(0);
    });

    it('returns the stored timestamp', () => {
      writeFileSync(join(tempDir, 'refresh-state'), '1700000000000');
      expect(getLastRefreshAttempt()).toBe(1700000000000);
    });
  });

  describe('scheduleRefresh', () => {
    it('skips refresh when last attempt was within cooldown', async () => {
      writeFileSync(join(tempDir, 'refresh-state'), String(Date.now()));

      const mockFetch = vi.fn();
      const result = await scheduleRefresh('anvil_beta_test', mockFetch);
      expect(result).toBe('skipped_cooldown');
      expect(mockFetch).not.toHaveBeenCalled();
    });

    it('calls the API and updates licence on success', async () => {
      const mockFetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({ license: 'new.jwt.token' }),
      });

      saveLicence('old.jwt.token');
      const result = await scheduleRefresh('anvil_beta_test', mockFetch);
      expect(result).toBe('refreshed');
      expect(loadLicence()).toBe('new.jwt.token');
    });

    it('deletes licence when API returns valid:false', async () => {
      const mockFetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({ valid: false, reason: 'revoked' }),
      });

      saveLicence('old.jwt.token');
      const result = await scheduleRefresh('anvil_beta_test', mockFetch);
      expect(result).toBe('revoked');
      expect(loadLicence()).toBeNull();
    });

    it('returns error on network failure without touching licence', async () => {
      const mockFetch = vi.fn().mockRejectedValue(new Error('Network error'));

      saveLicence('old.jwt.token');
      const result = await scheduleRefresh('anvil_beta_test', mockFetch);
      expect(result).toBe('error');
      expect(loadLicence()).toBe('old.jwt.token');
    });

    it('persists the refresh attempt timestamp', async () => {
      const mockFetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({ license: 'new.jwt.token' }),
      });

      const before = Date.now();
      await scheduleRefresh('anvil_beta_test', mockFetch);
      const after = Date.now();

      const ts = getLastRefreshAttempt();
      expect(ts).toBeGreaterThanOrEqual(before);
      expect(ts).toBeLessThanOrEqual(after);
    });
  });
});
