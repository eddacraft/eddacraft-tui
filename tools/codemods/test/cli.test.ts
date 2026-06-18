import { describe, expect, it } from 'vitest';

import { printImportsCompletion } from '../src/imports-completion.js';

describe('printImportsCompletion', () => {
  it('sets a failing exit code without printing the success message when errors occur', () => {
    const output: string[] = [];
    const exitCodes: number[] = [];

    printImportsCompletion({
      dryRun: false,
      totalErrors: 2,
      log: (message) => output.push(String(message)),
      setExitCode: (code) => exitCodes.push(code),
    });

    const joinedOutput = output.join('\n');

    expect(exitCodes).toEqual([1]);
    expect(joinedOutput).toContain('Codemod completed with errors');
    expect(joinedOutput).not.toContain('Changes applied successfully');
  });
});
