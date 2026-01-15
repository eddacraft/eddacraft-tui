import { describe, it, expect } from 'vitest';
import {
  parseSuppressions,
  isExpired,
  suppressionMatches,
  findMatchingSuppression,
  type ParsedSuppression,
} from './parser.js';

describe('parseSuppressions', () => {
  describe('basic parsing', () => {
    it('parses single-line comment with @anvil-ignore', () => {
      const content = `// @anvil-ignore AP-001: Legacy code, will refactor`;
      const result = parseSuppressions(content);

      expect(result.errors).toHaveLength(0);
      expect(result.suppressions).toHaveLength(1);
      expect(result.suppressions[0]).toMatchObject({
        warningId: 'AP-001',
        reason: 'Legacy code, will refactor',
        line: 1,
        scope: 'file',
      });
      expect(result.suppressions[0].expiresAt).toBeUndefined();
    });

    it('parses @anvil-ignore-until with expiry date', () => {
      const content = `// @anvil-ignore-until 2025-06-01 ARCH-001: Temp workaround for migration`;
      const result = parseSuppressions(content);

      expect(result.errors).toHaveLength(0);
      expect(result.suppressions).toHaveLength(1);
      expect(result.suppressions[0]).toMatchObject({
        warningId: 'ARCH-001',
        reason: 'Temp workaround for migration',
        line: 1,
      });
      expect(result.suppressions[0].expiresAt).toEqual(new Date('2025-06-01'));
    });

    it('parses block comment style', () => {
      const content = `/* @anvil-ignore BOUND-001: Cross-boundary call required */`;
      const result = parseSuppressions(content);

      expect(result.errors).toHaveLength(0);
      expect(result.suppressions).toHaveLength(1);
      expect(result.suppressions[0].warningId).toBe('BOUND-001');
    });

    it('parses JSDoc comment style', () => {
      const content = `/** @anvil-ignore AP-002: Intentional escape hatch */`;
      const result = parseSuppressions(content);

      expect(result.errors).toHaveLength(0);
      expect(result.suppressions).toHaveLength(1);
      expect(result.suppressions[0].warningId).toBe('AP-002');
    });

    it('parses multiple suppressions', () => {
      const content = `
// @anvil-ignore AP-001: First reason
const foo = 1;
// @anvil-ignore ARCH-001: Second reason
const bar = 2;
`;
      const result = parseSuppressions(content);

      expect(result.errors).toHaveLength(0);
      expect(result.suppressions).toHaveLength(2);
      expect(result.suppressions[0].warningId).toBe('AP-001');
      expect(result.suppressions[1].warningId).toBe('ARCH-001');
    });
  });

  describe('scope detection', () => {
    it('detects file scope for comment in top 5 lines', () => {
      const content = `// @anvil-ignore AP-001: File-level suppression
const x = 1;
`;
      const result = parseSuppressions(content);

      expect(result.suppressions[0].scope).toBe('file');
    });

    it('detects statement scope for comment above code', () => {
      const content = `
const x = 1;
// @anvil-ignore AP-001: Statement-level suppression
const y: any = 2;
`;
      const result = parseSuppressions(content);

      expect(result.suppressions[0].scope).toBe('statement');
    });

    it('detects line scope for end-of-line comment', () => {
      const content = `const x: any = 1; // @anvil-ignore AP-001: Line-level suppression`;
      const result = parseSuppressions(content);

      expect(result.suppressions[0].scope).toBe('line');
    });

    it('detects file scope only when no code precedes', () => {
      const content = `
const x = 1;
// @anvil-ignore AP-001: Not file scope because code exists above
`;
      const result = parseSuppressions(content);

      expect(result.suppressions[0].scope).toBe('statement');
    });
  });

  describe('validation', () => {
    it('rejects empty reason', () => {
      const content = `// @anvil-ignore AP-001:`;
      const result = parseSuppressions(content);

      expect(result.suppressions).toHaveLength(0);
      expect(result.errors).toHaveLength(1);
      expect(result.errors[0].message).toContain('non-empty reason');
    });

    it('rejects whitespace-only reason', () => {
      const content = `// @anvil-ignore AP-001:   `;
      const result = parseSuppressions(content);

      expect(result.suppressions).toHaveLength(0);
      expect(result.errors).toHaveLength(1);
    });

    it('rejects invalid warning ID format', () => {
      const content = `// @anvil-ignore INVALID-001: Some reason`;
      const result = parseSuppressions(content);

      expect(result.suppressions).toHaveLength(0);
      expect(result.errors).toHaveLength(1);
      expect(result.errors[0].message).toContain('Invalid suppression format');
    });

    it('rejects invalid date format in @anvil-ignore-until', () => {
      const content = `// @anvil-ignore-until not-a-date AP-001: Some reason`;
      const result = parseSuppressions(content);

      expect(result.suppressions).toHaveLength(0);
      expect(result.errors).toHaveLength(1);
    });

    it('accepts all valid warning ID prefixes', () => {
      const content = `
// @anvil-ignore AP-001: Anti-pattern
// @anvil-ignore ARCH-002: Architecture
// @anvil-ignore BOUND-003: Boundary
`;
      const result = parseSuppressions(content);

      expect(result.errors).toHaveLength(0);
      expect(result.suppressions).toHaveLength(3);
    });
  });

  describe('edge cases', () => {
    it('ignores lines without @anvil-ignore', () => {
      const content = `
const x = 1;
// Regular comment
const y = 2;
`;
      const result = parseSuppressions(content);

      expect(result.suppressions).toHaveLength(0);
      expect(result.errors).toHaveLength(0);
    });

    it('handles empty content', () => {
      const result = parseSuppressions('');

      expect(result.suppressions).toHaveLength(0);
      expect(result.errors).toHaveLength(0);
    });

    it('preserves reason with special characters', () => {
      const content = `// @anvil-ignore AP-001: See JIRA-123 (critical!) & related docs`;
      const result = parseSuppressions(content);

      expect(result.suppressions[0].reason).toBe('See JIRA-123 (critical!) & related docs');
    });

    it('captures raw comment text', () => {
      const content = `// @anvil-ignore AP-001: Test reason`;
      const result = parseSuppressions(content);

      expect(result.suppressions[0].raw).toBe('// @anvil-ignore AP-001: Test reason');
    });
  });
});

describe('isExpired', () => {
  it('returns false for suppression without expiry', () => {
    const suppression: ParsedSuppression = {
      warningId: 'AP-001',
      reason: 'Test',
      line: 1,
      scope: 'line',
      raw: '',
    };

    expect(isExpired(suppression)).toBe(false);
  });

  it('returns true for past expiry date', () => {
    const suppression: ParsedSuppression = {
      warningId: 'AP-001',
      reason: 'Test',
      expiresAt: new Date('2020-01-01'),
      line: 1,
      scope: 'line',
      raw: '',
    };

    expect(isExpired(suppression, new Date('2025-01-01'))).toBe(true);
  });

  it('returns false for future expiry date', () => {
    const suppression: ParsedSuppression = {
      warningId: 'AP-001',
      reason: 'Test',
      expiresAt: new Date('2030-01-01'),
      line: 1,
      scope: 'line',
      raw: '',
    };

    expect(isExpired(suppression, new Date('2025-01-01'))).toBe(false);
  });
});

describe('suppressionMatches', () => {
  it('matches file scope suppression to any line', () => {
    const suppression: ParsedSuppression = {
      warningId: 'AP-001',
      reason: 'Test',
      line: 1,
      scope: 'file',
      raw: '',
    };

    expect(suppressionMatches(suppression, 'AP-001', 1)).toBe(true);
    expect(suppressionMatches(suppression, 'AP-001', 100)).toBe(true);
    expect(suppressionMatches(suppression, 'AP-001', 500)).toBe(true);
  });

  it('matches line scope suppression only to same line', () => {
    const suppression: ParsedSuppression = {
      warningId: 'AP-001',
      reason: 'Test',
      line: 10,
      scope: 'line',
      raw: '',
    };

    expect(suppressionMatches(suppression, 'AP-001', 10)).toBe(true);
    expect(suppressionMatches(suppression, 'AP-001', 9)).toBe(false);
    expect(suppressionMatches(suppression, 'AP-001', 11)).toBe(false);
  });

  it('matches statement scope suppression to next line', () => {
    const suppression: ParsedSuppression = {
      warningId: 'AP-001',
      reason: 'Test',
      line: 10,
      scope: 'statement',
      raw: '',
    };

    expect(suppressionMatches(suppression, 'AP-001', 11)).toBe(true);
    expect(suppressionMatches(suppression, 'AP-001', 10)).toBe(false);
    expect(suppressionMatches(suppression, 'AP-001', 12)).toBe(false);
  });

  it('does not match different warning ID', () => {
    const suppression: ParsedSuppression = {
      warningId: 'AP-001',
      reason: 'Test',
      line: 1,
      scope: 'file',
      raw: '',
    };

    expect(suppressionMatches(suppression, 'AP-002', 1)).toBe(false);
    expect(suppressionMatches(suppression, 'ARCH-001', 1)).toBe(false);
  });
});

describe('findMatchingSuppression', () => {
  const suppressions: ParsedSuppression[] = [
    {
      warningId: 'AP-001',
      reason: 'File-level',
      line: 1,
      scope: 'file',
      raw: '',
    },
    {
      warningId: 'AP-002',
      reason: 'Statement-level',
      line: 10,
      scope: 'statement',
      raw: '',
    },
    {
      warningId: 'AP-003',
      reason: 'Expired',
      line: 1,
      scope: 'file',
      expiresAt: new Date('2020-01-01'),
      raw: '',
    },
  ];

  it('finds matching file-scope suppression', () => {
    const match = findMatchingSuppression(suppressions, 'AP-001', 50, new Date('2025-01-01'));

    expect(match).not.toBeNull();
    expect(match?.warningId).toBe('AP-001');
  });

  it('finds matching statement-scope suppression', () => {
    const match = findMatchingSuppression(suppressions, 'AP-002', 11, new Date('2025-01-01'));

    expect(match).not.toBeNull();
    expect(match?.warningId).toBe('AP-002');
  });

  it('returns null for expired suppression', () => {
    const match = findMatchingSuppression(suppressions, 'AP-003', 1, new Date('2025-01-01'));

    expect(match).toBeNull();
  });

  it('returns null when no match found', () => {
    const match = findMatchingSuppression(suppressions, 'ARCH-001', 1, new Date('2025-01-01'));

    expect(match).toBeNull();
  });
});
