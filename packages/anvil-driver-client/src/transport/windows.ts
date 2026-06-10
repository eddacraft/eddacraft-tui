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
 * Trust anchor: the gate is only as trustworthy as the resolved SID.
 * The default provider executes `%SystemRoot%\System32\whoami.exe` by
 * absolute path (never a PATH lookup) and blocks the event loop for at
 * most one resolution per process (cached). Two contexts MUST inject
 * their own provider instead: services using thread-level
 * impersonation (`whoami` reports the thread token, and the
 * process-lifetime cache would pin the first identity it sees), and
 * non-Windows hosts driving this transport with an explicit `pipeName`
 * (there is no SID to resolve, so the default provider fails the gate
 * CLOSED — inject the SID the rig expects, or use a fake transport).
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
 * String-form SID pattern (`S-1-<authority>-<subauths…>`). The /g flag
 * is for multi-match collection via `String.prototype.match`; matched
 * on word boundaries so it lifts the SID column out of table/list
 * `whoami /user` output without depending on localised column headers.
 */
const SID_PATTERN = /\bS-1-\d+(?:-\d+)+\b/g;

/** Anchored SID-shape check for a value that must be exactly one SID. */
const FULL_SID_PATTERN = /^S-1-\d+(?:-\d+)+$/i;

/**
 * Extract the current user's SID from `whoami /user` output. Exported
 * for tests; `null` means no SID could be confidently extracted.
 *
 * CSV (`/fo csv /nh`, the form {@link resolveCurrentUserSid} pins): the
 * SID is the last quoted field of the first data line. The parse is
 * anchored to that structure so an SID-shaped token anywhere else — a
 * crafted user name, or a trailing integrity-level SID such as
 * `S-1-16-4096` if the invocation ever drifts — can never win. A
 * CSV-shaped line whose last field is not a SID returns `null` (fail
 * closed) rather than falling through to a guess.
 *
 * Non-CSV fallback (table/list formats): the account-SID column is
 * last in both, so take the last SID-shaped token.
 */
export function parseSidFromWhoamiOutput(output: string): string | null {
  const firstLine = output.split(/\r?\n/).find((line) => line.trim().length > 0);
  if (firstLine !== undefined) {
    const quotedFields = firstLine.match(/"[^"]*"/g);
    if (quotedFields !== null && quotedFields.length >= 2) {
      const lastField = quotedFields[quotedFields.length - 1]!.slice(1, -1);
      return FULL_SID_PATTERN.test(lastField) ? lastField : null;
    }
  }
  const matches = output.match(SID_PATTERN);
  return matches === null ? null : matches[matches.length - 1]!;
}

/** Process-lifetime cache: a (non-impersonating) process's user SID
 *  cannot change, and the factory builds a fresh transport per
 *  reconnect attempt — without the cache every reconnect would spawn
 *  `whoami`. Only successful resolutions are cached; failures retry.
 *  CAVEAT: direct calls to {@link resolveCurrentUserSid} share this
 *  cache — tests must use the {@link WindowsTransportOptions}
 *  injection seam, and impersonating services must inject a
 *  thread-token-derived provider (see module header). */
let cachedWhoamiSid: string | null = null;

/**
 * Default current-user SID provider: `%SystemRoot%\System32\whoami.exe
 * /user /fo csv /nh`, executed by ABSOLUTE path — a PATH or CWD lookup
 * would let a planted `whoami` feed the gate an attacker-chosen SID.
 * Throws a structured `anvil-daemon-wrong-owner` error when the SID
 * cannot be resolved — the ownership gate fails closed (including on
 * non-Windows hosts, where the binary does not exist).
 */
export function resolveCurrentUserSid(): string {
  if (cachedWhoamiSid !== null) {
    return cachedWhoamiSid;
  }
  // Trust SystemRoot only when it is an absolute drive path — a
  // relative or UNC-ish value would reintroduce the CWD/PATH planting
  // risk the absolute invocation exists to close.
  const envRoot = process.env['SystemRoot'];
  const systemRoot =
    envRoot !== undefined && /^[A-Za-z]:\\/.test(envRoot) ? envRoot : 'C:\\Windows';
  const whoamiPath = `${systemRoot}\\System32\\whoami.exe`;
  let output: string;
  try {
    output = execFileSync(whoamiPath, ['/user', '/fo', 'csv', '/nh'], {
      encoding: 'utf8',
      timeout: 5_000,
      windowsHide: true,
    });
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    throw driverError(
      'anvil-daemon-wrong-owner',
      `cannot resolve current user SID (whoami /user failed): ${message}`
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
 * Ownership half of the pre-connect gate: the pipe name's SID suffix
 * must be the current user's SID. Self-contained — re-guards the
 * prefix and shape-validates the resolved SID so a malformed provider
 * return (empty string, user name, truncated value) can never
 * accidentally match a crafted suffix. SID string comparison is
 * case-insensitive — string SIDs are canonically upper-case `S-…` but
 * case carries no identity on Windows.
 */
export function validateWindowsPipeOwnership(pipeName: string, currentUserSid: string): void {
  if (!pipeName.startsWith(PIPE_PREFIX)) {
    throw driverError(
      'anvil-daemon-wrong-owner',
      `pipe name does not match daemon-bound pattern '${PIPE_PREFIX}<sid>': ${pipeName}`
    );
  }
  if (!FULL_SID_PATTERN.test(currentUserSid)) {
    throw driverError(
      'anvil-daemon-wrong-owner',
      'resolved current-user SID is not a canonical S-1-… SID string'
    );
  }
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
      const message = err instanceof Error ? err.message : String(err);
      throw driverError('anvil-daemon-wrong-owner', `cannot resolve current user SID: ${message}`);
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
