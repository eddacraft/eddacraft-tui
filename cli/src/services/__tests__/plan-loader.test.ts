import { describe, it, expect, beforeEach, vi } from 'vitest';
import { PlanLoader } from '../plan-loader.js';

// Mock dependencies
vi.mock('@anvil/core', () => ({
  APSValidator: vi.fn(() => ({
    validate: vi.fn(async (data) => ({
      valid: true,
      data,
      issues: [],
    })),
  })),
}));

vi.mock('@anvil/adapters', () => ({
  AdapterRegistry: {
    getInstance: vi.fn(() => ({
      listAdapters: vi.fn(() => []),
    })),
  },
}));

vi.mock('../format-detection.js', () => ({
  FormatDetectionService: vi.fn(() => ({
    detectFormat: vi.fn(() => null),
  })),
}));

describe('PlanLoader', () => {
  let loader: PlanLoader;

  beforeEach(() => {
    loader = new PlanLoader();
  });

  describe('constructor', () => {
    it('should create loader with default options', () => {
      const defaultLoader = new PlanLoader();
      expect(defaultLoader).toBeDefined();
    });

    it('should create loader with custom min confidence', () => {
      const customLoader = new PlanLoader({ minConfidence: 70 });
      expect(customLoader).toBeDefined();
    });
  });

  describe('loadPlanFromContent', () => {
    it('should load valid APS JSON content', async () => {
      const apsPlan = {
        schema_version: '0.1.0',
        id: 'aps-00000001',
        hash: '0'.repeat(64),
        intent: 'Test plan',
        proposed_changes: [],
        provenance: {
          timestamp: new Date().toISOString(),
          source: 'test',
          version: '1.0.0',
        },
        validations: {
          required_checks: [],
          skip_checks: [],
        },
      };

      const content = JSON.stringify(apsPlan);
      
      const result = await loader.loadPlanFromContent(content, {
        format: 'aps',
      });

      expect(result).toBeDefined();
      expect(result.plan).toBeDefined();
      expect(result.validation).toBeDefined();
    });

    it('should throw error for invalid JSON', async () => {
      const invalidJson = '{ invalid json }';

      await expect(async () => {
        await loader.loadPlanFromContent(invalidJson, { format: 'aps' });
      }).rejects.toThrow();
    });

    it('should detect format when not explicitly specified', async () => {
      const content = '# Some content';

      await expect(async () => {
        await loader.loadPlanFromContent(content);
      }).rejects.toThrow('Unable to detect plan format');
    });
  });

  describe('loadPlan', () => {
    it('should throw error for non-existent file', async () => {
      const nonExistentPath = '/path/to/nonexistent/file.json';

      await expect(async () => {
        await loader.loadPlan(nonExistentPath);
      }).rejects.toThrow('Failed to load plan');
    });
  });
});
