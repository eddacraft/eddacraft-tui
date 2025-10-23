import { describe, it, expect, beforeEach } from 'vitest';
import { readFile } from 'node:fs/promises';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { SpecKitImportAdapter } from '../speckit/import.js';
import type { ExternalSpec, SpecContext } from '../common/types.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const fixturesDir = join(__dirname, 'fixtures');

describe('SpecKitImportAdapter', () => {
  let adapter: SpecKitImportAdapter;

  beforeEach(() => {
    adapter = new SpecKitImportAdapter();
  });

  describe('basic properties', () => {
    it('should have correct name and version', () => {
      expect(adapter.name).toBe('speckit-import');
      expect(adapter.version).toBe('1.0.0');
    });

    it('should support speckit and spec.md formats', () => {
      expect(adapter.canImport('speckit')).toBe(true);
      expect(adapter.canImport('spec.md')).toBe(true);
      expect(adapter.canImport('unknown')).toBe(false);
    });
  });

  describe('generateSpec', () => {
    it('should generate a basic spec from intent', async () => {
      const context: SpecContext = {
        repositoryPath: '/test/repo',
        branch: 'main',
        author: 'test@example.com',
      };

      const result = await adapter.generateSpec('Test intent', context);

      expect(result).toBeDefined();
      expect(result.intent).toBe('Test intent');
      expect(result.schema_version).toBe('0.1.0');
      expect(result.proposed_changes).toHaveLength(1);
      expect(result.proposed_changes[0].type).toBe('file_create');
      expect(result.proposed_changes[0].path).toBe('spec.md');
    });
  });

  describe('convertToAPS', () => {
    it('should convert a valid spec.md to APS format', async () => {
      const specContent = await readFile(join(fixturesDir, 'speckit/sample-spec.md'), 'utf-8');

      const externalSpec: ExternalSpec = {
        format: 'speckit',
        version: '1.0.0',
        content: {
          specContent,
          metadata: {
            timestamp: '2024-01-15T10:00:00Z',
            author: 'test@example.com',
          },
        },
      };

      const result = await adapter.convertToAPS(externalSpec);

      expect(result.success).toBe(true);
      expect(result.data).toBeDefined();

      if (result.success && result.data) {
        console.log('Changes found:', result.data.proposed_changes.length);
        console.log(
          'Changes:',
          result.data.proposed_changes.map((c) => ({
            type: c.type,
            path: c.path,
            desc: c.description.substring(0, 50),
          }))
        );
        expect(result.data.intent).toContain('JWT tokens');
        expect(result.data.schema_version).toBe('0.1.0');
        expect(result.data.proposed_changes.length).toBeGreaterThan(0);

        // Check that goals and requirements are preserved in metadata
        expect(result.data.metadata?.goals).toBeDefined();
        expect(result.data.metadata?.requirements).toBeDefined();
        expect(result.data.metadata?.source_format).toBe('speckit');

        // Verify specific changes were parsed correctly
        const fileCreateChanges = result.data.proposed_changes.filter(
          (c) => c.type === 'file_create'
        );
        expect(fileCreateChanges.length).toBeGreaterThan(0);

        const authControllerChange = fileCreateChanges.find(
          (c) => c.path === 'src/controllers/auth.controller.ts'
        );
        expect(authControllerChange).toBeDefined();
        expect(authControllerChange?.content).toContain('AuthController');
      }
    });

    it('should handle missing spec content', async () => {
      const externalSpec: ExternalSpec = {
        format: 'speckit',
        version: '1.0.0',
        content: {
          // Missing specContent
        },
      };

      const result = await adapter.convertToAPS(externalSpec);

      expect(result.success).toBe(false);
      expect(result.errors).toBeDefined();
      expect(result.errors?.[0].code).toBe('MISSING_SPEC_CONTENT');
    });

    it('should handle spec without intent section', async () => {
      const externalSpec: ExternalSpec = {
        format: 'speckit',
        version: '1.0.0',
        content: {
          specContent: '# Specification\n\n## Overview\n\nSome overview without intent.',
        },
      };

      const result = await adapter.convertToAPS(externalSpec);

      expect(result.success).toBe(true);
      expect(result.warnings).toBeDefined();
      // Should use overview as fallback for intent
      expect(result.data?.intent).toContain('Some overview');
    });

    it('should reject unsupported formats', async () => {
      const externalSpec: ExternalSpec = {
        format: 'unknown-format',
        version: '1.0.0',
        content: {},
      };

      const result = await adapter.convertToAPS(externalSpec);

      expect(result.success).toBe(false);
      expect(result.errors?.[0].code).toBe('UNSUPPORTED_FORMAT');
    });
  });

  describe('validateSpec', () => {
    it('should validate a valid APS spec', async () => {
      const spec = {
        id: 'aps-12345678',
        hash: '0'.repeat(64),
        intent: 'This is a valid intent for testing',
        schema_version: '0.1.0' as const,
        proposed_changes: [
          {
            type: 'file_create' as const,
            path: 'test.ts',
            description: 'Create a test file',
          },
        ],
        provenance: {
          timestamp: new Date().toISOString(),
          source: 'cli' as const,
          version: '1.0.0',
        },
        validations: {
          required_checks: [],
          skip_checks: [],
        },
      };

      const result = await adapter.validateSpec(spec);

      expect(result.valid).toBe(true);
      expect(result.issues).toBeUndefined();
    });

    it('should warn about empty changes', async () => {
      const spec = {
        id: 'aps-12345678',
        hash: '0'.repeat(64),
        intent: 'This is a valid intent for testing',
        schema_version: '0.1.0' as const,
        proposed_changes: [],
        provenance: {
          timestamp: new Date().toISOString(),
          source: 'cli' as const,
          version: '1.0.0',
        },
        validations: {
          required_checks: [],
          skip_checks: [],
        },
      };

      const result = await adapter.validateSpec(spec);

      expect(result.valid).toBe(true);
      expect(result.issues).toBeDefined();
      expect(result.issues?.[0].severity).toBe('warning');
      expect(result.issues?.[0].message).toContain('No changes');
    });
  });
});
