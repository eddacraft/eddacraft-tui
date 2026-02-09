import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { Warning } from '@eddacraft/anvil-core/antipattern';
import { runInteractiveReview } from '../check.js';

function makeWarning(overrides: Partial<Warning> = {}): Warning {
  return {
    id: 'AP-003',
    category: 'anti-pattern',
    severity: 'warning',
    confidence: 'high',
    title: 'Explicit any type usage',
    message: 'Found Explicit any type at line 5',
    explanation: 'Using any defeats type checking',
    suggestion: 'Use unknown instead',
    nudge: "Don't use `any` here.",
    location: { file: 'src/foo.ts', line: 5, column: 10 },
    pattern: 'AP-003',
    ...overrides,
  };
}

describe('runInteractiveReview', () => {
  beforeEach(() => {
    vi.spyOn(console, 'log').mockImplementation(() => {});
  });

  it('should return empty results for empty warnings', async () => {
    const results = await runInteractiveReview([], vi.fn());
    expect(results).toEqual([]);
  });

  it('should skip suppressed warnings', async () => {
    const warnings = [
      makeWarning({
        suppressed: { reason: 'Legacy', scope: 'statement' },
      }),
    ];

    const results = await runInteractiveReview(warnings, vi.fn());
    expect(results).toEqual([]);
  });

  it('should call prompt for each non-suppressed warning', async () => {
    const warnings = [makeWarning(), makeWarning({ id: 'AP-004', title: '@ts-ignore' })];
    const promptFn = vi.fn().mockResolvedValue('skip');

    const results = await runInteractiveReview(warnings, promptFn);

    expect(promptFn).toHaveBeenCalledTimes(2);
    expect(results).toHaveLength(2);
    expect(results[0].action).toBe('skip');
    expect(results[1].action).toBe('skip');
  });

  it('should stop on quit action', async () => {
    const warnings = [makeWarning(), makeWarning(), makeWarning()];
    const promptFn = vi.fn().mockResolvedValueOnce('skip').mockResolvedValueOnce('quit');

    const results = await runInteractiveReview(warnings, promptFn);

    expect(promptFn).toHaveBeenCalledTimes(2);
    expect(results).toHaveLength(2);
    expect(results[1].action).toBe('quit');
  });

  it('should include fix choice only for fixable patterns', async () => {
    // AP-004 is fixable, AP-003 is not
    const warnings = [
      makeWarning({ id: 'AP-003' }),
      makeWarning({ id: 'AP-004', title: '@ts-ignore' }),
    ];

    const capturedChoices: Array<Array<{ name: string; value: string }>> = [];
    const promptFn = vi.fn().mockImplementation(async (choices) => {
      capturedChoices.push(choices);
      return 'skip';
    });

    await runInteractiveReview(warnings, promptFn);

    // AP-003: should NOT have fix option
    const ap003Choices = capturedChoices[0].map((c) => c.value);
    expect(ap003Choices).not.toContain('fix');

    // AP-004: should have fix option
    const ap004Choices = capturedChoices[1].map((c) => c.value);
    expect(ap004Choices).toContain('fix');
  });

  it('should always include skip, suppress, and quit choices', async () => {
    const warnings = [makeWarning()];

    let capturedChoices: Array<{ name: string; value: string }> = [];
    const promptFn = vi.fn().mockImplementation(async (choices) => {
      capturedChoices = choices;
      return 'skip';
    });

    await runInteractiveReview(warnings, promptFn);

    const values = capturedChoices.map((c) => c.value);
    expect(values).toContain('skip');
    expect(values).toContain('suppress');
    expect(values).toContain('quit');
  });

  it('should record fix and suppress actions', async () => {
    const warnings = [
      makeWarning({ id: 'AP-004', title: '@ts-ignore' }),
      makeWarning({ id: 'AP-003' }),
    ];

    const promptFn = vi.fn().mockResolvedValueOnce('fix').mockResolvedValueOnce('suppress');

    const results = await runInteractiveReview(warnings, promptFn);

    expect(results[0].action).toBe('fix');
    expect(results[1].action).toBe('suppress');
  });
});
