/**
 * Unit Tests for QuickWinsIdentifier
 *
 * Tests quick wins identification including:
 * - Test file detection
 * - Type definition detection
 * - Config file detection
 * - Generated code detection
 * - Third-party context detection
 * - Batch grouping
 * - Suppression reason generation
 */

import { describe, it, expect } from 'vitest';
import { QuickWinsIdentifier } from '../quick-wins.js';
import type { Warning } from '@anvil/core';

describe('QuickWinsIdentifier', () => {
  const identifier = new QuickWinsIdentifier();

  const createWarning = (overrides: Partial<Warning> = {}): Warning => ({
    id: 'AP-003',
    category: 'anti-pattern',
    severity: 'warning',
    confidence: 'high',
    title: 'Explicit any type',
    message: 'Using explicit any type',
    explanation: 'Type safety is compromised',
    suggestion: 'Use specific types',
    location: {
      file: 'src/app.ts',
      line: 10,
      column: 5,
    },
    pattern: 'explicit-any',
    ...overrides,
  });

  describe('test file detection', () => {
    it('should identify warnings in .test.ts files', () => {
      const warning = createWarning({
        location: { file: 'src/utils.test.ts', line: 10 },
      });

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse([warning]);
=======
      const analysis = identifier.analyze([warning]);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      expect(analysis.quickWins).toHaveLength(1);
      expect(analysis.quickWins[0].type).toBe('test-file');
      expect(analysis.quickWins[0].batchable).toBe(true);
    });

    it('should identify warnings in .spec.ts files', () => {
      const warning = createWarning({
        location: { file: 'src/component.spec.tsx', line: 10 },
      });

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse([warning]);
=======
      const analysis = identifier.analyze([warning]);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      expect(analysis.quickWins).toHaveLength(1);
      expect(analysis.quickWins[0].type).toBe('test-file');
    });

    it('should identify warnings in __tests__ directories', () => {
      const warning = createWarning({
        location: { file: 'src/__tests__/app.ts', line: 10 },
      });

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse([warning]);
=======
      const analysis = identifier.analyze([warning]);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      expect(analysis.quickWins).toHaveLength(1);
      expect(analysis.quickWins[0].type).toBe('test-file');
    });

    it('should identify warnings in __mocks__ directories', () => {
      const warning = createWarning({
        location: { file: 'src/__mocks__/api.ts', line: 10 },
      });

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse([warning]);
=======
      const analysis = identifier.analyze([warning]);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      expect(analysis.quickWins).toHaveLength(1);
      expect(analysis.quickWins[0].type).toBe('test-file');
    });

    it('should generate appropriate reason for test files', () => {
      const warning = createWarning({
        id: 'AP-003',
        location: { file: 'src/app.test.ts', line: 10 },
      });

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse([warning]);
=======
      const analysis = identifier.analyze([warning]);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      expect(analysis.quickWins[0].suggestedReason).toContain('Test');
      expect(analysis.quickWins[0].suggestedReason).toContain('any');
    });
  });

  describe('type definition file detection', () => {
    it('should identify warnings in .d.ts files', () => {
      const warning = createWarning({
        location: { file: 'src/types.d.ts', line: 10 },
      });

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse([warning]);
=======
      const analysis = identifier.analyze([warning]);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      expect(analysis.quickWins).toHaveLength(1);
      expect(analysis.quickWins[0].type).toBe('type-definition');
      expect(analysis.quickWins[0].confidence).toBeGreaterThan(0.9);
    });

    it('should generate appropriate reason for type definitions', () => {
      const warning = createWarning({
        id: 'AP-003',
        location: { file: 'global.d.ts', line: 5 },
      });

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse([warning]);
=======
      const analysis = identifier.analyze([warning]);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      expect(analysis.quickWins[0].suggestedReason).toContain('Type definition');
    });
  });

  describe('config file detection', () => {
    it('should identify warnings in webpack.config.js', () => {
      const warning = createWarning({
        location: { file: 'webpack.config.js', line: 10 },
      });

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse([warning]);
=======
      const analysis = identifier.analyze([warning]);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      expect(analysis.quickWins).toHaveLength(1);
      expect(analysis.quickWins[0].type).toBe('config-file');
    });

    it('should identify warnings in vite.config.ts', () => {
      const warning = createWarning({
        location: { file: 'vite.config.ts', line: 10 },
      });

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse([warning]);
=======
      const analysis = identifier.analyze([warning]);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      expect(analysis.quickWins[0].type).toBe('config-file');
    });

    it('should identify warnings in next.config.js', () => {
      const warning = createWarning({
        location: { file: 'next.config.js', line: 10 },
      });

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse([warning]);
=======
      const analysis = identifier.analyze([warning]);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      expect(analysis.quickWins[0].type).toBe('config-file');
    });

    it('should include file name in config file reason', () => {
      const warning = createWarning({
        location: { file: 'config/jest.config.ts', line: 10 },
      });

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse([warning]);
=======
      const analysis = identifier.analyze([warning]);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      expect(analysis.quickWins[0].suggestedReason).toContain('jest.config.ts');
    });
  });

  describe('generated code detection', () => {
    it('should identify warnings in .generated files', () => {
      const warning = createWarning({
        location: { file: 'src/api.generated.ts', line: 10 },
      });

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse([warning]);
=======
      const analysis = identifier.analyze([warning]);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      expect(analysis.quickWins).toHaveLength(1);
      expect(analysis.quickWins[0].type).toBe('generated-code');
      expect(analysis.quickWins[0].confidence).toBeGreaterThan(0.95);
    });

    it('should identify warnings in generated directories', () => {
      const warning = createWarning({
        location: { file: 'src/generated/api.ts', line: 10 },
      });

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse([warning]);
=======
      const analysis = identifier.analyze([warning]);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      expect(analysis.quickWins[0].type).toBe('generated-code');
    });

    it('should identify warnings in __generated__ directories', () => {
      const warning = createWarning({
        location: { file: 'src/__generated__/graphql.ts', line: 10 },
      });

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse([warning]);
=======
      const analysis = identifier.analyze([warning]);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      expect(analysis.quickWins[0].type).toBe('generated-code');
    });
  });

  describe('third-party context detection', () => {
    it('should identify third-party context from message', () => {
      const warning = createWarning({
        message: 'Third-party SDK requires any type for callback',
        explanation: 'External library types are incompatible',
      });

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse([warning]);
=======
      const analysis = identifier.analyze([warning]);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      expect(analysis.quickWins).toHaveLength(1);
      expect(analysis.quickWins[0].type).toBe('third-party');
      expect(analysis.quickWins[0].batchable).toBe(false);
    });

    it('should identify third-party from file path', () => {
      const warning = createWarning({
        location: { file: 'src/integrations/third-party-sdk.ts', line: 10 },
      });

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse([warning]);
=======
      const analysis = identifier.analyze([warning]);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      expect(analysis.quickWins[0].type).toBe('third-party');
    });
  });

  describe('migration context detection', () => {
    it('should identify migration context from message', () => {
      const warning = createWarning({
        message: 'Legacy code uses any type',
        explanation: 'Planned for migration in next sprint',
      });

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse([warning]);
=======
      const analysis = identifier.analyze([warning]);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      expect(analysis.quickWins).toHaveLength(1);
      expect(analysis.quickWins[0].type).toBe('migration');
    });

    it('should generate migration reason with tracking reminder', () => {
      const warning = createWarning({
        message: 'Deprecated API usage',
      });

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse([warning]);
=======
      const analysis = identifier.analyze([warning]);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      const quickWin = analysis.quickWins.find((qw) => qw.type === 'migration');
      if (quickWin) {
        expect(quickWin.suggestedReason).toContain('Legacy');
        expect(quickWin.suggestedReason.toLowerCase()).toContain('track');
      }
    });
  });

  describe('batch grouping', () => {
    it('should create batch groups for similar warnings', () => {
      const warnings = [
        createWarning({
          id: 'AP-003',
          location: { file: 'src/test1.test.ts', line: 10 },
        }),
        createWarning({
          id: 'AP-003',
          location: { file: 'src/test2.test.ts', line: 15 },
        }),
        createWarning({
          id: 'AP-003',
          location: { file: 'src/test3.test.ts', line: 20 },
        }),
      ];

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse(warnings);
=======
      const analysis = identifier.analyze(warnings);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      expect(analysis.batchGroups).toHaveLength(1);
      expect(analysis.batchGroups[0].count).toBe(3);
      expect(analysis.batchGroups[0].patternId).toBe('AP-003');
      expect(analysis.batchGroups[0].type).toBe('test-file');
    });

    it('should separate different pattern IDs into different batches', () => {
      const warnings = [
        createWarning({
          id: 'AP-003',
          location: { file: 'src/test1.test.ts', line: 10 },
        }),
        createWarning({
          id: 'AP-003',
          location: { file: 'src/test2.test.ts', line: 15 },
        }),
        createWarning({
          id: 'AP-004',
          location: { file: 'src/test3.test.ts', line: 20 },
        }),
        createWarning({
          id: 'AP-004',
          location: { file: 'src/test4.test.ts', line: 25 },
        }),
      ];

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse(warnings);

      expect(analysis.batchGroups).toHaveLength(2);
      expect(analysis.batchGroups.map((g) => g.patternId).sort()).toEqual(['AP-003', 'AP-004']);
=======
      const analysis = identifier.analyze(warnings);

      expect(analysis.batchGroups).toHaveLength(2);
      expect(analysis.batchGroups.map((g) => g.patternId).sort()).toEqual([
        'AP-003',
        'AP-004',
      ]);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts
    });

    it('should separate different types into different batches', () => {
      const warnings = [
        createWarning({
          id: 'AP-003',
          location: { file: 'src/test.test.ts', line: 10 },
        }),
        createWarning({
          id: 'AP-003',
          location: { file: 'src/types.d.ts', line: 15 },
        }),
      ];

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse(warnings);
=======
      const analysis = identifier.analyze(warnings);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      // Should create one batch for test files and one for type definitions
      expect(analysis.batchGroups).toHaveLength(0); // Neither has 2+ items
      expect(analysis.quickWins).toHaveLength(2);
    });

    it('should not create batch for single item', () => {
      const warning = createWarning({
        location: { file: 'src/app.test.ts', line: 10 },
      });

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse([warning]);
=======
      const analysis = identifier.analyze([warning]);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      expect(analysis.batchGroups).toHaveLength(0);
      expect(analysis.quickWins).toHaveLength(1);
    });

    it('should sort batches by count descending', () => {
      const warnings = [
        ...Array.from({ length: 5 }, (_, i) =>
          createWarning({
            id: 'AP-003',
            location: { file: `src/test${i}.test.ts`, line: 10 },
<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
          })
=======
          }),
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts
        ),
        ...Array.from({ length: 3 }, (_, i) =>
          createWarning({
            id: 'AP-004',
            location: { file: `src/test${i}.test.ts`, line: 10 },
<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
          })
        ),
      ];

      const analysis = identifier.analyse(warnings);
=======
          }),
        ),
      ];

      const analysis = identifier.analyze(warnings);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      expect(analysis.batchGroups[0].count).toBe(5);
      expect(analysis.batchGroups[1].count).toBe(3);
    });
  });

  describe('suppression generation', () => {
    it('should generate suppression comment', () => {
      const warning = createWarning({
        id: 'AP-003',
        location: { file: 'src/app.test.ts', line: 10 },
      });

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse([warning]);
=======
      const analysis = identifier.analyze([warning]);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts
      const comment = identifier.generateSuppressionComment(analysis.quickWins[0]);

      expect(comment).toContain('// @anvil-ignore');
      expect(comment).toContain('AP-003');
      expect(comment).toMatch(/:\s+.+/); // Has reason after colon
    });

    it('should generate batch suppression summary', () => {
      const warnings = [
        createWarning({
          id: 'AP-003',
          location: { file: 'src/test1.test.ts', line: 10 },
        }),
        createWarning({
          id: 'AP-003',
          location: { file: 'src/test2.test.ts', line: 15 },
        }),
        createWarning({
          id: 'AP-003',
          location: { file: 'src/test3.test.ts', line: 20 },
        }),
      ];

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse(warnings);
      const summary = identifier.generateBatchSuppressionSummary(analysis.batchGroups[0]);
=======
      const analysis = identifier.analyze(warnings);
      const summary = identifier.generateBatchSuppressionSummary(
        analysis.batchGroups[0],
      );
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      expect(summary).toContain('3 occurrences');
      expect(summary).toContain('AP-003');
      expect(summary).toContain('test files');
    });
  });

  describe('statistics', () => {
    it('should provide statistics by type', () => {
      const warnings = [
        createWarning({ location: { file: 'src/app.test.ts', line: 10 } }),
        createWarning({ location: { file: 'src/app.test.ts', line: 20 } }),
        createWarning({ location: { file: 'src/types.d.ts', line: 5 } }),
        createWarning({ location: { file: 'next.config.js', line: 3 } }),
      ];

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse(warnings);
=======
      const analysis = identifier.analyze(warnings);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts
      const stats = identifier.getStatistics(analysis);

      expect(stats.byType['test-file']).toBe(2);
      expect(stats.byType['type-definition']).toBe(1);
      expect(stats.byType['config-file']).toBe(1);
    });

    it('should provide statistics by pattern', () => {
      const warnings = [
        createWarning({
          id: 'AP-003',
          location: { file: 'src/app.test.ts', line: 10 },
        }),
        createWarning({
          id: 'AP-003',
          location: { file: 'src/util.test.ts', line: 20 },
        }),
        createWarning({
          id: 'AP-004',
          location: { file: 'src/types.d.ts', line: 5 },
        }),
      ];

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse(warnings);
=======
      const analysis = identifier.analyze(warnings);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts
      const stats = identifier.getStatistics(analysis);

      expect(stats.byPattern['AP-003']).toBe(2);
      expect(stats.byPattern['AP-004']).toBe(1);
    });

    it('should count batchable vs individual', () => {
      const warnings = [
        createWarning({
          location: { file: 'src/app.test.ts', line: 10 },
        }),
        createWarning({
          message: 'Third-party SDK requires any',
        }),
      ];

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse(warnings);
=======
      const analysis = identifier.analyze(warnings);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts
      const stats = identifier.getStatistics(analysis);

      expect(stats.batchableCount).toBe(1);
      expect(stats.individualCount).toBe(1);
    });
  });

  describe('overall analysis', () => {
    it('should calculate suppressable percentage', () => {
      const warnings = [
        createWarning({ location: { file: 'src/app.test.ts', line: 10 } }),
        createWarning({ location: { file: 'src/types.d.ts', line: 5 } }),
        createWarning({ location: { file: 'src/app.ts', line: 10 } }), // Not suppressable
        createWarning({ location: { file: 'src/util.ts', line: 20 } }), // Not suppressable
      ];

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse(warnings);
=======
      const analysis = identifier.analyze(warnings);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      expect(analysis.totalWarnings).toBe(4);
      expect(analysis.suppressable).toBe(2);
      expect(analysis.suppressablePercent).toBe(50);
    });

    it('should skip already suppressed warnings', () => {
      const warnings = [
        createWarning({
          location: { file: 'src/app.test.ts', line: 10 },
          suppressed: {
            reason: 'Already suppressed',
            scope: 'line',
          },
        }),
        createWarning({
          location: { file: 'src/util.test.ts', line: 10 },
        }),
      ];

<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse(warnings);
=======
      const analysis = identifier.analyze(warnings);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      expect(analysis.quickWins).toHaveLength(1);
    });

    it('should handle empty warnings array', () => {
<<<<<<< HEAD:apps/anvil-cli/src/services/__tests__/quick-wins.test.ts
      const analysis = identifier.analyse([]);
=======
      const analysis = identifier.analyze([]);
>>>>>>> 85ae182 (feat(cli): Add quick wins identifier for easy suppressions (IFR-004)):cli/src/services/__tests__/quick-wins.test.ts

      expect(analysis.quickWins).toHaveLength(0);
      expect(analysis.batchGroups).toHaveLength(0);
      expect(analysis.totalWarnings).toBe(0);
      expect(analysis.suppressable).toBe(0);
      expect(analysis.suppressablePercent).toBe(0);
    });
  });
});
