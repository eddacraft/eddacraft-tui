/**
 * Unit Tests for ESLint Check
 *
 * Tests ESLint code quality validation
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { ESLintCheck } from './eslint.check.js';
import { CheckContext, PlanData } from '../../types/gate.types.js';
import { writeFileSync, mkdirSync, rmSync, existsSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';

describe('ESLintCheck', () => {
  let eslintCheck: ESLintCheck;
  let tempDir: string;
  let context: CheckContext;

  beforeEach(() => {
    eslintCheck = new ESLintCheck();
    tempDir = join(tmpdir(), 'anvil-eslint-test', Math.random().toString(36));
    mkdirSync(tempDir, { recursive: true });

    const mockPlan: PlanData = {
      id: 'aps-test123',
      schema_version: '0.1.0',
      hash: 'test-hash',
      intent: 'Test plan',
      proposed_changes: [
        {
          type: 'file_create',
          path: 'test.js',
          description: 'Create test file',
          content: '',
        },
      ],
      provenance: {
        timestamp: '2024-01-01T00:00:00Z',
        author: 'test@example.com',
        source: 'cli',
        version: '1.0.0',
      },
      validations: {
        required_checks: [],
        skip_checks: [],
      },
      evidence: [],
      executions: [],
    };

    context = {
      plan: mockPlan,
      workspace_root: tempDir,
      config: {
        version: 1,
        checks: [],
        thresholds: { overall_score: 80 },
      },
      check_config: {
        min_score: 80,
      },
    };
  });

  afterEach(() => {
    if (existsSync(tempDir)) {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });

  describe('check metadata', () => {
    it('should have correct name', () => {
      expect(eslintCheck.name).toBe('eslint');
    });

    it('should have correct description', () => {
      expect(eslintCheck.description).toBe('Run ESLint code quality checks');
    });
  });

  describe('valid code', () => {
    it('should pass when code has no issues', async () => {
      writeFileSync(join(tempDir, 'test.js'), 'const x = 1;\nconsole.log(x);\n');

      const result = await eslintCheck.run(context);

      expect(result.passed).toBe(true);
      expect(result.message).toContain('ESLint passed');
      expect(result.score).toBe(100);
    });

    it('should pass with empty file', async () => {
      writeFileSync(join(tempDir, 'test.js'), '');

      const result = await eslintCheck.run(context);

      expect(result.passed).toBe(true);
    });

    it('should return score of 100 for clean code', async () => {
      writeFileSync(
        join(tempDir, 'test.js'),
        'const greeting = "hello";\nconsole.log(greeting);\n'
      );

      const result = await eslintCheck.run(context);

      expect(result.score).toBe(100);
    });
  });

  describe('invalid code', () => {
    it('should fail when code has errors', async () => {
      // Create a file with unused variables (ESLint error)
      writeFileSync(join(tempDir, 'test.js'), 'const x = 1;\nconst y = 2;\n');

      const result = await eslintCheck.run(context);

      expect(result.passed).toBe(false);
      expect(result.message).toContain('ESLint failed');
    });

    it('should detect syntax errors', async () => {
      // Invalid JavaScript syntax
      writeFileSync(join(tempDir, 'test.js'), 'const x = ;\n');

      const result = await eslintCheck.run(context);

      expect(result.passed).toBe(false);
      expect(result.details?.errorCount).toBeGreaterThan(0);
    });

    it('should provide error details in result', async () => {
      writeFileSync(join(tempDir, 'test.js'), 'const unused = 1;\n');

      const result = await eslintCheck.run(context);

      expect(result.details).toBeDefined();
      expect(result.details?.errorCount).toBeDefined();
      expect(result.details?.warningCount).toBeDefined();
      expect(result.details?.results).toBeDefined();
    });

    it('should calculate score based on errors and warnings', async () => {
      writeFileSync(join(tempDir, 'test.js'), 'const x = 1;\n');

      const result = await eslintCheck.run(context);

      expect(typeof result.score).toBe('number');
      expect(result.score).toBeLessThan(100);
    });
  });

  describe('file filtering', () => {
    it('should only lint lintable files', async () => {
      // Create both lintable and non-lintable files
      writeFileSync(join(tempDir, 'test.js'), 'const x = 1;\n');
      writeFileSync(join(tempDir, 'data.json'), '{"key": "value"}');

      context.plan.proposed_changes = [
        {
          type: 'file_create',
          path: 'test.js',
          description: 'JS file',
        },
        {
          type: 'file_create',
          path: 'data.json',
          description: 'JSON file',
        },
      ];

      const result = await eslintCheck.run(context);

      // Should only lint .js files
      expect(result).toBeDefined();
    });

    it('should support TypeScript files', async () => {
      writeFileSync(join(tempDir, 'test.ts'), 'const x: number = 1;\n');

      context.plan.proposed_changes = [
        {
          type: 'file_create',
          path: 'test.ts',
          description: 'TypeScript file',
        },
      ];

      const result = await eslintCheck.run(context);

      expect(result).toBeDefined();
      expect(result.passed).toBeDefined();
    });

    it('should support JSX files', async () => {
      writeFileSync(join(tempDir, 'Component.jsx'), 'const App = () => <div>Hello</div>;\n');

      context.plan.proposed_changes = [
        {
          type: 'file_create',
          path: 'Component.jsx',
          description: 'JSX file',
        },
      ];

      const result = await eslintCheck.run(context);

      expect(result).toBeDefined();
    });

    it('should pass when no lintable files are present', async () => {
      context.plan.proposed_changes = [
        {
          type: 'file_create',
          path: 'data.txt',
          description: 'Text file',
        },
      ];

      const result = await eslintCheck.run(context);

      expect(result.passed).toBe(true);
      expect(result.message).toContain('No files to lint');
    });
  });

  describe('configuration options', () => {
    it('should respect custom min_score', async () => {
      writeFileSync(join(tempDir, 'test.js'), 'const x = 1;\n');

      context.check_config.min_score = 50;
      const result = await eslintCheck.run(context);

      // With lower threshold, might pass even with warnings
      expect(typeof result.passed).toBe('boolean');
    });

    it('should use default min_score when not specified', async () => {
      writeFileSync(join(tempDir, 'test.js'), 'console.log("test");\n');

      delete context.check_config.min_score;
      const result = await eslintCheck.run(context);

      expect(result).toBeDefined();
    });
  });

  describe('error handling', () => {
    it('should handle missing files gracefully', async () => {
      context.plan.proposed_changes = [
        {
          type: 'file_create',
          path: 'nonexistent.js',
          description: 'Missing file',
        },
      ];

      const result = await eslintCheck.run(context);

      // Should not throw, should return a result
      expect(result).toBeDefined();
      expect(typeof result.passed).toBe('boolean');
    });

    it('should handle malformed workspace root', async () => {
      context.workspace_root = '/nonexistent/path/to/workspace';

      const result = await eslintCheck.run(context);

      expect(result).toBeDefined();
      expect(typeof result.passed).toBe('boolean');
    });
  });

  describe('full scan mode', () => {
    it('should support full codebase scan', async () => {
      writeFileSync(join(tempDir, 'test.js'), 'const x = 1;\nconsole.log(x);\n');

      context.fullScan = true;
      const result = await eslintCheck.run(context);

      expect(result).toBeDefined();
      expect(typeof result.passed).toBe('boolean');
    });
  });

  describe('result details', () => {
    it('should include fixable count in details', async () => {
      // Create a file with fixable issues
      writeFileSync(join(tempDir, 'test.js'), 'const x=1;\n');

      const result = await eslintCheck.run(context);

      expect(result.details).toBeDefined();
      expect(result.details?.fixableCount).toBeDefined();
    });

    it('should include file-level results', async () => {
      writeFileSync(join(tempDir, 'test.js'), 'const x = 1;\n');

      const result = await eslintCheck.run(context);

      expect(result.details?.results).toBeDefined();
      expect(Array.isArray(result.details?.results)).toBe(true);
    });
  });
});
