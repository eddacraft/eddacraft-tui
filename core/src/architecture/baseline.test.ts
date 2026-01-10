/**
 * Tests for baseline storage
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { existsSync, mkdirSync, writeFileSync, readFileSync, rmSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import {
  BASELINE_FILENAME,
  ANVIL_DIR,
  getBaselinePath,
  baselineExists,
  loadBaseline,
  saveBaseline,
  createBaseline,
  updateBaseline,
  mergeViolations,
  findNewViolations,
  findFixedViolations,
  BaselineManager,
  createBaselineManager,
} from './baseline.js';
import type { ArchitectureBaseline, BaselineViolation } from './types.js';

describe('Baseline Path Utilities', () => {
  describe('getBaselinePath', () => {
    it('should return correct path', () => {
      const path = getBaselinePath('/workspace');

      expect(path).toBe(`/workspace/${ANVIL_DIR}/${BASELINE_FILENAME}`);
    });

    it('should handle trailing slash', () => {
      const path = getBaselinePath('/workspace/');

      expect(path).toContain(ANVIL_DIR);
      expect(path).toContain(BASELINE_FILENAME);
    });
  });

  describe('baselineExists', () => {
    let testDir: string;

    beforeEach(() => {
      testDir = join(tmpdir(), `anvil-baseline-test-${Date.now()}`);
      mkdirSync(testDir, { recursive: true });
    });

    afterEach(() => {
      if (existsSync(testDir)) {
        rmSync(testDir, { recursive: true, force: true });
      }
    });

    it('should return false when baseline does not exist', () => {
      expect(baselineExists(testDir)).toBe(false);
    });

    it('should return true when baseline exists', () => {
      const anvilDir = join(testDir, ANVIL_DIR);
      mkdirSync(anvilDir, { recursive: true });
      writeFileSync(join(anvilDir, BASELINE_FILENAME), '{}');

      expect(baselineExists(testDir)).toBe(true);
    });
  });
});

describe('Baseline Load/Save', () => {
  let testDir: string;

  beforeEach(() => {
    testDir = join(tmpdir(), `anvil-baseline-test-${Date.now()}`);
    mkdirSync(testDir, { recursive: true });
  });

  afterEach(() => {
    if (existsSync(testDir)) {
      rmSync(testDir, { recursive: true, force: true });
    }
  });

  describe('loadBaseline', () => {
    it('should return null when baseline does not exist', () => {
      const result = loadBaseline(testDir);

      expect(result).toBeNull();
    });

    it('should load valid baseline', () => {
      const baseline = createBaseline({});
      const anvilDir = join(testDir, ANVIL_DIR);
      mkdirSync(anvilDir, { recursive: true });
      writeFileSync(join(anvilDir, BASELINE_FILENAME), JSON.stringify(baseline));

      const result = loadBaseline(testDir);

      expect(result).not.toBeNull();
      expect(result?.schema_version).toBe('0.1.0');
    });

    it('should return null for invalid JSON', () => {
      const anvilDir = join(testDir, ANVIL_DIR);
      mkdirSync(anvilDir, { recursive: true });
      writeFileSync(join(anvilDir, BASELINE_FILENAME), 'not valid json');

      const result = loadBaseline(testDir);

      expect(result).toBeNull();
    });

    it('should return null for invalid schema', () => {
      const anvilDir = join(testDir, ANVIL_DIR);
      mkdirSync(anvilDir, { recursive: true });
      writeFileSync(
        join(anvilDir, BASELINE_FILENAME),
        JSON.stringify({
          schema_version: '0.2.0', // Invalid version
        })
      );

      const result = loadBaseline(testDir);

      expect(result).toBeNull();
    });
  });

  describe('saveBaseline', () => {
    it('should create .anvil directory if not exists', () => {
      const baseline = createBaseline({});

      saveBaseline(testDir, baseline);

      expect(existsSync(join(testDir, ANVIL_DIR))).toBe(true);
    });

    it('should save baseline as formatted JSON', () => {
      const baseline = createBaseline({});

      saveBaseline(testDir, baseline);

      const content = readFileSync(getBaselinePath(testDir), 'utf-8');
      expect(content).toContain('\n'); // Should be formatted
      expect(content.endsWith('\n')).toBe(true); // Should end with newline
    });

    it('should throw for invalid baseline data', () => {
      const invalidBaseline = {
        schema_version: '0.2.0', // Invalid
      } as ArchitectureBaseline;

      expect(() => saveBaseline(testDir, invalidBaseline)).toThrow();
    });

    it('should overwrite existing baseline', () => {
      const baseline1 = createBaseline({ moduleCount: 100 });
      const baseline2 = createBaseline({ moduleCount: 200 });

      saveBaseline(testDir, baseline1);
      saveBaseline(testDir, baseline2);

      const loaded = loadBaseline(testDir);
      expect(loaded?.baseline_snapshot.module_count).toBe(200);
    });
  });
});

describe('Baseline Creation and Update', () => {
  describe('createBaseline', () => {
    it('should create baseline with defaults', () => {
      const baseline = createBaseline({});

      expect(baseline.schema_version).toBe('0.1.0');
      expect(baseline.entry_points).toEqual([]);
      expect(baseline.layers).toBeDefined();
      expect(baseline.boundaries.length).toBeGreaterThan(0);
      expect(baseline.baseline_snapshot.violations).toEqual([]);
    });

    it('should use provided entry points', () => {
      const entryPoints = [
        { path: 'src/index.ts', type: 'package' as const, confidence: 'high' as const },
      ];

      const baseline = createBaseline({ entryPoints });

      expect(baseline.entry_points).toEqual(entryPoints);
    });

    it('should use provided layers', () => {
      const layers = {
        custom: {
          patterns: ['src/custom/**'],
          depends_on: [],
        },
      };

      const baseline = createBaseline({ layers });

      expect(baseline.layers).toEqual(layers);
    });

    it('should use provided violations', () => {
      const violations: BaselineViolation[] = [
        {
          id: 'v-001',
          from_layer: 'presentation',
          to_layer: 'infrastructure',
          from_file: 'src/a.ts',
          to_file: 'src/b.ts',
          import_line: 5,
        },
      ];

      const baseline = createBaseline({ violations });

      expect(baseline.baseline_snapshot.violations).toEqual(violations);
    });

    it('should set timestamps', () => {
      const before = new Date().toISOString();
      const baseline = createBaseline({});
      const after = new Date().toISOString();

      expect(baseline.created_at >= before).toBe(true);
      expect(baseline.created_at <= after).toBe(true);
      expect(baseline.updated_at).toBe(baseline.created_at);
    });
  });

  describe('updateBaseline', () => {
    it('should update entry points', () => {
      vi.useFakeTimers();

      try {
        vi.setSystemTime(new Date('2020-01-01T00:00:00.000Z'));
        const original = createBaseline({});

        const newEntryPoints = [
          { path: 'src/new.ts', type: 'package' as const, confidence: 'high' as const },
        ];

        vi.setSystemTime(new Date('2020-01-01T00:00:01.000Z'));
        const updated = updateBaseline(original, { entryPoints: newEntryPoints });

        expect(updated.entry_points).toEqual(newEntryPoints);
        expect(updated.created_at).toBe(original.created_at);
        expect(updated.updated_at).not.toBe(original.updated_at);
      } finally {
        vi.useRealTimers();
      }
    });

    it('should update layers', () => {
      const original = createBaseline({});
      const newLayers = {
        newLayer: {
          patterns: ['src/new/**'],
          depends_on: [],
        },
      };

      const updated = updateBaseline(original, { layers: newLayers });

      expect(updated.layers).toEqual(newLayers);
    });

    it('should update violations', () => {
      const original = createBaseline({});
      const newViolations: BaselineViolation[] = [
        {
          id: 'v-new',
          from_layer: 'a',
          to_layer: 'b',
          from_file: 'a.ts',
          to_file: 'b.ts',
          import_line: 1,
        },
      ];

      const updated = updateBaseline(original, { violations: newViolations });

      expect(updated.baseline_snapshot.violations).toEqual(newViolations);
    });

    it('should preserve unchanged fields', () => {
      const original = createBaseline({
        entryPoints: [{ path: 'src/index.ts', type: 'package', confidence: 'high' }],
      });

      const updated = updateBaseline(original, { moduleCount: 500 });

      expect(updated.entry_points).toEqual(original.entry_points);
      expect(updated.baseline_snapshot.module_count).toBe(500);
    });
  });
});

describe('Violation Utilities', () => {
  describe('mergeViolations', () => {
    it('should merge violations by ID', () => {
      const existing: BaselineViolation[] = [
        {
          id: 'v-1',
          from_layer: 'a',
          to_layer: 'b',
          from_file: 'a.ts',
          to_file: 'b.ts',
          import_line: 1,
        },
        {
          id: 'v-2',
          from_layer: 'a',
          to_layer: 'c',
          from_file: 'a.ts',
          to_file: 'c.ts',
          import_line: 2,
        },
      ];

      const newViolations: BaselineViolation[] = [
        {
          id: 'v-2',
          from_layer: 'a',
          to_layer: 'c',
          from_file: 'a.ts',
          to_file: 'c.ts',
          import_line: 3,
        }, // Updated
        {
          id: 'v-3',
          from_layer: 'a',
          to_layer: 'd',
          from_file: 'a.ts',
          to_file: 'd.ts',
          import_line: 4,
        }, // New
      ];

      const merged = mergeViolations(existing, newViolations);

      expect(merged).toHaveLength(3);
      expect(merged.find((v) => v.id === 'v-1')).toBeDefined();
      expect(merged.find((v) => v.id === 'v-2')?.import_line).toBe(3); // Updated
      expect(merged.find((v) => v.id === 'v-3')).toBeDefined();
    });

    it('should handle empty arrays', () => {
      expect(mergeViolations([], [])).toEqual([]);
      expect(
        mergeViolations(
          [],
          [
            {
              id: 'v-1',
              from_layer: 'a',
              to_layer: 'b',
              from_file: 'a.ts',
              to_file: 'b.ts',
              import_line: 1,
            },
          ]
        )
      ).toHaveLength(1);
    });
  });

  describe('findNewViolations', () => {
    it('should find violations not in baseline', () => {
      const current: BaselineViolation[] = [
        {
          id: 'v-1',
          from_layer: 'a',
          to_layer: 'b',
          from_file: 'a.ts',
          to_file: 'b.ts',
          import_line: 1,
        },
        {
          id: 'v-2',
          from_layer: 'a',
          to_layer: 'c',
          from_file: 'a.ts',
          to_file: 'c.ts',
          import_line: 2,
        },
        {
          id: 'v-3',
          from_layer: 'a',
          to_layer: 'd',
          from_file: 'a.ts',
          to_file: 'd.ts',
          import_line: 3,
        },
      ];

      const baseline: BaselineViolation[] = [
        {
          id: 'v-1',
          from_layer: 'a',
          to_layer: 'b',
          from_file: 'a.ts',
          to_file: 'b.ts',
          import_line: 1,
        },
      ];

      const newViolations = findNewViolations(current, baseline);

      expect(newViolations).toHaveLength(2);
      expect(newViolations.map((v) => v.id)).toEqual(['v-2', 'v-3']);
    });

    it('should return empty if all in baseline', () => {
      const violations: BaselineViolation[] = [
        {
          id: 'v-1',
          from_layer: 'a',
          to_layer: 'b',
          from_file: 'a.ts',
          to_file: 'b.ts',
          import_line: 1,
        },
      ];

      const newViolations = findNewViolations(violations, violations);

      expect(newViolations).toHaveLength(0);
    });
  });

  describe('findFixedViolations', () => {
    it('should find violations in baseline but not current', () => {
      const current: BaselineViolation[] = [
        {
          id: 'v-1',
          from_layer: 'a',
          to_layer: 'b',
          from_file: 'a.ts',
          to_file: 'b.ts',
          import_line: 1,
        },
      ];

      const baseline: BaselineViolation[] = [
        {
          id: 'v-1',
          from_layer: 'a',
          to_layer: 'b',
          from_file: 'a.ts',
          to_file: 'b.ts',
          import_line: 1,
        },
        {
          id: 'v-2',
          from_layer: 'a',
          to_layer: 'c',
          from_file: 'a.ts',
          to_file: 'c.ts',
          import_line: 2,
        },
        {
          id: 'v-3',
          from_layer: 'a',
          to_layer: 'd',
          from_file: 'a.ts',
          to_file: 'd.ts',
          import_line: 3,
        },
      ];

      const fixed = findFixedViolations(current, baseline);

      expect(fixed).toHaveLength(2);
      expect(fixed.map((v) => v.id)).toEqual(['v-2', 'v-3']);
    });

    it('should return empty if none fixed', () => {
      const violations: BaselineViolation[] = [
        {
          id: 'v-1',
          from_layer: 'a',
          to_layer: 'b',
          from_file: 'a.ts',
          to_file: 'b.ts',
          import_line: 1,
        },
      ];

      const fixed = findFixedViolations(violations, violations);

      expect(fixed).toHaveLength(0);
    });
  });
});

describe('BaselineManager', () => {
  let testDir: string;
  let manager: BaselineManager;

  beforeEach(() => {
    testDir = join(tmpdir(), `anvil-baseline-test-${Date.now()}`);
    mkdirSync(testDir, { recursive: true });
    manager = createBaselineManager(testDir);
  });

  afterEach(() => {
    if (existsSync(testDir)) {
      rmSync(testDir, { recursive: true, force: true });
    }
  });

  describe('exists', () => {
    it('should return false when no baseline', () => {
      expect(manager.exists()).toBe(false);
    });

    it('should return true after creating baseline', () => {
      manager.create({});

      expect(manager.exists()).toBe(true);
    });
  });

  describe('load', () => {
    it('should return null when no baseline', () => {
      expect(manager.load()).toBeNull();
    });

    it('should cache loaded baseline', () => {
      manager.create({});

      const first = manager.load();
      const second = manager.load();

      expect(first).toBe(second); // Same reference (cached)
    });
  });

  describe('reload', () => {
    it('should reload from disk', () => {
      manager.create({ moduleCount: 100 });
      manager.load(); // Cache it

      // Modify on disk
      const baseline = loadBaseline(testDir)!;
      baseline.baseline_snapshot.module_count = 200;
      saveBaseline(testDir, baseline);

      const reloaded = manager.reload();

      expect(reloaded?.baseline_snapshot.module_count).toBe(200);
    });
  });

  describe('create', () => {
    it('should create and save baseline', () => {
      const baseline = manager.create({ moduleCount: 150 });

      expect(baseline.baseline_snapshot.module_count).toBe(150);
      expect(manager.exists()).toBe(true);
    });
  });

  describe('update', () => {
    it('should update existing baseline', () => {
      manager.create({ moduleCount: 100 });

      const updated = manager.update({ moduleCount: 200 });

      expect(updated?.baseline_snapshot.module_count).toBe(200);
    });

    it('should return null if no baseline exists', () => {
      const result = manager.update({ moduleCount: 100 });

      expect(result).toBeNull();
    });
  });

  describe('getLayers', () => {
    it('should return layers from baseline', () => {
      const customLayers = {
        custom: { patterns: ['src/custom/**'], depends_on: [] },
      };
      manager.create({ layers: customLayers });

      const layers = manager.getLayers();

      expect(layers).toEqual(customLayers);
    });

    it('should return defaults if no baseline', () => {
      const layers = manager.getLayers();

      expect(layers).toHaveProperty('presentation');
      expect(layers).toHaveProperty('application');
    });
  });

  describe('getBoundaries', () => {
    it('should return boundaries from baseline', () => {
      manager.create({});

      const boundaries = manager.getBoundaries();

      expect(boundaries.length).toBeGreaterThan(0);
    });
  });

  describe('isNewViolation', () => {
    it('should return true for new violations', () => {
      manager.create({ violations: [] });

      const violation: BaselineViolation = {
        id: 'new-violation',
        from_layer: 'a',
        to_layer: 'b',
        from_file: 'a.ts',
        to_file: 'b.ts',
        import_line: 1,
      };

      expect(manager.isNewViolation(violation)).toBe(true);
    });

    it('should return false for existing violations', () => {
      const violation: BaselineViolation = {
        id: 'existing-violation',
        from_layer: 'a',
        to_layer: 'b',
        from_file: 'a.ts',
        to_file: 'b.ts',
        import_line: 1,
      };

      manager.create({ violations: [violation] });

      expect(manager.isNewViolation(violation)).toBe(false);
    });

    it('should return true if no baseline', () => {
      const violation: BaselineViolation = {
        id: 'any-violation',
        from_layer: 'a',
        to_layer: 'b',
        from_file: 'a.ts',
        to_file: 'b.ts',
        import_line: 1,
      };

      expect(manager.isNewViolation(violation)).toBe(true);
    });
  });

  describe('getPath', () => {
    it('should return baseline path', () => {
      const path = manager.getPath();

      expect(path).toBe(getBaselinePath(testDir));
    });
  });
});

describe('createBaselineManager', () => {
  it('should create manager with workspace root', () => {
    const manager = createBaselineManager('/test/workspace');

    expect(manager).toBeInstanceOf(BaselineManager);
  });
});
