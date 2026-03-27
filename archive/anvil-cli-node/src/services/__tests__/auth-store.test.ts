import { describe, it, expect, afterEach, beforeAll, afterAll } from 'vitest';
import { mkdtempSync, readFileSync, statSync, existsSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import {
  loadAuth,
  saveAuth,
  clearAuth,
  isAuthenticated,
  setAuthDir,
  type StoredAuth,
} from '../auth-store.js';
import { safeCleanup } from '../../../../../tools/test-utils/safe-cleanup.js';

let tempDir: string;

beforeAll(() => {
  tempDir = mkdtempSync(join(tmpdir(), 'anvil-auth-test-'));
  setAuthDir(tempDir);
});

afterAll(async () => {
  setAuthDir(null);
  if (tempDir && existsSync(tempDir)) {
    await safeCleanup(tempDir);
  }
});

function makeAuth(overrides: Partial<StoredAuth> = {}): StoredAuth {
  return {
    token: 'anvil_beta_testtoken',
    user: { email: 'test@example.com' },
    scopes: ['beta'],
    expiresAt: new Date(Date.now() + 86400000).toISOString(),
    verifiedAt: new Date().toISOString(),
    ...overrides,
  };
}

describe('auth-store', () => {
  afterEach(() => {
    const authPath = join(tempDir, 'auth.json');
    if (existsSync(authPath)) {
      rmSync(authPath);
    }
  });

  describe('loadAuth', () => {
    it('returns null when no auth file exists', () => {
      expect(loadAuth()).toBeNull();
    });

    it('returns null for expired auth', () => {
      saveAuth(makeAuth({ expiresAt: new Date(Date.now() - 1000).toISOString() }));
      expect(loadAuth()).toBeNull();
    });

    it('returns stored auth for valid unexpired token', () => {
      const auth = makeAuth();
      saveAuth(auth);
      const loaded = loadAuth();
      expect(loaded).not.toBeNull();
      expect(loaded!.user.email).toBe('test@example.com');
      expect(loaded!.scopes).toEqual(['beta']);
    });
  });

  describe('saveAuth', () => {
    it('creates the auth file', () => {
      saveAuth(makeAuth());
      const authPath = join(tempDir, 'auth.json');
      expect(existsSync(authPath)).toBe(true);
    });

    it('writes valid JSON', () => {
      const auth = makeAuth();
      saveAuth(auth);
      const authPath = join(tempDir, 'auth.json');
      const raw = readFileSync(authPath, 'utf-8');
      const parsed = JSON.parse(raw);
      expect(parsed.user.email).toBe('test@example.com');
    });

    it.skipIf(process.platform === 'win32')('sets restrictive file permissions (0o600)', () => {
      saveAuth(makeAuth());
      const authPath = join(tempDir, 'auth.json');
      const stats = statSync(authPath);
      const mode = stats.mode & 0o777;
      expect(mode).toBe(0o600);
    });
  });

  describe('clearAuth', () => {
    it('removes the auth file', () => {
      saveAuth(makeAuth());
      clearAuth();
      expect(loadAuth()).toBeNull();
    });

    it('does not throw if no auth file exists', () => {
      expect(() => clearAuth()).not.toThrow();
    });
  });

  describe('isAuthenticated', () => {
    it('returns false when no auth stored', () => {
      expect(isAuthenticated()).toBe(false);
    });

    it('returns true when valid auth is stored', () => {
      saveAuth(makeAuth());
      expect(isAuthenticated()).toBe(true);
    });

    it('returns false when auth is expired', () => {
      saveAuth(makeAuth({ expiresAt: new Date(Date.now() - 1000).toISOString() }));
      expect(isAuthenticated()).toBe(false);
    });
  });
});
