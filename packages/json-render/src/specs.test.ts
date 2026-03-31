import { describe, it, expect, beforeAll } from 'vitest';
import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import { validateSpec, getComponentNames } from './schema-validator.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const specsDir = resolve(__dirname, '..', 'specs');

// Discover spec files inside beforeAll so missing directory produces a
// structured assertion failure rather than a module-load crash.
let specFiles: string[] = [];

beforeAll(() => {
  expect(existsSync(specsDir)).toBe(true);
  specFiles = readdirSync(specsDir)
    .filter((f) => f.endsWith('.dashboard.json'))
    .sort(); // stable ordering
});

const componentNames = getComponentNames();

describe('dashboard spec templates', () => {
  it('discovers at least one spec file', () => {
    expect(specFiles.length).toBeGreaterThan(0);
  });

  // Use a lazy describe so specFiles is populated by the time inner tests run
  describe.each([
    'gate-summary.dashboard.json',
    'watch-session.dashboard.json',
    'architecture-health.dashboard.json',
  ])('%s', (file) => {
    let raw: Record<string, unknown>;

    beforeAll(() => {
      const path = resolve(specsDir, file);
      // Parse in beforeAll so a malformed JSON produces a clean failure message
      // rather than a TypeError cascade across every test in the suite.
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
