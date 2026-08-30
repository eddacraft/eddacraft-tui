import { describe, expect, it } from 'vitest';
import type { ReactNode } from 'react';
import HomePage, * as landing from './page';

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

describe('docs-shell landing page', () => {
  it('does not force request-time rendering', () => {
    expect(landing).not.toHaveProperty('dynamic');
  });

  it('pins the public documentation destinations', () => {
    expect(hrefsOf(HomePage())).toEqual(
      expect.arrayContaining([
        'https://eddacraft.ai',
        '/blog',
        '/aps/overview',
        '/kindling/overview',
        '/anvil/overview',
      ])
    );
  });
});
