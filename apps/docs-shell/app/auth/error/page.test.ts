import { describe, expect, it } from 'vitest';
import type { ReactNode } from 'react';
import ErrorPage from './page';

function textOf(node: ReactNode): string {
  if (node == null || typeof node === 'boolean') return '';
  if (typeof node === 'string' || typeof node === 'number') return String(node);
  if (Array.isArray(node)) return node.map(textOf).join('');
  if (typeof node === 'object' && 'props' in node) {
    return textOf((node as { props?: { children?: ReactNode } }).props?.children);
  }
  return '';
}

function hrefsOf(node: ReactNode): string[] {
  if (node == null || typeof node === 'boolean') return [];
  if (Array.isArray(node)) return node.flatMap(hrefsOf);
  if (typeof node === 'object' && 'props' in node) {
    const props = (node as { props?: { href?: string; children?: ReactNode } }).props;
    const self = typeof props?.href === 'string' ? [props.href] : [];
    return [...self, ...hrefsOf(props?.children)];
  }
  return [];
}

describe('auth error page', () => {
  it.each([
    ['denied', 'You cancelled the GitHub sign-in.'],
    ['oauth_error', 'GitHub returned an OAuth error.'],
    ['missing_params', 'The callback URL is missing required parameters.'],
    ['invalid_state', 'The OAuth state parameter was invalid or tampered with.'],
    ['csrf_mismatch', 'CSRF nonce did not match. Please try signing in again.'],
    ['api_error', 'Could not reach the authentication service.'],
    ['auth_failed', 'Authentication failed.'],
    ['invalid_response', 'The authentication service returned an unexpected response.'],
  ])('explains recognised reason %s', async (reason, message) => {
    const tree = await ErrorPage({ searchParams: Promise.resolve({ reason }) });
    expect(textOf(tree)).toContain(message);
  });

  it('falls back when the reason is unknown', async () => {
    const tree = await ErrorPage({
      searchParams: Promise.resolve({ reason: 'not-a-reason' }),
    });
    expect(textOf(tree)).toContain('An unknown error occurred.');
  });

  it('offers retry and home recovery links', async () => {
    const tree = await ErrorPage({ searchParams: Promise.resolve({}) });
    expect(hrefsOf(tree)).toEqual(
      expect.arrayContaining(['/auth/login', '/', 'mailto:help@eddacraft.ai'])
    );
  });
});
