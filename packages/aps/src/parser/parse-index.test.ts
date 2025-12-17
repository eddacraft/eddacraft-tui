/**
 * Tests for parse-index module
 */

import { describe, it, expect } from 'vitest';
import { promises as fs } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseIndex } from './parse-index.js';
import { ParseError } from '../types/index.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const FIXTURES_DIR = join(__dirname, '__fixtures__');
const EXAMPLES_DIR = join(__dirname, '../../examples');

async function loadFixture(filename: string): Promise<string> {
  return fs.readFile(join(FIXTURES_DIR, filename), 'utf-8');
}

describe('parseIndex', () => {
  describe('basic parsing', () => {
    it('should parse a simple index file', async () => {
      const content = await loadFixture('simple-index.aps.md');
      const index = await parseIndex(content, 'simple-index.aps.md');

      expect(index.title).toBe('Simple Plan');
      expect(index.overview).toBe('A simple plan with two modules for testing.');
      expect(index.modules).toHaveLength(2);
      expect(index.sourcePath).toBe('simple-index.aps.md');
    });

    it('should parse module metadata correctly', async () => {
      const content = await loadFixture('simple-index.aps.md');
      const index = await parseIndex(content);

      // First module (auth)
      expect(index.modules[0]).toEqual({
        id: 'auth',
        path: './modules/auth.aps.md',
        scope: 'AUTH',
        owner: '@alice',
        priority: 'high',
        tags: ['security', 'core'],
        dependencies: [],
      });

      // Second module (api)
      expect(index.modules[1]).toEqual({
        id: 'api',
        path: './modules/api.aps.md',
        scope: 'API',
        owner: '@bob',
        priority: 'medium',
        tags: ['backend'],
        dependencies: ['auth'],
      });
    });

    it('should parse open questions', async () => {
      const content = await loadFixture('simple-index.aps.md');
      const index = await parseIndex(content);

      expect(index.openQuestions).toEqual([
        'Should we add rate limiting?',
        'What authentication method to use?',
      ]);
    });

    it('should parse decisions', async () => {
      const content = await loadFixture('simple-index.aps.md');
      const index = await parseIndex(content);

      expect(index.decisions).toEqual([
        'Using JWT tokens (decided 2025-01-15)',
        'PostgreSQL database (decided 2025-01-10)',
      ]);
    });
  });

  describe('real examples', () => {
    it('should parse system-ecommerce index file', async () => {
      const content = await fs.readFile(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'), 'utf-8');
      const index = await parseIndex(content, 'system-ecommerce/APS.md');

      expect(index.title).toBe('E-commerce Platform MVP');
      expect(index.modules).toHaveLength(4);

      // Check module IDs
      expect(index.modules.map((m) => m.id)).toEqual(['auth', 'products', 'cart', 'payments']);

      // Check auth module
      expect(index.modules[0]).toMatchObject({
        id: 'auth',
        path: './modules/auth.aps.md',
        scope: 'AUTH',
        owner: '@alice',
        priority: 'high',
        dependencies: [],
      });

      // Check payments module dependencies
      expect(index.modules[3]).toMatchObject({
        id: 'payments',
        dependencies: ['auth', 'cart'],
      });

      // Check open questions exist
      expect(index.openQuestions).toHaveLength(3);

      // Check decisions exist
      expect(index.decisions).toHaveLength(4);
    });
  });

  describe('error handling', () => {
    it('should throw ParseError for index without title', async () => {
      const content = '## Modules\n\n### auth\n- **Path:** ./auth.aps.md';

      await expect(parseIndex(content, 'no-title.md')).rejects.toThrow(ParseError);
      await expect(parseIndex(content, 'no-title.md')).rejects.toThrow(/must have an H1 title/);
    });

    it('should handle index with no modules', async () => {
      const content = '# Empty Plan\n\n## Overview\n\nJust an overview.';

      const index = await parseIndex(content);

      expect(index.title).toBe('Empty Plan');
      expect(index.modules).toHaveLength(0);
    });
  });

  describe('edge cases', () => {
    it('should handle module with minimal metadata', async () => {
      const content = `# Minimal

## Modules

### simple

- **Path:** [./simple.aps.md](./simple.aps.md)
`;

      const index = await parseIndex(content);

      expect(index.modules[0]).toEqual({
        id: 'simple',
        path: './simple.aps.md',
      });
    });

    it('should handle empty dependencies', async () => {
      const content = `# Test

## Modules

### mod

- **Path:** [./mod.aps.md](./mod.aps.md)
- **Dependencies:** (none)
`;

      const index = await parseIndex(content);
      expect(index.modules[0].dependencies).toEqual([]);
    });

    it('should handle multiple tags', async () => {
      const content = `# Test

## Modules

### mod

- **Path:** [./mod.aps.md](./mod.aps.md)
- **Tags:** one, two, three, four
`;

      const index = await parseIndex(content);
      expect(index.modules[0].tags).toEqual(['one', 'two', 'three', 'four']);
    });
  });
});
