import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  formatEntryPoints,
  formatLayerDiagram,
  hasExistingBaseline,
  saveArchitectureBaseline,
  loadExistingBaseline,
  type ArchitectureSummary,
} from '../architecture-service.js';
import type { EntryPoint, Layers } from '@anvil/core';

// Mock core module
vi.mock('@anvil/core', () => ({
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
  createBaseline: vi.fn((data) => ({
    version: '1.0.0',
    timestamp: new Date().toISOString(),
    ...data,
  })),
}));

describe('architecture-service', () => {
  describe('formatEntryPoints', () => {
    it('should return placeholder message for empty entry points', () => {
      const result = formatEntryPoints([]);

      expect(result).toHaveLength(1);
      expect(result[0]).toContain('no entry points');
    });

    it('should format entry points with type', () => {
      const entryPoints: EntryPoint[] = [
        {
          path: 'src/index.ts',
          type: 'cli',
          confidence: 'high',
        },
      ];

      const result = formatEntryPoints(entryPoints);

      expect(result).toHaveLength(1);
      expect(result[0]).toContain('src/index.ts');
      expect(result[0]).toContain('cli');
    });

    it('should show confidence for non-high confidence entries', () => {
      const entryPoints: EntryPoint[] = [
        {
          path: 'src/main.ts',
          type: 'server',
          confidence: 'medium',
        },
      ];

      const result = formatEntryPoints(entryPoints);

      expect(result[0]).toContain('medium confidence');
    });

    it('should not show confidence for high confidence entries', () => {
      const entryPoints: EntryPoint[] = [
        {
          path: 'src/app.ts',
          type: 'web',
          confidence: 'high',
        },
      ];

      const result = formatEntryPoints(entryPoints);

      expect(result[0]).not.toContain('high confidence');
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
      expect(baseline.version).toBeDefined();
      expect(baseline.timestamp).toBeDefined();
    });
  });
});
