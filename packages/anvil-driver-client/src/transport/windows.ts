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
 *   2. The deeper ACL check is deferred to a follow-up item alongside
 *      INTD-012 Windows CI matrix work; a documented gap is preferable
 *      to a false-comfort check that always returns OK.
 *
 * If the consumer needs a stronger Windows check before the follow-up
 * lands, they can supply a custom transport via the
 * `transportFactory` option on {@link DriverClient} and run their own
 * Win32 ACL inspection — the daemon's listener-side ACL is the
 * authoritative gate.
 */

import net from 'node:net';

import { driverError } from '../errors.js';
import type { Transport, TransportCloseCause, TransportHandlers } from './types.js';

const PIPE_PREFIX = '\\\\.\\pipe\\anvil-intercept-';

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
  private socket: net.Socket | null = null;
  private handlers: TransportHandlers | null = null;
  private closed = false;
  private closeFired = false;
  private writePromises: Array<() => void> = [];

  public constructor(pipeName: string) {
    this.pipeName = pipeName;
  }

  public async connect(handlers: TransportHandlers): Promise<void> {
    if (this.handlers !== null) {
      throw new TypeError('WindowsNamedPipeTransport.connect: already connected');
    }
    if (this.closed) {
      throw driverError('anvil-driver-closed', 'transport already closed');
    }

    validateWindowsPipeName(this.pipeName);
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
