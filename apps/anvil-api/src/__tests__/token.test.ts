import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { generateToken, hashToken, isValidTokenFormat } from '../lib/token.js';

describe('token utilities', () => {
  const originalPepper = process.env['TOKEN_PEPPER'];

  beforeEach(() => {
    delete process.env['TOKEN_PEPPER'];
  });

  afterEach(() => {
    if (originalPepper !== undefined) {
      process.env['TOKEN_PEPPER'] = originalPepper;
    } else {
      delete process.env['TOKEN_PEPPER'];
    }
  });

  describe('generateToken', () => {
    it('returns a string with the anvil_beta_ prefix', () => {
      const token = generateToken();
      expect(token).toMatch(/^anvil_beta_/);
    });

    it('generates unique tokens', () => {
      const a = generateToken();
      const b = generateToken();
      expect(a).not.toBe(b);
    });

    it('passes format validation', () => {
      const token = generateToken();
      expect(isValidTokenFormat(token)).toBe(true);
    });
  });

  describe('hashToken', () => {
    it('returns a 64-char hex string (SHA-256)', () => {
      const hash = hashToken('anvil_beta_test');
      expect(hash).toMatch(/^[0-9a-f]{64}$/);
    });

    it('is deterministic for the same input', () => {
      const a = hashToken('anvil_beta_test');
      const b = hashToken('anvil_beta_test');
      expect(a).toBe(b);
    });

    it('produces different hashes for different tokens', () => {
      const a = hashToken('anvil_beta_aaa');
      const b = hashToken('anvil_beta_bbb');
      expect(a).not.toBe(b);
    });

    it('incorporates TOKEN_PEPPER when set', () => {
      const withoutPepper = hashToken('anvil_beta_test');
      process.env['TOKEN_PEPPER'] = 'secret-pepper';
      const withPepper = hashToken('anvil_beta_test');
      expect(withoutPepper).not.toBe(withPepper);
    });
  });

  describe('isValidTokenFormat', () => {
    it('accepts valid tokens', () => {
      expect(isValidTokenFormat(generateToken())).toBe(true);
    });

    it('rejects tokens without prefix', () => {
      expect(isValidTokenFormat('no_prefix_here')).toBe(false);
    });

    it('rejects tokens with wrong prefix', () => {
      expect(isValidTokenFormat('anvil_alpha_' + 'a'.repeat(43))).toBe(false);
    });

    it('rejects tokens with wrong payload length', () => {
      expect(isValidTokenFormat('anvil_beta_tooshort')).toBe(false);
    });

    it('rejects empty string', () => {
      expect(isValidTokenFormat('')).toBe(false);
    });
  });
});
