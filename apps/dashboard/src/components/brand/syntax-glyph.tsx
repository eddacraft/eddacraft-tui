import type { HTMLAttributes } from 'react';

import { cn } from '@/lib/utils';

export type SyntaxGlyphKind = 'context' | 'action' | 'history' | 'unavailable';

const glyphs = {
  context: '[ ]',
  action: '[ = ]',
  history: '[ ≡ ]',
  unavailable: '[ N/A ]',
} satisfies Record<SyntaxGlyphKind, string>;

export function SyntaxGlyph({
  kind,
  className,
  ...props
}: Omit<HTMLAttributes<HTMLSpanElement>, 'children'> & { kind: SyntaxGlyphKind }) {
  return (
    <span
      aria-hidden="true"
      className={cn('syntax-glyph', className)}
      data-glyph={glyphs[kind]}
      {...props}
    />
  );
}
