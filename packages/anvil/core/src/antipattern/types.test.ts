import { describe, it, expect } from 'vitest';
import {
  WarningSchema,
  AntiPatternSchema,
  SuppressionRecordSchema,
  WarningResultSchema,
  DetectionConfigSchema,
  createWarningFingerprint,
  isBlockingWarning,
  countBySeverity,
  createWarningResult,
  validateWarningResultConsistency,
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

  it('should validate a warning with nudge field', () => {
    const warning = {
      id: 'AP-003',
      category: 'anti-pattern',
      severity: 'warning',
      confidence: 'high',
      title: 'Explicit any type usage',
      message: 'Found explicit any type',
      explanation: 'Using any defeats type checking',
      suggestion: 'Use unknown instead',
      nudge: "Don't use `any` here. Think about what type this value actually holds.",
      location: { file: 'src/foo.ts', line: 5 },
    };

    const result = WarningSchema.safeParse(warning);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.nudge).toBe(warning.nudge);
    }
  });

  it('should validate a warning without nudge field (optional)', () => {
    const warning = {
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

    const result = WarningSchema.safeParse(warning);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.nudge).toBeUndefined();
    }
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

describe('DetectionConfig Schema', () => {
  it('should validate regex detection with pattern', () => {
    const config = {
      type: 'regex',
      pattern: 'eslint-disable(?!-next-line)',
    };

    const result = DetectionConfigSchema.safeParse(config);
    expect(result.success).toBe(true);
  });

  it('should validate AST detection with astQuery', () => {
    const config = {
      type: 'ast',
      astQuery: 'NonNullExpression',
    };

    const result = DetectionConfigSchema.safeParse(config);
    expect(result.success).toBe(true);
  });

  it('should reject regex detection without pattern', () => {
    const config = {
      type: 'regex',
      // pattern is missing
    };

    const result = DetectionConfigSchema.safeParse(config);
    expect(result.success).toBe(false);
  });

  it('should reject AST detection without astQuery', () => {
    const config = {
      type: 'ast',
      // astQuery is missing
    };

    const result = DetectionConfigSchema.safeParse(config);
    expect(result.success).toBe(false);
  });

  it('should reject regex detection with empty pattern', () => {
    const config = {
      type: 'regex',
      pattern: '',
    };

    const result = DetectionConfigSchema.safeParse(config);
    expect(result.success).toBe(false);
  });

  it('should reject AST detection with empty astQuery', () => {
    const config = {
      type: 'ast',
      astQuery: '',
    };

    const result = DetectionConfigSchema.safeParse(config);
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

  it('should validate a pattern with nudge field', () => {
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
      nudge: "Don't disable all linting rules.",
      enabled: true,
      optIn: false,
    };

    const result = AntiPatternSchema.safeParse(pattern);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.nudge).toBe("Don't disable all linting rules.");
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

  it('should accept pattern without fileExtensions (optional)', () => {
    const pattern = {
      id: 'AP-001',
      name: 'Broad eslint-disable',
      category: 'escape-hatch',
      severity: 'warning',
      confidence: 'high',
      detection: {
        type: 'regex',
        pattern: 'eslint-disable',
      },
      title: 'Test',
      explanation: 'Test',
      suggestion: 'Test',
    };

    const result = AntiPatternSchema.safeParse(pattern);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.fileExtensions).toBeUndefined();
    }
  });

  it('should reject pattern with invalid detection config', () => {
    const pattern = {
      id: 'AP-001',
      name: 'Invalid pattern',
      category: 'escape-hatch',
      severity: 'warning',
      confidence: 'high',
      detection: {
        type: 'regex',
        // pattern is missing - should fail
      },
      title: 'Test',
      explanation: 'Test',
      suggestion: 'Test',
    };

    const result = AntiPatternSchema.safeParse(pattern);
    expect(result.success).toBe(false);
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
    it('should count warnings by severity including total', () => {
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
        total: 5,
        errors: 1,
        warnings: 2,
        info: 1,
        suppressed: 1,
      });
    });

    it('should return zero counts for empty array', () => {
      const counts = countBySeverity([]);
      expect(counts).toEqual({
        total: 0,
        errors: 0,
        warnings: 0,
        info: 0,
        suppressed: 0,
      });
    });
  });

  describe('createWarningResult', () => {
    it('should create a consistent WarningResult', () => {
      const warnings: Warning[] = [
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
        {
          id: 'AP-023',
          category: 'anti-pattern',
          severity: 'error',
          confidence: 'high',
          title: 'Error',
          message: 'Error',
          explanation: 'Error',
          suggestion: 'Error',
          location: { file: 'test.ts', line: 2 },
        },
      ];

      const result = createWarningResult(warnings, ['AP-001', 'AP-023']);

      expect(result.warnings).toBe(warnings);
      expect(result.patterns_checked).toEqual(['AP-001', 'AP-023']);
      expect(result.summary).toEqual({
        total: 2,
        errors: 1,
        warnings: 1,
        info: 0,
        suppressed: 0,
      });
    });
  });

  describe('validateWarningResultConsistency', () => {
    it('should return true for consistent result', () => {
      const warnings: Warning[] = [
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
      ];

      const result = createWarningResult(warnings, ['AP-001']);
      expect(validateWarningResultConsistency(result)).toBe(true);
    });

    it('should return false for inconsistent total', () => {
      const result = {
        warnings: [
          {
            id: 'AP-001',
            category: 'anti-pattern' as const,
            severity: 'warning' as const,
            confidence: 'high' as const,
            title: 'Test',
            message: 'Test',
            explanation: 'Test',
            suggestion: 'Test',
            location: { file: 'test.ts', line: 1 },
          },
        ],
        summary: {
          total: 5, // Wrong!
          errors: 0,
          warnings: 1,
          info: 0,
          suppressed: 0,
        },
        patterns_checked: ['AP-001'],
      };

      expect(validateWarningResultConsistency(result)).toBe(false);
    });

    it('should return false for inconsistent severity counts', () => {
      const result = {
        warnings: [
          {
            id: 'AP-001',
            category: 'anti-pattern' as const,
            severity: 'warning' as const,
            confidence: 'high' as const,
            title: 'Test',
            message: 'Test',
            explanation: 'Test',
            suggestion: 'Test',
            location: { file: 'test.ts', line: 1 },
          },
        ],
        summary: {
          total: 1,
          errors: 1, // Wrong! Should be 0
          warnings: 0, // Wrong! Should be 1
          info: 0,
          suppressed: 0,
        },
        patterns_checked: ['AP-001'],
      };

      expect(validateWarningResultConsistency(result)).toBe(false);
    });
  });
});
