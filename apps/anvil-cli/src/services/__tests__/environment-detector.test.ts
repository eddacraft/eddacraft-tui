/**
 * Unit Tests for EnvironmentDetector
 *
 * Tests environment detection functionality including:
 * - Git detection
 * - Package manager detection
 * - Tool detection (ESLint, Prettier, Vitest, Jest, TypeScript)
 * - Recommended checks
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { EnvironmentDetector } from '../environment-detector.js';
import {
  createTestWorkspace,
  createPackageJson,
  createEslintConfig,
  createTsConfig,
  initGitRepo,
  createLockfile,
  type TestWorkspace,
} from '../../__tests__/helpers/test-workspace.js';
import { writeFileSync } from 'fs';
import { join } from 'path';

describe('EnvironmentDetector', () => {
  let workspace: TestWorkspace;

  beforeEach(() => {
    workspace = createTestWorkspace();
  });

  afterEach(() => {
    workspace.cleanup();
  });

  describe('git detection', () => {
    it('should detect git repository', () => {
      initGitRepo(workspace.root);

      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.hasGit).toBe(true);
    });

    it('should return false when no git repository', () => {
      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.hasGit).toBe(false);
    });
  });

  describe('package.json detection', () => {
    it('should detect package.json', () => {
      createPackageJson(workspace.root);

      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.hasPackageJson).toBe(true);
    });

    it('should return false when no package.json', () => {
      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.hasPackageJson).toBe(false);
    });

    it('should extract project name from package.json', () => {
      createPackageJson(workspace.root, { name: 'my-project' });

      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.projectName).toBe('my-project');
    });

    it('should handle missing project name gracefully', () => {
      writeFileSync(join(workspace.root, 'package.json'), '{}', 'utf-8');

      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.projectName).toBeUndefined();
    });

    it('should handle malformed package.json gracefully', () => {
      writeFileSync(join(workspace.root, 'package.json'), 'invalid json', 'utf-8');

      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.projectName).toBeUndefined();
    });
  });

  describe('package manager detection', () => {
    it('should detect pnpm from lockfile', () => {
      createLockfile(workspace.root, 'pnpm');

      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.packageManager).toBe('pnpm');
    });

    it('should detect npm from lockfile', () => {
      createLockfile(workspace.root, 'npm');

      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.packageManager).toBe('npm');
    });

    it('should detect yarn from lockfile', () => {
      createLockfile(workspace.root, 'yarn');

      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.packageManager).toBe('yarn');
    });

    it('should return unknown when no lockfile present', () => {
      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.packageManager).toBe('unknown');
    });

    it('should prefer pnpm when multiple lockfiles exist', () => {
      createLockfile(workspace.root, 'pnpm');
      createLockfile(workspace.root, 'npm');
      createLockfile(workspace.root, 'yarn');

      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.packageManager).toBe('pnpm');
    });
  });

  describe('ESLint detection', () => {
    it('should detect ESLint from .eslintrc.json', () => {
      createEslintConfig(workspace.root);

      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.hasEslint).toBe(true);
    });

    it('should detect ESLint from .eslintrc.js', () => {
      writeFileSync(join(workspace.root, '.eslintrc.js'), 'module.exports = {}', 'utf-8');

      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.hasEslint).toBe(true);
    });

    it('should detect ESLint from package.json dependency', () => {
      createPackageJson(workspace.root, {
        devDependencies: { eslint: '^8.0.0' },
      });

      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.hasEslint).toBe(true);
    });

    it('should return false when ESLint not configured', () => {
      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.hasEslint).toBe(false);
    });
  });

  describe('Prettier detection', () => {
    it('should detect Prettier from .prettierrc', () => {
      writeFileSync(join(workspace.root, '.prettierrc'), '{}', 'utf-8');

      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.hasPrettier).toBe(true);
    });

    it('should detect Prettier from package.json dependency', () => {
      createPackageJson(workspace.root, {
        devDependencies: { prettier: '^3.0.0' },
      });

      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.hasPrettier).toBe(true);
    });

    it('should return false when Prettier not configured', () => {
      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.hasPrettier).toBe(false);
    });
  });

  describe('Vitest detection', () => {
    it('should detect Vitest from vitest.config.ts', () => {
      writeFileSync(join(workspace.root, 'vitest.config.ts'), 'export default {}', 'utf-8');

      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.hasVitest).toBe(true);
    });

    it('should detect Vitest from package.json dependency', () => {
      createPackageJson(workspace.root, {
        devDependencies: { vitest: '^3.0.0' },
      });

      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.hasVitest).toBe(true);
    });

    it('should detect Vitest from vite.config.ts', () => {
      writeFileSync(join(workspace.root, 'vite.config.ts'), 'export default {}', 'utf-8');

      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.hasVitest).toBe(true);
    });

    it('should return false when Vitest not configured', () => {
      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.hasVitest).toBe(false);
    });
  });

  describe('Jest detection', () => {
    it('should detect Jest from jest.config.js', () => {
      writeFileSync(join(workspace.root, 'jest.config.js'), 'module.exports = {}', 'utf-8');

      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.hasJest).toBe(true);
    });

    it('should detect Jest from package.json dependency', () => {
      createPackageJson(workspace.root, {
        devDependencies: { jest: '^29.0.0' },
      });

      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.hasJest).toBe(true);
    });

    it('should return false when Jest not configured', () => {
      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.hasJest).toBe(false);
    });
  });

  describe('TypeScript detection', () => {
    it('should detect TypeScript from tsconfig.json', () => {
      createTsConfig(workspace.root);

      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.hasTypeScript).toBe(true);
    });

    it('should return false when TypeScript not configured', () => {
      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env.hasTypeScript).toBe(false);
    });
  });

  describe('recommended checks', () => {
    it('should recommend eslint when ESLint is detected', () => {
      createEslintConfig(workspace.root);

      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();
      const checks = detector.getRecommendedChecks(env);

      expect(checks).toContain('eslint');
    });

    it('should recommend test and coverage when Vitest is detected', () => {
      createPackageJson(workspace.root, {
        devDependencies: { vitest: '^3.0.0' },
      });

      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();
      const checks = detector.getRecommendedChecks(env);

      expect(checks).toContain('test');
      expect(checks).toContain('coverage');
    });

    it('should recommend test and coverage when Jest is detected', () => {
      createPackageJson(workspace.root, {
        devDependencies: { jest: '^29.0.0' },
      });

      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();
      const checks = detector.getRecommendedChecks(env);

      expect(checks).toContain('test');
      expect(checks).toContain('coverage');
    });

    it('should always recommend secret scanning', () => {
      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();
      const checks = detector.getRecommendedChecks(env);

      expect(checks).toContain('secret');
    });

    it('should recommend all checks for fully configured project', () => {
      createPackageJson(workspace.root, {
        devDependencies: {
          eslint: '^8.0.0',
          vitest: '^3.0.0',
        },
      });
      createEslintConfig(workspace.root);

      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();
      const checks = detector.getRecommendedChecks(env);

      expect(checks).toEqual(['eslint', 'test', 'coverage', 'secret']);
    });

    it('should recommend only secret for minimal project', () => {
      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();
      const checks = detector.getRecommendedChecks(env);

      expect(checks).toEqual(['secret']);
    });
  });

  describe('complete environment detection', () => {
    it('should detect all tools in a full-featured project', () => {
      createPackageJson(workspace.root, {
        name: 'full-project',
        devDependencies: {
          eslint: '^8.0.0',
          prettier: '^3.0.0',
          vitest: '^3.0.0',
          typescript: '^5.0.0',
        },
      });
      createEslintConfig(workspace.root);
      createTsConfig(workspace.root);
      initGitRepo(workspace.root);
      createLockfile(workspace.root, 'pnpm');

      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env).toEqual({
        hasGit: true,
        hasPackageJson: true,
        hasEslint: true,
        hasPrettier: true,
        hasVitest: true,
        hasJest: false,
        hasTypeScript: true,
        packageManager: 'pnpm',
        projectName: 'full-project',
        projectRoot: workspace.root,
      });
    });

    it('should handle completely empty project', () => {
      const detector = new EnvironmentDetector(workspace.root);
      const env = detector.detect();

      expect(env).toEqual({
        hasGit: false,
        hasPackageJson: false,
        hasEslint: false,
        hasPrettier: false,
        hasVitest: false,
        hasJest: false,
        hasTypeScript: false,
        packageManager: 'unknown',
        projectName: undefined,
        projectRoot: workspace.root,
      });
    });

    it('should use current working directory as default', () => {
      const detector = new EnvironmentDetector();

      expect(detector.detect().projectRoot).toBe(process.cwd());
    });
  });
});
