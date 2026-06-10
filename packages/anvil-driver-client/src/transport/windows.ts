/**
 * Windows named-pipe transport.
 *
 * Node treats `\\.\pipe\<name>` as a regular `net.Socket`, so the
 * `connect` / `send` / `close` plumbing is structurally the same as
 * the Unix transport. The owner check is platform-specific:
 *
 * - **UDS (Linux/macOS):** stat the socket path and require
 *   mode-0600 + current-uid ownership.
 * - **Named pipe (Windows):** the equivalent gate is the pipe's ACL.
 *   Node does not surface the pipe SD via `fs.statSync`, and a pure-JS
 *   ACL-walk is non-trivial. The contract this module ships:
 *
 *   1. The pipe name MUST be the SID-derived form
 *      `\\.\pipe\anvil-intercept-<sid>` — the daemon refuses to bind
 *      any other name. Connecting to a path that doesn't match the
 *      pattern is rejected client-side as
 *      `anvil-daemon-wrong-owner`.
 *   2. The `<sid>` suffix MUST be the **current user's** SID. The
 *      daemon only ever binds `anvil-intercept-<current-user-sid>`
 *      (`crates/anvil-intercept-win32::pipe_name_for_current_user`),
 *      so a name carrying any other SID is by definition not this
 *      user's daemon. The SID is resolved via `whoami /user`
 *      (injectable for tests and richer consumers via
 *      {@link WindowsTransportOptions.currentUserSid}); the gate fails
 *      CLOSED — if the SID cannot be resolved, connect() rejects as
 *      `anvil-daemon-wrong-owner` rather than skipping the check.
 *   3. The deeper server-identity check (pipe security descriptor /
 *      `GetNamedPipeServerProcessId`) needs native access and stays a
 *      documented gap, tracked in issue #2484: a local attacker who
 *      pre-creates the correctly-named pipe before the daemon binds is
 *      not detected client-side. The daemon-side owner-only DACL plus
 *      client-SID check (DSV-010b / ADR-070) remains the authoritative
 *      gate.
 *
 * If the consumer needs a stronger Windows check before the #2484
 * follow-up lands, they can supply a custom transport via the
 * `transportFactory` option on {@link DriverClient} and run their own
 * Win32 ACL inspection.
 */

import { execFileSync } from 'node:child_process';
import net from 'node:net';

import { DriverClientError, driverError } from '../errors.js';
import type { Transport, TransportCloseCause, TransportHandlers } from './types.js';

const PIPE_PREFIX = '\\\\.\\pipe\\anvil-intercept-';

/**
 * String-form SID pattern (`S-1-<authority>-<subauths…>`). Matched on
 * word boundaries so it lifts the SID column out of any `whoami /user`
 * output format (table, csv, list) without depending on localised
 * column headers.
 */
const SID_PATTERN = /\bS-1-\d+(?:-\d+)+\b/gi;

/**
 * Extract the current user's SID from `whoami /user` output. Returns
 * the LAST match: the SID column follows the user-name column in every
 * `whoami` format, and a user name could in principle embed an
 * SID-shaped substring. Exported for tests; `null` means no SID found.
 */
export function parseSidFromWhoamiOutput(output: string): string | null {
  const matches = output.match(SID_PATTERN);
  return matches === null ? null : matches[matches.length - 1]!;
}

/** Process-lifetime cache: a user's SID cannot change under a running
 *  process, and the factory builds a fresh transport per reconnect
 *  attempt — without the cache every reconnect would spawn `whoami`. */
let cachedWhoamiSid: string | null = null;

/**
 * Default current-user SID provider: `whoami /user /fo csv /nh`.
 * Throws a structured `anvil-daemon-wrong-owner` error when the SID
 * cannot be resolved — the ownership gate fails closed.
 */
export function resolveCurrentUserSid(): string {
  if (cachedWhoamiSid !== null) {
    return cachedWhoamiSid;
  }
  let output: string;
  try {
    output = execFileSync('whoami', ['/user', '/fo', 'csv', '/nh'], {
      encoding: 'utf8',
      timeout: 5_000,
      windowsHide: true,
    });
  } catch (err) {
    throw driverError(
      'anvil-daemon-wrong-owner',
      `cannot resolve current user SID (whoami /user failed): ${(err as Error).message}`
    );
  }
  const sid = parseSidFromWhoamiOutput(output);
  if (sid === null) {
    throw driverError(
      'anvil-daemon-wrong-owner',
      'cannot resolve current user SID: no SID in whoami output'
    );
  }
  cachedWhoamiSid = sid;
  return sid;
}

/**
 * Ownership half of the pre-connect gate: the (shape-valid) pipe
 * name's SID suffix must be the current user's SID. SID string
 * comparison is case-insensitive — string SIDs are canonically
 * upper-case `S-…` but case carries no identity on Windows.
 */
export function validateWindowsPipeOwnership(pipeName: string, currentUserSid: string): void {
  const suffix = pipeName.slice(PIPE_PREFIX.length);
  if (suffix.toUpperCase() !== currentUserSid.toUpperCase()) {
    throw driverError(
      'anvil-daemon-wrong-owner',
      `pipe name SID suffix does not match the current user's SID: ${pipeName}`
    );
  }
}

/**
 * Construction-time options for {@link WindowsNamedPipeTransport}.
 */
export interface WindowsTransportOptions {
  /** Current-user SID provider, overriding the default `whoami /user`
   *  resolution. Tests inject a fixed value; a consumer with native
   *  Win32 access can supply a token-derived SID. Must return the
   *  canonical `S-1-…` string form; thrown errors fail the gate
   *  closed (`anvil-daemon-wrong-owner`). */
  currentUserSid?: () => string;
}

/**
 * Pre-connect pipe-name validation. Throws a structured
 * {@link import('../errors.js').DriverClientError} on failure.
 *
 * The check is intentionally narrow — it confirms only that the pipe
 * name follows the daemon's documented `anvil-intercept-<sid>`
 * pattern. The deeper ACL check is documented as a deferred gap (see
 * the module header).
 */
export function validateWindowsPipeName(pipeName: string): void {
  if (!pipeName.startsWith(PIPE_PREFIX)) {
    throw driverError(
      'anvil-daemon-wrong-owner',
      `pipe name does not match daemon-bound pattern '${PIPE_PREFIX}<sid>': ${pipeName}`
    );
  }
  const suffix = pipeName.slice(PIPE_PREFIX.length);
  if (suffix.length === 0) {
    throw driverError('anvil-daemon-wrong-owner', `pipe name has empty SID suffix: ${pipeName}`);
  }
  // Defensive: refuse paths containing whitespace or path separators
  // — the daemon's bind name is a single SID component, never a
  // nested path. This blocks naive name-injection through env-var
  // overrides.
  if (/[\\/\s]/u.test(suffix)) {
    throw driverError(
      'anvil-daemon-wrong-owner',
      `pipe name SID suffix contains an invalid character: ${pipeName}`
    );
  }
}

export class WindowsNamedPipeTransport implements Transport {
  private readonly pipeName: string;
  private readonly currentUserSid: () => string;
  private socket: net.Socket | null = null;
  private handlers: TransportHandlers | null = null;
  private closed = false;
  private closeFired = false;
  private writePromises: Array<() => void> = [];

  public constructor(pipeName: string, options: WindowsTransportOptions = {}) {
    this.pipeName = pipeName;
    this.currentUserSid = options.currentUserSid ?? resolveCurrentUserSid;
  }

  public async connect(handlers: TransportHandlers): Promise<void> {
    if (this.handlers !== null) {
      throw new TypeError('WindowsNamedPipeTransport.connect: already connected');
    }
    if (this.closed) {
      throw driverError('anvil-driver-closed', 'transport already closed');
    }

    validateWindowsPipeName(this.pipeName);
    validateWindowsPipeOwnership(this.pipeName, this.resolveSidFailClosed());
    this.handlers = handlers;

    await new Promise<void>((resolve, reject) => {
      const sock = net.createConnection(this.pipeName);
      let settled = false;

      sock.once('connect', () => {
        if (settled) {
          return;
        }
        settled = true;
        this.socket = sock;
        this.attachStreamHandlers(sock);
        resolve();
      });

      sock.once('error', (err) => {
        if (settled) {
          return;
        }
        settled = true;
        this.handlers = null;
        const code = (err as NodeJS.ErrnoException).code;
        if (code === 'ENOENT' || code === 'ECONNREFUSED') {
          reject(
            driverError(
              'anvil-daemon-unavailable',
              `cannot connect to ${this.pipeName}: ${err.message}`
            )
          );
          return;
        }
        if (code === 'EACCES' || code === 'EPERM') {
          reject(
            driverError(
              'anvil-daemon-wrong-owner',
              `cannot connect to ${this.pipeName}: ${err.message}`
            )
          );
          return;
        }
        reject(
          driverError(
            'anvil-daemon-unavailable',
            `cannot connect to ${this.pipeName}: ${err.message}`,
            { data: { code } }
          )
        );
      });
    });
  }

  public async send(chunk: string): Promise<void> {
    if (this.closed || this.socket === null) {
      throw driverError('anvil-daemon-transport-drop', 'transport closed before send');
    }
    return new Promise<void>((resolve, reject) => {
      const ok = this.socket!.write(chunk, 'utf8', (err) => {
        if (err) {
          reject(driverError('anvil-daemon-transport-drop', `pipe write failed: ${err.message}`));
        }
      });
      if (ok) {
        resolve();
        return;
      }
      this.writePromises.push(resolve);
    });
  }

  public async close(): Promise<void> {
    if (this.closed) {
      return;
    }
    this.closed = true;
    if (this.socket === null) {
      this.fireClose('local');
      return;
    }
    this.socket.end();
  }

  /** Run the injected SID provider, mapping any non-structured throw
   *  to a fail-closed `anvil-daemon-wrong-owner` rejection. */
  private resolveSidFailClosed(): string {
    try {
      return this.currentUserSid();
    } catch (err) {
      if (err instanceof DriverClientError) {
        throw err;
      }
      throw driverError(
        'anvil-daemon-wrong-owner',
        `cannot resolve current user SID: ${(err as Error).message}`
      );
    }
  }

  private attachStreamHandlers(sock: net.Socket): void {
    sock.on('data', (chunk: Buffer) => {
      this.handlers?.onData(chunk);
    });
    sock.on('drain', () => {
      const pending = this.writePromises;
      this.writePromises = [];
      for (const resolve of pending) {
        resolve();
      }
    });
    sock.on('error', () => {
      this.fireClose('error');
    });
    sock.on('close', () => {
      this.fireClose(this.closed ? 'local' : 'peer');
    });
  }

  private fireClose(cause: TransportCloseCause): void {
    if (this.closeFired) {
      return;
    }
    this.closeFired = true;
    for (const resolve of this.writePromises) {
      resolve();
    }
    this.writePromises = [];
    const handlers = this.handlers;
    this.handlers = null;
    handlers?.onClose(cause);
  }
}
