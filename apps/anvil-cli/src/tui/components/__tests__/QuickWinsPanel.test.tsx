/**
 * Unit Tests for QuickWinsPanel TUI Component
 *
 * Tests panel rendering including:
 * - Empty state (no quick wins)
 * - Progress bar rendering
 * - Batch group display
 * - Focused/unfocused styling
 * - Overflow (more than 5 batch groups)
 */
import React from 'react';
import { render } from 'ink-testing-library';
import { describe, it, expect } from 'vitest';
import { QuickWinsPanel } from '../QuickWinsPanel.js';
import type { QuickWinsAnalysis, BatchGroup } from '../../../services/quick-wins.js';

function createEmptyAnalysis(): QuickWinsAnalysis {
  return {
    quickWins: [],
    batchGroups: [],
    totalWarnings: 10,
    suppressable: 0,
    suppressablePercent: 0,
  };
}

function createAnalysisWithQuickWins(
  suppressable: number = 3,
  total: number = 10,
  batchGroups: BatchGroup[] = []
): QuickWinsAnalysis {
  const quickWins = Array.from({ length: suppressable }, (_, i) => ({
    warning: {
      id: 'AP-003',
      category: 'anti-pattern' as const,
      severity: 'warning' as const,
      confidence: 'high' as const,
      title: 'Explicit any type',
      message: 'Using any',
      explanation: 'Type safety issue',
      suggestion: 'Use specific type',
      location: { file: `src/test${i}.test.ts`, line: 10 + i },
      pattern: 'explicit-any',
    },
    type: 'test-file' as const,
    suggestedReason: 'Test file - relaxed rules',
    confidence: 0.9,
    batchable: true,
    batchKey: 'test-AP-003',
  }));

  return {
    quickWins,
    batchGroups,
    totalWarnings: total,
    suppressable,
    suppressablePercent: total > 0 ? (suppressable / total) * 100 : 0,
  };
}

function createBatchGroup(overrides: Partial<BatchGroup> = {}): BatchGroup {
  return {
    key: 'test-AP-003',
    patternId: 'AP-003',
    type: 'test-file',
    warnings: [],
    suggestedReason: 'Test file - relaxed rules',
    count: 5,
    ...overrides,
  };
}

describe('QuickWinsPanel', () => {
  describe('empty state', () => {
    it('renders no quick wins message when none identified', () => {
      const analysis = createEmptyAnalysis();
      const { lastFrame } = render(<QuickWinsPanel analysis={analysis} />);
      const output = lastFrame();

      expect(output).toContain('QUICK WINS');
      expect(output).toContain('No quick wins identified');
    });

    it('does not render progress bar when no quick wins', () => {
      const analysis = createEmptyAnalysis();
      const { lastFrame } = render(<QuickWinsPanel analysis={analysis} />);
      const output = lastFrame();

      // Should not contain percentage
      expect(output).not.toContain('%');
    });
  });

  describe('with quick wins', () => {
    it('renders suppressable count', () => {
      const analysis = createAnalysisWithQuickWins(3, 10);
      const { lastFrame } = render(<QuickWinsPanel analysis={analysis} />);
      const output = lastFrame();

      expect(output).toContain('3 of 10 suppressable');
    });

    it('renders progress bar with percentage', () => {
      const analysis = createAnalysisWithQuickWins(5, 10);
      const { lastFrame } = render(<QuickWinsPanel analysis={analysis} />);
      const output = lastFrame();

      expect(output).toContain('50%');
    });

    it('renders batch suppression tip', () => {
      const analysis = createAnalysisWithQuickWins(3, 10);
      const { lastFrame } = render(<QuickWinsPanel analysis={analysis} />);
      const output = lastFrame();

      expect(output).toContain('anvil suppress --batch');
    });
  });

  describe('batch groups', () => {
    it('renders batch group rows', () => {
      const batchGroups: BatchGroup[] = [
        createBatchGroup({
          key: 'test-AP-003',
          patternId: 'AP-003',
          type: 'test-file',
          count: 5,
        }),
      ];
      const analysis = createAnalysisWithQuickWins(5, 10, batchGroups);
      const { lastFrame } = render(<QuickWinsPanel analysis={analysis} />);
      const output = lastFrame();

      expect(output).toContain('Batch Suppressions Available');
      expect(output).toContain('Test Files');
      expect(output).toContain('AP-003');
      expect(output).toContain('5 issues');
    });

    it('renders multiple batch groups', () => {
      const batchGroups: BatchGroup[] = [
        createBatchGroup({
          key: 'test-AP-003',
          patternId: 'AP-003',
          type: 'test-file',
          count: 5,
        }),
        createBatchGroup({
          key: 'types-AP-003',
          patternId: 'AP-003',
          type: 'type-definition',
          count: 3,
        }),
      ];
      const analysis = createAnalysisWithQuickWins(8, 15, batchGroups);
      const { lastFrame } = render(<QuickWinsPanel analysis={analysis} />);
      const output = lastFrame();

      expect(output).toContain('Test Files');
      expect(output).toContain('Type Definitions');
    });

    it('shows overflow message when more than 5 batch groups', () => {
      const batchGroups: BatchGroup[] = Array.from({ length: 7 }, (_, i) =>
        createBatchGroup({
          key: `batch-${i}`,
          patternId: `AP-00${i + 1}`,
          count: 10 - i,
        })
      );
      const analysis = createAnalysisWithQuickWins(20, 30, batchGroups);
      const { lastFrame } = render(<QuickWinsPanel analysis={analysis} />);
      const output = lastFrame();

      expect(output).toContain('+2 more batch groups');
    });

    it('does not show overflow message when 5 or fewer batch groups', () => {
      const batchGroups: BatchGroup[] = [
        createBatchGroup({ key: 'batch-1', count: 5 }),
        createBatchGroup({ key: 'batch-2', count: 3 }),
      ];
      const analysis = createAnalysisWithQuickWins(8, 15, batchGroups);
      const { lastFrame } = render(<QuickWinsPanel analysis={analysis} />);
      const output = lastFrame();

      expect(output).not.toContain('more batch groups');
    });

    it('does not show batch section when no batch groups', () => {
      const analysis = createAnalysisWithQuickWins(3, 10, []);
      const { lastFrame } = render(<QuickWinsPanel analysis={analysis} />);
      const output = lastFrame();

      expect(output).not.toContain('Batch Suppressions Available');
    });
  });

  describe('focused state', () => {
    it('renders focused indicator when focused', () => {
      const analysis = createAnalysisWithQuickWins(3, 10);
      const { lastFrame } = render(<QuickWinsPanel analysis={analysis} focused={true} />);
      const output = lastFrame();

      expect(output).toContain('(focused)');
    });

    it('does not render focused indicator when not focused', () => {
      const analysis = createAnalysisWithQuickWins(3, 10);
      const { lastFrame } = render(<QuickWinsPanel analysis={analysis} focused={false} />);
      const output = lastFrame();

      expect(output).not.toContain('(focused)');
    });

    it('defaults to not focused', () => {
      const analysis = createAnalysisWithQuickWins(3, 10);
      const { lastFrame } = render(<QuickWinsPanel analysis={analysis} />);
      const output = lastFrame();

      expect(output).not.toContain('(focused)');
    });
  });

  describe('progress bar edge cases', () => {
    it('renders 100% when all warnings are suppressable', () => {
      const analysis = createAnalysisWithQuickWins(10, 10);
      const { lastFrame } = render(<QuickWinsPanel analysis={analysis} />);
      const output = lastFrame();

      expect(output).toContain('100%');
    });

    it('renders correct percentage for partial suppression', () => {
      const analysis = createAnalysisWithQuickWins(1, 4);
      const { lastFrame } = render(<QuickWinsPanel analysis={analysis} />);
      const output = lastFrame();

      expect(output).toContain('25%');
    });
  });

  describe('quick win type labels', () => {
    it('renders config files label', () => {
      const batchGroups: BatchGroup[] = [
        createBatchGroup({
          key: 'config-AP-003',
          patternId: 'AP-003',
          type: 'config-file',
          count: 3,
        }),
      ];
      const analysis = createAnalysisWithQuickWins(3, 10, batchGroups);
      const { lastFrame } = render(<QuickWinsPanel analysis={analysis} />);
      const output = lastFrame();

      expect(output).toContain('Config Files');
    });

    it('renders generated code label', () => {
      const batchGroups: BatchGroup[] = [
        createBatchGroup({
          key: 'gen-AP-003',
          patternId: 'AP-003',
          type: 'generated-code',
          count: 4,
        }),
      ];
      const analysis = createAnalysisWithQuickWins(4, 10, batchGroups);
      const { lastFrame } = render(<QuickWinsPanel analysis={analysis} />);
      const output = lastFrame();

      expect(output).toContain('Generated Code');
    });
  });
});
