import React from 'react';
import { describe, it, expect } from 'vitest';
import { render } from 'ink-testing-library';
import { ParallelProgress } from '../ParallelProgress.js';
import { CheckProgressBar } from '../CheckProgressBar.js';
import {
  type CheckProgress,
  calculateOverallProgress,
  calculateETA,
  formatDuration,
  getStatusColour,
  getStatusIcon,
} from '../types.js';

function createMockCheck(
  id: string,
  status: CheckProgress['status'],
  progress: number = 0,
  durationMs?: number
): CheckProgress {
  return {
    id,
    name: `Check ${id}`,
    status,
    progress,
    durationMs,
  };
}

describe('ParallelProgress', () => {
  it('renders all checks', () => {
    const checks: CheckProgress[] = [
      createMockCheck('lint', 'passed', 100, 1200),
      createMockCheck('test', 'running', 50),
      createMockCheck('coverage', 'pending', 0),
    ];

    const { lastFrame } = render(<ParallelProgress checks={checks} />);

    expect(lastFrame()).toContain('Check lint');
    expect(lastFrame()).toContain('Check test');
    expect(lastFrame()).toContain('Check coverage');
  });

  it('shows title', () => {
    const checks: CheckProgress[] = [createMockCheck('lint', 'running', 50)];

    const { lastFrame } = render(<ParallelProgress checks={checks} title="Gate Checks" />);

    expect(lastFrame()).toContain('Gate Checks');
  });

  it('shows completion count', () => {
    const checks: CheckProgress[] = [
      createMockCheck('a', 'passed', 100),
      createMockCheck('b', 'passed', 100),
      createMockCheck('c', 'running', 50),
    ];

    const { lastFrame } = render(<ParallelProgress checks={checks} />);

    expect(lastFrame()).toContain('2/3 complete');
  });

  it('shows Complete when all done', () => {
    const checks: CheckProgress[] = [
      createMockCheck('a', 'passed', 100),
      createMockCheck('b', 'passed', 100),
    ];

    const { lastFrame } = render(<ParallelProgress checks={checks} />);

    expect(lastFrame()).toContain('Complete');
  });

  it('shows failed count in completion message', () => {
    const checks: CheckProgress[] = [
      createMockCheck('a', 'passed', 100),
      createMockCheck('b', 'failed', 100),
    ];

    const { lastFrame } = render(<ParallelProgress checks={checks} />);

    expect(lastFrame()).toContain('1 failed');
  });

  it('shows overall progress bar', () => {
    const checks: CheckProgress[] = [
      createMockCheck('a', 'passed', 100),
      createMockCheck('b', 'running', 50),
    ];

    const { lastFrame } = render(<ParallelProgress checks={checks} showOverall={true} />);

    expect(lastFrame()).toContain('Overall:');
    expect(lastFrame()).toContain('%');
  });

  it('hides overall progress when disabled', () => {
    const checks: CheckProgress[] = [createMockCheck('a', 'running', 50)];

    const { lastFrame } = render(<ParallelProgress checks={checks} showOverall={false} />);

    expect(lastFrame()).not.toContain('Overall:');
  });
});

describe('CheckProgressBar', () => {
  it('renders check name', () => {
    const check = createMockCheck('lint', 'running', 50);

    const { lastFrame } = render(<CheckProgressBar check={check} />);

    expect(lastFrame()).toContain('Check lint');
  });

  it('shows pending state', () => {
    const check = createMockCheck('test', 'pending', 0);

    const { lastFrame } = render(<CheckProgressBar check={check} />);

    expect(lastFrame()).toContain('pending');
  });

  it('shows passed state', () => {
    const check = createMockCheck('test', 'passed', 100, 500);

    const { lastFrame } = render(<CheckProgressBar check={check} />);

    expect(lastFrame()).toContain('passed');
  });

  it('shows failed state', () => {
    const check = createMockCheck('test', 'failed', 100);

    const { lastFrame } = render(<CheckProgressBar check={check} />);

    expect(lastFrame()).toContain('failed');
  });

  it('shows cached state', () => {
    const check = createMockCheck('test', 'cached', 100);

    const { lastFrame } = render(<CheckProgressBar check={check} />);

    expect(lastFrame()).toContain('Cached');
  });

  it('shows duration when complete', () => {
    const check = createMockCheck('test', 'passed', 100, 1500);

    const { lastFrame } = render(<CheckProgressBar check={check} />);

    expect(lastFrame()).toContain('1.5s');
  });

  it('shows progress percentage when running', () => {
    const check = createMockCheck('test', 'running', 75);

    const { lastFrame } = render(<CheckProgressBar check={check} />);

    expect(lastFrame()).toContain('75%');
  });

  it('shows message when provided', () => {
    const check: CheckProgress = {
      ...createMockCheck('test', 'failed', 100),
      message: 'Coverage below threshold',
    };

    const { lastFrame } = render(<CheckProgressBar check={check} />);

    expect(lastFrame()).toContain('Coverage below threshold');
  });
});

describe('calculateOverallProgress', () => {
  it('returns 0 for empty checks', () => {
    expect(calculateOverallProgress([])).toBe(0);
  });

  it('returns 100 when all passed', () => {
    const checks: CheckProgress[] = [
      createMockCheck('a', 'passed', 100),
      createMockCheck('b', 'passed', 100),
    ];

    expect(calculateOverallProgress(checks)).toBe(100);
  });

  it('averages progress of running checks', () => {
    const checks: CheckProgress[] = [
      createMockCheck('a', 'passed', 100),
      createMockCheck('b', 'running', 50),
    ];

    expect(calculateOverallProgress(checks)).toBe(75);
  });

  it('treats cached as complete', () => {
    const checks: CheckProgress[] = [
      createMockCheck('a', 'cached', 0),
      createMockCheck('b', 'running', 50),
    ];

    expect(calculateOverallProgress(checks)).toBe(75);
  });

  it('treats skipped as complete', () => {
    const checks: CheckProgress[] = [
      createMockCheck('a', 'skipped', 0),
      createMockCheck('b', 'running', 50),
    ];

    expect(calculateOverallProgress(checks)).toBe(75);
  });
});

describe('calculateETA', () => {
  it('returns undefined with no completed checks', () => {
    const checks: CheckProgress[] = [
      createMockCheck('a', 'running', 50),
      createMockCheck('b', 'pending', 0),
    ];

    expect(calculateETA(checks)).toBeUndefined();
  });

  it('estimates based on completed check durations', () => {
    const checks: CheckProgress[] = [
      { ...createMockCheck('a', 'passed', 100), durationMs: 1000 },
      { ...createMockCheck('b', 'passed', 100), durationMs: 1000 },
      createMockCheck('c', 'pending', 0),
    ];

    const eta = calculateETA(checks);
    expect(eta).toBeDefined();
    expect(eta).toBeGreaterThan(0);
  });
});

describe('formatDuration', () => {
  it('formats milliseconds', () => {
    expect(formatDuration(500)).toBe('500ms');
  });

  it('formats seconds', () => {
    expect(formatDuration(1500)).toBe('1.5s');
  });

  it('formats minutes and seconds', () => {
    expect(formatDuration(90000)).toBe('1m 30s');
  });
});

describe('getStatusColour', () => {
  it('returns correct colours for each status', () => {
    expect(getStatusColour('passed')).toBe('#64748b');
    expect(getStatusColour('failed')).toBe('#dc2626');
    expect(getStatusColour('running')).toBe('#f97316');
    expect(getStatusColour('pending')).toBe('#475569');
    expect(getStatusColour('skipped')).toBe('#94a3b8');
    expect(getStatusColour('cached')).toBe('#22c55e');
  });
});

describe('getStatusIcon', () => {
  it('returns correct icons for each status', () => {
    expect(getStatusIcon('passed')).toBe('◆');
    expect(getStatusIcon('failed')).toBe('✖');
    expect(getStatusIcon('running')).toBe('●');
    expect(getStatusIcon('pending')).toBe('○');
    expect(getStatusIcon('skipped')).toBe('○');
    expect(getStatusIcon('cached')).toBe('⚡');
  });
});
