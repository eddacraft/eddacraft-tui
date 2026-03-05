/**
 * Unit Tests for ResultsDashboard TUI Component
 *
 * Tests rendering with various data shapes:
 * - Minimal results (project only)
 * - Full results with analysis, quick wins, and historical data
 * - Edge cases: zero warnings, no history, no quick wins
 */
import React from 'react';
import { render } from 'ink-testing-library';
import { describe, it, expect } from 'vitest';
import { ResultsDashboard, type InitAnalysisResults } from '../ResultsDashboard.js';
import type { ProjectContext } from '../../../services/project-detector.js';
import type { QuickWinsAnalysis } from '../../../services/quick-wins.js';
import type { HistoricalAnalysis } from '../../../services/historical-analyzer.js';

function createMinimalProject(): ProjectContext {
  return {
    framework: 'react',
    monorepo: 'none',
    tsStrictness: 'strict',
    size: 'medium',
    fileCount: 150,
    hasEslint: true,
    hasPrettier: true,
    hasTests: true,
    packageManager: 'pnpm',
    projectRoot: '/test/project',
    workspacePackages: [],
  };
}

function createMinimalResults(overrides: Partial<InitAnalysisResults> = {}): InitAnalysisResults {
  return {
    project: createMinimalProject(),
    configPath: '/test/project/.anvilrc',
    ...overrides,
  };
}

function createFullResults(): InitAnalysisResults {
  const quickWins: QuickWinsAnalysis = {
    quickWins: [
      {
        warning: {
          id: 'AP-003',
          category: 'anti-pattern',
          severity: 'warning',
          confidence: 'high',
          title: 'Explicit any type',
          message: 'Using any',
          explanation: 'Type safety issue',
          suggestion: 'Use specific type',
          location: { file: 'src/app.test.ts', line: 10 },
          pattern: 'explicit-any',
        },
        type: 'test-file',
        suggestedReason: 'Test file - relaxed rules',
        confidence: 0.9,
        batchable: true,
        batchKey: 'test-AP-003',
      },
    ],
    batchGroups: [
      {
        key: 'test-AP-003',
        patternId: 'AP-003',
        type: 'test-file',
        warnings: [
          {
            id: 'AP-003',
            category: 'anti-pattern',
            severity: 'warning',
            confidence: 'high',
            title: 'Explicit any type',
            message: 'Using any',
            explanation: 'Type safety issue',
            suggestion: 'Use specific type',
            location: { file: 'src/app.test.ts', line: 10 },
            pattern: 'explicit-any',
          },
          {
            id: 'AP-003',
            category: 'anti-pattern',
            severity: 'warning',
            confidence: 'high',
            title: 'Explicit any type',
            message: 'Using any',
            explanation: 'Type safety issue',
            suggestion: 'Use specific type',
            location: { file: 'src/util.test.ts', line: 20 },
            pattern: 'explicit-any',
          },
        ],
        suggestedReason: 'Test file - relaxed rules',
        count: 2,
      },
    ],
    totalWarnings: 5,
    suppressable: 2,
    suppressablePercent: 40,
  };

  const historical: HistoricalAnalysis = {
    commits: [
      {
        hash: 'abc12345',
        message: 'feat: add feature',
        author: 'Test Author',
        date: new Date('2025-01-15'),
        filesChanged: ['src/app.ts'],
        estimatedViolations: 3,
      },
    ],
    totalCommits: 10,
    totalViolations: 15,
    avgViolationsPerCommit: 1.5,
    patternOccurrences: [
      {
        patternId: 'AP-003',
        patternName: 'Explicit any type',
        count: 8,
        commits: ['abc12345'],
      },
      {
        patternId: 'AP-004',
        patternName: '@ts-ignore directive',
        count: 4,
        commits: ['abc12345'],
      },
    ],
    timeline: [],
    dateRange: {
      from: new Date('2025-01-01'),
      to: new Date('2025-01-31'),
    },
  };

  return {
    project: createMinimalProject(),
    configPath: '/test/project/.anvilrc',
    sampleFiles: {
      analyzed: 42,
      total: 150,
    },
    analysis: {
      totalChecks: 5,
      passedChecks: 3,
      warnings: 8,
      errors: 2,
      suppressions: 3,
    },
    quickWins,
    historical,
  };
}

describe('ResultsDashboard', () => {
  describe('minimal rendering', () => {
    it('renders with minimal project data', () => {
      const results = createMinimalResults();
      const { lastFrame } = render(<ResultsDashboard results={results} />);
      const output = lastFrame();

      expect(output).toContain('ANVIL INITIALIZATION COMPLETE');
      expect(output).toContain('react');
    });

    it('renders project overview section', () => {
      const results = createMinimalResults();
      const { lastFrame } = render(<ResultsDashboard results={results} />);
      const output = lastFrame();

      expect(output).toContain('PROJECT OVERVIEW');
      expect(output).toContain('Framework');
      expect(output).toContain('react');
      expect(output).toContain('Project Size');
      expect(output).toContain('medium');
    });

    it('renders next steps section', () => {
      const results = createMinimalResults();
      const { lastFrame } = render(<ResultsDashboard results={results} />);
      const output = lastFrame();

      expect(output).toContain('NEXT STEPS');
      expect(output).toContain('.anvilrc');
      expect(output).toContain('anvil gate check');
    });

    it('does not render quick wins when not provided', () => {
      const results = createMinimalResults();
      const { lastFrame } = render(<ResultsDashboard results={results} />);
      const output = lastFrame();

      expect(output).not.toContain('QUICK WINS');
    });

    it('does not render historical section when not provided', () => {
      const results = createMinimalResults();
      const { lastFrame } = render(<ResultsDashboard results={results} />);
      const output = lastFrame();

      expect(output).not.toContain('GIT HISTORY INSIGHTS');
    });

    it('does not render analysis results when not provided', () => {
      const results = createMinimalResults();
      const { lastFrame } = render(<ResultsDashboard results={results} />);
      const output = lastFrame();

      expect(output).not.toContain('Gate Checks');
    });
  });

  describe('full rendering', () => {
    it('renders all sections with full data', () => {
      const results = createFullResults();
      const { lastFrame } = render(<ResultsDashboard results={results} />);
      const output = lastFrame();

      expect(output).toContain('PROJECT OVERVIEW');
      expect(output).toContain('QUICK WINS');
      expect(output).toContain('GIT HISTORY INSIGHTS');
      expect(output).toContain('NEXT STEPS');
    });

    it('renders analysis results', () => {
      const results = createFullResults();
      const { lastFrame } = render(<ResultsDashboard results={results} />);
      const output = lastFrame();

      expect(output).toContain('Gate Checks');
      expect(output).toContain('3/5 passed');
    });

    it('renders warning count', () => {
      const results = createFullResults();
      const { lastFrame } = render(<ResultsDashboard results={results} />);
      const output = lastFrame();

      const warningLine = output.split('\n').find((line) => line.toLowerCase().includes('warning'));
      expect(warningLine).toBeDefined();
      expect(warningLine).toContain('8 issues');
    });

    it('renders error count', () => {
      const results = createFullResults();
      const { lastFrame } = render(<ResultsDashboard results={results} />);
      const output = lastFrame();

      const errorLine = output.split('\n').find((line) => line.toLowerCase().includes('error'));
      expect(errorLine).toBeDefined();
      expect(errorLine).toContain('2 issues');
    });

    it('renders suppression count', () => {
      const results = createFullResults();
      const { lastFrame } = render(<ResultsDashboard results={results} />);
      const output = lastFrame();

      expect(output).toContain('3');
    });

    it('renders sample files analyzed', () => {
      const results = createFullResults();
      const { lastFrame } = render(<ResultsDashboard results={results} />);
      const output = lastFrame();

      expect(output).toContain('42');
      expect(output).toContain('150');
    });

    it('renders historical commit count', () => {
      const results = createFullResults();
      const { lastFrame } = render(<ResultsDashboard results={results} />);
      const output = lastFrame();

      expect(output).toContain('10');
    });

    it('renders historical violation count', () => {
      const results = createFullResults();
      const { lastFrame } = render(<ResultsDashboard results={results} />);
      const output = lastFrame();

      expect(output).toContain('15 issues');
    });

    it('renders average violations per commit with the correct value', () => {
      const results = createFullResults();
      const { lastFrame } = render(<ResultsDashboard results={results} />);
      const output = lastFrame();

      const avgLine = output.split('\n').find((line) => line.includes('Average per Commit'));
      expect(avgLine).toBeDefined();
      // The average line should display the correct calculated value
      expect(avgLine).toContain('Average per Commit');
      expect(avgLine).toContain('1.5');
    });

    it('renders most common patterns', () => {
      const results = createFullResults();
      const { lastFrame } = render(<ResultsDashboard results={results} />);
      const output = lastFrame();

      expect(output).toContain('Explicit any type');
      expect(output).toContain('8');
    });
  });

  describe('monorepo display', () => {
    it('renders monorepo info when present', () => {
      const results = createMinimalResults({
        project: {
          ...createMinimalProject(),
          monorepo: 'pnpm-workspace',
          workspacePackages: ['packages/*', 'apps/*'],
        },
      });

      const { lastFrame } = render(<ResultsDashboard results={results} />);
      const output = lastFrame();

      expect(output).toContain('Monorepo');
      expect(output).toContain('pnpm-workspace');
      expect(output).toContain('2 packages');
    });

    it('does not render monorepo info when none', () => {
      const results = createMinimalResults();
      const { lastFrame } = render(<ResultsDashboard results={results} />);
      const output = lastFrame();

      expect(output).not.toContain('Monorepo');
    });
  });

  describe('historical panel edge cases', () => {
    it('renders empty history message when no commits', () => {
      const results = createMinimalResults({
        historical: {
          commits: [],
          totalCommits: 0,
          totalViolations: 0,
          avgViolationsPerCommit: 0,
          patternOccurrences: [],
          timeline: [],
          dateRange: {
            from: new Date(),
            to: new Date(),
          },
        },
      });

      const { lastFrame } = render(<ResultsDashboard results={results} />);
      const output = lastFrame();

      expect(output).toContain('No git history available');
    });
  });

  describe('zero warnings scenario', () => {
    it('renders passing checks with check mark when all pass', () => {
      const results = createMinimalResults({
        analysis: {
          totalChecks: 5,
          passedChecks: 5,
          warnings: 0,
          errors: 0,
          suppressions: 0,
        },
      });

      const { lastFrame } = render(<ResultsDashboard results={results} />);
      const output = lastFrame();

      expect(output).toContain('5/5 passed');
      // Should not show warnings or errors sections
      expect(output).not.toContain('issues');
    });
  });

  describe('keyboard hint', () => {
    it('shows press Enter/q instructions', () => {
      const results = createMinimalResults();
      const { lastFrame } = render(<ResultsDashboard results={results} />);
      const output = lastFrame();

      expect(output).toContain('Enter');
      expect(output).toContain('q');
    });
  });

  describe('TypeScript strictness display', () => {
    it('renders TypeScript strictness level', () => {
      const results = createMinimalResults();
      const { lastFrame } = render(<ResultsDashboard results={results} />);
      const output = lastFrame();

      expect(output).toContain('TypeScript');
      expect(output).toContain('strict');
    });
  });
});
