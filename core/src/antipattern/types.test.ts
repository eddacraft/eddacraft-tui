import { describe, it, expect } from 'vitest';
import {
  WarningSchema,
  AntiPatternSchema,
  SuppressionRecordSchema,
  WarningResultSchema,
  createWarningFingerprint,
  isBlockingWarning,
  countBySeverity,
  type Warning,
} from './types.js';

describe('Warning Schema', () => {
  it('should validate a complete warning', () => {
    const warning = {
      id: 'AP-001',
      fingerprint: 'AP-001:src/foo.ts:10:',
      category: 'anti-pattern',
      severity: 'warning',
      confidence: 'high',
      title: 'Broad eslint-disable added',
      message: 'File contains eslint-disable without specific rule',
      explanation: 'Disabling all ESLint rules bypasses important safety checks',
      suggestion: 'Use eslint-disable-next-line with specific rule names',
      location: {
        file: 'src/foo.ts',
        line: 10,
        column: 0,
      },
      pattern: 'broad-eslint-disable',
    };

    const result = WarningSchema.safeParse(warning);
    expect(result.success).toBe(true);
  });

  it('should validate a boundary warning with drift', () => {
    const warning = {
      id: 'BOUND-001',
      category: 'boundary',
      severity: 'error',
      confidence: 'high',
      title: 'Architectural boundary crossed',
      message: 'Presentation layer importing from infrastructure',
      explanation: 'This bypasses the application layer',
      suggestion: 'Inject via service or move to application layer',
      location: {
        file: 'src/controllers/payment.ts',
        line: 15,
      },
      drift: {
        isNew: true,
        existingCount: 2,
      },
    };

    const result = WarningSchema.safeParse(warning);
    expect(result.success).toBe(true);
  });

  it('should validate a suppressed warning', () => {
    const warning = {
      id: 'AP-004',
      category: 'anti-pattern',
      severity: 'warning',
      confidence: 'high',
      title: 'New any type',
      message: 'Using any type',
      explanation: 'any bypasses type checking',
      suggestion: 'Use unknown or specific type',
      location: {
        file: 'src/legacy.ts',
        line: 42,
      },
      suppressed: {
        reason: 'Legacy code, will refactor in Q2',
        author: '@jane',
        scope: 'statement',
      },
    };

    const result = WarningSchema.safeParse(warning);
    expect(result.success).toBe(true);
  });

  it('should reject invalid warning ID format', () => {
    const warning = {
      id: 'INVALID-001', // Wrong prefix
      category: 'anti-pattern',
      severity: 'warning',
      confidence: 'high',
      title: 'Test',
      message: 'Test',
      explanation: 'Test',
      suggestion: 'Test',
      location: { file: 'test.ts', line: 1 },
    };

    const result = WarningSchema.safeParse(warning);
    expect(result.success).toBe(false);
  });

  it('should reject invalid severity', () => {
    const warning = {
      id: 'AP-001',
      category: 'anti-pattern',
      severity: 'critical', // Invalid
      confidence: 'high',
      title: 'Test',
      message: 'Test',
      explanation: 'Test',
      suggestion: 'Test',
      location: { file: 'test.ts', line: 1 },
    };

    const result = WarningSchema.safeParse(warning);
    expect(result.success).toBe(false);
  });
});

describe('AntiPattern Schema', () => {
  it('should validate a regex-based pattern', () => {
    const pattern = {
      id: 'AP-001',
      name: 'Broad eslint-disable',
      category: 'escape-hatch',
      severity: 'warning',
      confidence: 'high',
      detection: {
        type: 'regex',
        pattern: 'eslint-disable(?!-next-line)',
      },
      title: 'Broad eslint-disable added',
      explanation: 'Disabling all rules bypasses safety checks',
      suggestion: 'Use specific rule names',
      enabled: true,
      optIn: false,
    };

    const result = AntiPatternSchema.safeParse(pattern);
    expect(result.success).toBe(true);
  });

  it('should validate an AST-based pattern with threshold', () => {
    const pattern = {
      id: 'AP-006',
      name: 'Non-null assertion overuse',
      category: 'type-safety',
      severity: 'info',
      confidence: 'medium',
      detection: {
        type: 'ast',
        astQuery: 'NonNullExpression',
      },
      title: 'Non-null assertion overuse',
      explanation: 'Excessive use of ! operator bypasses null checks',
      suggestion: 'Use proper null handling',
      threshold: 3,
      enabled: true,
      optIn: true, // Noisy pattern, opt-in only
    };

    const result = AntiPatternSchema.safeParse(pattern);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.optIn).toBe(true);
      expect(result.data.threshold).toBe(3);
    }
  });

  it('should apply default values', () => {
    const pattern = {
      id: 'AP-023',
      name: 'Debugger statement',
      category: 'code-quality',
      severity: 'error',
      confidence: 'high',
      detection: {
        type: 'ast',
        astQuery: 'DebuggerStatement',
      },
      title: 'Debugger statement found',
      explanation: 'Debugger statements should not be in production code',
      suggestion: 'Remove the debugger statement',
      // enabled and optIn not specified - should use defaults
    };

    const result = AntiPatternSchema.safeParse(pattern);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.enabled).toBe(true);
      expect(result.data.optIn).toBe(false);
    }
  });
});

describe('SuppressionRecord Schema', () => {
  it('should validate a complete suppression record', () => {
    const record = {
      id: 'supp-abc123',
      pattern_id: 'AP-001',
      file: 'src/legacy.ts',
      line: 42,
      reason: 'Legacy code, will refactor in Q2',
      author: '@jane',
      timestamp: '2025-01-15T10:30:00Z',
      commit: 'abc123def',
      scope: 'statement',
    };

    const result = SuppressionRecordSchema.safeParse(record);
    expect(result.success).toBe(true);
  });
});

describe('WarningResult Schema', () => {
  it('should validate a warning result with summary', () => {
    const result = {
      warnings: [
        {
          id: 'AP-001',
          category: 'anti-pattern',
          severity: 'warning',
          confidence: 'high',
          title: 'Test',
          message: 'Test',
          explanation: 'Test',
          suggestion: 'Test',
          location: { file: 'test.ts', line: 1 },
        },
      ],
      summary: {
        total: 1,
        errors: 0,
        warnings: 1,
        info: 0,
        suppressed: 0,
      },
      patterns_checked: ['AP-001', 'AP-002', 'AP-003'],
    };

    const parsed = WarningResultSchema.safeParse(result);
    expect(parsed.success).toBe(true);
  });
});

describe('Utility Functions', () => {
  describe('createWarningFingerprint', () => {
    it('should create consistent fingerprints', () => {
      const warning: Omit<Warning, 'fingerprint'> = {
        id: 'AP-001',
        category: 'anti-pattern',
        severity: 'warning',
        confidence: 'high',
        title: 'Test',
        message: 'Test',
        explanation: 'Test',
        suggestion: 'Test',
        location: { file: 'src/foo.ts', line: 10 },
        pattern: 'test-pattern',
      };

      const fp1 = createWarningFingerprint(warning);
      const fp2 = createWarningFingerprint(warning);
      expect(fp1).toBe(fp2);
      expect(fp1).toBe('AP-001:src/foo.ts:10:test-pattern');
    });
  });

  describe('isBlockingWarning', () => {
    it('should return true for unsuppressed errors', () => {
      const warning: Warning = {
        id: 'AP-023',
        category: 'anti-pattern',
        severity: 'error',
        confidence: 'high',
        title: 'Test',
        message: 'Test',
        explanation: 'Test',
        suggestion: 'Test',
        location: { file: 'test.ts', line: 1 },
      };

      expect(isBlockingWarning(warning)).toBe(true);
    });

    it('should return false for suppressed errors', () => {
      const warning: Warning = {
        id: 'AP-023',
        category: 'anti-pattern',
        severity: 'error',
        confidence: 'high',
        title: 'Test',
        message: 'Test',
        explanation: 'Test',
        suggestion: 'Test',
        location: { file: 'test.ts', line: 1 },
        suppressed: {
          reason: 'Intentional',
          scope: 'statement',
        },
      };

      expect(isBlockingWarning(warning)).toBe(false);
    });

    it('should return false for warnings', () => {
      const warning: Warning = {
        id: 'AP-001',
        category: 'anti-pattern',
        severity: 'warning',
        confidence: 'high',
        title: 'Test',
        message: 'Test',
        explanation: 'Test',
        suggestion: 'Test',
        location: { file: 'test.ts', line: 1 },
      };

      expect(isBlockingWarning(warning)).toBe(false);
    });
  });

  describe('countBySeverity', () => {
    it('should count warnings by severity', () => {
      const warnings: Warning[] = [
        {
          id: 'AP-023',
          category: 'anti-pattern',
          severity: 'error',
          confidence: 'high',
          title: 'E1',
          message: 'E1',
          explanation: 'E1',
          suggestion: 'E1',
          location: { file: 'a.ts', line: 1 },
        },
        {
          id: 'AP-001',
          category: 'anti-pattern',
          severity: 'warning',
          confidence: 'high',
          title: 'W1',
          message: 'W1',
          explanation: 'W1',
          suggestion: 'W1',
          location: { file: 'b.ts', line: 1 },
        },
        {
          id: 'AP-002',
          category: 'anti-pattern',
          severity: 'warning',
          confidence: 'high',
          title: 'W2',
          message: 'W2',
          explanation: 'W2',
          suggestion: 'W2',
          location: { file: 'c.ts', line: 1 },
        },
        {
          id: 'AP-021',
          category: 'anti-pattern',
          severity: 'info',
          confidence: 'high',
          title: 'I1',
          message: 'I1',
          explanation: 'I1',
          suggestion: 'I1',
          location: { file: 'd.ts', line: 1 },
        },
        {
          id: 'AP-004',
          category: 'anti-pattern',
          severity: 'warning',
          confidence: 'high',
          title: 'S1',
          message: 'S1',
          explanation: 'S1',
          suggestion: 'S1',
          location: { file: 'e.ts', line: 1 },
          suppressed: { reason: 'OK', scope: 'statement' },
        },
      ];

      const counts = countBySeverity(warnings);
      expect(counts).toEqual({
        errors: 1,
        warnings: 2,
        info: 1,
        suppressed: 1,
      });
    });
  });
});
