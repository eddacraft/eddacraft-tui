// @vitest-environment node
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  readPackageJson,
  hasPackageDependency,
  hasConfigFile,
  detectEslint,
  detectPrettier,
  detectPackageManager,
  ESLINT_CONFIG_FILES,
  PRETTIER_CONFIG_FILES,
  VITEST_CONFIG_FILES,
  JEST_CONFIG_FILES,
} from './tool-detection.js';

// Mock file-io (readJsonFileSync)
vi.mock('./file-io.js', () => ({
  readJsonFileSync: vi.fn(),
}));

// Mock node:fs
vi.mock('node:fs', async (importOriginal) => {
  const actual = await importOriginal<typeof import('node:fs')>();
  return {
    ...actual,
    default: actual,
    existsSync: vi.fn(),
  };
});

import { readJsonFileSync } from './file-io.js';
import { existsSync } from 'node:fs';

describe('tool-detection', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('config file constants', () => {
    it('should include standard eslint config file names', () => {
      expect(ESLINT_CONFIG_FILES).toContain('.eslintrc');
      expect(ESLINT_CONFIG_FILES).toContain('eslint.config.js');
      expect(ESLINT_CONFIG_FILES).toContain('eslint.config.mjs');
    });

    it('should include standard prettier config file names', () => {
      expect(PRETTIER_CONFIG_FILES).toContain('.prettierrc');
      expect(PRETTIER_CONFIG_FILES).toContain('prettier.config.js');
    });

    it('should include vitest and vite config file names', () => {
      expect(VITEST_CONFIG_FILES).toContain('vitest.config.ts');
      expect(VITEST_CONFIG_FILES).toContain('vite.config.ts');
    });

    it('should include jest config file names', () => {
      expect(JEST_CONFIG_FILES).toContain('jest.config.ts');
      expect(JEST_CONFIG_FILES).toContain('jest.config.json');
    });
  });

  describe('readPackageJson', () => {
    it('should return parsed package.json contents', () => {
      vi.mocked(readJsonFileSync).mockReturnValue({
        name: 'my-project',
        version: '1.0.0',
        dependencies: { express: '^4.0.0' },
      });

      const result = readPackageJson('/project');

      expect(result).toEqual({
        name: 'my-project',
        version: '1.0.0',
        dependencies: { express: '^4.0.0' },
      });
    });

    it('should return null when package.json does not exist', () => {
      vi.mocked(readJsonFileSync).mockReturnValue(null);

      expect(readPackageJson('/project')).toBeNull();
    });
  });

  describe('hasPackageDependency', () => {
    it('should return true when package is in dependencies', () => {
      vi.mocked(readJsonFileSync).mockReturnValue({
        dependencies: { express: '^4.0.0' },
      });

      expect(hasPackageDependency('/project', 'express')).toBe(true);
    });

    it('should return true when package is in devDependencies', () => {
      vi.mocked(readJsonFileSync).mockReturnValue({
        devDependencies: { vitest: '^1.0.0' },
      });

      expect(hasPackageDependency('/project', 'vitest')).toBe(true);
    });

    it('should return false when package is not in any dependencies', () => {
      vi.mocked(readJsonFileSync).mockReturnValue({
        dependencies: { express: '^4.0.0' },
      });

      expect(hasPackageDependency('/project', 'react')).toBe(false);
    });

    it('should return false when package.json does not exist', () => {
      vi.mocked(readJsonFileSync).mockReturnValue(null);

      expect(hasPackageDependency('/project', 'express')).toBe(false);
    });
  });

  describe('hasConfigFile', () => {
    it('should return true when a config file exists', () => {
      vi.mocked(existsSync).mockImplementation((path) => {
        return String(path).endsWith('.eslintrc.json');
      });

      expect(hasConfigFile('/project', ESLINT_CONFIG_FILES)).toBe(true);
    });

    it('should return false when no config files exist', () => {
      vi.mocked(existsSync).mockReturnValue(false);

      expect(hasConfigFile('/project', ESLINT_CONFIG_FILES)).toBe(false);
    });
  });

  describe('detectEslint', () => {
    it('should detect via config file', () => {
      vi.mocked(existsSync).mockImplementation((path) => {
        return String(path).endsWith('eslint.config.js');
      });

      expect(detectEslint('/project')).toBe(true);
    });

    it('should detect via package.json dependency', () => {
      vi.mocked(existsSync).mockReturnValue(false);
      vi.mocked(readJsonFileSync).mockReturnValue({
        devDependencies: { eslint: '^8.0.0' },
      });

      expect(detectEslint('/project')).toBe(true);
    });

    it('should return false when not configured', () => {
      vi.mocked(existsSync).mockReturnValue(false);
      vi.mocked(readJsonFileSync).mockReturnValue({
        dependencies: { express: '^4.0.0' },
      });

      expect(detectEslint('/project')).toBe(false);
    });
  });

  describe('detectPrettier', () => {
    it('should detect via config file', () => {
      vi.mocked(existsSync).mockImplementation((path) => {
        return String(path).endsWith('.prettierrc');
      });

      expect(detectPrettier('/project')).toBe(true);
    });

    it('should detect via package.json dependency', () => {
      vi.mocked(existsSync).mockReturnValue(false);
      vi.mocked(readJsonFileSync).mockReturnValue({
        devDependencies: { prettier: '^3.0.0' },
      });

      expect(detectPrettier('/project')).toBe(true);
    });
  });

  describe('detectPackageManager', () => {
    it('should detect pnpm from lock file', () => {
      vi.mocked(existsSync).mockImplementation((path) => {
        return String(path).endsWith('pnpm-lock.yaml');
      });

      expect(detectPackageManager('/project')).toBe('pnpm');
    });

    it('should detect yarn from lock file', () => {
      vi.mocked(existsSync).mockImplementation((path) => {
        return String(path).endsWith('yarn.lock');
      });

      expect(detectPackageManager('/project')).toBe('yarn');
    });

    it('should detect npm from lock file', () => {
      vi.mocked(existsSync).mockImplementation((path) => {
        return String(path).endsWith('package-lock.json');
      });

      expect(detectPackageManager('/project')).toBe('npm');
    });

    it('should return unknown when no lock file found', () => {
      vi.mocked(existsSync).mockReturnValue(false);

      expect(detectPackageManager('/project')).toBe('unknown');
    });

    it('should prefer pnpm over yarn over npm', () => {
      // All lock files exist
      vi.mocked(existsSync).mockReturnValue(true);

      expect(detectPackageManager('/project')).toBe('pnpm');
    });
  });
});
