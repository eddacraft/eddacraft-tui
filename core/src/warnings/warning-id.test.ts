import { describe, it, expect } from 'vitest';
import type { Warning } from '../antipattern/types.js';
import {
  generateWarningId,
  createWarningId,
  parseWarningId,
  isValidWarningId,
  findWarningById,
  findWarningsByRule,
  findWarningsByFile,
  indexWarningsById,
  getWarningIds,
  generateShortId,
  resolveShortId,
} from './warning-id.js';

function createMockWarning(overrides: Partial<Warning> = {}): Warning {
  return {
    id: 'AP-003',
    category: 'anti-pattern',
    severity: 'warning',
    confidence: 'high',
    title: 'Explicit any type usage',
    message: 'Using any type',
    explanation: 'This defeats TypeScript',
    suggestion: 'Use unknown instead',
    location: {
      file: 'src/utils/helpers.ts',
      line: 42,
    },
    ...overrides,
  };
}

describe('generateWarningId', () => {
  it('creates ID from warning object', () => {
    const warning = createMockWarning();
    const id = generateWarningId(warning);
    expect(id).toBe('AP-003-src/utils/helpers.ts:42');
  });

  it('handles architecture warnings', () => {
    const warning = createMockWarning({
      id: 'ARCH-001',
      location: { file: 'src/api/handler.ts', line: 15 },
    });
    const id = generateWarningId(warning);
    expect(id).toBe('ARCH-001-src/api/handler.ts:15');
  });

  it('handles boundary warnings', () => {
    const warning = createMockWarning({
      id: 'BOUND-001',
      location: { file: 'src/db/queries.ts', line: 100 },
    });
    const id = generateWarningId(warning);
    expect(id).toBe('BOUND-001-src/db/queries.ts:100');
  });
});

describe('createWarningId', () => {
  it('creates ID from components', () => {
    const id = createWarningId('AP-003', 'src/utils/helpers.ts', 42);
    expect(id).toBe('AP-003-src/utils/helpers.ts:42');
  });
});

describe('parseWarningId', () => {
  it('parses valid anti-pattern warning ID', () => {
    const parsed = parseWarningId('AP-003-src/utils/helpers.ts:42');
    expect(parsed).toEqual({
      rule: 'AP-003',
      file: 'src/utils/helpers.ts',
      line: 42,
    });
  });

  it('parses valid architecture warning ID', () => {
    const parsed = parseWarningId('ARCH-001-src/api/handler.ts:15');
    expect(parsed).toEqual({
      rule: 'ARCH-001',
      file: 'src/api/handler.ts',
      line: 15,
    });
  });

  it('parses valid boundary warning ID', () => {
    const parsed = parseWarningId('BOUND-001-src/db/queries.ts:100');
    expect(parsed).toEqual({
      rule: 'BOUND-001',
      file: 'src/db/queries.ts',
      line: 100,
    });
  });

  it('handles paths with multiple slashes', () => {
    const parsed = parseWarningId('AP-001-src/deep/nested/path/file.ts:99');
    expect(parsed).toEqual({
      rule: 'AP-001',
      file: 'src/deep/nested/path/file.ts',
      line: 99,
    });
  });

  it('returns null for invalid format', () => {
    expect(parseWarningId('invalid')).toBeNull();
    expect(parseWarningId('AP-003')).toBeNull();
    expect(parseWarningId('AP-003-file.ts')).toBeNull();
    expect(parseWarningId('XX-003-file.ts:10')).toBeNull();
    expect(parseWarningId('')).toBeNull();
  });

  it('returns null for invalid line numbers', () => {
    expect(parseWarningId('AP-003-file.ts:0')).toBeNull();
    expect(parseWarningId('AP-003-file.ts:-1')).toBeNull();
    expect(parseWarningId('AP-003-file.ts:abc')).toBeNull();
  });
});

describe('isValidWarningId', () => {
  it('returns true for valid IDs', () => {
    expect(isValidWarningId('AP-003-src/utils/helpers.ts:42')).toBe(true);
    expect(isValidWarningId('ARCH-001-src/api/handler.ts:15')).toBe(true);
    expect(isValidWarningId('BOUND-001-src/db/queries.ts:100')).toBe(true);
  });

  it('returns false for invalid IDs', () => {
    expect(isValidWarningId('invalid')).toBe(false);
    expect(isValidWarningId('AP-003')).toBe(false);
    expect(isValidWarningId('')).toBe(false);
  });
});

describe('findWarningById', () => {
  const warnings: Warning[] = [
    createMockWarning({ id: 'AP-001', location: { file: 'a.ts', line: 1 } }),
    createMockWarning({ id: 'AP-003', location: { file: 'b.ts', line: 10 } }),
    createMockWarning({ id: 'ARCH-001', location: { file: 'c.ts', line: 20 } }),
  ];

  it('finds warning by full ID', () => {
    const found = findWarningById(warnings, 'AP-003-b.ts:10');
    expect(found).toBeDefined();
    expect(found?.id).toBe('AP-003');
    expect(found?.location.file).toBe('b.ts');
  });

  it('returns undefined for non-existent ID', () => {
    expect(findWarningById(warnings, 'AP-007-x.ts:1')).toBeUndefined();
  });

  it('returns undefined for invalid ID format', () => {
    expect(findWarningById(warnings, 'invalid')).toBeUndefined();
  });
});

describe('findWarningsByRule', () => {
  const warnings: Warning[] = [
    createMockWarning({ id: 'AP-003', location: { file: 'a.ts', line: 1 } }),
    createMockWarning({ id: 'AP-003', location: { file: 'b.ts', line: 10 } }),
    createMockWarning({ id: 'AP-001', location: { file: 'c.ts', line: 20 } }),
  ];

  it('finds all warnings for a rule', () => {
    const found = findWarningsByRule(warnings, 'AP-003');
    expect(found).toHaveLength(2);
  });

  it('returns empty array when no matches', () => {
    const found = findWarningsByRule(warnings, 'AP-007');
    expect(found).toHaveLength(0);
  });
});

describe('findWarningsByFile', () => {
  const warnings: Warning[] = [
    createMockWarning({ id: 'AP-003', location: { file: 'shared.ts', line: 1 } }),
    createMockWarning({ id: 'AP-001', location: { file: 'shared.ts', line: 10 } }),
    createMockWarning({ id: 'ARCH-001', location: { file: 'other.ts', line: 20 } }),
  ];

  it('finds all warnings in a file', () => {
    const found = findWarningsByFile(warnings, 'shared.ts');
    expect(found).toHaveLength(2);
  });

  it('returns empty array when no matches', () => {
    const found = findWarningsByFile(warnings, 'nonexistent.ts');
    expect(found).toHaveLength(0);
  });
});

describe('indexWarningsById', () => {
  it('creates a map indexed by warning ID', () => {
    const warnings: Warning[] = [
      createMockWarning({ id: 'AP-003', location: { file: 'a.ts', line: 1 } }),
      createMockWarning({ id: 'AP-001', location: { file: 'b.ts', line: 10 } }),
    ];

    const index = indexWarningsById(warnings);
    expect(index.size).toBe(2);
    expect(index.get('AP-003-a.ts:1')).toBeDefined();
    expect(index.get('AP-001-b.ts:10')).toBeDefined();
  });
});

describe('getWarningIds', () => {
  it('returns array of warning IDs', () => {
    const warnings: Warning[] = [
      createMockWarning({ id: 'AP-003', location: { file: 'a.ts', line: 1 } }),
      createMockWarning({ id: 'AP-001', location: { file: 'b.ts', line: 10 } }),
    ];

    const ids = getWarningIds(warnings);
    expect(ids).toEqual(['AP-003-a.ts:1', 'AP-001-b.ts:10']);
  });
});

describe('generateShortId', () => {
  it('creates short ID from warning', () => {
    const warning = createMockWarning();
    const shortId = generateShortId(warning);
    expect(shortId).toBe('AP-003:42');
  });
});

describe('resolveShortId', () => {
  const warnings: Warning[] = [
    createMockWarning({ id: 'AP-003', location: { file: 'a.ts', line: 42 } }),
    createMockWarning({ id: 'AP-003', location: { file: 'b.ts', line: 42 } }),
    createMockWarning({ id: 'AP-001', location: { file: 'c.ts', line: 10 } }),
  ];

  it('resolves unique short ID to full ID', () => {
    const resolved = resolveShortId(warnings, 'AP-001:10');
    expect(resolved).toBe('AP-001-c.ts:10');
  });

  it('returns array of matches for ambiguous short ID', () => {
    const resolved = resolveShortId(warnings, 'AP-003:42');
    expect(Array.isArray(resolved)).toBe(true);
    expect(resolved).toHaveLength(2);
    expect(resolved).toContain('AP-003-a.ts:42');
    expect(resolved).toContain('AP-003-b.ts:42');
  });

  it('returns null for non-matching short ID', () => {
    expect(resolveShortId(warnings, 'AP-007:99')).toBeNull();
  });

  it('returns null for invalid short ID format', () => {
    expect(resolveShortId(warnings, 'invalid')).toBeNull();
    expect(resolveShortId(warnings, 'AP-003')).toBeNull();
  });
});
