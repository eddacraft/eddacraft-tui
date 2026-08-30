import { describe, expect, it } from 'vitest';
import type { ReactNode } from 'react';
import PendingPage from './page';

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

function textOf(node: ReactNode): string {
  if (node == null || typeof node === 'boolean') return '';
  if (typeof node === 'string' || typeof node === 'number') return String(node);
  if (Array.isArray(node)) return node.map(textOf).join('');
  if (typeof node === 'object' && 'props' in node) {
    return textOf((node as { props?: { children?: ReactNode } }).props?.children);
  }
  return '';
}

describe('auth pending page', () => {
  it('explains waitlist and GitHub email mismatch recoveries', () => {
    const tree = PendingPage();
    const text = textOf(tree);
    expect(text).toContain('Waiting for an invite');
    expect(text).toContain("GitHub email doesn't match");
  });

  it('links to GitHub email settings, retry, home, and help', () => {
    expect(hrefsOf(PendingPage())).toEqual(
      expect.arrayContaining([
        'https://github.com/settings/emails',
        'mailto:help@eddacraft.ai',
        '/auth/login',
        '/',
      ])
    );
  });
});
