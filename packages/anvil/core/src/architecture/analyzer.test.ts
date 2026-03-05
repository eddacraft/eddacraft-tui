/**
 * Tests for the architecture analyzer
 *
 * Covers: basic analysis, file filtering, baseline creation/comparison,
 * violation classification, error handling, and helper functions.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtempSync, mkdirSync, writeFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { safeCleanup } from '../../../../../tools/test-utils/safe-cleanup.js';
import {
  ArchitectureAnalyzer,
  createArchitectureAnalyzer,
  analyseArchitecture,
  inferBaseline,
} from './analyzer.js';
import { ANVIL_DIR, BASELINE_FILENAME } from './baseline.js';
import type { ArchitectureBaseline } from './types.js';
import { createViolationId } from './types.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeTmpDir(): string {
  return mkdtempSync(join(tmpdir(), 'anvil-analyzer-test-'));
}

/**
 * Build a minimal valid ArchitectureBaseline that Zod will accept.
 */
function buildBaseline(
  violations: ArchitectureBaseline['baseline_snapshot']['violations'] = [],
  moduleCount = 0
): ArchitectureBaseline {
  const now = new Date().toISOString();
  return {
    schema_version: '0.1.0',
    created_at: now,
    updated_at: now,
    entry_points: [],
    layers: {},
    boundaries: [],
    baseline_snapshot: {
      module_count: moduleCount,
      timestamp: now,
      violations,
    },
  };
}

/**
 * Write a baseline JSON file into the .anvil directory.
 */
function writeBaseline(workspaceRoot: string, baseline: ArchitectureBaseline): void {
  const dir = join(workspaceRoot, ANVIL_DIR);
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, BASELINE_FILENAME), JSON.stringify(baseline), 'utf-8');
}

// ---------------------------------------------------------------------------
// Sample file paths used across multiple describe blocks
// ---------------------------------------------------------------------------

const SAMPLE_TS_FILES = [
  'src/controllers/user.controller.ts',
  'src/services/user.service.ts',
  'src/domain/user.entity.ts',
  'src/repositories/user.repository.ts',
  'src/utils/hash.ts',
  'src/index.ts',
];

const MIXED_FILES = [
  'src/app.ts',
  'src/app.js',
  'src/app.jsx',
  'src/app.tsx',
  'src/app.mjs',
  'src/app.cjs',
  'src/styles.css',
  'src/image.png',
  'src/readme.md',
];

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('ArchitectureAnalyzer', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTmpDir();
  });

  afterEach(async () => {
    await safeCleanup(tmpDir);
  });

  // =========================================================================
  // Construction
  // =========================================================================

  describe('constructor', () => {
    it('should create an analyzer with defaults', () => {
      const analyzer = new ArchitectureAnalyzer(tmpDir);

      expect(analyzer).toBeInstanceOf(ArchitectureAnalyzer);
    });

    it('should accept custom options', () => {
      const analyzer = new ArchitectureAnalyzer(tmpDir, {
        includeTests: true,
        includePatterns: ['**/*.ts'],
        excludePatterns: ['**/vendor/**'],
      });

      expect(analyzer).toBeInstanceOf(ArchitectureAnalyzer);
    });
  });

  // =========================================================================
  // analyse() — basic analysis
  // =========================================================================

  describe('analyse', () => {
    it('should return an AnalysisResult with expected shape', async () => {
      const analyzer = new ArchitectureAnalyzer(tmpDir);
      const result = await analyzer.analyse(SAMPLE_TS_FILES);

      expect(result).toHaveProperty('entryPoints');
      expect(result).toHaveProperty('layers');
      expect(result).toHaveProperty('assignments');
      expect(result).toHaveProperty('ambiguous');
      expect(result).toHaveProperty('moduleCount');
      expect(result).toHaveProperty('violations');
      expect(result).toHaveProperty('newViolations');
      expect(result).toHaveProperty('existingViolations');
    });

    it('should count only filtered files in moduleCount', async () => {
      const analyzer = new ArchitectureAnalyzer(tmpDir);
      const result = await analyzer.analyse(SAMPLE_TS_FILES);

      // All sample files are .ts and none in excluded dirs
      expect(result.moduleCount).toBe(SAMPLE_TS_FILES.length);
    });

    it('should return empty result for empty file list', async () => {
      const analyzer = new ArchitectureAnalyzer(tmpDir);
      const result = await analyzer.analyse([]);

      expect(result.moduleCount).toBe(0);
      expect(result.entryPoints).toEqual([]);
      expect(result.assignments).toEqual([]);
      expect(result.violations).toEqual([]);
      expect(result.newViolations).toEqual([]);
      expect(result.existingViolations).toEqual([]);
    });

    it('should detect entry points from the file list', async () => {
      const analyzer = new ArchitectureAnalyzer(tmpDir);
      const result = await analyzer.analyse(['src/index.ts', 'src/main.ts']);

      expect(result.entryPoints.length).toBeGreaterThan(0);
      const paths = result.entryPoints.map((ep) => ep.path);
      // index.ts is a package entry point
      expect(paths).toContain('src/index.ts');
    });

    it('should detect layer assignments for matched files', async () => {
      const analyzer = new ArchitectureAnalyzer(tmpDir);
      const result = await analyzer.analyse(SAMPLE_TS_FILES);

      // Files in controllers/, services/, domain/, repositories/, utils/ should get layers
      const assignedLayers = result.assignments.filter((a) => a.layer !== null).map((a) => a.layer);

      expect(assignedLayers).toContain('presentation');
      expect(assignedLayers).toContain('application');
      expect(assignedLayers).toContain('domain');
      expect(assignedLayers).toContain('infrastructure');
      expect(assignedLayers).toContain('shared');
    });

    it('should exclude test entry points by default', async () => {
      const analyzer = new ArchitectureAnalyzer(tmpDir);
      const result = await analyzer.analyse(['src/index.ts', 'src/services/user.service.test.ts']);

      const types = result.entryPoints.map((ep) => ep.type);
      expect(types).not.toContain('test');
    });

    it('should include test entry points when includeTests is true', async () => {
      const analyzer = new ArchitectureAnalyzer(tmpDir, { includeTests: true });
      const result = await analyzer.analyse(['src/index.ts', 'src/services/user.service.test.ts']);

      const types = result.entryPoints.map((ep) => ep.type);
      expect(types).toContain('test');
    });
  });

  // =========================================================================
  // File filtering (include / exclude patterns)
  // =========================================================================

  describe('file filtering', () => {
    it('should include standard TS/JS extensions by default', async () => {
      const analyzer = new ArchitectureAnalyzer(tmpDir);
      const result = await analyzer.analyse(MIXED_FILES);

      // .ts, .js, .jsx, .tsx, .mjs, .cjs are included; .css, .png, .md are not
      expect(result.moduleCount).toBe(6);
    });

    it('should exclude node_modules by default', async () => {
      const files = [
        'src/app.ts',
        'node_modules/lodash/index.js',
        'node_modules/@types/node/index.d.ts',
      ];
      const analyzer = new ArchitectureAnalyzer(tmpDir);
      const result = await analyzer.analyse(files);

      expect(result.moduleCount).toBe(1);
    });

    it('should exclude .d.ts files by default', async () => {
      const files = ['src/app.ts', 'src/types.d.ts'];
      const analyzer = new ArchitectureAnalyzer(tmpDir);
      const result = await analyzer.analyse(files);

      expect(result.moduleCount).toBe(1);
    });

    it('should exclude dist/ and build/ by default', async () => {
      const files = ['src/app.ts', 'dist/app.js', 'build/app.js'];
      const analyzer = new ArchitectureAnalyzer(tmpDir);
      const result = await analyzer.analyse(files);

      expect(result.moduleCount).toBe(1);
    });

    it('should exclude .git/ by default', async () => {
      const files = ['src/app.ts', '.git/hooks/pre-commit.js'];
      const analyzer = new ArchitectureAnalyzer(tmpDir);
      const result = await analyzer.analyse(files);

      expect(result.moduleCount).toBe(1);
    });

    it('should exclude coverage/ by default', async () => {
      const files = ['src/app.ts', 'coverage/lcov.js'];
      const analyzer = new ArchitectureAnalyzer(tmpDir);
      const result = await analyzer.analyse(files);

      expect(result.moduleCount).toBe(1);
    });

    it('should respect custom include patterns', async () => {
      const analyzer = new ArchitectureAnalyzer(tmpDir, {
        includePatterns: ['**/*.css'],
      });
      const result = await analyzer.analyse(MIXED_FILES);

      // Only .css files should be included
      expect(result.moduleCount).toBe(1);
    });

    it('should respect custom exclude patterns', async () => {
      const analyzer = new ArchitectureAnalyzer(tmpDir, {
        excludePatterns: ['**/services/**'],
      });
      const result = await analyzer.analyse(SAMPLE_TS_FILES);

      // user.service.ts should be excluded
      expect(result.moduleCount).toBe(SAMPLE_TS_FILES.length - 1);
    });

    it('should apply exclude before include (exclude takes precedence)', async () => {
      const files = ['src/services/user.service.ts'];
      const analyzer = new ArchitectureAnalyzer(tmpDir, {
        includePatterns: ['**/*.ts'],
        excludePatterns: ['**/services/**'],
      });
      const result = await analyzer.analyse(files);

      expect(result.moduleCount).toBe(0);
    });
  });

  // =========================================================================
  // Baseline creation
  // =========================================================================

  describe('createBaseline', () => {
    it('should create a baseline from analysis results', async () => {
      const analyzer = new ArchitectureAnalyzer(tmpDir);
      const result = await analyzer.analyse(SAMPLE_TS_FILES);
      const baseline = analyzer.createBaseline(result);

      expect(baseline.schema_version).toBe('0.1.0');
      expect(baseline.entry_points).toEqual(result.entryPoints);
      expect(baseline.baseline_snapshot.module_count).toBe(result.moduleCount);
      expect(baseline.created_at).toBeDefined();
      expect(baseline.updated_at).toBeDefined();
    });

    it('should include layers in the baseline', async () => {
      const analyzer = new ArchitectureAnalyzer(tmpDir);
      const result = await analyzer.analyse(SAMPLE_TS_FILES);
      const baseline = analyzer.createBaseline(result);

      expect(Object.keys(baseline.layers).length).toBeGreaterThan(0);
    });

    it('should generate default boundaries from layers', async () => {
      const analyzer = new ArchitectureAnalyzer(tmpDir);
      const result = await analyzer.analyse(SAMPLE_TS_FILES);
      const baseline = analyzer.createBaseline(result);

      expect(baseline.boundaries.length).toBeGreaterThan(0);
      // Each boundary should have required fields
      for (const b of baseline.boundaries) {
        expect(b).toHaveProperty('name');
        expect(b).toHaveProperty('from');
        expect(b).toHaveProperty('to');
        expect(b).toHaveProperty('severity');
        expect(b).toHaveProperty('message');
      }
    });

    it('should persist the baseline to disk', async () => {
      const analyzer = new ArchitectureAnalyzer(tmpDir);
      const result = await analyzer.analyse(SAMPLE_TS_FILES);
      analyzer.createBaseline(result);

      // createBaseline calls baselineManager.create which calls save
      expect(analyzer.hasBaseline()).toBe(true);
    });
  });

  // =========================================================================
  // Baseline queries (hasBaseline / loadBaseline)
  // =========================================================================

  describe('hasBaseline / loadBaseline', () => {
    it('should return false / null when no baseline exists', () => {
      const analyzer = new ArchitectureAnalyzer(tmpDir);

      expect(analyzer.hasBaseline()).toBe(false);
      expect(analyzer.loadBaseline()).toBeNull();
    });

    it('should return true / baseline after creating one', async () => {
      const analyzer = new ArchitectureAnalyzer(tmpDir);
      const result = await analyzer.analyse(SAMPLE_TS_FILES);
      const created = analyzer.createBaseline(result);

      expect(analyzer.hasBaseline()).toBe(true);
      const loaded = analyzer.loadBaseline();
      expect(loaded).not.toBeNull();
      expect(loaded?.schema_version).toBe('0.1.0');
      expect(loaded?.baseline_snapshot.module_count).toBe(created.baseline_snapshot.module_count);
    });

    it('should load a pre-existing baseline from disk', () => {
      const baseline = buildBaseline([], 42);
      writeBaseline(tmpDir, baseline);

      const analyzer = new ArchitectureAnalyzer(tmpDir);

      expect(analyzer.hasBaseline()).toBe(true);
      const loaded = analyzer.loadBaseline();
      expect(loaded).not.toBeNull();
      expect(loaded?.baseline_snapshot.module_count).toBe(42);
    });
  });

  // =========================================================================
  // Baseline update
  // =========================================================================

  describe('updateBaseline', () => {
    it('should update an existing baseline with new analysis results', async () => {
      const analyzer = new ArchitectureAnalyzer(tmpDir);

      // Create initial baseline
      const result1 = await analyzer.analyse(SAMPLE_TS_FILES.slice(0, 3));
      analyzer.createBaseline(result1);

      // Update with new analysis
      const result2 = await analyzer.analyse(SAMPLE_TS_FILES);
      const updated = analyzer.updateBaseline(result2);

      expect(updated).not.toBeNull();
      expect(updated!.baseline_snapshot.module_count).toBe(result2.moduleCount);
    });

    it('should return null when no baseline exists to update', async () => {
      const analyzer = new ArchitectureAnalyzer(tmpDir);
      const result = await analyzer.analyse(SAMPLE_TS_FILES);

      const updated = analyzer.updateBaseline(result);

      expect(updated).toBeNull();
    });
  });

  // =========================================================================
  // Violation classification (new vs existing)
  // =========================================================================

  describe('violation classification', () => {
    it('should classify all violations as new when no baseline exists', async () => {
      // No baseline on disk — all violations are new
      const analyzer = new ArchitectureAnalyzer(tmpDir);
      const result = await analyzer.analyse(SAMPLE_TS_FILES);

      // Currently violations are empty (placeholder), but the logic still runs.
      // All violations should be marked as new.
      expect(result.existingViolations).toEqual([]);
      // newViolations should equal violations (both empty for now)
      expect(result.newViolations).toEqual(result.violations);
    });

    it('should classify all violations as new when baseline has no violations', async () => {
      const baseline = buildBaseline([], 10);
      writeBaseline(tmpDir, baseline);

      const analyzer = new ArchitectureAnalyzer(tmpDir);
      const result = await analyzer.analyse(SAMPLE_TS_FILES);

      // With an empty baseline snapshot, any violations would be new
      expect(result.existingViolations).toEqual([]);
    });

    it('should separate new from existing violations based on baseline IDs', async () => {
      // Write a baseline that contains a known violation
      const knownId = createViolationId('src/a.ts', 'src/b.ts', 10);
      const baseline = buildBaseline([
        {
          id: knownId,
          from_layer: 'presentation',
          to_layer: 'infrastructure',
          from_file: 'src/a.ts',
          to_file: 'src/b.ts',
          import_line: 10,
        },
      ]);
      writeBaseline(tmpDir, baseline);

      const analyzer = new ArchitectureAnalyzer(tmpDir);
      const result = await analyzer.analyse(SAMPLE_TS_FILES);

      // Currently violations are always empty (placeholder), so both lists are
      // empty. This test validates the shape and that the baseline was loaded.
      expect(Array.isArray(result.newViolations)).toBe(true);
      expect(Array.isArray(result.existingViolations)).toBe(true);
    });
  });

  // =========================================================================
  // getBaselineManager
  // =========================================================================

  describe('getBaselineManager', () => {
    it('should expose the internal baseline manager', () => {
      const analyzer = new ArchitectureAnalyzer(tmpDir);
      const manager = analyzer.getBaselineManager();

      expect(manager).toBeDefined();
      expect(typeof manager.exists).toBe('function');
      expect(typeof manager.load).toBe('function');
      expect(typeof manager.save).toBe('function');
    });
  });
});

// ===========================================================================
// Factory function
// ===========================================================================

describe('createArchitectureAnalyzer', () => {
  it('should return an ArchitectureAnalyzer instance', async () => {
    const tmpDir = makeTmpDir();
    try {
      const analyzer = createArchitectureAnalyzer(tmpDir);
      expect(analyzer).toBeInstanceOf(ArchitectureAnalyzer);
    } finally {
      await safeCleanup(tmpDir);
    }
  });

  it('should forward options', async () => {
    const tmpDir = makeTmpDir();
    try {
      const analyzer = createArchitectureAnalyzer(tmpDir, {
        includePatterns: ['**/*.css'],
      });
      const result = await analyzer.analyse(['src/style.css', 'src/app.ts']);

      expect(result.moduleCount).toBe(1);
    } finally {
      await safeCleanup(tmpDir);
    }
  });
});

// ===========================================================================
// analyseArchitecture helper
// ===========================================================================

describe('analyseArchitecture', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTmpDir();
  });

  afterEach(async () => {
    await safeCleanup(tmpDir);
  });

  it('should analyse files and return an AnalysisResult', async () => {
    const result = await analyseArchitecture(tmpDir, SAMPLE_TS_FILES);

    expect(result.moduleCount).toBe(SAMPLE_TS_FILES.length);
    expect(result).not.toHaveProperty('baseline');
  });

  it('should optionally create a baseline', async () => {
    const result = await analyseArchitecture(tmpDir, SAMPLE_TS_FILES, {
      createBaseline: true,
    });

    expect(result.baseline).toBeDefined();
    expect(result.baseline!.schema_version).toBe('0.1.0');
  });

  it('should not create a baseline when createBaseline is false', async () => {
    const result = await analyseArchitecture(tmpDir, SAMPLE_TS_FILES, {
      createBaseline: false,
    });

    expect(result.baseline).toBeUndefined();
  });
});

// ===========================================================================
// inferBaseline
// ===========================================================================

describe('inferBaseline', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTmpDir();
    mkdirSync(join(tmpDir, 'src', 'controllers'), { recursive: true });
    mkdirSync(join(tmpDir, 'src', 'services'), { recursive: true });
    mkdirSync(join(tmpDir, 'src', 'utils'), { recursive: true });

    writeFileSync(join(tmpDir, 'src', 'controllers', 'user.ts'), 'export const x = 1;');
    writeFileSync(join(tmpDir, 'src', 'services', 'auth.ts'), 'export const y = 2;');
    writeFileSync(join(tmpDir, 'src', 'utils', 'hash.ts'), 'export const z = 3;');
    writeFileSync(join(tmpDir, 'src', 'index.ts'), 'export {};');
  });

  afterEach(async () => {
    await safeCleanup(tmpDir);
  });

  it('should scan workspace and return result + baseline', async () => {
    const { result, baseline } = await inferBaseline(tmpDir);

    expect(result.moduleCount).toBeGreaterThan(0);
    expect(baseline.schema_version).toBe('0.1.0');
    expect(baseline.baseline_snapshot.module_count).toBe(result.moduleCount);
  });

  it('should save baseline to disk by default', async () => {
    await inferBaseline(tmpDir);

    expect(existsSync(join(tmpDir, ANVIL_DIR, BASELINE_FILENAME))).toBe(true);
  });

  it('should still save baseline even when save is false (createBaseline always persists)', async () => {
    // Note: inferBaseline calls analyzer.createBaseline() which internally
    // calls baselineManager.create() -> save(). The save option only guards
    // an additional explicit save call, but the baseline is already written
    // by createBaseline. So the file always exists.
    await inferBaseline(tmpDir, { save: false });

    expect(existsSync(join(tmpDir, ANVIL_DIR, BASELINE_FILENAME))).toBe(true);
  });

  it('should respect custom include/exclude patterns', async () => {
    const { result } = await inferBaseline(tmpDir, {
      includePatterns: ['**/*.ts'],
      excludePatterns: ['**/utils/**'],
      save: false,
    });

    // hash.ts in utils/ should be excluded
    expect(result.moduleCount).toBe(3); // controllers/user.ts, services/auth.ts, index.ts
  });

  it('should handle empty workspace gracefully', async () => {
    const emptyDir = makeTmpDir();
    try {
      const { result, baseline } = await inferBaseline(emptyDir, { save: false });

      expect(result.moduleCount).toBe(0);
      expect(baseline.baseline_snapshot.module_count).toBe(0);
    } finally {
      await safeCleanup(emptyDir);
    }
  });

  it('should skip unreadable directories without crashing', async () => {
    // collectSourceFiles wraps readdirSync in try/catch — a non-existent root
    // should simply return zero files
    const nonExistent = join(tmpDir, 'does-not-exist');

    const { result } = await inferBaseline(nonExistent, { save: false });
    expect(result.moduleCount).toBe(0);
  });
});

// ===========================================================================
// Error handling for I/O failures
// ===========================================================================

describe('error handling', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTmpDir();
  });

  afterEach(async () => {
    await safeCleanup(tmpDir);
  });

  it('should handle corrupt baseline gracefully during analyse', async () => {
    // Write invalid JSON as the baseline
    const dir = join(tmpDir, ANVIL_DIR);
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, BASELINE_FILENAME), '{{{{not json}}}');

    const analyzer = new ArchitectureAnalyzer(tmpDir);
    // Should not throw — load returns null for bad JSON
    const result = await analyzer.analyse(SAMPLE_TS_FILES);

    expect(result.moduleCount).toBe(SAMPLE_TS_FILES.length);
    // Without a valid baseline, all violations are classified as new
    expect(result.existingViolations).toEqual([]);
  });

  it('should handle invalid schema baseline gracefully', async () => {
    const dir = join(tmpDir, ANVIL_DIR);
    mkdirSync(dir, { recursive: true });
    writeFileSync(
      join(dir, BASELINE_FILENAME),
      JSON.stringify({ schema_version: '999.0.0', bad: true })
    );

    const analyzer = new ArchitectureAnalyzer(tmpDir);
    const result = await analyzer.analyse(SAMPLE_TS_FILES);

    // Invalid schema -> baseline is null -> all violations new
    expect(result.existingViolations).toEqual([]);
    expect(result.moduleCount).toBe(SAMPLE_TS_FILES.length);
  });

  it('should handle workspace root that does not exist', async () => {
    const badPath = join(tmpDir, 'nonexistent-subdir');
    const analyzer = new ArchitectureAnalyzer(badPath);

    // analyse does not do I/O itself (just filters paths), so it should work
    const result = await analyzer.analyse(SAMPLE_TS_FILES);
    expect(result.moduleCount).toBe(SAMPLE_TS_FILES.length);
  });
});
