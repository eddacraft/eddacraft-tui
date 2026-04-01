import { describe, it, expect, beforeAll } from 'vitest';
import { readFileSync, readdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import { validateSpec, getComponentNames } from './schema-validator.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const specsDir = resolve(__dirname, '..', 'specs');

// Discover spec files at module level so describe.each can iterate dynamically.
// Wrapped in try/catch so a missing directory produces a clear assertion failure
// in the "discovers at least one spec file" test rather than a module-load crash.
let specFiles: string[] = [];
let specDirError: Error | null = null;

try {
  specFiles = readdirSync(specsDir)
    .filter((f) => f.endsWith('.dashboard.json'))
    .sort();
} catch (err) {
  specDirError = err as Error;
}

const componentNames = getComponentNames();

describe('dashboard spec templates', () => {
  it('discovers at least one spec file', () => {
    if (specDirError) {
      throw new Error(`Failed to read dashboard specs from "${specsDir}": ${specDirError.message}`);
    }
    expect(specFiles.length).toBeGreaterThan(0);
  });

  describe.each(specFiles)('%s', (file) => {
    let raw: Record<string, unknown>;

    beforeAll(() => {
      const path = resolve(specsDir, file);
      try {
        raw = JSON.parse(readFileSync(path, 'utf-8')) as Record<string, unknown>;
      } catch (err) {
        throw new Error(`Failed to parse spec ${file}: ${String(err)}`);
      }
    });

    it('is valid JSON with required metadata fields', () => {
      expect(raw.title).toEqual(expect.any(String));
      expect(raw.description).toEqual(expect.any(String));
      expect(raw.version).toBe('1.0');
    });

    it('has root and elements fields', () => {
      expect(raw.root).toEqual(expect.any(String));
      expect(raw.elements).toEqual(expect.any(Object));
      expect((raw.elements as Record<string, unknown>)[raw.root as string]).toBeDefined();
    });

    it('passes catalog validation', () => {
      const result = validateSpec(raw);
      expect(result.errors).toEqual([]);
      expect(result.valid).toBe(true);
    });

    it('only uses components from the catalog', () => {
      for (const [, el] of Object.entries(raw.elements as Record<string, unknown>)) {
        const element = el as { type: string };
        expect(componentNames).toContain(element.type);
      }
    });

    it('has no orphaned elements (all children reference existing keys)', () => {
      const elements = raw.elements as Record<string, unknown>;
      for (const [, el] of Object.entries(elements)) {
        const element = el as { children?: string[] };
        if (element.children) {
          for (const child of element.children) {
            expect(elements[child]).toBeDefined();
          }
        }
      }
    });

    it('all elements are reachable from root', () => {
      const elements = raw.elements as Record<string, { children?: string[] }>;
      const visited = new Set<string>();
      const queue = [raw.root as string];
      while (queue.length > 0) {
        const key = queue.shift()!;
        if (visited.has(key)) continue;
        visited.add(key);
        const el = elements[key];
        if (el?.children) queue.push(...el.children);
      }
      const allKeys = Object.keys(elements);
      expect([...visited].sort()).toEqual(allKeys.sort());
    });
  });
});
