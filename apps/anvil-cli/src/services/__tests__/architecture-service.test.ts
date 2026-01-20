import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  formatEntryPoints,
  formatLayerDiagram,
  hasExistingBaseline,
  saveArchitectureBaseline,
  loadExistingBaseline,
  type ArchitectureSummary,
} from '../architecture-service.js';
import type { EntryPoint, Layers } from '@eddacraft/anvil-core';

// Mock core module
vi.mock('@eddacraft/anvil-core', () => ({
  createArchitectureAnalyzer: vi.fn(() => ({
    analyse: vi.fn(async () => ({
      entryPoints: [],
      layers: {},
      assignments: [],
    })),
  })),
  createBaselineManager: vi.fn(() => ({
    exists: vi.fn(() => false),
    load: vi.fn(() => null),
    save: vi.fn(),
  })),
  createBaseline: vi.fn(
    (data: { entryPoints?: unknown; layers?: unknown; moduleCount?: number }) => {
      const now = new Date().toISOString();

      return {
        schema_version: '0.1.0',
        created_at: now,
        updated_at: now,
        entry_points: data.entryPoints ?? [],
        layers: data.layers ?? {},
        boundaries: [],
        baseline_snapshot: {
          module_count: data.moduleCount ?? 0,
          timestamp: now,
          violations: [],
        },
      };
    }
  ),
}));

describe('architecture-service', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('formatEntryPoints', () => {
    it('should return placeholder message for empty entry points', () => {
      const result = formatEntryPoints([]);

      expect(result).toHaveLength(1);
      expect(result[0]).toContain('no entry points');
    });

    it('should format entry points grouped by type', () => {
      const entryPoints: EntryPoint[] = [
        {
          path: 'src/index.ts',
          type: 'cli',
          confidence: 'high',
        },
      ];

      const result = formatEntryPoints(entryPoints);
      const output = result.join('\n');

      // New format groups by type with header
      expect(output).toContain('CLI (1)');
      expect(output).toContain('src/index.ts');
    });

    it('should show confidence for non-high confidence entries', () => {
      const entryPoints: EntryPoint[] = [
        {
          path: 'src/main.ts',
          type: 'application',
          confidence: 'medium',
        },
      ];

      const result = formatEntryPoints(entryPoints);
      const output = result.join('\n');

      // New format uses [medium] notation
      expect(output).toContain('[medium]');
    });

    it('should not show confidence for high confidence entries', () => {
      const entryPoints: EntryPoint[] = [
        {
          path: 'src/app.ts',
          type: 'package',
          confidence: 'high',
        },
      ];

      const result = formatEntryPoints(entryPoints);
      const output = result.join('\n');

      expect(output).not.toContain('[high]');
      expect(output).toContain('src/app.ts');
    });

    it('should group multiple entry points by type', () => {
      const entryPoints: EntryPoint[] = [
        { path: 'src/index.ts', type: 'package', confidence: 'high' },
        { path: 'packages/core/index.ts', type: 'package', confidence: 'high' },
        { path: 'src/app.ts', type: 'application', confidence: 'high' },
      ];

      const result = formatEntryPoints(entryPoints);
      const output = result.join('\n');

      expect(output).toContain('Package (2)');
      expect(output).toContain('Application (1)');
    });

    it('should limit examples and show remaining count', () => {
      const entryPoints: EntryPoint[] = [
        { path: 'packages/a/index.ts', type: 'package', confidence: 'high' },
        { path: 'packages/b/index.ts', type: 'package', confidence: 'high' },
        { path: 'packages/c/index.ts', type: 'package', confidence: 'high' },
        { path: 'packages/d/index.ts', type: 'package', confidence: 'high' },
        { path: 'packages/e/index.ts', type: 'package', confidence: 'high' },
      ];

      const result = formatEntryPoints(entryPoints);
      const output = result.join('\n');

      // Should show 3 examples and "and 2 more"
      expect(output).toContain('and 2 more');
    });
  });

  describe('formatLayerDiagram', () => {
    it('should format empty layers', () => {
      const layers: Layers = {};
      const assignments = new Map<string, string[]>();

      const result = formatLayerDiagram(layers, assignments);

      expect(result).toBeDefined();
      expect(result.length).toBeGreaterThan(0);
      expect(result[0]).toContain('┌');
    });

    it('should format layers with file counts', () => {
      const layers: Layers = {
        domain: {
          patterns: ['*.domain.ts'],
          depends_on: [],
        },
      };
      const assignments = new Map([['domain', ['core.domain.ts']]]);

      const result = formatLayerDiagram(layers, assignments);

      expect(result.join('\n')).toContain('domain');
      expect(result.join('\n')).toContain('[1 files]');
    });

    it('should include box borders', () => {
      const layers: Layers = {
        infrastructure: {
          patterns: ['**/*.infra.ts'],
          depends_on: [],
        },
      };
      const assignments = new Map([['infrastructure', []]]);

      const result = formatLayerDiagram(layers, assignments);

      // Should have top border
      expect(result[0]).toContain('┌');
      // Should have bottom border
      expect(result[result.length - 1]).toContain('└');
    });
  });

  describe('baseline management', () => {
    let mockSummary: ArchitectureSummary;

    beforeEach(() => {
      mockSummary = {
        moduleCount: 10,
        entryPoints: [
          {
            path: 'src/index.ts',
            type: 'cli',
            confidence: 'high',
          },
        ],
        layers: {
          presentation: {
            patterns: ['**/*.ui.ts'],
            depends_on: [],
          },
        },
        layerAssignments: new Map([['presentation', ['file1.ui.ts']]]),
      };
    });

    it('should check if baseline exists', () => {
      const exists = hasExistingBaseline('/test/project');

      expect(typeof exists).toBe('boolean');
    });

    it('should load existing baseline', () => {
      const baseline = loadExistingBaseline('/test/project');

      // Returns null when not exists (due to mock)
      expect(baseline).toBeNull();
    });

    it('should save architecture baseline', () => {
      const baseline = saveArchitectureBaseline('/test/project', mockSummary);

      expect(baseline).toBeDefined();
      expect(baseline.schema_version).toBe('0.1.0');
      expect(baseline.created_at).toBeDefined();
      expect(baseline.baseline_snapshot.timestamp).toBeDefined();
      expect(baseline.baseline_snapshot.module_count).toBe(mockSummary.moduleCount);
    });
  });
});
