import React from 'react';
import { describe, it, expect } from 'vitest';
import { render } from 'ink-testing-library';
import { GateExplorer } from '../GateExplorer.js';
import { CheckTree } from '../panels/CheckTree.js';
import { DetailPanel } from '../panels/DetailPanel.js';
import { FilterBar } from '../panels/FilterBar.js';
import {
  type GateResult,
  type CheckResult,
  getFilteredChecks,
  getFailedCheckIndices,
  getStatusIcon,
  getStatusColour,
  formatScore,
  formatDuration,
} from '../types.js';

function createMockCheck(
  id: string,
  status: CheckResult['status'] = 'passed',
  score: number = 100
): CheckResult {
  return {
    id,
    name: `check-${id}`,
    status,
    score,
    message: `Message for ${id}`,
    details: status === 'failed' ? ['Detail 1', 'Detail 2'] : undefined,
    category: 'quality',
  };
}

function createMockGateResult(overrides: Partial<GateResult> = {}): GateResult {
  return {
    planId: 'test-plan',
    planPath: 'plans/test.aps.md',
    overall: true,
    score: 95,
    checks: [
      createMockCheck('lint', 'passed', 100),
      createMockCheck('test', 'passed', 90),
      createMockCheck('coverage', 'failed', 60),
      createMockCheck('secrets', 'skipped', 0),
    ],
    duration: 1500,
    timestamp: new Date(),
    ...overrides,
  };
}

describe('GateExplorer', () => {
  it('renders header with plan info', () => {
    const result = createMockGateResult();
    const { lastFrame } = render(<GateExplorer result={result} />);

    expect(lastFrame()).toContain('GATE RESULTS');
    expect(lastFrame()).toContain('test.aps.md');
  });

  it('shows overall status - passed', () => {
    const result = createMockGateResult({ overall: true });
    const { lastFrame } = render(<GateExplorer result={result} />);

    expect(lastFrame()).toContain('PASSED');
  });

  it('shows overall status - failed', () => {
    const result = createMockGateResult({ overall: false });
    const { lastFrame } = render(<GateExplorer result={result} />);

    expect(lastFrame()).toContain('FAILED');
  });

  it('shows score and duration', () => {
    const result = createMockGateResult({ score: 85, duration: 2000 });
    const { lastFrame } = render(<GateExplorer result={result} />);

    expect(lastFrame()).toContain('85%');
    expect(lastFrame()).toContain('2.0s');
  });

  it('shows all checks', () => {
    const result = createMockGateResult();
    const { lastFrame } = render(<GateExplorer result={result} />);

    expect(lastFrame()).toContain('check-lint');
    expect(lastFrame()).toContain('check-test');
    expect(lastFrame()).toContain('check-coverage');
    expect(lastFrame()).toContain('check-secrets');
  });

  it('shows keyboard shortcuts', () => {
    const result = createMockGateResult();
    const { lastFrame } = render(<GateExplorer result={result} />);

    expect(lastFrame()).toContain('j/k navigate');
    expect(lastFrame()).toContain('n/N failures');
    expect(lastFrame()).toContain('q quit');
  });
});

describe('CheckTree', () => {
  it('renders checks with status icons', () => {
    const checks = [createMockCheck('a', 'passed'), createMockCheck('b', 'failed')];
    const { lastFrame } = render(
      <CheckTree checks={checks} selectedIndex={0} expandedChecks={new Set()} />
    );

    expect(lastFrame()).toContain('check-a');
    expect(lastFrame()).toContain('check-b');
  });

  it('shows selection indicator', () => {
    const checks = [createMockCheck('a')];
    const { lastFrame } = render(
      <CheckTree checks={checks} selectedIndex={0} expandedChecks={new Set()} />
    );

    expect(lastFrame()).toContain('▸');
  });

  it('shows score for each check', () => {
    const checks = [createMockCheck('a', 'passed', 85)];
    const { lastFrame } = render(
      <CheckTree checks={checks} selectedIndex={0} expandedChecks={new Set()} />
    );

    expect(lastFrame()).toContain('85%');
  });

  it('shows empty state when no checks', () => {
    const { lastFrame } = render(
      <CheckTree checks={[]} selectedIndex={0} expandedChecks={new Set()} />
    );

    expect(lastFrame()).toContain('No checks to display');
  });

  it('shows expansion indicator for checks with details', () => {
    const checks = [createMockCheck('a', 'failed')];
    const { lastFrame } = render(
      <CheckTree checks={checks} selectedIndex={0} expandedChecks={new Set()} />
    );

    expect(lastFrame()).toContain('▶');
  });

  it('shows expanded details when expanded', () => {
    const checks = [createMockCheck('a', 'failed')];
    const { lastFrame } = render(
      <CheckTree checks={checks} selectedIndex={0} expandedChecks={new Set(['a'])} />
    );

    expect(lastFrame()).toContain('Detail 1');
    expect(lastFrame()).toContain('▼');
  });
});

describe('DetailPanel', () => {
  it('shows empty state when no check selected', () => {
    const { lastFrame } = render(<DetailPanel check={null} />);

    expect(lastFrame()).toContain('Select a check to view details');
  });

  it('shows check details', () => {
    const check = createMockCheck('test', 'failed', 60);
    const { lastFrame } = render(<DetailPanel check={check} />);

    expect(lastFrame()).toContain('CHECK-TEST');
    expect(lastFrame()).toContain('failed');
    expect(lastFrame()).toContain('60%');
    expect(lastFrame()).toContain('Message for test');
  });

  it('shows details list when present', () => {
    const check = createMockCheck('test', 'failed');
    const { lastFrame } = render(<DetailPanel check={check} />);

    expect(lastFrame()).toContain('Detail 1');
    expect(lastFrame()).toContain('Detail 2');
  });

  it('shows category when present', () => {
    const check = createMockCheck('test');
    const { lastFrame } = render(<DetailPanel check={check} />);

    expect(lastFrame()).toContain('quality');
  });
});

describe('FilterBar', () => {
  it('shows filter options', () => {
    const { lastFrame } = render(
      <FilterBar currentFilter="all" searchTerm="" failedCount={0} currentFailureIndex={0} />
    );

    expect(lastFrame()).toContain('[a]ll');
    expect(lastFrame()).toContain('[p]assed');
    expect(lastFrame()).toContain('[f]ailed');
    expect(lastFrame()).toContain('[s]kipped');
  });

  it('highlights current filter', () => {
    const { lastFrame } = render(
      <FilterBar currentFilter="failed" searchTerm="" failedCount={2} currentFailureIndex={0} />
    );

    expect(lastFrame()).toContain('[f]ailed');
  });

  it('shows search term', () => {
    const { lastFrame } = render(
      <FilterBar
        currentFilter="all"
        searchTerm="coverage"
        failedCount={0}
        currentFailureIndex={0}
      />
    );

    expect(lastFrame()).toContain('coverage');
  });

  it('shows failure navigation when failures exist', () => {
    const { lastFrame } = render(
      <FilterBar currentFilter="all" searchTerm="" failedCount={3} currentFailureIndex={1} />
    );

    expect(lastFrame()).toContain('2/3');
  });
});

describe('Type utilities', () => {
  describe('getFilteredChecks', () => {
    const checks = [
      createMockCheck('a', 'passed'),
      createMockCheck('b', 'failed'),
      createMockCheck('c', 'skipped'),
    ];

    it('returns all checks with "all" filter', () => {
      expect(getFilteredChecks(checks, 'all', '')).toHaveLength(3);
    });

    it('filters by passed', () => {
      const filtered = getFilteredChecks(checks, 'passed', '');
      expect(filtered).toHaveLength(1);
      expect(filtered[0].id).toBe('a');
    });

    it('filters by failed', () => {
      const filtered = getFilteredChecks(checks, 'failed', '');
      expect(filtered).toHaveLength(1);
      expect(filtered[0].id).toBe('b');
    });

    it('filters by search term', () => {
      const filtered = getFilteredChecks(checks, 'all', 'check-b');
      expect(filtered).toHaveLength(1);
      expect(filtered[0].id).toBe('b');
    });

    it('is case-insensitive for search', () => {
      const filtered = getFilteredChecks(checks, 'all', 'CHECK-A');
      expect(filtered).toHaveLength(1);
    });
  });

  describe('getFailedCheckIndices', () => {
    it('returns indices of failed checks', () => {
      const checks = [
        createMockCheck('a', 'passed'),
        createMockCheck('b', 'failed'),
        createMockCheck('c', 'passed'),
        createMockCheck('d', 'failed'),
      ];

      const indices = getFailedCheckIndices(checks);
      expect(indices).toEqual([1, 3]);
    });

    it('includes warning status', () => {
      const checks = [createMockCheck('a', 'passed'), createMockCheck('b', 'warning')];

      const indices = getFailedCheckIndices(checks);
      expect(indices).toEqual([1]);
    });

    it('returns empty array when no failures', () => {
      const checks = [createMockCheck('a', 'passed'), createMockCheck('b', 'passed')];

      expect(getFailedCheckIndices(checks)).toEqual([]);
    });
  });

  describe('getStatusIcon', () => {
    it('returns correct icons', () => {
      expect(getStatusIcon('passed')).toBe('◆');
      expect(getStatusIcon('failed')).toBe('✖');
      expect(getStatusIcon('skipped')).toBe('○');
      expect(getStatusIcon('warning')).toBe('◈');
    });
  });

  describe('getStatusColour', () => {
    it('returns correct colours', () => {
      expect(getStatusColour('passed')).toBe('#64748b');
      expect(getStatusColour('failed')).toBe('#dc2626');
      expect(getStatusColour('skipped')).toBe('#94a3b8');
      expect(getStatusColour('warning')).toBe('#fbbf24');
    });
  });

  describe('formatScore', () => {
    it('formats score as percentage', () => {
      expect(formatScore(85.5)).toBe('86%');
      expect(formatScore(100)).toBe('100%');
      expect(formatScore(0)).toBe('0%');
    });
  });

  describe('formatDuration', () => {
    it('formats milliseconds', () => {
      expect(formatDuration(500)).toBe('500ms');
    });

    it('formats seconds', () => {
      expect(formatDuration(2500)).toBe('2.5s');
    });
  });
});
