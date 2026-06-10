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

  it('refuses a malformed current-user SID instead of comparing it', () => {
    // A provider returning an empty string or a user name must never
    // be able to match a crafted suffix — the shape gate fails closed.
    expect(() => validateWindowsPipeOwnership(pipeFor(''), '')).toThrowError(DriverClientError);
    expect(() => validateWindowsPipeOwnership(pipeFor('josh'), 'josh')).toThrowError(
      DriverClientError
    );
  });

  it('is self-contained: refuses a pipe name without the daemon prefix', () => {
    expect(() =>
      validateWindowsPipeOwnership('\\\\.\\pipe\\rogue-server', CURRENT_SID)
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

  it('extracts the SID from list-format `whoami /user /fo list` output', () => {
    expect(
      parseSidFromWhoamiOutput(`User Name: desktop\\josh\r\nSID:       ${CURRENT_SID}\r\n`)
    ).toBe(CURRENT_SID);
  });

  it('is not fooled by a trailing integrity-level SID after the CSV line', () => {
    // If the invocation ever drifts towards `/all`-style output, the
    // account SID must still win over an appended S-1-16-… token.
    expect(parseSidFromWhoamiOutput(`"desktop\\josh","${CURRENT_SID}"\r\nS-1-16-4096\r\n`)).toBe(
      CURRENT_SID
    );
  });

  it('is not fooled by an SID-shaped user name in the CSV user column', () => {
    expect(parseSidFromWhoamiOutput(`"${OTHER_SID}","${CURRENT_SID}"\r\n`)).toBe(CURRENT_SID);
  });

  it('fails closed when the CSV SID column is not a SID', () => {
    expect(parseSidFromWhoamiOutput(`"desktop\\josh","not-a-sid"\r\n`)).toBeNull();
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
    // passed gate surfaces as unavailable — NOT wrong-owner. The
    // provider call count proves the gate actually executed (an
    // ENOENT alone could also mean the validators were skipped).
    let providerCalls = 0;
    const transport = new WindowsNamedPipeTransport(pipeFor(CURRENT_SID), {
      currentUserSid: () => {
        providerCalls += 1;
        return CURRENT_SID;
      },
    });
    await expect(transport.connect(handlers)).rejects.toMatchObject({
      code: 'anvil-daemon-unavailable',
    });
    expect(providerCalls).toBe(1);
  });

  it('is not left in the already-connected state after a pre-socket owner rejection', async () => {
    // The gate throws before handlers are registered; a second
    // connect() must hit the gate again, not the "already connected"
    // TypeError.
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

  it('fails closed when the provider returns a non-SID value', async () => {
    const transport = new WindowsNamedPipeTransport(pipeFor(CURRENT_SID), {
      currentUserSid: () => 'desktop\\josh',
    });
    await expect(transport.connect(handlers)).rejects.toMatchObject({
      code: 'anvil-daemon-wrong-owner',
    });
  });

  it('maps a non-Error provider throw to a structured wrong-owner rejection', async () => {
    const transport = new WindowsNamedPipeTransport(pipeFor(CURRENT_SID), {
      currentUserSid: () => {
        throw 'string throw';
      },
    });
    await expect(transport.connect(handlers)).rejects.toMatchObject({
      code: 'anvil-daemon-wrong-owner',
    });
  });
});
