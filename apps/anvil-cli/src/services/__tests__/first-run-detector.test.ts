import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import {
  isFirstRun,
  markFirstRunComplete,
  isWelcomeSkipped,
  getMarkerPath,
} from '../first-run-detector.js';
import { safeCleanup } from '../../../../../tools/test-utils/safe-cleanup.js';

describe('first-run-detector', () => {
  let testDir: string;

  beforeEach(() => {
    testDir = join(tmpdir(), `anvil-test-${Date.now()}-${Math.random().toString(36).slice(2)}`);
    mkdirSync(testDir, { recursive: true });
    delete process.env.ANVIL_SKIP_WELCOME;
  });

  afterEach(async () => {
    await safeCleanup(testDir);
    delete process.env.ANVIL_SKIP_WELCOME;
  });

  describe('isFirstRun', () => {
    it('returns true when marker file does not exist', () => {
      expect(isFirstRun({ projectRoot: testDir })).toBe(true);
    });

    it('returns false when marker file exists', () => {
      const anvilDir = join(testDir, '.anvil');
      mkdirSync(anvilDir, { recursive: true });
      const markerPath = join(anvilDir, 'first-run');
      writeFileSync(markerPath, '{}');

      expect(isFirstRun({ projectRoot: testDir })).toBe(false);
    });

    it('returns false when ANVIL_SKIP_WELCOME=1', () => {
      process.env.ANVIL_SKIP_WELCOME = '1';
      expect(isFirstRun({ projectRoot: testDir })).toBe(false);
    });

    it('returns false when ANVIL_SKIP_WELCOME=true', () => {
      process.env.ANVIL_SKIP_WELCOME = 'true';
      expect(isFirstRun({ projectRoot: testDir })).toBe(false);
    });

    it('returns true when ANVIL_SKIP_WELCOME has other value', () => {
      process.env.ANVIL_SKIP_WELCOME = '0';
      expect(isFirstRun({ projectRoot: testDir })).toBe(true);
    });
  });

  describe('markFirstRunComplete', () => {
    it('creates .anvil directory if it does not exist', () => {
      markFirstRunComplete({ projectRoot: testDir });
      expect(existsSync(join(testDir, '.anvil'))).toBe(true);
    });

    it('creates first-run marker file', () => {
      markFirstRunComplete({ projectRoot: testDir });
      expect(existsSync(join(testDir, '.anvil', 'first-run'))).toBe(true);
    });

    it('writes JSON content with timestamp', () => {
      markFirstRunComplete({ projectRoot: testDir });
      const content = JSON.parse(readFileSync(join(testDir, '.anvil', 'first-run'), 'utf-8'));
      expect(content).toHaveProperty('createdAt');
      expect(content).toHaveProperty('version', '1.0.0');
      expect(new Date(content.createdAt).getTime()).toBeLessThanOrEqual(Date.now());
    });

    it('makes isFirstRun return false after marking complete', () => {
      expect(isFirstRun({ projectRoot: testDir })).toBe(true);
      markFirstRunComplete({ projectRoot: testDir });
      expect(isFirstRun({ projectRoot: testDir })).toBe(false);
    });
  });

  describe('isWelcomeSkipped', () => {
    it('returns false when env var is not set', () => {
      expect(isWelcomeSkipped()).toBe(false);
    });

    it('returns true when ANVIL_SKIP_WELCOME=1', () => {
      process.env.ANVIL_SKIP_WELCOME = '1';
      expect(isWelcomeSkipped()).toBe(true);
    });

    it('returns true when ANVIL_SKIP_WELCOME=true', () => {
      process.env.ANVIL_SKIP_WELCOME = 'true';
      expect(isWelcomeSkipped()).toBe(true);
    });

    it('returns false when ANVIL_SKIP_WELCOME=0', () => {
      process.env.ANVIL_SKIP_WELCOME = '0';
      expect(isWelcomeSkipped()).toBe(false);
    });
  });

  describe('getMarkerPath', () => {
    const toFwd = (p: string): string => p.replace(/\\/g, '/');

    it('returns correct path for given project root', () => {
      expect(toFwd(getMarkerPath({ projectRoot: '/foo/bar' }))).toBe('/foo/bar/.anvil/first-run');
    });

    it('uses cwd when no project root provided', () => {
      const path = getMarkerPath();
      expect(toFwd(path)).toContain('.anvil/first-run');
    });
  });
});
