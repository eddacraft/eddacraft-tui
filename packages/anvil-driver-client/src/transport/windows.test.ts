/**
 * Windows pipe-name validation tests. Pure: doesn't open any pipe —
 * the ownership-gate transport tests inject a SID provider and only
 * ever observe a rejected `connect()`, so they run on any platform.
 */

import { describe, expect, it } from 'vitest';

import { DriverClientError } from '../errors.js';
import {
  parseSidFromWhoamiOutput,
  validateWindowsPipeName,
  validateWindowsPipeOwnership,
  WindowsNamedPipeTransport,
} from './windows.js';

describe('validateWindowsPipeName', () => {
  it('accepts the canonical SID-suffixed pipe name', () => {
    expect(() =>
      validateWindowsPipeName('\\\\.\\pipe\\anvil-intercept-S-1-5-21-1234')
    ).not.toThrow();
  });

  it('refuses a pipe name without the daemon prefix', () => {
    let err: unknown;
    try {
      validateWindowsPipeName('\\\\.\\pipe\\rogue-server');
    } catch (e) {
      err = e;
    }
    expect(err).toBeInstanceOf(DriverClientError);
    expect((err as DriverClientError).code).toBe('anvil-daemon-wrong-owner');
  });

  it('refuses an empty SID suffix', () => {
    expect(() => validateWindowsPipeName('\\\\.\\pipe\\anvil-intercept-')).toThrowError(
      DriverClientError
    );
  });

  it('refuses a SID suffix containing whitespace or path separators', () => {
    expect(() => validateWindowsPipeName('\\\\.\\pipe\\anvil-intercept-foo bar')).toThrowError(
      DriverClientError
    );
    expect(() => validateWindowsPipeName('\\\\.\\pipe\\anvil-intercept-foo\\bar')).toThrowError(
      DriverClientError
    );
    expect(() => validateWindowsPipeName('\\\\.\\pipe\\anvil-intercept-foo/bar')).toThrowError(
      DriverClientError
    );
  });
});

const CURRENT_SID = 'S-1-5-21-3623811015-3361044348-30300820-1013';
const OTHER_SID = 'S-1-5-21-1111111111-2222222222-3333333333-1001';
const pipeFor = (sid: string): string => `\\\\.\\pipe\\anvil-intercept-${sid}`;

describe('validateWindowsPipeOwnership', () => {
  it('accepts a suffix matching the current user SID', () => {
    expect(() => validateWindowsPipeOwnership(pipeFor(CURRENT_SID), CURRENT_SID)).not.toThrow();
  });

  it('accepts a matching SID regardless of case', () => {
    expect(() =>
      validateWindowsPipeOwnership(pipeFor(CURRENT_SID.toLowerCase()), CURRENT_SID)
    ).not.toThrow();
  });

  it("refuses a suffix that is another user's SID", () => {
    let err: unknown;
    try {
      validateWindowsPipeOwnership(pipeFor(OTHER_SID), CURRENT_SID);
    } catch (e) {
      err = e;
    }
    expect(err).toBeInstanceOf(DriverClientError);
    expect((err as DriverClientError).code).toBe('anvil-daemon-wrong-owner');
  });

  it('refuses a SID that is a strict prefix of the current user SID', () => {
    expect(() =>
      validateWindowsPipeOwnership(pipeFor(CURRENT_SID.slice(0, -2)), CURRENT_SID)
    ).toThrowError(DriverClientError);
  });
});

describe('parseSidFromWhoamiOutput', () => {
  it('extracts the SID from `whoami /user /fo csv /nh` output', () => {
    expect(parseSidFromWhoamiOutput(`"desktop\\josh","${CURRENT_SID}"\r\n`)).toBe(CURRENT_SID);
  });

  it('extracts the SID from table-format `whoami /user` output', () => {
    const table = [
      'USER INFORMATION',
      '----------------',
      '',
      'User Name      SID',
      '============== ==============================================',
      `desktop\\josh   ${CURRENT_SID}`,
      '',
    ].join('\r\n');
    expect(parseSidFromWhoamiOutput(table)).toBe(CURRENT_SID);
  });

  it('returns null when no SID is present', () => {
    expect(parseSidFromWhoamiOutput('josh\n')).toBeNull();
    expect(parseSidFromWhoamiOutput('')).toBeNull();
  });
});

describe('WindowsNamedPipeTransport ownership gate', () => {
  const handlers = { onData: (): void => {}, onClose: (): void => {} };

  it("rejects connect() to another user's pipe with anvil-daemon-wrong-owner", async () => {
    const transport = new WindowsNamedPipeTransport(pipeFor(OTHER_SID), {
      currentUserSid: () => CURRENT_SID,
    });
    await expect(transport.connect(handlers)).rejects.toMatchObject({
      code: 'anvil-daemon-wrong-owner',
    });
  });

  it('fails closed when the current user SID cannot be resolved', async () => {
    const transport = new WindowsNamedPipeTransport(pipeFor(CURRENT_SID), {
      currentUserSid: () => {
        throw new Error('whoami exploded');
      },
    });
    await expect(transport.connect(handlers)).rejects.toMatchObject({
      code: 'anvil-daemon-wrong-owner',
    });
  });

  it('passes the gate on a SID match and proceeds to the connection attempt', async () => {
    // No daemon listens on this path in the test environment, so a
    // passed gate surfaces as unavailable — NOT wrong-owner. That
    // ordering is the assertion: the gate ran and let the name through.
    const transport = new WindowsNamedPipeTransport(pipeFor(CURRENT_SID), {
      currentUserSid: () => CURRENT_SID,
    });
    await expect(transport.connect(handlers)).rejects.toMatchObject({
      code: 'anvil-daemon-unavailable',
    });
  });

  it('can still connect() after a wrong-owner rejection was raised pre-socket', async () => {
    // The gate throws before handlers are registered; the transport
    // must not be left half-connected ("already connected" trap).
    const transport = new WindowsNamedPipeTransport(pipeFor(OTHER_SID), {
      currentUserSid: () => CURRENT_SID,
    });
    await expect(transport.connect(handlers)).rejects.toMatchObject({
      code: 'anvil-daemon-wrong-owner',
    });
    await expect(transport.connect(handlers)).rejects.toMatchObject({
      code: 'anvil-daemon-wrong-owner',
    });
  });
});
