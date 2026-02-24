import { describe, it, expect } from 'vitest';
import { CliError, CliExit } from './cli-error.js';

describe('CliError', () => {
  it('extends Error', () => {
    const err = new CliError('something failed');
    expect(err).toBeInstanceOf(Error);
    expect(err).toBeInstanceOf(CliError);
  });

  it('has name "CliError"', () => {
    expect(new CliError('x').name).toBe('CliError');
  });

  it('stores message', () => {
    expect(new CliError('disk full').message).toBe('disk full');
  });

  it('defaults exitCode to 1', () => {
    expect(new CliError('fail').exitCode).toBe(1);
  });

  it('accepts a custom exitCode', () => {
    expect(new CliError('fail', 2).exitCode).toBe(2);
  });

  it('is catchable in async flows', async () => {
    const action = async () => {
      throw new CliError('bad input');
    };
    await expect(action()).rejects.toThrow(CliError);
    await expect(action()).rejects.toThrow('bad input');
  });

  it('is not an instance of CliExit', () => {
    expect(new CliError('x')).not.toBeInstanceOf(CliExit);
  });
});

describe('CliExit', () => {
  it('extends Error', () => {
    const exit = new CliExit();
    expect(exit).toBeInstanceOf(Error);
    expect(exit).toBeInstanceOf(CliExit);
  });

  it('has name "CliExit"', () => {
    expect(new CliExit().name).toBe('CliExit');
  });

  it('has exitCode 0', () => {
    expect(new CliExit().exitCode).toBe(0);
  });

  it('has default message "Clean exit"', () => {
    expect(new CliExit().message).toBe('Clean exit');
  });

  it('accepts a custom message', () => {
    expect(new CliExit('done early').message).toBe('done early');
  });

  it('is not an instance of CliError', () => {
    expect(new CliExit()).not.toBeInstanceOf(CliError);
  });

  it('is catchable in async flows', async () => {
    const action = async () => {
      throw new CliExit();
    };
    await expect(action()).rejects.toThrow(CliExit);
  });
});

describe('CliError vs CliExit disambiguation', () => {
  it('can be distinguished with instanceof in a catch block', () => {
    const errors: Array<CliError | CliExit> = [new CliError('fail'), new CliExit()];

    for (const err of errors) {
      if (err instanceof CliError) {
        expect(err.exitCode).toBeGreaterThanOrEqual(1);
      } else if (err instanceof CliExit) {
        expect(err.exitCode).toBe(0);
      }
    }
  });
});
