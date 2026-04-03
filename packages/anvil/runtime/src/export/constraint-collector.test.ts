/**
 * Tests for constraint collector
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import {
  ConstraintCollector,
  collectConstraints,
  hasAnyConstraints,
  countConstraints,
  type Constraints,
} from './constraint-collector.js';
import { ANVIL_DIR, BASELINE_FILENAME } from '@eddacraft/anvil-core/architecture';
import type { ArchitectureBaseline } from '@eddacraft/anvil-core/architecture';
import { PATTERNS } from '@eddacraft/anvil-core/antipattern';
import { safeCleanup } from '../../../../../tools/test-utils/safe-cleanup.js';

describe('ConstraintCollector', () => {
  let testDir: string;

  beforeEach(() => {
    testDir = mkdtempSync(join(tmpdir(), 'anvil-constraint-test-'));
  });

  afterEach(async () => {
    await safeCleanup(testDir);
  });

  describe('without baseline', () => {
    it('should collect constraints without architecture baseline', async () => {
      const collector = new ConstraintCollector({ workspaceRoot: testDir });
      const constraints = await collector.collect();

      expect(constraints.metadata.hasBaseline).toBe(false);
      expect(constraints.metadata.workspaceRoot).toBe(testDir);
      expect(constraints.boundaries).toHaveLength(0);
      expect(constraints.layers).toHaveLength(0);
      expect(constraints.antiPatterns.length).toBeGreaterThan(0);
      expect(constraints.conventions.length).toBeGreaterThan(0);
    });

    it('should include metadata with collection timestamp', async () => {
      const collector = new ConstraintCollector({ workspaceRoot: testDir });
      const constraints = await collector.collect();

      expect(constraints.metadata.collectedAt).toBeDefined();
      const timestamp = new Date(constraints.metadata.collectedAt);
      expect(timestamp.getTime()).toBeLessThanOrEqual(Date.now());
    });
  });

  describe('with baseline', () => {
    beforeEach(() => {
      // Create a baseline
      const anvilDir = join(testDir, ANVIL_DIR);
      mkdirSync(anvilDir, { recursive: true });

      const baseline: ArchitectureBaseline = {
        schema_version: '0.1.0',
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        entry_points: [
          {
            path: 'src/index.ts',
            type: 'package',
            confidence: 'high',
            exports: ['main'],
          },
        ],
        layers: {
          presentation: {
            patterns: ['src/api/**'],
            depends_on: ['application'],
            description: 'API layer',
          },
          application: {
            patterns: ['src/services/**'],
            depends_on: ['domain'],
            description: 'Business logic',
          },
          domain: {
            patterns: ['src/domain/**'],
            depends_on: [],
            description: 'Domain models',
          },
        },
        boundaries: [
          {
            name: 'no-presentation-to-domain',
            from: 'presentation',
            to: 'domain',
            severity: 'error',
            message: 'Presentation layer must not directly access domain',
          },
          {
            name: 'no-domain-to-application',
            from: 'domain',
            to: 'application',
            severity: 'warning',
            message: 'Domain should not depend on application',
          },
        ],
        baseline_snapshot: {
          module_count: 10,
          timestamp: new Date().toISOString(),
          violations: [],
        },
      };

      writeFileSync(join(anvilDir, BASELINE_FILENAME), JSON.stringify(baseline, null, 2));
    });

    it('should collect architecture boundaries from baseline', async () => {
      const collector = new ConstraintCollector({ workspaceRoot: testDir });
      const constraints = await collector.collect();

      expect(constraints.metadata.hasBaseline).toBe(true);
      expect(constraints.boundaries).toHaveLength(2);

      const boundary1 = constraints.boundaries[0];
      expect(boundary1.name).toBe('no-presentation-to-domain');
      expect(boundary1.from).toBe('presentation');
      expect(boundary1.to).toBe('domain');
      expect(boundary1.severity).toBe('error');
      expect(boundary1.message).toBe('Presentation layer must not directly access domain');

      const boundary2 = constraints.boundaries[1];
      expect(boundary2.name).toBe('no-domain-to-application');
      expect(boundary2.from).toBe('domain');
      expect(boundary2.to).toBe('application');
      expect(boundary2.severity).toBe('warning');
    });

    it('should collect layer definitions from baseline', async () => {
      const collector = new ConstraintCollector({ workspaceRoot: testDir });
      const constraints = await collector.collect();

      expect(constraints.layers).toHaveLength(3);

      const presentationLayer = constraints.layers.find((l) => l.name === 'presentation');
      expect(presentationLayer).toBeDefined();
      expect(presentationLayer?.patterns).toEqual(['src/api/**']);
      expect(presentationLayer?.dependsOn).toEqual(['application']);
      expect(presentationLayer?.description).toBe('API layer');

      const domainLayer = constraints.layers.find((l) => l.name === 'domain');
      expect(domainLayer).toBeDefined();
      expect(domainLayer?.patterns).toEqual(['src/domain/**']);
      expect(domainLayer?.dependsOn).toEqual([]);
    });
  });

  describe('with invalid baseline', () => {
    it('should report hasBaseline as false when baseline is invalid JSON', async () => {
      const anvilDir = join(testDir, ANVIL_DIR);
      mkdirSync(anvilDir, { recursive: true });
      writeFileSync(join(anvilDir, BASELINE_FILENAME), 'not valid json {{{');

      const collector = new ConstraintCollector({ workspaceRoot: testDir });
      const constraints = await collector.collect();

      expect(constraints.metadata.hasBaseline).toBe(false);
      expect(constraints.boundaries).toHaveLength(0);
      expect(constraints.layers).toHaveLength(0);
    });

    it('should report hasBaseline as false when baseline fails schema validation', async () => {
      const anvilDir = join(testDir, ANVIL_DIR);
      mkdirSync(anvilDir, { recursive: true });

      // Valid JSON but invalid schema (missing required fields)
      const invalidBaseline = {
        schema_version: '0.1.0',
        // Missing required fields: created_at, updated_at, entry_points, layers, boundaries, baseline_snapshot
      };
      writeFileSync(join(anvilDir, BASELINE_FILENAME), JSON.stringify(invalidBaseline, null, 2));

      const collector = new ConstraintCollector({ workspaceRoot: testDir });
      const constraints = await collector.collect();

      expect(constraints.metadata.hasBaseline).toBe(false);
      expect(constraints.boundaries).toHaveLength(0);
      expect(constraints.layers).toHaveLength(0);
    });
  });

  describe('anti-pattern collection', () => {
    it('should collect default anti-patterns', async () => {
      const collector = new ConstraintCollector({ workspaceRoot: testDir });
      const constraints = await collector.collect();

      // By default, should include enabled patterns that are not opt-in
      const defaultPatterns = PATTERNS.filter((p) => p.enabled && !p.optIn);
      expect(constraints.antiPatterns.length).toBe(defaultPatterns.length);

      // Check that each collected pattern has required fields
      for (const pattern of constraints.antiPatterns) {
        expect(pattern.id).toMatch(/^AP-\d{3}$/);
        expect(pattern.name).toBeTruthy();
        expect(pattern.category).toBeTruthy();
        expect(pattern.explanation).toBeTruthy();
        expect(pattern.suggestion).toBeTruthy();
        expect(['error', 'warning', 'info']).toContain(pattern.severity);
        expect(pattern.enabled).toBe(true);
      }
    });

    it('should include opt-in patterns when configured', async () => {
      const collector = new ConstraintCollector({
        workspaceRoot: testDir,
        includeOptInPatterns: true,
      });
      const constraints = await collector.collect();

      // Should include all enabled patterns, including opt-in ones
      const enabledPatterns = PATTERNS.filter((p) => p.enabled);
      expect(constraints.antiPatterns.length).toBe(enabledPatterns.length);

      // Check that opt-in patterns are included
      const optInPatternIds = PATTERNS.filter((p) => p.optIn).map((p) => p.id);
      const collectedIds = constraints.antiPatterns.map((p) => p.id);
      for (const id of optInPatternIds) {
        expect(collectedIds).toContain(id);
      }
    });

    it('should include disabled patterns when configured', async () => {
      const collector = new ConstraintCollector({
        workspaceRoot: testDir,
        includeDisabledPatterns: true,
        includeOptInPatterns: true,
      });
      const constraints = await collector.collect();

      // Should include all patterns regardless of enabled status
      expect(constraints.antiPatterns.length).toBe(PATTERNS.length);
    });

    it('should map anti-pattern fields correctly', async () => {
      const collector = new ConstraintCollector({ workspaceRoot: testDir });
      const constraints = await collector.collect();

      const ap001 = constraints.antiPatterns.find((p) => p.id === 'AP-001');
      expect(ap001).toBeDefined();
      expect(ap001?.name).toBe('Broad eslint-disable');
      expect(ap001?.category).toBe('escape-hatch');
      expect(ap001?.severity).toBe('warning');
      expect(ap001?.explanation).toContain('Disabling all ESLint rules');
      expect(ap001?.suggestion).toContain('Disable specific rules');
    });
  });

  describe('convention collection', () => {
    it('should collect project conventions', async () => {
      const collector = new ConstraintCollector({ workspaceRoot: testDir });
      const constraints = await collector.collect();

      expect(constraints.conventions.length).toBeGreaterThan(0);

      // Check for expected convention categories
      const categories = constraints.conventions.map((c) => c.category);
      expect(categories).toContain('spelling');
      expect(categories).toContain('imports');
      expect(categories).toContain('schemas');
      expect(categories).toContain('naming');
      expect(categories).toContain('type-safety');
    });

    it('should include convention descriptions and examples', async () => {
      const collector = new ConstraintCollector({ workspaceRoot: testDir });
      const constraints = await collector.collect();

      const spellingConvention = constraints.conventions.find((c) => c.category === 'spelling');
      expect(spellingConvention).toBeDefined();
      expect(spellingConvention?.description).toBe('Use UK English spelling');
      expect(spellingConvention?.examples).toBeDefined();
      expect(spellingConvention?.examples?.length).toBeGreaterThan(0);
    });
  });

  describe('suppression collection', () => {
    it('should return empty suppressions when no store exists', async () => {
      const collector = new ConstraintCollector({ workspaceRoot: testDir });
      const constraints = await collector.collect();

      expect(constraints.suppressions).toHaveLength(0);
    });

    it('should collect active suppressions from store', async () => {
      const anvilDir = join(testDir, ANVIL_DIR);
      mkdirSync(anvilDir, { recursive: true });

      const storeData = {
        version: 1,
        suppressions: [
          {
            id: 'src/legacy.ts:10:AP-003',
            pattern_id: 'AP-003',
            file: 'src/legacy.ts',
            line: 10,
            reason: 'Legacy code not yet migrated',
            timestamp: '2024-01-15T10:00:00.000Z',
            scope: 'file',
          },
          {
            id: 'src/api.ts:25:AP-001',
            pattern_id: 'AP-001',
            file: 'src/api.ts',
            line: 25,
            reason: 'Third-party integration requires broad disable',
            timestamp: '2024-01-15T10:00:00.000Z',
            scope: 'statement',
          },
        ],
        lastUpdated: '2024-01-15T10:00:00.000Z',
      };

      writeFileSync(join(anvilDir, 'suppressions.json'), JSON.stringify(storeData, null, 2));

      const collector = new ConstraintCollector({ workspaceRoot: testDir });
      const constraints = await collector.collect();

      expect(constraints.suppressions).toHaveLength(2);

      const first = constraints.suppressions[0];
      expect(first.patternId).toBe('AP-003');
      expect(first.file).toBe('src/legacy.ts');
      expect(first.scope).toBe('file');
      expect(first.reason).toBe('Legacy code not yet migrated');
    });

    it('should filter out expired suppressions', async () => {
      const anvilDir = join(testDir, ANVIL_DIR);
      mkdirSync(anvilDir, { recursive: true });

      const pastDate = new Date(Date.now() - 86400000).toISOString(); // yesterday
      const futureDate = new Date(Date.now() + 86400000 * 30).toISOString(); // 30 days from now

      const storeData = {
        version: 1,
        suppressions: [
          {
            id: 'src/old.ts:5:AP-001',
            pattern_id: 'AP-001',
            file: 'src/old.ts',
            line: 5,
            reason: 'Expired suppression',
            timestamp: '2024-01-01T00:00:00.000Z',
            scope: 'line',
            expires_at: pastDate,
          },
          {
            id: 'src/new.ts:10:AP-002',
            pattern_id: 'AP-002',
            file: 'src/new.ts',
            line: 10,
            reason: 'Active time-boxed suppression',
            timestamp: '2024-01-15T00:00:00.000Z',
            scope: 'statement',
            expires_at: futureDate,
          },
          {
            id: 'src/permanent.ts:1:AP-003',
            pattern_id: 'AP-003',
            file: 'src/permanent.ts',
            line: 1,
            reason: 'Permanent suppression',
            timestamp: '2024-01-15T00:00:00.000Z',
            scope: 'file',
          },
        ],
        lastUpdated: '2024-01-15T00:00:00.000Z',
      };

      writeFileSync(join(anvilDir, 'suppressions.json'), JSON.stringify(storeData, null, 2));

      const collector = new ConstraintCollector({ workspaceRoot: testDir });
      const constraints = await collector.collect();

      // Should have 2 active (future + permanent), not the expired one
      expect(constraints.suppressions).toHaveLength(2);
      expect(constraints.suppressions.map((s) => s.file)).toEqual(
        expect.arrayContaining(['src/new.ts', 'src/permanent.ts'])
      );
      expect(constraints.suppressions.map((s) => s.file)).not.toContain('src/old.ts');
    });

    it('should include expiresAt for time-boxed suppressions', async () => {
      const anvilDir = join(testDir, ANVIL_DIR);
      mkdirSync(anvilDir, { recursive: true });

      const futureDate = new Date(Date.now() + 86400000 * 30).toISOString();

      const storeData = {
        version: 1,
        suppressions: [
          {
            id: 'src/temp.ts:5:AP-001',
            pattern_id: 'AP-001',
            file: 'src/temp.ts',
            line: 5,
            reason: 'Temporary exception',
            timestamp: '2024-01-15T00:00:00.000Z',
            scope: 'line',
            expires_at: futureDate,
          },
        ],
        lastUpdated: '2024-01-15T00:00:00.000Z',
      };

      writeFileSync(join(anvilDir, 'suppressions.json'), JSON.stringify(storeData, null, 2));

      const collector = new ConstraintCollector({ workspaceRoot: testDir });
      const constraints = await collector.collect();

      expect(constraints.suppressions).toHaveLength(1);
      expect(constraints.suppressions[0].expiresAt).toBe(futureDate);
    });
  });
});

describe('collectConstraints', () => {
  let testDir: string;

  beforeEach(() => {
    testDir = mkdtempSync(join(tmpdir(), 'anvil-constraint-test-'));
  });

  afterEach(async () => {
    await safeCleanup(testDir);
  });

  it('should collect constraints with default configuration', async () => {
    const constraints = await collectConstraints(testDir);

    expect(constraints).toBeDefined();
    expect(constraints.metadata.workspaceRoot).toBe(testDir);
    expect(constraints.antiPatterns.length).toBeGreaterThan(0);
    expect(constraints.conventions.length).toBeGreaterThan(0);
  });
});

describe('hasAnyConstraints', () => {
  it('should return false for empty constraints', () => {
    const constraints: Constraints = {
      boundaries: [],
      layers: [],
      antiPatterns: [],
      conventions: [],
      suppressions: [],
      metadata: {
        collectedAt: new Date().toISOString(),
        workspaceRoot: '/test',
        hasBaseline: false,
      },
    };

    expect(hasAnyConstraints(constraints)).toBe(false);
  });

  it('should return true when boundaries exist', () => {
    const constraints: Constraints = {
      boundaries: [
        {
          name: 'test',
          from: 'a',
          to: 'b',
          message: 'test',
          severity: 'error',
        },
      ],
      layers: [],
      antiPatterns: [],
      conventions: [],
      suppressions: [],
      metadata: {
        collectedAt: new Date().toISOString(),
        workspaceRoot: '/test',
        hasBaseline: false,
      },
    };

    expect(hasAnyConstraints(constraints)).toBe(true);
  });

  it('should return true when anti-patterns exist', () => {
    const constraints: Constraints = {
      boundaries: [],
      layers: [],
      antiPatterns: [
        {
          id: 'AP-001',
          name: 'Test',
          category: 'test',
          explanation: 'test',
          suggestion: 'test',
          severity: 'warning',
          enabled: true,
        },
      ],
      conventions: [],
      suppressions: [],
      metadata: {
        collectedAt: new Date().toISOString(),
        workspaceRoot: '/test',
        hasBaseline: false,
      },
    };

    expect(hasAnyConstraints(constraints)).toBe(true);
  });

  it('should return true when suppressions exist', () => {
    const constraints: Constraints = {
      boundaries: [],
      layers: [],
      antiPatterns: [],
      conventions: [],
      suppressions: [
        {
          patternId: 'AP-001',
          file: 'src/legacy.ts',
          scope: 'file',
          reason: 'Legacy code migration',
        },
      ],
      metadata: {
        collectedAt: new Date().toISOString(),
        workspaceRoot: '/test',
        hasBaseline: false,
      },
    };

    expect(hasAnyConstraints(constraints)).toBe(true);
  });
});

describe('countConstraints', () => {
  it('should count zero constraints', () => {
    const constraints: Constraints = {
      boundaries: [],
      layers: [],
      antiPatterns: [],
      conventions: [],
      suppressions: [],
      metadata: {
        collectedAt: new Date().toISOString(),
        workspaceRoot: '/test',
        hasBaseline: false,
      },
    };

    expect(countConstraints(constraints)).toBe(0);
  });

  it('should count all constraint types', () => {
    const constraints: Constraints = {
      boundaries: [
        {
          name: 'test1',
          from: 'a',
          to: 'b',
          message: 'test',
          severity: 'error',
        },
        {
          name: 'test2',
          from: 'c',
          to: 'd',
          message: 'test',
          severity: 'warning',
        },
      ],
      layers: [
        {
          name: 'layer1',
          patterns: ['src/**'],
          dependsOn: [],
        },
      ],
      antiPatterns: [
        {
          id: 'AP-001',
          name: 'Test',
          category: 'test',
          explanation: 'test',
          suggestion: 'warning',
          enabled: true,
        },
      ],
      conventions: [
        {
          category: 'test',
          description: 'test',
        },
      ],
      suppressions: [
        {
          patternId: 'AP-001',
          file: 'src/foo.ts',
          scope: 'file',
          reason: 'test',
        },
      ],
      metadata: {
        collectedAt: new Date().toISOString(),
        workspaceRoot: '/test',
        hasBaseline: false,
      },
    };

    // 2 boundaries + 1 layer + 1 anti-pattern + 1 convention + 1 suppression = 6
    expect(countConstraints(constraints)).toBe(6);
  });
});
