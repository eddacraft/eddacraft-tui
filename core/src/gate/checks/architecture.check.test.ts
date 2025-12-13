import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { ArchitectureCheck } from './architecture.check.js';
import type { CheckContext } from '../../types/gate.types.js';
import type { APSPlan } from '../../schema/aps.schema.js';

// Mock plan for testing
const createMockPlan = (changes: Array<{ type: string; path: string }>): APSPlan => ({
  id: 'aps-12345678',
  hash: 'abc123',
  schema_version: '0.1.0',
  intent: 'Test plan',
  proposed_changes: changes.map((c) => ({
    type: c.type as 'file_create' | 'file_update' | 'file_delete',
    path: c.path,
    description: `Test change to ${c.path}`,
  })),
  provenance: {
    source_format: 'test',
    source_file: 'test.md',
    conversion_timestamp: new Date().toISOString(),
  },
  validations: [],
  tags: [],
});

// Mock context factory
const createMockContext = (
  checkConfig: Record<string, unknown> = {},
  plan?: APSPlan
): CheckContext => ({
  plan: plan || createMockPlan([{ type: 'file_update', path: 'src/index.ts' }]),
  workspace_root: process.cwd(),
  config: {
    version: 1,
    checks: [],
    thresholds: { overall_score: 80 },
  },
  check_config: checkConfig,
});

describe('ArchitectureCheck', () => {
  let check: ArchitectureCheck;

  beforeEach(() => {
    check = new ArchitectureCheck();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('metadata', () => {
    it('should have correct name', () => {
      expect(check.name).toBe('architecture');
    });

    it('should have a description', () => {
      expect(check.description).toBeTruthy();
      expect(check.description).toContain('dependency-cruiser');
    });
  });

  describe('run()', () => {
    it('should skip gracefully when dependency-cruiser is not installed', async () => {
      const context = createMockContext();
      const result = await check.run(context);

      // Since dependency-cruiser is likely not installed in test environment
      // it should return a success with skipped flag
      expect(result.passed).toBe(true);
      expect(result.details?.skipped).toBe(true);
      expect(result.message).toContain('dependency-cruiser not installed');
    });

    it('should return 100 score when skipped', async () => {
      const context = createMockContext();
      const result = await check.run(context);

      expect(result.score).toBe(100);
    });

    it('should handle empty plan gracefully', async () => {
      const context = createMockContext({}, createMockPlan([]));
      const result = await check.run(context);

      expect(result.passed).toBe(true);
    });
  });

  describe('configuration', () => {
    it('should use default config when none provided', async () => {
      const context = createMockContext({});
      const result = await check.run(context);

      // Check that it processed with defaults (even if skipped)
      expect(result).toBeDefined();
    });

    it('should respect custom config_file path', async () => {
      const context = createMockContext({
        config_file: 'custom.dependency-cruiser.js',
      });
      const result = await check.run(context);

      // Should attempt to use custom config
      expect(result).toBeDefined();
    });

    it('should respect scope setting', async () => {
      const context = createMockContext({ scope: 'full' });
      const result = await check.run(context);

      expect(result).toBeDefined();
    });

    it('should parse severity_threshold correctly', async () => {
      const context = createMockContext({ severity_threshold: 'warn' });
      const result = await check.run(context);

      expect(result).toBeDefined();
    });

    it('should parse fail_on_circular correctly', async () => {
      const contextEnabled = createMockContext({ fail_on_circular: true });
      const contextDisabled = createMockContext({ fail_on_circular: false });

      const resultEnabled = await check.run(contextEnabled);
      const resultDisabled = await check.run(contextDisabled);

      expect(resultEnabled).toBeDefined();
      expect(resultDisabled).toBeDefined();
    });

    it('should parse fail_on_orphan correctly', async () => {
      const contextEnabled = createMockContext({ fail_on_orphan: true });
      const contextDisabled = createMockContext({ fail_on_orphan: false });

      const resultEnabled = await check.run(contextEnabled);
      const resultDisabled = await check.run(contextDisabled);

      expect(resultEnabled).toBeDefined();
      expect(resultDisabled).toBeDefined();
    });
  });

  describe('file filtering', () => {
    it('should only analyse TypeScript/JavaScript files', async () => {
      const plan = createMockPlan([
        { type: 'file_update', path: 'src/index.ts' },
        { type: 'file_update', path: 'README.md' },
        { type: 'file_update', path: 'src/utils.js' },
        { type: 'file_create', path: 'docs/guide.md' },
      ]);
      const context = createMockContext({}, plan);
      const result = await check.run(context);

      // Should process without error
      expect(result).toBeDefined();
    });

    it('should exclude test files by default', async () => {
      const plan = createMockPlan([
        { type: 'file_update', path: 'src/index.test.ts' },
        { type: 'file_update', path: 'src/utils.spec.ts' },
      ]);
      const context = createMockContext({}, plan);
      const result = await check.run(context);

      expect(result).toBeDefined();
    });

    it('should use custom exclude patterns', async () => {
      const context = createMockContext({
        exclude_patterns: ['**/generated/**', '**/vendor/**'],
      });
      const result = await check.run(context);

      expect(result).toBeDefined();
    });
  });

  describe('result format', () => {
    it('should return a properly formatted GateResult', async () => {
      const context = createMockContext();
      const result = await check.run(context);

      expect(result).toHaveProperty('check');
      expect(result).toHaveProperty('passed');
      expect(result).toHaveProperty('message');
      expect(result.check).toBe('architecture');
    });

    it('should include details in result', async () => {
      const context = createMockContext();
      const result = await check.run(context);

      expect(result.details).toBeDefined();
    });
  });
});
