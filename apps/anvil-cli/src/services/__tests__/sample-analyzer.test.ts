/**
 * Unit Tests for SampleAnalyzer
 *
 * Tests file selection for initial analysis including:
 * - Git-based recent file selection
 * - Filesystem fallback
 * - File filtering and limiting
 * - Pattern matching
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { SampleAnalyzer } from '../sample-analyzer.js';
import {
  createTestWorkspace,
  type TestWorkspace,
  initGitRepo,
} from '../../__tests__/helpers/test-workspace.js';
import { writeFileSync, mkdirSync } from 'fs';
import { join } from 'path';
import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

describe('SampleAnalyzer', () => {
  let workspace: TestWorkspace;
  let analyzer: SampleAnalyzer;

  beforeEach(() => {
    workspace = createTestWorkspace();
    analyzer = new SampleAnalyzer(workspace.root);
  });

  afterEach(() => {
    workspace.cleanup();
  });

  describe('filesystem-based selection', () => {
    it('should find TypeScript files in src directory', async () => {
      mkdirSync(join(workspace.root, 'src'), { recursive: true });
      writeFileSync(join(workspace.root, 'src', 'app.ts'), 'export const app = 1;', 'utf-8');
      writeFileSync(join(workspace.root, 'src', 'utils.ts'), 'export const utils = 1;', 'utf-8');

      const selection = await analyzer.selectFiles();

      expect(selection.files.length).toBe(2);
      expect(selection.files).toContain('src/app.ts');
      expect(selection.files).toContain('src/utils.ts');
      expect(selection.strategy).toBe('filesystem');
    });

    it('should find files in multiple source directories', async () => {
      mkdirSync(join(workspace.root, 'src'), { recursive: true });
      mkdirSync(join(workspace.root, 'lib'), { recursive: true });

      writeFileSync(join(workspace.root, 'src', 'app.ts'), 'content', 'utf-8');
      writeFileSync(join(workspace.root, 'lib', 'utils.ts'), 'content', 'utf-8');

      const selection = await analyzer.selectFiles();

      expect(selection.files.length).toBeGreaterThanOrEqual(2);
      expect(selection.strategy).toBe('filesystem');
    });

    it('should exclude test files by default', async () => {
      mkdirSync(join(workspace.root, 'src'), { recursive: true });
      writeFileSync(join(workspace.root, 'src', 'app.ts'), 'content', 'utf-8');
      writeFileSync(join(workspace.root, 'src', 'app.test.ts'), 'content', 'utf-8');
      writeFileSync(join(workspace.root, 'src', 'app.spec.ts'), 'content', 'utf-8');

      const selection = await analyzer.selectFiles();

      expect(selection.files).toContain('src/app.ts');
      expect(selection.files).not.toContain('src/app.test.ts');
      expect(selection.files).not.toContain('src/app.spec.ts');
    });

    it('should exclude node_modules directory', async () => {
      mkdirSync(join(workspace.root, 'src'), { recursive: true });
      mkdirSync(join(workspace.root, 'node_modules', 'pkg'), { recursive: true });

      writeFileSync(join(workspace.root, 'src', 'app.ts'), 'content', 'utf-8');
      writeFileSync(join(workspace.root, 'node_modules', 'pkg', 'index.ts'), 'content', 'utf-8');

      const selection = await analyzer.selectFiles();

      expect(selection.files).toContain('src/app.ts');
      expect(selection.files).not.toContain('node_modules/pkg/index.ts');
    });

    it('should exclude build directories', async () => {
      mkdirSync(join(workspace.root, 'src'), { recursive: true });
      mkdirSync(join(workspace.root, 'dist'), { recursive: true });
      mkdirSync(join(workspace.root, 'build'), { recursive: true });

      writeFileSync(join(workspace.root, 'src', 'app.ts'), 'content', 'utf-8');
      writeFileSync(join(workspace.root, 'dist', 'app.js'), 'content', 'utf-8');
      writeFileSync(join(workspace.root, 'build', 'app.js'), 'content', 'utf-8');

      const selection = await analyzer.selectFiles();

      expect(selection.files).toContain('src/app.ts');
      expect(selection.files.some((f) => f.includes('dist'))).toBe(false);
      expect(selection.files.some((f) => f.includes('build'))).toBe(false);
    });

    it('should limit files to maxFiles', async () => {
      mkdirSync(join(workspace.root, 'src'), { recursive: true });

      // Create 100 files
      for (let i = 0; i < 100; i++) {
        writeFileSync(join(workspace.root, 'src', `file${i}.ts`), 'content', 'utf-8');
      }

      const selection = await analyzer.selectFiles({ maxFiles: 20 });

      expect(selection.files.length).toBe(20);
      expect(selection.totalFound).toBe(100);
    });

    it('should include TypeScript React files', async () => {
      mkdirSync(join(workspace.root, 'src'), { recursive: true });
      writeFileSync(join(workspace.root, 'src', 'Component.tsx'), 'content', 'utf-8');

      const selection = await analyzer.selectFiles();

      expect(selection.files).toContain('src/Component.tsx');
    });

    it('should include JavaScript files', async () => {
      mkdirSync(join(workspace.root, 'src'), { recursive: true });
      writeFileSync(join(workspace.root, 'src', 'app.js'), 'content', 'utf-8');
      writeFileSync(join(workspace.root, 'src', 'Component.jsx'), 'content', 'utf-8');

      const selection = await analyzer.selectFiles();

      expect(selection.files).toContain('src/app.js');
      expect(selection.files).toContain('src/Component.jsx');
    });
  });

  describe('git-based selection', () => {
    it('should use git strategy when git is available', async () => {
      mkdirSync(join(workspace.root, 'src'), { recursive: true });
      writeFileSync(join(workspace.root, 'src', 'app.ts'), 'content', 'utf-8');

      initGitRepo(workspace.root);
      await execAsync('git add .', { cwd: workspace.root });
      await execAsync('git commit -m "initial"', { cwd: workspace.root });

      const selection = await analyzer.selectFiles();

      expect(selection.gitAvailable).toBe(true);
      if (selection.files.length > 0) {
        expect(selection.strategy).toBe('git-recent');
      }
    });

    it('should fallback to filesystem when git has no recent changes', async () => {
      mkdirSync(join(workspace.root, 'src'), { recursive: true });
      writeFileSync(join(workspace.root, 'src', 'app.ts'), 'content', 'utf-8');

      initGitRepo(workspace.root);
      // Don't commit anything - git will be available but have no history

      const selection = await analyzer.selectFiles();

      expect(selection.gitAvailable).toBe(true);
      expect(selection.strategy).toBe('filesystem');
      expect(selection.files.length).toBeGreaterThan(0);
    });

    it('should use filesystem strategy when git is not available', async () => {
      mkdirSync(join(workspace.root, 'src'), { recursive: true });
      writeFileSync(join(workspace.root, 'src', 'app.ts'), 'content', 'utf-8');

      const selection = await analyzer.selectFiles();

      expect(selection.gitAvailable).toBe(false);
      expect(selection.strategy).toBe('filesystem');
    });
  });

  describe('custom configuration', () => {
    it('should respect custom maxFiles', async () => {
      mkdirSync(join(workspace.root, 'src'), { recursive: true });

      for (let i = 0; i < 50; i++) {
        writeFileSync(join(workspace.root, 'src', `file${i}.ts`), 'content', 'utf-8');
      }

      const selection = await analyzer.selectFiles({ maxFiles: 10 });

      expect(selection.files.length).toBe(10);
    });

    it('should respect custom include patterns', async () => {
      mkdirSync(join(workspace.root, 'src'), { recursive: true });
      writeFileSync(join(workspace.root, 'src', 'app.ts'), 'content', 'utf-8');
      writeFileSync(join(workspace.root, 'src', 'data.json'), '{}', 'utf-8');

      const selection = await analyzer.selectFiles({
        includePatterns: ['.ts', '.json'],
      });

      expect(selection.files).toContain('src/app.ts');
      expect(selection.files).toContain('src/data.json');
    });

    it('should respect custom exclude patterns', async () => {
      mkdirSync(join(workspace.root, 'src'), { recursive: true });
      writeFileSync(join(workspace.root, 'src', 'app.ts'), 'content', 'utf-8');
      writeFileSync(join(workspace.root, 'src', 'generated.ts'), 'content', 'utf-8');

      const selection = await analyzer.selectFiles({
        excludePatterns: ['generated'],
      });

      expect(selection.files).toContain('src/app.ts');
      expect(selection.files).not.toContain('src/generated.ts');
    });
  });

  describe('selection statistics', () => {
    it('should provide accurate selection statistics', async () => {
      mkdirSync(join(workspace.root, 'src'), { recursive: true });

      for (let i = 0; i < 30; i++) {
        writeFileSync(join(workspace.root, 'src', `file${i}.ts`), 'content', 'utf-8');
      }

      const stats = await analyzer.getSelectionStats({ maxFiles: 10 });

      expect(stats.totalSourceFiles).toBe(30);
      expect(stats.selectedFiles).toBe(10);
    });

    it('should count recent files when git is available', async () => {
      mkdirSync(join(workspace.root, 'src'), { recursive: true });
      writeFileSync(join(workspace.root, 'src', 'app.ts'), 'content', 'utf-8');

      initGitRepo(workspace.root);
      await execAsync('git add .', { cwd: workspace.root });
      await execAsync('git commit -m "initial"', { cwd: workspace.root });

      const stats = await analyzer.getSelectionStats();

      expect(stats.totalSourceFiles).toBeGreaterThan(0);
    });
  });

  describe('file diversity', () => {
    it('should distribute selection across files when limiting', async () => {
      mkdirSync(join(workspace.root, 'src'), { recursive: true });

      // Create files in sequence
      const fileNames: string[] = [];
      for (let i = 0; i < 100; i++) {
        const fileName = `file${String(i).padStart(3, '0')}.ts`;
        fileNames.push(`src/${fileName}`);
        writeFileSync(join(workspace.root, 'src', fileName), 'content', 'utf-8');
      }

      const selection = await analyzer.selectFiles({ maxFiles: 10 });

      // Should get files from different parts of the list
      // Not just the first 10
      expect(selection.files.length).toBe(10);

      // Check that files are distributed (not all from the start)
<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/sample-analyzer.test.ts
<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/sample-analyzer.test.ts
      const allFromStart = selection.files.every((f) => fileNames.slice(0, 20).includes(f));
=======
      const allFromStart = selection.files.every((f) =>
        fileNames.slice(0, 20).includes(f),
      );
>>>>>>> 0c9fbc0 (feat(cli): Add sample analyzer for post-init analysis (IFR-003)):cli/src/services/__tests__/sample-analyzer.test.ts
=======
      const allFromStart = selection.files.every((f) => fileNames.slice(0, 20).includes(f));
>>>>>>> 177f91e (style: Apply Prettier formatting to IFR files):cli/src/services/__tests__/sample-analyzer.test.ts
      expect(allFromStart).toBe(false);
    });
  });

  describe('nested directories', () => {
    it('should find files in nested directories', async () => {
      mkdirSync(join(workspace.root, 'src', 'components', 'ui'), { recursive: true });
      writeFileSync(
        join(workspace.root, 'src', 'components', 'ui', 'Button.tsx'),
        'content',
<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/sample-analyzer.test.ts
<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/sample-analyzer.test.ts
        'utf-8'
=======
        'utf-8',
>>>>>>> 0c9fbc0 (feat(cli): Add sample analyzer for post-init analysis (IFR-003)):cli/src/services/__tests__/sample-analyzer.test.ts
=======
        'utf-8'
>>>>>>> 177f91e (style: Apply Prettier formatting to IFR files):cli/src/services/__tests__/sample-analyzer.test.ts
      );

      const selection = await analyzer.selectFiles();

      expect(selection.files).toContain('src/components/ui/Button.tsx');
    });

    it('should respect depth limit', async () => {
      // Create very deep nesting
      let currentPath = join(workspace.root, 'src');
      mkdirSync(currentPath, { recursive: true });

      for (let i = 0; i < 15; i++) {
        currentPath = join(currentPath, `level${i}`);
        mkdirSync(currentPath, { recursive: true });
      }

      writeFileSync(join(currentPath, 'deep.ts'), 'content', 'utf-8');

      const selection = await analyzer.selectFiles();

      // Should not find the deeply nested file (depth limit is 10)
      expect(selection.files).not.toContain(expect.stringContaining('level14'));
    });
  });

  describe('empty project', () => {
    it('should handle project with no source files', async () => {
      const selection = await analyzer.selectFiles();

      expect(selection.files.length).toBe(0);
      expect(selection.totalFound).toBe(0);
      expect(selection.strategy).toBe('filesystem');
    });

    it('should handle project with only excluded files', async () => {
      mkdirSync(join(workspace.root, 'src'), { recursive: true });
      writeFileSync(join(workspace.root, 'src', 'app.test.ts'), 'content', 'utf-8');

      const selection = await analyzer.selectFiles();

      expect(selection.files.length).toBe(0);
    });
  });
});
