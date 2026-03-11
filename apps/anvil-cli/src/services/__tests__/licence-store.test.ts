import { describe, it, expect, afterEach, beforeAll, afterAll } from 'vitest';
import { mkdtempSync, writeFileSync, existsSync, readFileSync, rmSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { setAuthDir } from '../auth-store.js';
import { loadLicence, saveLicence, clearLicence, resolveLicencePath } from '../licence-store.js';
import { safeCleanup } from '../../../../../tools/test-utils/safe-cleanup.js';

let tempDir: string;

beforeAll(() => {
  tempDir = mkdtempSync(join(tmpdir(), 'anvil-licence-test-'));
  setAuthDir(tempDir);
});

afterAll(async () => {
  setAuthDir(null);
  if (tempDir && existsSync(tempDir)) {
    await safeCleanup(tempDir);
  }
});

afterEach(() => {
  const licPath = join(tempDir, 'license');
  if (existsSync(licPath)) rmSync(licPath);
  delete process.env['ANVIL_LICENSE'];
});

describe('licence-store', () => {
  describe('saveLicence', () => {
    it('writes the JWT string to the license file', () => {
      saveLicence('eyJhbGciOiJFUzI1NiJ9.test.sig');
      const content = readFileSync(join(tempDir, 'license'), 'utf-8');
      expect(content).toBe('eyJhbGciOiJFUzI1NiJ9.test.sig');
    });

    it.skipIf(process.platform === 'win32')('sets restrictive permissions (0o600)', () => {
      saveLicence('test-jwt');
      const stats = statSync(join(tempDir, 'license'));
      expect(stats.mode & 0o777).toBe(0o600);
    });
  });

  describe('loadLicence', () => {
    it('returns null when no license file exists', () => {
      expect(loadLicence()).toBeNull();
    });

    it('returns the JWT string when file exists', () => {
      saveLicence('my.jwt.token');
      expect(loadLicence()).toBe('my.jwt.token');
    });

    it('trims whitespace from the file content', () => {
      writeFileSync(join(tempDir, 'license'), '  my.jwt.token  \n');
      expect(loadLicence()).toBe('my.jwt.token');
    });
  });

  describe('clearLicence', () => {
    it('deletes the license file', () => {
      saveLicence('test');
      clearLicence();
      expect(existsSync(join(tempDir, 'license'))).toBe(false);
    });

    it('does not throw if no file exists', () => {
      expect(() => clearLicence()).not.toThrow();
    });
  });

  describe('resolveLicencePath', () => {
    it('returns ANVIL_LICENSE env var path when set and file exists', () => {
      const envPath = join(tempDir, 'env-license');
      writeFileSync(envPath, 'env-jwt');
      process.env['ANVIL_LICENSE'] = envPath;
      expect(resolveLicencePath()).toBe(envPath);
    });

    it('falls back to user-level when env var file does not exist', () => {
      process.env['ANVIL_LICENSE'] = '/nonexistent/path';
      saveLicence('user-jwt');
      expect(resolveLicencePath()).toBe(join(tempDir, 'license'));
    });

    it('returns user-level path when no env var set', () => {
      saveLicence('user-jwt');
      expect(resolveLicencePath()).toBe(join(tempDir, 'license'));
    });

    it('returns null when no license file exists anywhere', () => {
      expect(resolveLicencePath()).toBeNull();
    });
  });
});
