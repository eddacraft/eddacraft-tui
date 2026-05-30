/**
 * Regression tests for issue #1826 findings
 * `fnd_sig-feat-library-255c3bcb97-cad5_4ed698ff19` /
 * `fnd_sig-feat-library-c6a8d0fc79-8c4a_24e8f78078`:
 * "Bundle downloads can follow redirects to non-HTTPS hosts with credentials"
 * and "Bundle redirects can downgrade HTTPS and leak auth headers".
 *
 * A redirect must not (a) downgrade to a non-HTTPS host, nor (b) forward the
 * caller's credentials (Authorization header / configured auth) to a
 * different origin.
 *
 * NOTE: URLs use the literal `127.0.0.1` rather than `localhost` so the
 * loopback address matches the server bind address — `localhost` can resolve
 * to `::1` (IPv6) while the test servers listen on `127.0.0.1` (IPv4).
 * Both `127.0.0.1` and `localhost` are inside the manager's HTTPS carve-out.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { BundleManager } from './bundle-manager.js';
import { mkdtempSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { createServer, type Server, type IncomingMessage, type ServerResponse } from 'node:http';
import { safeCleanup } from '../../../../tools/test-utils/safe-cleanup.js';

function listen(server: Server): Promise<number> {
  return new Promise((resolve, reject) => {
    server.listen(0, '127.0.0.1', () => {
      const addr = server.address();
      if (addr && typeof addr !== 'string') resolve(addr.port);
      else reject(new Error('no port'));
    });
  });
}

function close(server: Server | null): Promise<void> {
  return new Promise((resolve) => {
    if (!server) return resolve();
    server.close(() => resolve());
    setTimeout(resolve, 1000);
  });
}

describe('BundleManager — redirect hardening (issue #1826)', () => {
  let cacheDir: string;
  let manager: BundleManager;
  let serverA: Server | null = null;
  let serverB: Server | null = null;
  let portA = 0;
  let portB = 0;
  type Captured = {
    path?: string;
    authorization?: string | string[];
    apiKey?: string | string[];
    cookie?: string | string[];
  };
  const captured: Captured[] = [];

  const record = (req: IncomingMessage): void => {
    captured.push({
      path: req.url,
      authorization: req.headers['authorization'],
      apiKey: req.headers['x-api-key'],
      cookie: req.headers['cookie'],
    });
  };

  beforeEach(async () => {
    cacheDir = mkdtempSync(join(tmpdir(), 'anvil-bundle-redirect-test-'));
    manager = new BundleManager({ cacheDir, verifySignatures: false, timeoutMs: 5000 });
    captured.length = 0;

    // Server B: the cross-origin redirect *target*. Records the credential
    // headers it sees. Responds with a non-tarball body — the assertions are
    // on captured headers, not on extraction success.
    serverB = createServer((req: IncomingMessage, res: ServerResponse) => {
      record(req);
      res.statusCode = 200;
      res.end('not-a-real-bundle');
    });
    portB = await listen(serverB);

    // Server A: issues redirects.
    serverA = createServer((req: IncomingMessage, res: ServerResponse) => {
      if (req.url === '/downgrade') {
        // Redirect to a non-HTTPS, non-localhost host.
        res.statusCode = 302;
        res.setHeader('Location', 'http://127.0.0.2:9/evil.tar.gz');
        res.end();
      } else if (req.url === '/cross-origin') {
        // Redirect to a different origin (server B, different port).
        res.statusCode = 302;
        res.setHeader('Location', `http://127.0.0.1:${portB}/capture`);
        res.end();
      } else if (req.url === '/same-origin') {
        // Redirect within the same origin (server A).
        res.statusCode = 302;
        res.setHeader('Location', `http://127.0.0.1:${portA}/capture-same`);
        res.end();
      } else if (req.url === '/loop') {
        // Redirect to itself forever.
        res.statusCode = 302;
        res.setHeader('Location', `http://127.0.0.1:${portA}/loop`);
        res.end();
      } else if (req.url === '/capture-same') {
        record(req);
        res.statusCode = 200;
        res.end('not-a-real-bundle');
      } else {
        res.statusCode = 404;
        res.end('nope');
      }
    });
    portA = await listen(serverA);
  });

  afterEach(async () => {
    await close(serverA);
    serverA = null;
    await close(serverB);
    serverB = null;
    await safeCleanup(cacheDir);
  });

  it('refuses to follow a redirect that downgrades to a non-HTTPS host', async () => {
    manager.addBundle({ name: 'dg', url: `http://127.0.0.1:${portA}/downgrade` });
    const result = await manager.downloadBundle('dg');
    expect(result.success).toBe(false);
    expect(result.error ?? '').toMatch(/redirect|https/i);
  });

  it('does not forward auth or caller credential headers to a cross-origin redirect target', async () => {
    process.env.ANVIL_TEST_BUNDLE_TOKEN = 'super-secret-token';
    try {
      manager.addBundle({
        name: 'xo',
        url: `http://127.0.0.1:${portA}/cross-origin`,
        auth: { type: 'bearer', token_env: 'ANVIL_TEST_BUNDLE_TOKEN' },
        // Arbitrary caller-supplied headers that commonly carry credentials.
        headers: { 'X-Api-Key': 'secret-api-key', Cookie: 'session=abc123' },
      });
      await manager.downloadBundle('xo');
      const atB = captured.find((c) => c.path === '/capture');
      expect(atB, 'redirect target should have been reached').toBeDefined();
      expect(atB?.authorization).toBeUndefined();
      expect(atB?.apiKey).toBeUndefined();
      expect(atB?.cookie).toBeUndefined();
    } finally {
      delete process.env.ANVIL_TEST_BUNDLE_TOKEN;
    }
  });

  it('rejects an infinite redirect loop', async () => {
    manager.addBundle({ name: 'loop', url: `http://127.0.0.1:${portA}/loop` });
    const result = await manager.downloadBundle('loop');
    expect(result.success).toBe(false);
    expect(result.error ?? '').toMatch(/too many redirects/i);
  });

  it('preserves auth and caller headers on a same-origin redirect', async () => {
    process.env.ANVIL_TEST_BUNDLE_TOKEN = 'super-secret-token';
    try {
      manager.addBundle({
        name: 'so',
        url: `http://127.0.0.1:${portA}/same-origin`,
        auth: { type: 'bearer', token_env: 'ANVIL_TEST_BUNDLE_TOKEN' },
        headers: { 'X-Api-Key': 'secret-api-key' },
      });
      await manager.downloadBundle('so');
      const atSame = captured.find((c) => c.path === '/capture-same');
      expect(atSame, 'same-origin redirect target should have been reached').toBeDefined();
      expect(atSame?.authorization).toBe('Bearer super-secret-token');
      expect(atSame?.apiKey).toBe('secret-api-key');
    } finally {
      delete process.env.ANVIL_TEST_BUNDLE_TOKEN;
    }
  });
});
