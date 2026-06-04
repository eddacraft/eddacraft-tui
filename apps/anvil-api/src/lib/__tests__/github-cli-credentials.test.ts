import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { getGitHubCliCredentials, verifyGitHubCliCredentials } from '../github-cli-credentials.js';

const ID = process.env['GITHUB_CLI_CLIENT_ID'];
const SECRET = process.env['GITHUB_CLI_CLIENT_SECRET'];

function setCreds(id: string | undefined, secret: string | undefined): void {
  if (id === undefined) delete process.env['GITHUB_CLI_CLIENT_ID'];
  else process.env['GITHUB_CLI_CLIENT_ID'] = id;
  if (secret === undefined) delete process.env['GITHUB_CLI_CLIENT_SECRET'];
  else process.env['GITHUB_CLI_CLIENT_SECRET'] = secret;
}

beforeEach(() => setCreds('cli-id', 'cli-secret'));
afterEach(() => setCreds(ID, SECRET));

describe('getGitHubCliCredentials', () => {
  it('returns the configured client id and secret', () => {
    expect(getGitHubCliCredentials()).toEqual({ clientId: 'cli-id', clientSecret: 'cli-secret' });
  });

  it('throws when the client id is missing', () => {
    setCreds(undefined, 'cli-secret');
    expect(() => getGitHubCliCredentials()).toThrow(/GITHUB_CLI_CLIENT_ID/);
  });

  it('throws when the client secret is missing', () => {
    setCreds('cli-id', undefined);
    expect(() => getGitHubCliCredentials()).toThrow(/GITHUB_CLI_CLIENT_SECRET/);
  });

  it('treats an empty-string value as missing', () => {
    setCreds('', 'cli-secret');
    expect(() => getGitHubCliCredentials()).toThrow(/GITHUB_CLI_CLIENT_ID/);
  });
});

describe('verifyGitHubCliCredentials', () => {
  it('reports ok when both credentials are present', () => {
    expect(verifyGitHubCliCredentials()).toEqual({ ok: true });
  });

  it('reports unavailable (without throwing) when credentials are missing', () => {
    setCreds(undefined, undefined);
    const result = verifyGitHubCliCredentials();
    expect(result.ok).toBe(false);
    expect(result).toMatchObject({ ok: false, error: expect.stringContaining('GITHUB_CLI') });
  });
});
