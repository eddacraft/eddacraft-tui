import React from 'react';
import { render } from 'ink-testing-library';
import { describe, it, expect } from 'vitest';
import { HooksPanel } from '../panels/HooksPanel.js';
import { ProfilePanel } from '../panels/ProfilePanel.js';
import { ResultsPanel } from '../panels/ResultsPanel.js';
import type { HooksStatus, RepoProfile, RecentResults } from '../types.js';
import { getNextPanel, getPreviousPanel, PANELS } from '../types.js';

function createMockHooksStatus(overrides: Partial<HooksStatus> = {}): HooksStatus {
  return {
    huskyInstalled: true,
    hooksDir: '/test/.husky',
    hooks: [
      { name: 'pre-commit', state: 'active', isAnvilManaged: true },
      { name: 'commit-msg', state: 'missing', isAnvilManaged: false },
      { name: 'pre-push', state: 'disabled', isAnvilManaged: false },
    ],
    ...overrides,
  };
}

function createMockProfile(overrides: Partial<RepoProfile> = {}): RepoProfile {
  return {
    hasConfig: true,
    configPath: '/test/.anvilrc',
    planningDir: 'docs/plans',
    format: 'speckit',
    coverageThreshold: 80,
    checks: [
      { name: 'eslint', enabled: true },
      { name: 'test', enabled: true },
      { name: 'coverage', enabled: false },
    ],
    ...overrides,
  };
}

function createMockResults(overrides: Partial<RecentResults> = {}): RecentResults {
  return {
    hasCache: true,
    cacheDir: '/test/.anvil/cache',
    results: [
      {
        id: 'result-1',
        timestamp: new Date('2025-12-28T10:00:00Z'),
        planPath: 'docs/plans/spec.md',
        passed: true,
        passedChecks: 3,
        totalChecks: 3,
      },
      {
        id: 'result-2',
        timestamp: new Date('2025-12-27T15:30:00Z'),
        planPath: 'docs/plans/plan.md',
        passed: false,
        passedChecks: 2,
        totalChecks: 3,
      },
    ],
    ...overrides,
  };
}

describe('Status Dashboard', () => {
  describe('HooksPanel', () => {
    it('renders hook statuses', () => {
      const data = createMockHooksStatus();
      const { lastFrame } = render(<HooksPanel data={data} focused={false} />);

      expect(lastFrame()).toContain('pre-commit');
      expect(lastFrame()).toContain('commit-msg');
      expect(lastFrame()).toContain('pre-push');
      expect(lastFrame()).toContain('active');
      expect(lastFrame()).toContain('missing');
      expect(lastFrame()).toContain('disabled');
    });

    it('shows Husky not installed message', () => {
      const data = createMockHooksStatus({ huskyInstalled: false, hooks: [] });
      const { lastFrame } = render(<HooksPanel data={data} focused={false} />);

      expect(lastFrame()).toContain('Husky not installed');
    });

    it('shows no hooks message when empty', () => {
      const data = createMockHooksStatus({ hooks: [] });
      const { lastFrame } = render(<HooksPanel data={data} focused={false} />);

      expect(lastFrame()).toContain('No hooks configured');
    });

    it('shows [anvil] tag for managed hooks', () => {
      const data = createMockHooksStatus();
      const { lastFrame } = render(<HooksPanel data={data} focused={false} />);

      expect(lastFrame()).toContain('[anvil]');
    });

    it('shows focused indicator when focused', () => {
      const data = createMockHooksStatus();
      const { lastFrame } = render(<HooksPanel data={data} focused={true} />);

      expect(lastFrame()).toContain('(focused)');
    });
  });

  describe('ProfilePanel', () => {
    it('renders config summary', () => {
      const data = createMockProfile();
      const { lastFrame } = render(<ProfilePanel data={data} focused={false} />);

      expect(lastFrame()).toContain('docs/plans');
      expect(lastFrame()).toContain('speckit');
      expect(lastFrame()).toContain('80%');
    });

    it('shows no config message when missing', () => {
      const data = createMockProfile({ hasConfig: false, checks: [] });
      const { lastFrame } = render(<ProfilePanel data={data} focused={false} />);

      expect(lastFrame()).toContain('No .anvilrc found');
      expect(lastFrame()).toContain('anvil init');
    });

    it('renders enabled checks', () => {
      const data = createMockProfile();
      const { lastFrame } = render(<ProfilePanel data={data} focused={false} />);

      expect(lastFrame()).toContain('eslint');
      expect(lastFrame()).toContain('test');
    });

    it('shows focused indicator when focused', () => {
      const data = createMockProfile();
      const { lastFrame } = render(<ProfilePanel data={data} focused={true} />);

      expect(lastFrame()).toContain('(focused)');
    });
  });

  describe('ResultsPanel', () => {
    it('renders validation results', () => {
      const data = createMockResults();
      const { lastFrame } = render(<ResultsPanel data={data} focused={false} />);

      expect(lastFrame()).toContain('spec.md');
      expect(lastFrame()).toContain('plan.md');
      expect(lastFrame()).toContain('3/3');
      expect(lastFrame()).toContain('2/3');
    });

    it('shows no cache message when missing', () => {
      const data = createMockResults({ hasCache: false, results: [] });
      const { lastFrame } = render(<ResultsPanel data={data} focused={false} />);

      expect(lastFrame()).toContain('No validation history');
    });

    it('shows no results message when empty', () => {
      const data = createMockResults({ results: [] });
      const { lastFrame } = render(<ResultsPanel data={data} focused={false} />);

      expect(lastFrame()).toContain('No recent validations');
    });

    it('shows pass/fail status', () => {
      const data = createMockResults();
      const { lastFrame } = render(<ResultsPanel data={data} focused={false} />);

      expect(lastFrame()).toContain('pass');
      expect(lastFrame()).toContain('fail');
    });

    it('shows focused indicator when focused', () => {
      const data = createMockResults();
      const { lastFrame } = render(<ResultsPanel data={data} focused={true} />);

      expect(lastFrame()).toContain('(focused)');
    });
  });

  describe('Panel Navigation', () => {
    it('has correct panel order', () => {
      expect(PANELS).toEqual(['hooks', 'profile', 'results']);
    });

    it('getNextPanel wraps around', () => {
      expect(getNextPanel('hooks')).toBe('profile');
      expect(getNextPanel('profile')).toBe('results');
      expect(getNextPanel('results')).toBe('hooks');
    });

    it('getPreviousPanel wraps around', () => {
      expect(getPreviousPanel('hooks')).toBe('results');
      expect(getPreviousPanel('profile')).toBe('hooks');
      expect(getPreviousPanel('results')).toBe('profile');
    });
  });
});
