import { describe, expect, it, beforeEach, afterEach } from 'vitest';
import { mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

import {
  createEdgeFingerprint,
  fingerprintEdge,
  extractImports,
  extractImportsFromFiles,
  compareToBaseline,
  toDependencyEdge,
  deduplicateEdges,
  filterCrossLayerEdges,
  resolveImportPath,
  type ImportEdge,
} from './edge-detector.js';
import { createViolationId, type BaselineViolation } from './types.js';

describe('Edge Detector', () => {
  let testDir: string;

  beforeEach(() => {
    testDir = join(tmpdir(), `edge-detector-test-${Date.now()}`);
    mkdirSync(testDir, { recursive: true });
  });

  afterEach(() => {
    rmSync(testDir, { recursive: true, force: true });
  });

  describe('createEdgeFingerprint', () => {
    it('should create consistent fingerprints', () => {
      const fp1 = createEdgeFingerprint('src/a.ts', 'src/b.ts', 10);
      const fp2 = createEdgeFingerprint('src/a.ts', 'src/b.ts', 10);
      expect(fp1).toBe(fp2);
    });

    it('should create different fingerprints for different inputs', () => {
      const fp1 = createEdgeFingerprint('src/a.ts', 'src/b.ts', 10);
      const fp2 = createEdgeFingerprint('src/a.ts', 'src/b.ts', 11);
      const fp3 = createEdgeFingerprint('src/a.ts', 'src/c.ts', 10);
      expect(fp1).not.toBe(fp2);
      expect(fp1).not.toBe(fp3);
    });

    it('should return 16-character hex string', () => {
      const fp = createEdgeFingerprint('src/a.ts', 'src/b.ts', 10);
      expect(fp).toMatch(/^[a-f0-9]{16}$/);
    });
  });

  describe('fingerprintEdge', () => {
    it('should fingerprint an ImportEdge', () => {
      const edge: ImportEdge = {
        from: 'src/a.ts',
        to: 'src/b.ts',
        line: 10,
        type: 'import',
        specifier: './b.js',
      };
      const fp = fingerprintEdge(edge);
      expect(fp).toMatch(/^[a-f0-9]{16}$/);
    });
  });

  describe('resolveImportPath', () => {
    it('should resolve relative imports', () => {
      expect(resolveImportPath('./foo.js', 'src/bar.ts')).toBe('src/foo.js');
      expect(resolveImportPath('../utils/index.js', 'src/deep/file.ts')).toBe('src/utils/index.js');
    });

    it('should return package imports as-is', () => {
      expect(resolveImportPath('lodash', 'src/file.ts')).toBe('lodash');
      expect(resolveImportPath('@org/pkg', 'src/file.ts')).toBe('@org/pkg');
    });

    it('should normalise paths', () => {
      expect(resolveImportPath('./a/../b.js', 'src/file.ts')).toBe('src/b.js');
    });
  });

  describe('extractImports', () => {
    it('should extract ES import statements with resolved paths', () => {
      const filePath = 'test.ts';
      writeFileSync(
        join(testDir, filePath),
        `import { foo } from './foo.js';
import bar from './bar.js';
import * as utils from '../utils/index.js';`
      );

      const edges = extractImports(filePath, testDir);

      expect(edges).toHaveLength(3);
      expect(edges[0]).toMatchObject({
        from: 'test.ts',
        to: 'foo.js',
        specifier: './foo.js',
        line: 1,
        type: 'import',
      });
      expect(edges[1]).toMatchObject({
        from: 'test.ts',
        to: 'bar.js',
        specifier: './bar.js',
        line: 2,
        type: 'import',
      });
      expect(edges[2]).toMatchObject({
        from: 'test.ts',
        to: '../utils/index.js',
        specifier: '../utils/index.js',
        line: 3,
        type: 'import',
      });
    });

    it('should extract re-exports with resolved paths', () => {
      const filePath = 'index.ts';
      writeFileSync(
        join(testDir, filePath),
        `export { foo } from './foo.js';
export * from './bar.js';`
      );

      const edges = extractImports(filePath, testDir);

      expect(edges).toHaveLength(2);
      expect(edges[0].to).toBe('foo.js');
      expect(edges[1].to).toBe('bar.js');
    });

    it('should extract dynamic imports with resolved paths', () => {
      const filePath = 'lazy.ts';
      writeFileSync(
        join(testDir, filePath),
        `const module = await import('./lazy-module.js');
import('./another-lazy.js').then(m => m.init());`
      );

      const edges = extractImports(filePath, testDir);

      expect(edges).toHaveLength(2);
      expect(edges[0]).toMatchObject({
        to: 'lazy-module.js',
        type: 'dynamic',
      });
      expect(edges[1]).toMatchObject({
        to: 'another-lazy.js',
        type: 'dynamic',
      });
    });

    it('should extract require calls', () => {
      const filePath = 'cjs.js';
      writeFileSync(
        join(testDir, filePath),
        `const fs = require('fs');
const path = require('path');`
      );

      const edges = extractImports(filePath, testDir);

      expect(edges).toHaveLength(2);
      expect(edges[0]).toMatchObject({
        to: 'fs',
        type: 'require',
      });
      expect(edges[1]).toMatchObject({
        to: 'path',
        type: 'require',
      });
    });

    it('should respect includeDynamic option', () => {
      const filePath = 'test.ts';
      writeFileSync(
        join(testDir, filePath),
        `import { foo } from './foo.js';
const lazy = await import('./lazy.js');`
      );

      const edges = extractImports(filePath, testDir, { includeDynamic: false });

      expect(edges).toHaveLength(1);
      expect(edges[0].to).toBe('foo.js');
    });

    it('should respect includeRequire option', () => {
      const filePath = 'test.js';
      writeFileSync(
        join(testDir, filePath),
        `import { foo } from './foo.js';
const bar = require('./bar.js');`
      );

      const edges = extractImports(filePath, testDir, { includeRequire: false });

      expect(edges).toHaveLength(1);
      expect(edges[0].to).toBe('foo.js');
    });

    it('should return empty array for non-existent file', () => {
      const edges = extractImports('non-existent.ts', testDir);
      expect(edges).toEqual([]);
    });

    it('should handle file with no imports', () => {
      const filePath = 'no-imports.ts';
      writeFileSync(join(testDir, filePath), `const x = 1;\nconst y = 2;`);

      const edges = extractImports(filePath, testDir);
      expect(edges).toEqual([]);
    });
  });

  describe('extractImportsFromFiles', () => {
    it('should extract imports from multiple files with resolved paths', () => {
      writeFileSync(join(testDir, 'a.ts'), `import { b } from './b.js';`);
      writeFileSync(join(testDir, 'b.ts'), `import { c } from './c.js';`);

      const edges = extractImportsFromFiles(['a.ts', 'b.ts'], testDir);

      expect(edges).toHaveLength(2);
      expect(edges[0].from).toBe('a.ts');
      expect(edges[0].to).toBe('b.js');
      expect(edges[1].from).toBe('b.ts');
      expect(edges[1].to).toBe('c.js');
    });
  });

  describe('compareToBaseline', () => {
    it('should identify new edges not in baseline', () => {
      const currentEdges: ImportEdge[] = [
        { from: 'a.ts', to: 'b.ts', line: 1, type: 'import', specifier: './b.js' },
        { from: 'a.ts', to: 'c.ts', line: 2, type: 'import', specifier: './c.js' },
      ];
      const baselineViolations: BaselineViolation[] = [];

      const result = compareToBaseline(currentEdges, baselineViolations);

      expect(result.new).toHaveLength(2);
      expect(result.existing).toHaveLength(0);
      expect(result.fixed).toHaveLength(0);
    });

    it('should identify existing edges in baseline using createViolationId format', () => {
      const edge: ImportEdge = {
        from: 'a.ts',
        to: 'b.ts',
        line: 1,
        type: 'import',
        specifier: './b.js',
      };
      const violationId = createViolationId(edge.from, edge.to, edge.line);

      const baselineViolations: BaselineViolation[] = [
        {
          id: violationId,
          from_layer: 'presentation',
          to_layer: 'infrastructure',
          from_file: 'a.ts',
          to_file: 'b.ts',
          import_line: 1,
        },
      ];

      const result = compareToBaseline([edge], baselineViolations);

      expect(result.existing).toHaveLength(1);
      expect(result.new).toHaveLength(0);
    });

    it('should identify fixed violations', () => {
      const baselineViolations: BaselineViolation[] = [
        {
          id: 'fixed-violation-id',
          from_layer: 'presentation',
          to_layer: 'infrastructure',
          from_file: 'old.ts',
          to_file: 'removed.ts',
          import_line: 5,
        },
      ];

      const result = compareToBaseline([], baselineViolations);

      expect(result.fixed).toHaveLength(1);
      expect(result.fixed[0].id).toBe('fixed-violation-id');
    });
  });

  describe('toDependencyEdge', () => {
    it('should convert ImportEdge to DependencyEdge', () => {
      const importEdge: ImportEdge = {
        from: 'src/a.ts',
        to: 'src/b.ts',
        line: 10,
        type: 'import',
        specifier: './b.js',
      };

      const depEdge = toDependencyEdge(importEdge, 'presentation', 'domain');

      expect(depEdge).toEqual({
        from: 'src/a.ts',
        to: 'src/b.ts',
        from_layer: 'presentation',
        to_layer: 'domain',
        line: 10,
        type: 'import',
      });
    });

    it('should handle null layers', () => {
      const importEdge: ImportEdge = {
        from: 'src/a.ts',
        to: 'src/b.ts',
        line: 10,
        type: 'import',
        specifier: './b.js',
      };

      const depEdge = toDependencyEdge(importEdge);

      expect(depEdge.from_layer).toBeNull();
      expect(depEdge.to_layer).toBeNull();
    });
  });

  describe('deduplicateEdges', () => {
    it('should remove duplicate edges', () => {
      const edges: ImportEdge[] = [
        { from: 'a.ts', to: 'b.ts', line: 1, type: 'import', specifier: './b.js' },
        { from: 'a.ts', to: 'b.ts', line: 1, type: 'import', specifier: './b.js' },
        { from: 'a.ts', to: 'c.ts', line: 2, type: 'import', specifier: './c.js' },
      ];

      const unique = deduplicateEdges(edges);

      expect(unique).toHaveLength(2);
    });

    it('should keep edges with different lines', () => {
      const edges: ImportEdge[] = [
        { from: 'a.ts', to: 'b.ts', line: 1, type: 'import', specifier: './b.js' },
        { from: 'a.ts', to: 'b.ts', line: 5, type: 'import', specifier: './b.js' },
      ];

      const unique = deduplicateEdges(edges);

      expect(unique).toHaveLength(2);
    });
  });

  describe('filterCrossLayerEdges', () => {
    const getLayer = (path: string): string | null => {
      if (path.includes('presentation')) return 'presentation';
      if (path.includes('domain')) return 'domain';
      if (path.includes('infrastructure')) return 'infrastructure';
      return null;
    };

    it('should filter to only cross-layer edges', () => {
      const edges: ImportEdge[] = [
        {
          from: 'src/presentation/handler.ts',
          to: 'src/domain/entity.ts',
          line: 1,
          type: 'import',
          specifier: '../domain/entity.js',
        },
        {
          from: 'src/presentation/utils.ts',
          to: 'src/presentation/handler.ts',
          line: 1,
          type: 'import',
          specifier: './handler.js',
        },
      ];

      const crossLayer = filterCrossLayerEdges(edges, getLayer);

      expect(crossLayer).toHaveLength(1);
      expect(crossLayer[0].from).toContain('presentation');
      expect(crossLayer[0].to).toContain('domain');
    });

    it('should exclude edges where layer is unknown', () => {
      const edges: ImportEdge[] = [
        {
          from: 'src/unknown/file.ts',
          to: 'src/domain/entity.ts',
          line: 1,
          type: 'import',
          specifier: '../domain/entity.js',
        },
      ];

      const crossLayer = filterCrossLayerEdges(edges, getLayer);

      expect(crossLayer).toHaveLength(0);
    });
  });
});
