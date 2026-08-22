import { describe, expect, it } from 'vitest';

import { evaluateDocsAccess } from './feature-flags';

describe('evaluateDocsAccess', () => {
  it.each(['beta', 'pro', 'enterprise'])('grants the entitled %s plan', (plan) => {
    expect(evaluateDocsAccess(plan)).toBe(true);
  });

  it.each(['free', 'platinum', ''])('fails closed for the non-entitled %s plan', (plan) => {
    expect(evaluateDocsAccess(plan)).toBe(false);
  });
});
