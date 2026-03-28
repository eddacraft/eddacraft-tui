/**
 * Unit Tests for SmartDefaultsGenerator
 *
 * Tests smart defaults generation based on project context including:
 * - Framework-specific configurations
 * - Project size adjustments
 * - TypeScript strictness adjustments
 * - Monorepo-aware patterns
 * - Intelligent allowlists
 */

import { describe, it, expect } from 'vitest';
import { SmartDefaultsGenerator } from '../smart-defaults.js';
import type { ProjectContext } from '../project-detector.js';

describe('SmartDefaultsGenerator', () => {
  const generator = new SmartDefaultsGenerator();

  describe('basic generation', () => {
    it('should generate valid configuration for minimal project', () => {
      const context: ProjectContext = {
        framework: 'unknown',
        monorepo: 'none',
        tsStrictness: 'none',
        size: 'small',
        fileCount: 10,
        hasEslint: false,
        hasPrettier: false,
        hasTests: false,
        packageManager: 'npm',
        projectRoot: '/test',
        workspacePackages: [],
      };

      const config = generator.generate(context);

      expect(config.version).toBe(1);
      expect(config.checks).toBeDefined();
      expect(config.thresholds).toBeDefined();
      expect(config.thresholds.overall_score).toBeGreaterThan(0);
    });

    it('should always include secret scanning', () => {
      const context: ProjectContext = {
        framework: 'unknown',
        monorepo: 'none',
        tsStrictness: 'none',
        size: 'small',
        fileCount: 10,
        hasEslint: false,
        hasPrettier: false,
        hasTests: false,
        packageManager: 'npm',
        projectRoot: '/test',
        workspacePackages: [],
      };

      const config = generator.generate(context);
      const secretCheck = config.checks.find((c) => c.name === 'secret');

      expect(secretCheck).toBeDefined();
      expect(secretCheck?.enabled).toBe(true);
    });

    it('should always include dependency scanning', () => {
      const context: ProjectContext = {
        framework: 'unknown',
        monorepo: 'none',
        tsStrictness: 'none',
        size: 'small',
        fileCount: 10,
        hasEslint: false,
        hasPrettier: false,
        hasTests: false,
        packageManager: 'npm',
        projectRoot: '/test',
        workspacePackages: [],
      };

      const config = generator.generate(context);
      const depCheck = config.checks.find((c) => c.name === 'dependency');

      expect(depCheck).toBeDefined();
      expect(depCheck?.enabled).toBe(true);
    });

    it('should always include anti-pattern check', () => {
      const context: ProjectContext = {
        framework: 'unknown',
        monorepo: 'none',
        tsStrictness: 'none',
        size: 'small',
        fileCount: 10,
        hasEslint: false,
        hasPrettier: false,
        hasTests: false,
        packageManager: 'npm',
        projectRoot: '/test',
        workspacePackages: [],
      };

      const config = generator.generate(context);
      const antipatternCheck = config.checks.find((c) => c.name === 'antipattern');

      expect(antipatternCheck).toBeDefined();
      expect(antipatternCheck?.enabled).toBe(true);
      expect(antipatternCheck?.config?.patterns).toContain('AP-001');
    });
  });

  describe('tooling detection', () => {
    it('should enable ESLint check when ESLint is detected', () => {
      const context: ProjectContext = {
        framework: 'react',
        monorepo: 'none',
        tsStrictness: 'moderate',
        size: 'medium',
        fileCount: 100,
        hasEslint: true,
        hasPrettier: false,
        hasTests: false,
        packageManager: 'npm',
        projectRoot: '/test',
        workspacePackages: [],
      };

      const config = generator.generate(context);
      const eslintCheck = config.checks.find((c) => c.name === 'eslint');

      expect(eslintCheck).toBeDefined();
      expect(eslintCheck?.enabled).toBe(true);
      expect(eslintCheck?.config?.min_score).toBeGreaterThan(0);
    });

    it('should not enable ESLint check when ESLint is not detected', () => {
      const context: ProjectContext = {
        framework: 'react',
        monorepo: 'none',
        tsStrictness: 'moderate',
        size: 'medium',
        fileCount: 100,
        hasEslint: false,
        hasPrettier: false,
        hasTests: false,
        packageManager: 'npm',
        projectRoot: '/test',
        workspacePackages: [],
      };

      const config = generator.generate(context);
      const eslintCheck = config.checks.find((c) => c.name === 'eslint');

      expect(eslintCheck).toBeUndefined();
    });

    it('should enable coverage check when tests are detected', () => {
      const context: ProjectContext = {
        framework: 'react',
        monorepo: 'none',
        tsStrictness: 'moderate',
        size: 'medium',
        fileCount: 100,
        hasEslint: false,
        hasPrettier: false,
        hasTests: true,
        packageManager: 'npm',
        projectRoot: '/test',
        workspacePackages: [],
      };

      const config = generator.generate(context);
      const coverageCheck = config.checks.find((c) => c.name === 'coverage');

      expect(coverageCheck).toBeDefined();
      expect(coverageCheck?.enabled).toBe(true);
      expect(coverageCheck?.config?.thresholds).toBeDefined();
    });

    it('should not enable coverage check when no tests are detected', () => {
      const context: ProjectContext = {
        framework: 'react',
        monorepo: 'none',
        tsStrictness: 'moderate',
        size: 'medium',
        fileCount: 100,
        hasEslint: false,
        hasPrettier: false,
        hasTests: false,
        packageManager: 'npm',
        projectRoot: '/test',
        workspacePackages: [],
      };

      const config = generator.generate(context);
      const coverageCheck = config.checks.find((c) => c.name === 'coverage');

      expect(coverageCheck).toBeUndefined();
    });
  });

  describe('project size adjustments', () => {
    it('should use higher thresholds for small projects', () => {
      const smallContext: ProjectContext = {
        framework: 'react',
        monorepo: 'none',
        tsStrictness: 'moderate',
        size: 'small',
        fileCount: 30,
        hasEslint: true,
        hasPrettier: false,
        hasTests: true,
        packageManager: 'npm',
        projectRoot: '/test',
        workspacePackages: [],
      };

      const config = generator.generate(smallContext);
      const coverageCheck = config.checks.find((c) => c.name === 'coverage');
      const eslintCheck = config.checks.find((c) => c.name === 'eslint');

      expect(coverageCheck?.config?.thresholds?.lines).toBeGreaterThanOrEqual(80);
      expect(eslintCheck?.config?.min_score).toBeGreaterThanOrEqual(80);
    });

    it('should use lower thresholds for large projects', () => {
      const largeContext: ProjectContext = {
        framework: 'react',
        monorepo: 'none',
        tsStrictness: 'moderate',
        size: 'large',
        fileCount: 800,
        hasEslint: true,
        hasPrettier: false,
        hasTests: true,
        packageManager: 'npm',
        projectRoot: '/test',
        workspacePackages: [],
      };

      const config = generator.generate(largeContext);
      const coverageCheck = config.checks.find((c) => c.name === 'coverage');
      const eslintCheck = config.checks.find((c) => c.name === 'eslint');

      expect(coverageCheck?.config?.thresholds?.lines).toBeLessThan(85);
      expect(eslintCheck?.config?.min_score).toBeLessThan(85);
    });

    it('should use lowest thresholds for xlarge projects', () => {
      const xlargeContext: ProjectContext = {
        framework: 'react',
        monorepo: 'none',
        tsStrictness: 'moderate',
        size: 'xlarge',
        fileCount: 2000,
        hasEslint: true,
        hasPrettier: false,
        hasTests: true,
        packageManager: 'npm',
        projectRoot: '/test',
        workspacePackages: [],
      };

      const config = generator.generate(xlargeContext);
      const coverageCheck = config.checks.find((c) => c.name === 'coverage');

      expect(coverageCheck?.config?.thresholds?.lines).toBeLessThan(80);
    });
  });

  describe('TypeScript strictness adjustments', () => {
    it('should use higher thresholds for strict TypeScript projects', () => {
      const strictContext: ProjectContext = {
        framework: 'react',
        monorepo: 'none',
        tsStrictness: 'strict',
        size: 'medium',
        fileCount: 100,
        hasEslint: true,
        hasPrettier: false,
        hasTests: true,
        packageManager: 'npm',
        projectRoot: '/test',
        workspacePackages: [],
      };

      const config = generator.generate(strictContext);
      const coverageCheck = config.checks.find((c) => c.name === 'coverage');
      const eslintCheck = config.checks.find((c) => c.name === 'eslint');

      expect(coverageCheck?.config?.thresholds?.lines).toBeGreaterThanOrEqual(80);
      expect(eslintCheck?.config?.min_score).toBeGreaterThanOrEqual(80);
    });

    it('should use lower thresholds for loose TypeScript projects', () => {
      const looseContext: ProjectContext = {
        framework: 'react',
        monorepo: 'none',
        tsStrictness: 'loose',
        size: 'medium',
        fileCount: 100,
        hasEslint: true,
        hasPrettier: false,
        hasTests: true,
        packageManager: 'npm',
        projectRoot: '/test',
        workspacePackages: [],
      };

      const config = generator.generate(looseContext);
      const coverageCheck = config.checks.find((c) => c.name === 'coverage');
      const eslintCheck = config.checks.find((c) => c.name === 'eslint');

      expect(coverageCheck?.config?.thresholds?.lines).toBeLessThanOrEqual(80);
      expect(eslintCheck?.config?.min_score).toBeLessThanOrEqual(80);
    });

    it('should use most lenient thresholds for non-TypeScript projects', () => {
      const noTsContext: ProjectContext = {
        framework: 'react',
        monorepo: 'none',
        tsStrictness: 'none',
        size: 'medium',
        fileCount: 100,
        hasEslint: true,
        hasPrettier: false,
        hasTests: true,
        packageManager: 'npm',
        projectRoot: '/test',
        workspacePackages: [],
      };

      const config = generator.generate(noTsContext);
      const coverageCheck = config.checks.find((c) => c.name === 'coverage');

      expect(coverageCheck?.config?.thresholds?.lines).toBeLessThan(75);
    });
  });

  describe('framework-specific allowlists', () => {
    it('should include Next.js-specific allowlist patterns', () => {
      const context: ProjectContext = {
        framework: 'nextjs',
        monorepo: 'none',
        tsStrictness: 'moderate',
        size: 'medium',
        fileCount: 100,
        hasEslint: false,
        hasPrettier: false,
        hasTests: false,
        packageManager: 'npm',
        projectRoot: '/test',
        workspacePackages: [],
      };

      const config = generator.generate(context);
      const antipatternCheck = config.checks.find((c) => c.name === 'antipattern');
      const allowlist = antipatternCheck?.config?.allowlist as string[];

      expect(allowlist).toContain('**/*.d.ts');
      expect(allowlist).toContain('next.config.js');
      expect(allowlist).toContain('**/app/layout.tsx');
    });

    it('should include React-specific allowlist patterns', () => {
      const context: ProjectContext = {
        framework: 'react',
        monorepo: 'none',
        tsStrictness: 'moderate',
        size: 'medium',
        fileCount: 100,
        hasEslint: false,
        hasPrettier: false,
        hasTests: false,
        packageManager: 'npm',
        projectRoot: '/test',
        workspacePackages: [],
      };

      const config = generator.generate(context);
      const antipatternCheck = config.checks.find((c) => c.name === 'antipattern');
      const allowlist = antipatternCheck?.config?.allowlist as string[];

      expect(allowlist).toContain('**/*.d.ts');
      expect(allowlist).toContain('vite.config.ts');
      expect(allowlist).toContain('**/*.test.ts');
    });

    it('should include NestJS-specific allowlist patterns', () => {
      const context: ProjectContext = {
        framework: 'nestjs',
        monorepo: 'none',
        tsStrictness: 'moderate',
        size: 'medium',
        fileCount: 100,
        hasEslint: false,
        hasPrettier: false,
        hasTests: false,
        packageManager: 'npm',
        projectRoot: '/test',
        workspacePackages: [],
      };

      const config = generator.generate(context);
      const antipatternCheck = config.checks.find((c) => c.name === 'antipattern');
      const allowlist = antipatternCheck?.config?.allowlist as string[];

      expect(allowlist).toContain('**/*.d.ts');
      expect(allowlist).toContain('**/main.ts');
      expect(allowlist).toContain('**/*.module.ts');
    });

    it('should always include common test file patterns in allowlist', () => {
      const context: ProjectContext = {
        framework: 'unknown',
        monorepo: 'none',
        tsStrictness: 'none',
        size: 'small',
        fileCount: 10,
        hasEslint: false,
        hasPrettier: false,
        hasTests: false,
        packageManager: 'npm',
        projectRoot: '/test',
        workspacePackages: [],
      };

      const config = generator.generate(context);
      const antipatternCheck = config.checks.find((c) => c.name === 'antipattern');
      const allowlist = antipatternCheck?.config?.allowlist as string[];

      expect(allowlist).toContain('**/*.test.ts');
      expect(allowlist).toContain('**/*.spec.ts');
      expect(allowlist).toContain('**/__tests__/**');
      expect(allowlist).toContain('**/__mocks__/**');
    });
  });

  describe('monorepo support', () => {
    it('should include monorepo patterns for pnpm workspace', () => {
      const context: ProjectContext = {
        framework: 'nextjs',
        monorepo: 'pnpm-workspace',
        tsStrictness: 'strict',
        size: 'large',
        fileCount: 800,
        hasEslint: true,
        hasPrettier: true,
        hasTests: true,
        packageManager: 'pnpm',
        projectRoot: '/test',
        workspacePackages: ['packages/*', 'apps/*'],
      };

      const config = generator.generate(context);

      // Monorepos should have architecture check enabled
      const archCheck = config.checks.find((c) => c.name === 'architecture');
      expect(archCheck).toBeDefined();
    });

    it('should not enable architecture check for small projects', () => {
      const context: ProjectContext = {
        framework: 'react',
        monorepo: 'none',
        tsStrictness: 'strict',
        size: 'small',
        fileCount: 30,
        hasEslint: true,
        hasPrettier: false,
        hasTests: true,
        packageManager: 'npm',
        projectRoot: '/test',
        workspacePackages: [],
      };

      const config = generator.generate(context);
      const archCheck = config.checks.find((c) => c.name === 'architecture');

      expect(archCheck).toBeUndefined();
    });

    it('should enable architecture check for medium+ TypeScript projects', () => {
      const context: ProjectContext = {
        framework: 'react',
        monorepo: 'none',
        tsStrictness: 'moderate',
        size: 'medium',
        fileCount: 150,
        hasEslint: true,
        hasPrettier: false,
        hasTests: true,
        packageManager: 'npm',
        projectRoot: '/test',
        workspacePackages: [],
      };

      const config = generator.generate(context);
      const archCheck = config.checks.find((c) => c.name === 'architecture');

      expect(archCheck).toBeDefined();
      expect(archCheck?.enabled).toBe(true);
    });
  });

  describe('complete configurations', () => {
    it('should generate appropriate config for a full-featured Next.js project', () => {
      const context: ProjectContext = {
        framework: 'nextjs',
        monorepo: 'pnpm-workspace',
        tsStrictness: 'strict',
        size: 'large',
        fileCount: 650,
        hasEslint: true,
        hasPrettier: true,
        hasTests: true,
        packageManager: 'pnpm',
        projectRoot: '/test',
        workspacePackages: ['packages/*', 'apps/*'],
      };

      const config = generator.generate(context);

      expect(config.version).toBe(1);
      expect(config.checks.length).toBeGreaterThan(4);

      const eslintCheck = config.checks.find((c) => c.name === 'eslint');
      const coverageCheck = config.checks.find((c) => c.name === 'coverage');
      const secretCheck = config.checks.find((c) => c.name === 'secret');
      const archCheck = config.checks.find((c) => c.name === 'architecture');

      expect(eslintCheck?.enabled).toBe(true);
      expect(coverageCheck?.enabled).toBe(true);
      expect(secretCheck?.enabled).toBe(true);
      expect(archCheck?.enabled).toBe(true);
    });

    it('should generate minimal config for a basic JavaScript project', () => {
      const context: ProjectContext = {
        framework: 'unknown',
        monorepo: 'none',
        tsStrictness: 'none',
        size: 'small',
        fileCount: 15,
        hasEslint: false,
        hasPrettier: false,
        hasTests: false,
        packageManager: 'npm',
        projectRoot: '/test',
        workspacePackages: [],
      };

      const config = generator.generate(context);

      expect(config.version).toBe(1);

      const eslintCheck = config.checks.find((c) => c.name === 'eslint');
      const coverageCheck = config.checks.find((c) => c.name === 'coverage');
      const secretCheck = config.checks.find((c) => c.name === 'secret');
      const antipatternCheck = config.checks.find((c) => c.name === 'antipattern');

      expect(eslintCheck).toBeUndefined();
      expect(coverageCheck).toBeUndefined();
      expect(secretCheck?.enabled).toBe(true);
      expect(antipatternCheck?.enabled).toBe(true);
    });
  });

  describe('config explanation', () => {
    it('should generate human-readable explanation', () => {
      const context: ProjectContext = {
        framework: 'react',
        monorepo: 'none',
        tsStrictness: 'strict',
        size: 'medium',
        fileCount: 120,
        hasEslint: true,
        hasPrettier: false,
        hasTests: true,
        packageManager: 'npm',
        projectRoot: '/test',
        workspacePackages: [],
      };

      const config = generator.generate(context);
      const explanation = generator.explainConfig(context, config);

      expect(explanation).toContain('Framework: react');
      expect(explanation).toContain('Project size: medium');
      expect(explanation).toContain('TypeScript strictness: strict');
      expect(explanation).toContain('Enabled checks:');
      expect(explanation).toContain('Overall score threshold:');
    });

    it('should include monorepo information when present', () => {
      const context: ProjectContext = {
        framework: 'nextjs',
        monorepo: 'pnpm-workspace',
        tsStrictness: 'strict',
        size: 'large',
        fileCount: 500,
        hasEslint: true,
        hasPrettier: true,
        hasTests: true,
        packageManager: 'pnpm',
        projectRoot: '/test',
        workspacePackages: ['packages/*'],
      };

      const config = generator.generate(context);
      const explanation = generator.explainConfig(context, config);

      expect(explanation).toContain('Monorepo: pnpm-workspace');
    });
  });
});
