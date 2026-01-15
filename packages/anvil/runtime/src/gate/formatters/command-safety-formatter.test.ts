import { describe, it, expect } from 'vitest';
import {
  formatBlockedCommands,
  formatWarningCommands,
  formatSummary,
} from './command-safety-formatter.js';
import type { CommandSafetyFinding, CommandAnalysisSummary } from '../rules/types.js';

describe('formatBlockedCommands', () => {
  const blockedFinding: CommandSafetyFinding = {
    command: 'git reset --hard',
    ruleId: 'git-reset-hard',
    category: 'git',
    action: 'block',
    severity: 'error',
    reason: 'Destroys uncommitted changes',
    suggestion: 'Use git stash first',
    references: ['https://git-scm.com/docs/git-reset'],
  };

  it('returns empty string for empty array', () => {
    expect(formatBlockedCommands([])).toBe('');
  });

  it('formats single blocked command with all details', () => {
    const output = formatBlockedCommands([blockedFinding]);
    expect(output).toContain('Blocked 1 dangerous command(s)');
    expect(output).toContain('git reset --hard');
    expect(output).toContain('Reason: Destroys uncommitted changes');
    expect(output).toContain('Suggestion: Use git stash first');
    expect(output).toContain('Reference: https://git-scm.com/docs/git-reset');
  });

  it('respects verbose=false option', () => {
    const output = formatBlockedCommands([blockedFinding], { verbose: false });
    expect(output).toContain('git reset --hard');
    expect(output).not.toContain('Reason:');
  });

  it('respects showSuggestions=false option', () => {
    const output = formatBlockedCommands([blockedFinding], { showSuggestions: false });
    expect(output).not.toContain('Suggestion:');
  });

  it('respects showReferences=false option', () => {
    const output = formatBlockedCommands([blockedFinding], { showReferences: false });
    expect(output).not.toContain('Reference:');
  });

  it('formats multiple blocked commands', () => {
    const output = formatBlockedCommands([
      blockedFinding,
      { ...blockedFinding, command: 'rm -rf /' },
    ]);
    expect(output).toContain('Blocked 2 dangerous command(s)');
    expect(output).toContain('1. git reset --hard');
    expect(output).toContain('2. rm -rf /');
  });
});

describe('formatWarningCommands', () => {
  const warningFinding: CommandSafetyFinding = {
    command: 'git clean -f',
    ruleId: 'git-clean-force',
    category: 'git',
    action: 'warn',
    severity: 'warning',
    reason: 'Removes untracked files',
    suggestion: 'Use git clean -n first',
  };

  it('returns empty string for empty array', () => {
    expect(formatWarningCommands([])).toBe('');
  });

  it('formats warnings correctly', () => {
    const output = formatWarningCommands([warningFinding]);
    expect(output).toContain('Found 1 potentially dangerous command(s)');
    expect(output).toContain('git clean -f');
    expect(output).toContain('Reason: Removes untracked files');
  });
});

describe('formatSummary', () => {
  it('handles zero commands', () => {
    const summary: CommandAnalysisSummary = {
      total: 0,
      blocked: 0,
      warned: 0,
      allowed: 0,
    };
    expect(formatSummary(summary)).toBe('No commands to analyse');
  });

  it('handles all passed', () => {
    const summary: CommandAnalysisSummary = {
      total: 5,
      blocked: 0,
      warned: 0,
      allowed: 5,
    };
    expect(formatSummary(summary)).toBe('All 5 command(s) passed safety check');
  });

  it('handles warnings only', () => {
    const summary: CommandAnalysisSummary = {
      total: 5,
      blocked: 0,
      warned: 2,
      allowed: 3,
    };
    expect(formatSummary(summary)).toBe('5 command(s) analysed: 2 warning(s)');
  });

  it('handles blocked commands', () => {
    const summary: CommandAnalysisSummary = {
      total: 5,
      blocked: 1,
      warned: 2,
      allowed: 2,
    };
    expect(formatSummary(summary)).toBe(
      'Command safety check failed: 1 blocked, 2 warning(s) of 5 total'
    );
  });
});
