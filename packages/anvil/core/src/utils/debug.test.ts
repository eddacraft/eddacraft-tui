import { afterEach, describe, expect, it, vi } from 'vitest';
import { debug } from './debug.js';

const FAKE_COPILOT_TOKEN = 'ghu_testfixture000000000000000000000000';

describe('debug structured-data redaction', () => {
  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllEnvs();
  });

  it('redacts credentials nested in objects and arrays', () => {
    vi.stubEnv('ANVIL_DEBUG', '1');
    const consoleDebug = vi.spyOn(console, 'debug').mockImplementation(() => undefined);

    debug('provenance', 'context', {
      session: 'session-42',
      nested: {
        values: [`Bearer ${FAKE_COPILOT_TOKEN}`, FAKE_COPILOT_TOKEN],
        accessToken: 'opaque-short-secret',
        tokenValue: 'another-opaque-secret',
        apiKeyValue: 'provider-independent-api-key',
        credentials: { value: 'nested-opaque-secret' },
        privateKey: 'opaque-private-key',
        privateKeyData: 'provider-independent-private-key',
        privateKeyPem: 'pem-secret',
        cookieJar: 'session=opaque-cookie',
        cookieStore: { active: 'nested-cookie' },
      },
    });

    expect(consoleDebug).toHaveBeenCalledOnce();
    const rendered = JSON.stringify(consoleDebug.mock.calls[0]);
    expect(rendered).not.toContain(FAKE_COPILOT_TOKEN);
    expect(rendered).not.toContain('opaque-short-secret');
    expect(rendered).not.toContain('another-opaque-secret');
    expect(rendered).not.toContain('provider-independent-api-key');
    expect(rendered).not.toContain('nested-opaque-secret');
    expect(rendered).not.toContain('opaque-private-key');
    expect(rendered).not.toContain('provider-independent-private-key');
    expect(rendered).not.toContain('pem-secret');
    expect(rendered).not.toContain('opaque-cookie');
    expect(rendered).not.toContain('nested-cookie');
    expect(rendered).toContain('[redacted]');
    expect(rendered).toContain('session-42');
  });

  it('contains circular structured data instead of throwing', () => {
    vi.stubEnv('ANVIL_DEBUG', '1');
    vi.spyOn(console, 'debug').mockImplementation(() => undefined);
    const data: Record<string, unknown> = {};
    data.self = data;

    expect(() => debug('provenance', 'context', data)).not.toThrow();
  });

  it('preserves ordinary keys that merely contain credential words', () => {
    vi.stubEnv('ANVIL_DEBUG', '1');
    const consoleDebug = vi.spyOn(console, 'debug').mockImplementation(() => undefined);

    debug('provenance', 'context', {
      tokenizer: 'cl100k_base',
      cookiePolicy: 'same-site',
      session: 'session-42',
      commit: '50c40cf374b927ae2b6ba4dd12618ac7754950ec',
      objectId: 'a'.repeat(64),
    });

    const rendered = JSON.stringify(consoleDebug.mock.calls[0]);
    expect(rendered).toContain('cl100k_base');
    expect(rendered).toContain('same-site');
    expect(rendered).toContain('session-42');
    expect(rendered).toContain('50c40cf374b927ae2b6ba4dd12618ac7754950ec');
    expect(rendered).toContain('a'.repeat(64));
  });
});
