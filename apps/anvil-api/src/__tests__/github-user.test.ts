import { afterEach, describe, expect, it, vi } from 'vitest';
import { fetchGitHubUser } from '../lib/github-user.js';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

function mockGitHubIdentity(emails: Array<{ email: string; primary: boolean; verified: boolean }>) {
  return vi.spyOn(globalThis, 'fetch').mockImplementation(async (input: RequestInfo | URL) => {
    const url = String(input);
    if (url === 'https://api.github.com/user') {
      return jsonResponse({ id: 42, login: 'octo', name: 'Octo', avatar_url: null });
    }
    if (url === 'https://api.github.com/user/emails') {
      return jsonResponse(emails);
    }
    throw new Error(`no mock for fetch(${url})`);
  });
}

describe('fetchGitHubUser', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('uses a verified primary as the canonical email', async () => {
    mockGitHubIdentity([
      { email: 'Primary@Example.com', primary: true, verified: true },
      { email: 'Second@Example.com', primary: false, verified: true },
    ]);

    await expect(fetchGitHubUser('token')).resolves.toEqual({
      id: 42,
      login: 'octo',
      email: 'primary@example.com',
      verifiedEmails: ['primary@example.com', 'second@example.com'],
    });
  });

  it('falls back to a verified secondary when the primary is unverified', async () => {
    mockGitHubIdentity([
      { email: 'Unverified@Example.com', primary: true, verified: false },
      { email: 'Second@Example.com', primary: false, verified: true },
    ]);

    await expect(fetchGitHubUser('token')).resolves.toEqual({
      id: 42,
      login: 'octo',
      email: 'second@example.com',
      verifiedEmails: ['second@example.com'],
    });
  });

  it('fails only when no verified email exists', async () => {
    mockGitHubIdentity([
      { email: 'Unverified@Example.com', primary: true, verified: false },
      { email: 'Also@Example.com', primary: false, verified: false },
    ]);

    await expect(fetchGitHubUser('token')).rejects.toThrow(/No verified email/);
  });
});
