import { describe, expect, it, beforeEach } from 'vitest';
import * as path from 'node:path';
import * as fs from 'node:fs';
import * as os from 'node:os';

import {
  loadCompiledRegistry,
  loadRegistryPatterns,
  compiledToAntiPattern,
  resetRegistryCache,
} from './registry-loader.js';
import type { CompiledPattern } from './format/schemas.js';

function workspaceRegistryPath(): string {
  // The test runs from `packages/anvil/core/`; the compiled registry
  // lives at `<workspace-root>/patterns/compiled/registry.json`.
  return path.resolve(__dirname, '../../../../../patterns/compiled/registry.json');
}

describe('registry-loader', () => {
  beforeEach(() => {
    resetRegistryCache();
  });

  describe('loadCompiledRegistry', () => {
    it('loads the workspace registry when an explicit path is provided', () => {
      const result = loadCompiledRegistry({ registryPath: workspaceRegistryPath() });
      expect(result.registry).not.toBeNull();
      expect(result.registry!.patterns.length).toBeGreaterThan(0);
      expect(result.warnings).toEqual([]);
    });

    it('caches results for the same path', () => {
      const registryPath = workspaceRegistryPath();
      const first = loadCompiledRegistry({ registryPath });
      const second = loadCompiledRegistry({ registryPath });
      expect(first).toBe(second);
    });

    it('returns null registry when the file does not exist', () => {
      const result = loadCompiledRegistry({ registryPath: '/nonexistent/registry.json' });
      expect(result.registry).toBeNull();
      expect(result.warnings.length).toBeGreaterThan(0);
    });

    it('returns null registry when the file is malformed JSON', () => {
      const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'anvil-registry-'));
      const bad = path.join(tmp, 'registry.json');
      fs.writeFileSync(bad, '{ not json');
      try {
        const result = loadCompiledRegistry({ registryPath: bad });
        expect(result.registry).toBeNull();
        expect(result.warnings.join(' ')).toMatch(/Failed to read/);
      } finally {
        fs.rmSync(tmp, { recursive: true, force: true });
      }
    });

    it('returns null registry when the JSON fails schema validation', () => {
      const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'anvil-registry-'));
      const bad = path.join(tmp, 'registry.json');
      fs.writeFileSync(bad, JSON.stringify({ schema_version: 99 }));
      try {
        const result = loadCompiledRegistry({ registryPath: bad });
        expect(result.registry).toBeNull();
        expect(result.warnings.join(' ')).toMatch(/schema validation/);
      } finally {
        fs.rmSync(tmp, { recursive: true, force: true });
      }
    });
  });

  describe('compiledToAntiPattern', () => {
    const sample: CompiledPattern = {
      id: 'AP-001',
      family: 'guardrail-suppression',
      title: 'Broad eslint-disable',
      version: 1,
      severity: 'warning',
      confidence: 'high',
      spectrum_position: 1,
      targets: ['source'],
      detection: { type: 'regex', pattern: 'eslint-disable' },
      file_extensions: ['.ts', '.js'],
      allowlist: ['**/__tests__/**'],
      nudge: "Don't disable all rules.",
      related: [],
      enabled: true,
      opt_in: false,
      family_name: 'Guardrail Suppression',
      category: 'escape-hatch',
      explanation: 'Blanket disables hide real bugs.',
      suggestion: 'Disable just the failing rule instead.',
      definition_ref: 'patterns/guardrail-suppression/definition.anvil',
      tensions: [],
      related_families: [],
    };

    it('maps compiled fields onto AntiPattern shape', () => {
      const ap = compiledToAntiPattern(sample);
      expect(ap.id).toBe('AP-001');
      expect(ap.title).toBe('Broad eslint-disable');
      expect(ap.name).toBe('Broad eslint-disable');
      expect(ap.severity).toBe('warning');
      expect(ap.confidence).toBe('high');
      expect(ap.nudge).toBe("Don't disable all rules.");
      expect(ap.fileExtensions).toEqual(['.ts', '.js']);
      expect(ap.allowlist).toEqual(['**/__tests__/**']);
    });

    it('carries family provenance onto the AntiPattern', () => {
      const ap = compiledToAntiPattern(sample);
      expect(ap.family).toBe('guardrail-suppression');
      expect(ap.definitionRef).toBe('patterns/guardrail-suppression/definition.anvil');
      expect(ap.spectrumPosition).toBe(1);
      expect(ap.targets).toEqual(['source']);
    });

    it('omits allowlist when empty', () => {
      const ap = compiledToAntiPattern({ ...sample, allowlist: [] });
      expect(ap.allowlist).toBeUndefined();
    });

    it('omits fileExtensions when undefined', () => {
      const ap = compiledToAntiPattern({ ...sample, file_extensions: undefined });
      expect(ap.fileExtensions).toBeUndefined();
    });

    it('maps compiled category to AntiPattern category', () => {
      const ap = compiledToAntiPattern({ ...sample, category: 'accountability' });
      expect(ap.category).toBe('accountability');
    });

    it('maps ast_query to astQuery for AST detections', () => {
      const ap = compiledToAntiPattern({
        ...sample,
        detection: { type: 'ast', ast_query: 'MemberExpression[object.name="console"]' },
      });
      expect(ap.detection).toEqual({
        type: 'ast',
        astQuery: 'MemberExpression[object.name="console"]',
      });
    });
  });

  describe('loadRegistryPatterns', () => {
    it('returns the mapped AntiPattern[] when registry is found', () => {
      const patterns = loadRegistryPatterns({ registryPath: workspaceRegistryPath() });
      expect(patterns.length).toBeGreaterThan(0);
      const ap001 = patterns.find((p) => p.id === 'AP-001');
      expect(ap001?.family).toBe('guardrail-suppression');
    });

    it('returns [] when the registry cannot be loaded', () => {
      expect(loadRegistryPatterns({ registryPath: '/nonexistent/registry.json' })).toEqual([]);
    });
  });
});
