/**
 * Unix domain socket transport.
 *
 * Mirrors the daemon's owner-only invariant
 * (`crates/anvil-intercept/src/ipc.rs::validate_socket_path_for_client`).
 * Specifically the client-side checks performed BEFORE writing the
 * first byte:
 *   - The socket path's parent directory exists, is not a symlink, is
 *     mode 0700, and is owned by the current user.
 *   - The socket path itself is not a symlink, is a socket file, is
 *     mode 0600, and is owned by the current user.
 *
 * If any check fails, `connect()` rejects with
 * `anvil-daemon-wrong-owner` (or `anvil-daemon-unavailable` if the
 * path simply does not exist). The connection is never established
 * with a hostile peer.
 *
 * The brief flags this as security-critical:
 * > Driver-side, `DriverClient.connect()` refuses a socket / named
 * > pipe that is not owned by the current user (matches INTD-002's
 * > daemon-side permissioning).
 *
 * Linux additionally exposes `SO_PEERCRED` on a connected stream;
 * verifying the peer there belongs to a future hardening pass once
 * Node exposes the equivalent (see "Limitations" below).
 */

import net from 'node:net';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';

import { driverError, DriverClientError } from '../errors.js';
import type { Transport, TransportCloseCause, TransportHandlers } from './types.js';

/**
 * Perform the pre-connect path validation. Returns void on success,
 * throws a {@link DriverClientError} on failure.
 *
 * Pure-ish: stats the filesystem but does not connect. Exposed for
 * tests so the wrong-owner branch can be exercised deterministically.
 */
export function validateUnixSocketOwnership(socketPath: string): void {
  // Parent directory: must exist, not be a symlink, be mode 0700 and
  // owner-current-user.
  const parent = path.dirname(socketPath);
  let parentStat: fs.Stats;
  try {
    parentStat = fs.lstatSync(parent);
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') {
      throw driverError(
        'anvil-daemon-unavailable',
        `socket parent directory does not exist: ${parent}`
      );
    }
    throw driverError(
      'anvil-daemon-unavailable',
      `cannot stat socket parent directory ${parent}: ${(err as Error).message}`
    );
  }
  if (parentStat.isSymbolicLink()) {
    throw driverError(
      'anvil-daemon-wrong-owner',
      `socket parent directory is a symlink: ${parent}`
    );
  }
  if (!parentStat.isDirectory()) {
    throw driverError(
      'anvil-daemon-wrong-owner',
      `socket parent path is not a directory: ${parent}`
    );
  }
  const currentUid = process.getuid?.() ?? -1;
  if (currentUid < 0) {
    // process.getuid is undefined on Windows. We don't reach here on
    // Windows in normal flow (the path resolver returns a pipe), but
    // be defensive — refuse rather than skip the check.
    throw driverError(
      'anvil-daemon-wrong-owner',
      'cannot determine current user id; refusing to connect to Unix socket'
    );
  }
  if ((parentStat.mode & 0o777) !== 0o700 || parentStat.uid !== currentUid) {
    throw driverError(
      'anvil-daemon-wrong-owner',
      `socket parent directory ${parent} has wrong permissions: mode=${(parentStat.mode & 0o777).toString(8)}, owner=${parentStat.uid}, current=${currentUid}`
    );
  }

  // Socket file: must exist, not be a symlink, be a socket, mode
  // 0600, owner-current-user.
  let stat: fs.Stats;
  try {
    stat = fs.lstatSync(socketPath);
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') {
      throw driverError('anvil-daemon-unavailable', `socket path does not exist: ${socketPath}`);
    }
    throw driverError(
      'anvil-daemon-unavailable',
      `cannot stat socket path ${socketPath}: ${(err as Error).message}`
    );
  }
  if (stat.isSymbolicLink()) {
    throw driverError('anvil-daemon-wrong-owner', `socket path is a symlink: ${socketPath}`);
  }
  if (!stat.isSocket()) {
    throw driverError(
      'anvil-daemon-wrong-owner',
      `socket path is not a socket file: ${socketPath}`
    );
  }
  if ((stat.mode & 0o777) !== 0o600 || stat.uid !== currentUid) {
    throw driverError(
      'anvil-daemon-wrong-owner',
      `socket path ${socketPath} has wrong permissions: mode=${(stat.mode & 0o777).toString(8)}, owner=${stat.uid}, current=${currentUid}`
    );
  }
}

/**
 * Concrete Unix-domain-socket transport. Owner-checks the socket
 * before `net.createConnection`.
 *
 * Limitations:
 * - Node does not expose `SO_PEERCRED` on a connected stream as a
 *   stable API. The pre-connect path-stat check is the v1 gate the
 *   brief asks for; a follow-up can call into the daemon via a `peer
 *   identity` introspection RPC if a stronger gate is needed
 *   post-connect. Until then, an attacker who unlinks and re-binds a
 *   socket between our `lstat` and the kernel's `connect()` syscall
 *   wins a TOCTOU race; the daemon's own peer-credential check on
 *   the listener side blocks them from doing anything destructive,
 *   so this is documented rather than masked.
 * - `os.platform() === 'linux' || 'darwin' || ...` is the supported
 *   set; other Unixes go through the same code path and rely on the
 *   same `lstat`-based gate.
 */
export class UnixSocketTransport implements Transport {
  private readonly socketPath: string;
  private socket: net.Socket | null = null;
  private handlers: TransportHandlers | null = null;
  private closed = false;
  private closeFired = false;
  /** Buffer of `send()` resolvers awaiting `drain` events. Held so
   *  high-throughput callers don't pile up backpressure indefinitely
   *  on a slow daemon. */
  private writePromises: Array<() => void> = [];

  public constructor(socketPath: string) {
    this.socketPath = socketPath;
  }

  public async connect(handlers: TransportHandlers): Promise<void> {
    if (this.handlers !== null) {
      throw new TypeError('UnixSocketTransport.connect: already connected');
    }
    if (this.closed) {
      throw driverError('anvil-driver-closed', 'transport already closed');
    }

    // Synchronous owner-gate BEFORE the connect attempt. If this
    // throws, the consumer never sees a partial transport.
    validateUnixSocketOwnership(this.socketPath);

    this.handlers = handlers;

    await new Promise<void>((resolve, reject) => {
      const sock = net.createConnection(this.socketPath);
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
          // Post-connect error: surface via onClose, don't re-reject
          // the connect promise. Tests cover the post-connect drop.
          return;
        }
        settled = true;
        this.handlers = null;
        const code = (err as NodeJS.ErrnoException).code;
        if (code === 'ENOENT' || code === 'ECONNREFUSED') {
          reject(
            driverError(
              'anvil-daemon-unavailable',
              `cannot connect to ${this.socketPath}: ${err.message}`
            )
          );
          return;
        }
        if (code === 'EACCES') {
          reject(
            driverError(
              'anvil-daemon-wrong-owner',
              `cannot connect to ${this.socketPath}: ${err.message}`
            )
          );
          return;
        }
        reject(
          driverError(
            'anvil-daemon-unavailable',
            `cannot connect to ${this.socketPath}: ${err.message}`,
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
          reject(driverError('anvil-daemon-transport-drop', `socket write failed: ${err.message}`));
        }
      });
      if (ok) {
        resolve();
        return;
      }
      // Slow daemon — queue resolve until the buffer drains so the
      // caller's `await send()` honours backpressure rather than
      // marching ahead.
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
    // `end()` flushes; the `close` event will fire `onClose` via the
    // attached handlers below.
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
      // Surface as `onClose('error')`. The daemon-side socket may go
      // away mid-stream (process killed, signal); the higher-level
      // client cancels in-flight requests with a structured error.
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
    // Best-effort: drain any backpressure waiters so they don't hang
    // forever. Their wrapping `send()` rejected synchronously when
    // the socket dropped — these resolves are idempotent.
    for (const resolve of this.writePromises) {
      resolve();
    }
    this.writePromises = [];
    const handlers = this.handlers;
    this.handlers = null;
    handlers?.onClose(cause);
  }
}

// Re-export for convenience in unit tests that want to operate
// against a fresh socket path.
export { os, DriverClientError };
