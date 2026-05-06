/**
 * Unix-domain-socket transport tests.
 *
 * The wrong-owner check is the security-critical surface; cover:
 *   - Missing path -> anvil-daemon-unavailable
 *   - Symlinked parent -> anvil-daemon-wrong-owner
 *   - Wide-open mode (0o777) -> anvil-daemon-wrong-owner
 *   - Regular file at the socket path -> anvil-daemon-wrong-owner
 *   - Successful path: real Unix socket bound by an in-test server,
 *     happy-path connect / send / drop / close cycle.
 */

import fs from 'node:fs';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import { DriverClientError } from '../errors.js';
import { UnixSocketTransport, validateUnixSocketOwnership } from './unix.js';

const cleanupTargets: string[] = [];

afterEach(() => {
  while (cleanupTargets.length > 0) {
    const target = cleanupTargets.pop()!;
    try {
      fs.rmSync(target, { recursive: true, force: true });
    } catch {
      // best effort
    }
  }
});

function makeTmpDir(mode = 0o700): string {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'anvil-driver-client-'));
  fs.chmodSync(dir, mode);
  cleanupTargets.push(dir);
  return dir;
}

function bindServer(socketPath: string): Promise<net.Server> {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.once('error', reject);
    server.listen(socketPath, () => {
      try {
        fs.chmodSync(socketPath, 0o600);
      } catch (err) {
        reject(err);
        return;
      }
      resolve(server);
    });
  });
}

describe('validateUnixSocketOwnership', () => {
  // These tests run only on Unix-style platforms where the daemon is
  // actually bound. On Windows CI the suite is skipped.
  const isUnix = process.platform !== 'win32';
  it.skipIf(!isUnix)('rejects a missing path with anvil-daemon-unavailable', () => {
    const dir = makeTmpDir();
    const sock = path.join(dir, 'intercept.sock');
    let err: unknown;
    try {
      validateUnixSocketOwnership(sock);
    } catch (e) {
      err = e;
    }
    expect(err).toBeInstanceOf(DriverClientError);
    expect((err as DriverClientError).code).toBe('anvil-daemon-unavailable');
  });

  it.skipIf(!isUnix)('rejects a symlinked parent directory as wrong-owner', () => {
    const realDir = makeTmpDir();
    const linkParent = fs.mkdtempSync(path.join(os.tmpdir(), 'anvil-link-'));
    cleanupTargets.push(linkParent);
    const linkPath = path.join(linkParent, 'anvil');
    fs.symlinkSync(realDir, linkPath);
    cleanupTargets.push(linkPath);
    const sockPath = path.join(linkPath, 'intercept.sock');
    let err: unknown;
    try {
      validateUnixSocketOwnership(sockPath);
    } catch (e) {
      err = e;
    }
    expect((err as DriverClientError).code).toBe('anvil-daemon-wrong-owner');
  });

  it.skipIf(!isUnix)('rejects a wide-open parent directory as wrong-owner', async () => {
    const dir = makeTmpDir(0o777);
    const sock = path.join(dir, 'intercept.sock');
    const server = await bindServer(sock);
    let err: unknown;
    try {
      validateUnixSocketOwnership(sock);
    } catch (e) {
      err = e;
    } finally {
      server.close();
    }
    expect((err as DriverClientError).code).toBe('anvil-daemon-wrong-owner');
  });

  it.skipIf(!isUnix)('rejects a regular file at the socket path as wrong-owner', () => {
    const dir = makeTmpDir();
    const sock = path.join(dir, 'intercept.sock');
    fs.writeFileSync(sock, 'not a socket', { mode: 0o600 });
    let err: unknown;
    try {
      validateUnixSocketOwnership(sock);
    } catch (e) {
      err = e;
    }
    expect((err as DriverClientError).code).toBe('anvil-daemon-wrong-owner');
  });

  it.skipIf(!isUnix)('accepts an owner-only socket file', async () => {
    const dir = makeTmpDir();
    const sock = path.join(dir, 'intercept.sock');
    const server = await bindServer(sock);
    try {
      validateUnixSocketOwnership(sock);
    } finally {
      server.close();
    }
  });
});

describe('UnixSocketTransport happy path', () => {
  const isUnix = process.platform !== 'win32';

  it.skipIf(!isUnix)('connects, sends, receives, and closes', async () => {
    const dir = makeTmpDir();
    const sock = path.join(dir, 'intercept.sock');
    const received: string[] = [];
    const server = net.createServer((conn) => {
      conn.on('data', (chunk: Buffer) => {
        received.push(chunk.toString('utf8'));
        conn.write('hello back\n');
      });
    });
    await new Promise<void>((resolve, reject) => {
      server.once('error', reject);
      server.listen(sock, () => {
        fs.chmodSync(sock, 0o600);
        resolve();
      });
    });

    try {
      const transport = new UnixSocketTransport(sock);
      const incoming: string[] = [];
      const closes: string[] = [];
      await transport.connect({
        onData: (chunk) => incoming.push(Buffer.from(chunk).toString('utf8')),
        onClose: (cause) => closes.push(cause),
      });
      await transport.send('hello daemon\n');
      // wait for round-trip
      await new Promise((r) => setTimeout(r, 50));
      expect(received.length).toBeGreaterThan(0);
      expect(incoming.join('')).toContain('hello back');
      await transport.close();
      // Close fires asynchronously
      await new Promise((r) => setTimeout(r, 20));
      expect(closes.length).toBeGreaterThan(0);
    } finally {
      server.close();
    }
  });

  it.skipIf(!isUnix)(
    'connect surfaces anvil-daemon-unavailable when no listener exists',
    async () => {
      const dir = makeTmpDir();
      const sock = path.join(dir, 'intercept.sock');
      // Bind and immediately tear down so the path is missing.
      const transport = new UnixSocketTransport(sock);
      let err: unknown;
      try {
        await transport.connect({ onData: () => undefined, onClose: () => undefined });
      } catch (e) {
        err = e;
      }
      expect(err).toBeInstanceOf(DriverClientError);
      expect((err as DriverClientError).code).toBe('anvil-daemon-unavailable');
    }
  );
});
