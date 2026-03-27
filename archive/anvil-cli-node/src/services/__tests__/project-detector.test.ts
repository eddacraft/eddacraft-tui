/**
 * Unit Tests for ProjectDetector
 *
 * Tests comprehensive project context detection including:
 * - Framework detection (Next.js, React, Vue, etc.)
 * - Monorepo structure detection (Nx, Lerna, pnpm, yarn)
 * - TypeScript strictness analysis
 * - Project size categorization
 * - Workspace package detection
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { ProjectDetector } from '../project-detector.js';
import {
  createTestWorkspace,
  createPackageJson,
  createEslintConfig,
  type TestWorkspace,
} from '../../__tests__/helpers/test-workspace.js';
import { writeFileSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';

describe('ProjectDetector', () => {
  let workspace: TestWorkspace;

  beforeEach(() => {
    workspace = createTestWorkspace();
  });

  afterEach(() => {
    workspace.cleanup();
  });

  describe('framework detection', () => {
    it('should detect Next.js from package.json', () => {
      createPackageJson(workspace.root, {
        dependencies: { next: '^14.0.0', react: '^18.0.0' },
      });

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.framework).toBe('nextjs');
    });

    it('should detect React from package.json', () => {
      createPackageJson(workspace.root, {
        dependencies: { react: '^18.0.0' },
      });

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.framework).toBe('react');
    });

    it('should detect Vue from package.json', () => {
      createPackageJson(workspace.root, {
        dependencies: { vue: '^3.0.0' },
      });

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.framework).toBe('vue');
    });

    it('should detect Angular from package.json', () => {
      createPackageJson(workspace.root, {
        dependencies: { '@angular/core': '^17.0.0' },
      });

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.framework).toBe('angular');
    });

    it('should detect Svelte from package.json', () => {
      createPackageJson(workspace.root, {
        devDependencies: { svelte: '^4.0.0' },
      });

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.framework).toBe('svelte');
    });

    it('should detect Express from package.json', () => {
      createPackageJson(workspace.root, {
        dependencies: { express: '^4.18.0' },
      });

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.framework).toBe('express');
    });

    it('should detect NestJS from package.json', () => {
      createPackageJson(workspace.root, {
        dependencies: { '@nestjs/core': '^10.0.0' },
      });

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.framework).toBe('nestjs');
    });

    it('should detect Nx from package.json', () => {
      createPackageJson(workspace.root, {
        devDependencies: { '@nx/workspace': '^18.0.0' },
      });

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.framework).toBe('nx');
    });

    it('should detect Node.js project from type module', () => {
      writeFileSync(
        join(workspace.root, 'package.json'),
        JSON.stringify({ type: 'module' }, null, 2),
        'utf-8'
      );

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.framework).toBe('node');
    });

    it('should return unknown for unrecognized framework', () => {
      createPackageJson(workspace.root, {
        dependencies: { lodash: '^4.17.21' },
      });

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.framework).toBe('unknown');
    });

    it('should prioritize Next.js over React', () => {
      createPackageJson(workspace.root, {
        dependencies: { next: '^14.0.0', react: '^18.0.0' },
      });

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.framework).toBe('nextjs');
    });

    it('should prioritize NestJS over Express', () => {
      createPackageJson(workspace.root, {
        dependencies: { '@nestjs/core': '^10.0.0', express: '^4.18.0' },
      });

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.framework).toBe('nestjs');
    });
  });

  describe('monorepo detection', () => {
    it('should detect Nx monorepo from nx.json', () => {
      writeFileSync(join(workspace.root, 'nx.json'), '{}', 'utf-8');

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.monorepo).toBe('nx');
    });

    it('should detect Turborepo from turbo.json', () => {
      writeFileSync(join(workspace.root, 'turbo.json'), '{}', 'utf-8');

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.monorepo).toBe('turborepo');
    });

    it('should detect Lerna from lerna.json', () => {
      writeFileSync(join(workspace.root, 'lerna.json'), '{}', 'utf-8');

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.monorepo).toBe('lerna');
    });

    it('should detect pnpm workspace from pnpm-workspace.yaml', () => {
      writeFileSync(
        join(workspace.root, 'pnpm-workspace.yaml'),
        'packages:\n  - "packages/*"\n',
        'utf-8'
      );

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.monorepo).toBe('pnpm-workspace');
    });

    it('should detect yarn workspace from package.json', () => {
      createPackageJson(workspace.root, {
        name: 'monorepo',
      });
      writeFileSync(
        join(workspace.root, 'package.json'),
        JSON.stringify(
          {
            name: 'monorepo',
            workspaces: ['packages/*'],
          },
          null,
          2
        ),
        'utf-8'
      );
      writeFileSync(join(workspace.root, 'yarn.lock'), '', 'utf-8');

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.monorepo).toBe('yarn-workspace');
    });

    it('should detect npm workspace from package.json', () => {
      writeFileSync(
        join(workspace.root, 'package.json'),
        JSON.stringify(
          {
            name: 'monorepo',
            workspaces: ['packages/*'],
          },
          null,
          2
        ),
        'utf-8'
      );
      writeFileSync(join(workspace.root, 'package-lock.json'), '{}', 'utf-8');

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.monorepo).toBe('npm-workspace');
    });

    it('should return none for non-monorepo project', () => {
      createPackageJson(workspace.root);

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.monorepo).toBe('none');
    });
  });

  describe('TypeScript strictness detection', () => {
    it('should detect strict mode', () => {
      writeFileSync(
        join(workspace.root, 'tsconfig.json'),
        JSON.stringify(
          {
            compilerOptions: {
              strict: true,
            },
          },
          null,
          2
        ),
        'utf-8'
      );

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.tsStrictness).toBe('strict');
    });

    it('should detect strict when multiple strict flags are enabled', () => {
      writeFileSync(
        join(workspace.root, 'tsconfig.json'),
        JSON.stringify(
          {
            compilerOptions: {
              strictNullChecks: true,
              strictFunctionTypes: true,
              strictBindCallApply: true,
              noImplicitAny: true,
            },
          },
          null,
          2
        ),
        'utf-8'
      );

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.tsStrictness).toBe('strict');
    });

    it('should detect moderate strictness', () => {
      writeFileSync(
        join(workspace.root, 'tsconfig.json'),
        JSON.stringify(
          {
            compilerOptions: {
              strictNullChecks: true,
              noImplicitAny: true,
            },
          },
          null,
          2
        ),
        'utf-8'
      );

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.tsStrictness).toBe('moderate');
    });

    it('should detect loose strictness', () => {
      writeFileSync(
        join(workspace.root, 'tsconfig.json'),
        JSON.stringify(
          {
            compilerOptions: {
              strictNullChecks: true,
            },
          },
          null,
          2
        ),
        'utf-8'
      );

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.tsStrictness).toBe('loose');
    });

    it('should return none when no tsconfig.json exists', () => {
      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.tsStrictness).toBe('none');
    });

    it('should handle malformed tsconfig.json gracefully', () => {
      writeFileSync(join(workspace.root, 'tsconfig.json'), 'invalid json', 'utf-8');

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.tsStrictness).toBe('none');
    });
  });

  describe('project size categorization', () => {
    it('should categorize small project (< 50 files)', () => {
      createPackageJson(workspace.root);
      mkdirSync(join(workspace.root, 'src'), { recursive: true });

      // Create 20 files
      for (let i = 0; i < 20; i++) {
        writeFileSync(join(workspace.root, 'src', `file${i}.ts`), '', 'utf-8');
      }

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.size).toBe('small');
      expect(context.fileCount).toBe(20);
    });

    it('should categorize medium project (50-200 files)', () => {
      createPackageJson(workspace.root);
      mkdirSync(join(workspace.root, 'src'), { recursive: true });

      // Create 100 files
      for (let i = 0; i < 100; i++) {
        writeFileSync(join(workspace.root, 'src', `file${i}.ts`), '', 'utf-8');
      }

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.size).toBe('medium');
      expect(context.fileCount).toBeGreaterThanOrEqual(100);
    });

    it('should skip node_modules and dist directories', () => {
      createPackageJson(workspace.root);
      mkdirSync(join(workspace.root, 'src'), { recursive: true });
      mkdirSync(join(workspace.root, 'node_modules'), { recursive: true });
      mkdirSync(join(workspace.root, 'dist'), { recursive: true });

      // Create files in different directories
      writeFileSync(join(workspace.root, 'src', 'app.ts'), '', 'utf-8');
      writeFileSync(join(workspace.root, 'node_modules', 'package.js'), '', 'utf-8');
      writeFileSync(join(workspace.root, 'dist', 'bundle.js'), '', 'utf-8');

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.fileCount).toBe(1); // Only counts src/app.ts
    });

    it('should only count source files', () => {
      createPackageJson(workspace.root);
      mkdirSync(join(workspace.root, 'src'), { recursive: true });

      writeFileSync(join(workspace.root, 'src', 'app.ts'), '', 'utf-8');
      writeFileSync(join(workspace.root, 'src', 'README.md'), '', 'utf-8');
      writeFileSync(join(workspace.root, 'src', 'config.json'), '', 'utf-8');

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.fileCount).toBe(1); // Only counts app.ts
    });
  });

  describe('workspace packages detection', () => {
    it('should detect packages from package.json workspaces array', () => {
      writeFileSync(
        join(workspace.root, 'package.json'),
        JSON.stringify(
          {
            workspaces: ['packages/*', 'apps/*'],
          },
          null,
          2
        ),
        'utf-8'
      );

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.workspacePackages).toEqual(['packages/*', 'apps/*']);
    });

    it('should detect packages from package.json workspaces.packages', () => {
      writeFileSync(
        join(workspace.root, 'package.json'),
        JSON.stringify(
          {
            workspaces: {
              packages: ['packages/*'],
            },
          },
          null,
          2
        ),
        'utf-8'
      );

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.workspacePackages).toEqual(['packages/*']);
    });

    it('should detect packages from pnpm-workspace.yaml', () => {
      writeFileSync(
        join(workspace.root, 'pnpm-workspace.yaml'),
        `packages:
  - 'packages/*'
  - 'apps/*'
`,
        'utf-8'
      );

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.workspacePackages).toContain('packages/*');
      expect(context.workspacePackages).toContain('apps/*');
    });

    it('should detect packages from lerna.json', () => {
      writeFileSync(
        join(workspace.root, 'lerna.json'),
        JSON.stringify(
          {
            packages: ['packages/*', 'libs/*'],
          },
          null,
          2
        ),
        'utf-8'
      );

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.workspacePackages).toEqual(['packages/*', 'libs/*']);
    });

    it('should return empty array for non-monorepo', () => {
      createPackageJson(workspace.root);

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.workspacePackages).toEqual([]);
    });
  });

  describe('tooling detection', () => {
    it('should detect ESLint', () => {
      createEslintConfig(workspace.root);

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.hasEslint).toBe(true);
    });

    it('should detect Prettier', () => {
      writeFileSync(join(workspace.root, '.prettierrc'), '{}', 'utf-8');

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.hasPrettier).toBe(true);
    });

    it('should detect test frameworks', () => {
      createPackageJson(workspace.root, {
        devDependencies: { vitest: '^3.0.0' },
      });

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.hasTests).toBe(true);
    });

    it('should detect package manager', () => {
      writeFileSync(join(workspace.root, 'pnpm-lock.yaml'), '', 'utf-8');

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.packageManager).toBe('pnpm');
    });
  });

  describe('complete project context detection', () => {
    it('should detect all context for a full-featured Next.js monorepo', () => {
      // Set up a realistic Next.js monorepo
      writeFileSync(
        join(workspace.root, 'package.json'),
        JSON.stringify(
          {
            name: 'nextjs-monorepo',
            workspaces: ['packages/*', 'apps/*'],
            devDependencies: {
              next: '^14.0.0',
              react: '^18.0.0',
              typescript: '^5.0.0',
              eslint: '^8.0.0',
              prettier: '^3.0.0',
              vitest: '^3.0.0',
            },
          },
          null,
          2
        ),
        'utf-8'
      );
      writeFileSync(
        join(workspace.root, 'tsconfig.json'),
        JSON.stringify({ compilerOptions: { strict: true } }, null, 2),
        'utf-8'
      );
      writeFileSync(join(workspace.root, '.eslintrc.json'), '{}', 'utf-8');
      writeFileSync(join(workspace.root, '.prettierrc'), '{}', 'utf-8');
      writeFileSync(join(workspace.root, 'pnpm-lock.yaml'), '', 'utf-8');
      writeFileSync(
        join(workspace.root, 'pnpm-workspace.yaml'),
        'packages:\n  - "packages/*"\n  - "apps/*"\n',
        'utf-8'
      );

      mkdirSync(join(workspace.root, 'apps/web/src'), { recursive: true });
      for (let i = 0; i < 60; i++) {
        writeFileSync(join(workspace.root, 'apps/web/src', `component${i}.tsx`), '', 'utf-8');
      }

      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.framework).toBe('nextjs');
      expect(context.monorepo).toBe('pnpm-workspace');
      expect(context.tsStrictness).toBe('strict');
      expect(context.size).toBe('medium');
      expect(context.hasEslint).toBe(true);
      expect(context.hasPrettier).toBe(true);
      expect(context.hasTests).toBe(true);
      expect(context.packageManager).toBe('pnpm');
      expect(context.workspacePackages).toContain('packages/*');
      expect(context.workspacePackages).toContain('apps/*');
    });

    it('should handle minimal project gracefully', () => {
      const detector = new ProjectDetector(workspace.root);
      const context = detector.detect();

      expect(context.framework).toBe('unknown');
      expect(context.monorepo).toBe('none');
      expect(context.tsStrictness).toBe('none');
      expect(context.size).toBe('small');
      expect(context.hasEslint).toBe(false);
      expect(context.hasPrettier).toBe(false);
      expect(context.hasTests).toBe(false);
      expect(context.packageManager).toBe('unknown');
      expect(context.workspacePackages).toEqual([]);
    });

    it('should use current working directory as default', () => {
      const detector = new ProjectDetector();

      expect(detector.detect().projectRoot).toBe(process.cwd());
    });
  });
});
