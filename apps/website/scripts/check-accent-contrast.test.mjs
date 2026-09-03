import { describe, expect, it } from 'vitest';
import { PAIRS, contrastRatio } from './check-accent-contrast.mjs';

const AA = 4.5;

describe('brand accent contrast', () => {
  it('checks every accent/background pairing the product ships', () => {
    // Guards against a token being added without a pairing: dark-only surfaces
    // contribute one each, and the two docs sites four each (accent + hover,
    // in light and dark).
    expect(PAIRS.length).toBeGreaterThanOrEqual(11);
  });

  it.each(PAIRS)('%s clears WCAG AA', (_label, fg, bg) => {
    expect(contrastRatio(fg, bg)).toBeGreaterThanOrEqual(AA);
  });

  it('rejects the pre-fix values, so the guard is falsifiable', () => {
    // The three failures this change was opened for. #cc5500 on The Void is
    // 4.4998:1 — below the floor, not on it, which is why "it looks like 4.5"
    // is not a safe reading.
    expect(contrastRatio('#cc5500', '#0d0d0f')).toBeLessThan(AA);
    expect(contrastRatio('#cc5500', '#fafafa')).toBeLessThan(AA);
    expect(contrastRatio('#a34400', '#0d0d0f')).toBeLessThan(AA);
  });
});
