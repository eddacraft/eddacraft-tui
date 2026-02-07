import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  formatEntryPoints,
  formatEntryPointsSummary,
  formatLayerDiagram,
  layersToMermaid,
  generateArchitectureExplanation,
  formatArchitectureExplanation,
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

      // New format groups by type with header and count
      expect(output).toContain('[CLI]');
      expect(output).toContain('CLI Tools: 1');
      expect(output).toContain('index');
    });

    it('should show confidence for non-high confidence entries in verbose mode', () => {
      const entryPoints: EntryPoint[] = [
        {
          path: 'src/main.ts',
          type: 'application',
          confidence: 'medium',
        },
      ];

      // Verbose mode shows detailed examples with confidence
      const result = formatEntryPoints(entryPoints, { verbose: true });
      const output = result.join('\n');

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

      const result = formatEntryPoints(entryPoints, { verbose: true });
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

      // New format shows "Packages: 2" and "Applications: 1"
      expect(output).toContain('Packages: 2');
      expect(output).toContain('Applications: 1');
    });

    it('should show truncated examples with ... for many entries', () => {
      const entryPoints: EntryPoint[] = [
        { path: 'packages/a/index.ts', type: 'package', confidence: 'high' },
        { path: 'packages/b/index.ts', type: 'package', confidence: 'high' },
        { path: 'packages/c/index.ts', type: 'package', confidence: 'high' },
        { path: 'packages/d/index.ts', type: 'package', confidence: 'high' },
        { path: 'packages/e/index.ts', type: 'package', confidence: 'high' },
      ];

      const result = formatEntryPoints(entryPoints);
      const output = result.join('\n');

      // Summary line shows first 3 examples plus "..."
      expect(output).toContain('...');
    });
  });

  describe('formatEntryPointsSummary', () => {
    it('should return placeholder for empty entry points', () => {
      const result = formatEntryPointsSummary([]);

      expect(result).toContain('No entry points');
    });

    it('should show total count and type breakdown', () => {
      const entryPoints: EntryPoint[] = [
        { path: 'src/index.ts', type: 'package', confidence: 'high' },
        { path: 'packages/core/index.ts', type: 'package', confidence: 'high' },
        { path: 'src/app.ts', type: 'application', confidence: 'high' },
        { path: 'src/cli.ts', type: 'cli', confidence: 'high' },
      ];

      const result = formatEntryPointsSummary(entryPoints);

      expect(result).toContain('Entry Points (4 total)');
      expect(result).toContain('2 packages');
      expect(result).toContain('1 applications');
      expect(result).toContain('1 cli tools');
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
      expect(result.join('\n')).toContain('[1 file]');
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

  describe('generateArchitectureExplanation', () => {
    it('should detect monorepo structure with workspace details', () => {
      const summary: ArchitectureSummary = {
        moduleCount: 50,
        entryPoints: [
          { path: 'apps/web/src/index.ts', type: 'application', confidence: 'high' },
          { path: 'apps/api/src/index.ts', type: 'application', confidence: 'high' },
          { path: 'packages/core/src/index.ts', type: 'package', confidence: 'high' },
          { path: 'packages/utils/src/index.ts', type: 'package', confidence: 'high' },
        ],
        layers: {},
        layerAssignments: new Map(),
      };

      const result = generateArchitectureExplanation(summary);

      expect(result.structure).toBe('monorepo');
      expect(result.workspaceDetails).toBeDefined();
      expect(result.workspaceDetails?.appsCount).toBe(2);
      expect(result.workspaceDetails?.packagesCount).toBe(2);
      expect(result.recommendedTemplate).toBe('monorepo');
      expect(result.patternDisplayName).toContain('Monorepo');
    });

    it('should detect single-app with layered architecture', () => {
      const summary: ArchitectureSummary = {
        moduleCount: 30,
        entryPoints: [{ path: 'src/index.ts', type: 'application', confidence: 'high' }],
        layers: {
          presentation: { patterns: ['**/controllers/**'], depends_on: ['application'] },
          application: { patterns: ['**/services/**'], depends_on: ['domain'] },
          domain: { patterns: ['**/domain/**'], depends_on: [] },
        },
        layerAssignments: new Map([
          ['presentation', ['src/controllers/user.ts', 'src/controllers/auth.ts']],
          ['application', ['src/services/user.ts', 'src/services/auth.ts']],
          ['domain', ['src/domain/user.ts', 'src/domain/auth.ts']],
        ]),
      };

      const result = generateArchitectureExplanation(summary);

      expect(result.structure).toBe('single-app');
      expect(result.pattern).toBe('layered');
      expect(result.detectedLayers).toContain('presentation');
      expect(result.detectedLayers).toContain('application');
      expect(result.detectedLayers).toContain('domain');
      expect(result.recommendedTemplate).toBe('layered');
    });

    it('should detect library structure', () => {
      const summary: ArchitectureSummary = {
        moduleCount: 20,
        entryPoints: [{ path: 'src/index.ts', type: 'package', confidence: 'high' }],
        layers: {},
        layerAssignments: new Map(),
      };

      const result = generateArchitectureExplanation(summary);

      expect(result.structure).toBe('library');
      expect(result.recommendedTemplate).toBe('starter');
    });

    it('should include confidence level', () => {
      const summary: ArchitectureSummary = {
        moduleCount: 100,
        entryPoints: [{ path: 'src/index.ts', type: 'application', confidence: 'high' }],
        layers: {
          shared: { patterns: ['**/utils/**'], depends_on: [] },
        },
        layerAssignments: new Map([
          ['shared', Array(60).fill('file.ts')], // 60% coverage
        ]),
      };

      const result = generateArchitectureExplanation(summary);

      // Should have at least medium confidence with 60% layer coverage
      expect(['high', 'medium']).toContain(result.confidence);
    });

    it('should provide actionable next steps', () => {
      const summary: ArchitectureSummary = {
        moduleCount: 10,
        entryPoints: [],
        layers: {},
        layerAssignments: new Map(),
      };

      const result = generateArchitectureExplanation(summary);

      expect(result.nextSteps).toBeDefined();
      expect(result.nextSteps.length).toBeGreaterThan(0);
    });

    it('should detect Nx-style workspace with libs directory', () => {
      const summary: ArchitectureSummary = {
        moduleCount: 100,
        entryPoints: [
          { path: 'apps/web/src/index.ts', type: 'application', confidence: 'high' },
          { path: 'libs/shared/src/index.ts', type: 'package', confidence: 'high' },
          { path: 'libs/ui/src/index.ts', type: 'package', confidence: 'high' },
        ],
        layers: {},
        layerAssignments: new Map(),
      };

      const result = generateArchitectureExplanation(summary);

      expect(result.structure).toBe('monorepo');
      expect(result.workspaceDetails?.libsCount).toBe(2);
      expect(result.recommendedTemplate).toBe('nx-workspace');
    });
  });

  describe('formatArchitectureExplanation', () => {
    it('should format explanation with all sections', () => {
      const summary: ArchitectureSummary = {
        moduleCount: 50,
        entryPoints: [
          { path: 'apps/web/src/index.ts', type: 'application', confidence: 'high' },
          { path: 'packages/core/src/index.ts', type: 'package', confidence: 'high' },
        ],
        layers: {},
        layerAssignments: new Map(),
      };

      const explanation = generateArchitectureExplanation(summary);
      const lines = formatArchitectureExplanation(explanation);
      const output = lines.join('\n');

      expect(output).toContain('Architecture Analysis:');
      expect(output).toContain('Pattern:');
      expect(output).toContain('Confidence:');
      expect(output).toContain('Recommended Template:');
      expect(output).toContain('Next Steps:');
    });

    it('should include workspace details for monorepos', () => {
      const summary: ArchitectureSummary = {
        moduleCount: 100,
        entryPoints: [
          { path: 'apps/web/src/index.ts', type: 'application', confidence: 'high' },
          { path: 'apps/api/src/index.ts', type: 'application', confidence: 'high' },
          { path: 'packages/core/src/index.ts', type: 'package', confidence: 'high' },
        ],
        layers: {},
        layerAssignments: new Map(),
      };

      const explanation = generateArchitectureExplanation(summary);
      const lines = formatArchitectureExplanation(explanation);
      const output = lines.join('\n');

      expect(output).toContain('Structure:');
      expect(output).toContain('2 apps');
      expect(output).toContain('1 packages');
    });

    it('should display confidence indicator', () => {
      const summary: ArchitectureSummary = {
        moduleCount: 10,
        entryPoints: [],
        layers: {},
        layerAssignments: new Map(),
      };

      const explanation = generateArchitectureExplanation(summary);
      const lines = formatArchitectureExplanation(explanation);
      const output = lines.join('\n');

      // Should contain a confidence indicator like [***], [** ], or [*  ]
      expect(output).toMatch(/\[\*{1,3}\s*\]/);
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

  describe('layersToMermaid', () => {
    it('should generate mermaid definition from layers', () => {
      const layers: Layers = {
        presentation: { patterns: ['src/ui/**'], depends_on: ['application'] },
        application: { patterns: ['src/app/**'], depends_on: ['domain'] },
        domain: { patterns: ['src/domain/**'], depends_on: [] },
      };

      const result = layersToMermaid(layers);
      expect(result).toContain('graph TD');
      expect(result).toContain('presentation --> application');
      expect(result).toContain('application --> domain');
    });

    it('should include file counts in node labels when assignments provided', () => {
      const layers: Layers = {
        domain: { patterns: ['src/domain/**'], depends_on: [] },
        shared: { patterns: ['src/shared/**'], depends_on: [] },
      };
      const assignments = new Map([
        ['domain', ['a.ts', 'b.ts', 'c.ts']],
        ['shared', ['x.ts']],
      ]);

      const result = layersToMermaid(layers, assignments);
      expect(result).toContain('domain["domain (3 files)"]');
      expect(result).toContain('shared["shared (1 file)"]');
    });

    it('should not add edges to non-existent layers', () => {
      const layers: Layers = {
        app: { patterns: ['src/app/**'], depends_on: ['missing_layer'] },
      };

      const result = layersToMermaid(layers);
      expect(result).toBe('graph TD');
    });

    it('should not produce duplicate edges', () => {
      const layers: Layers = {
        a: { patterns: ['a/**'], depends_on: ['b'] },
        b: { patterns: ['b/**'], depends_on: [] },
      };

      const result = layersToMermaid(layers);
      const edges = result.split('\n').filter((l) => l.includes('-->'));
      expect(edges).toHaveLength(1);
    });
  });
});
