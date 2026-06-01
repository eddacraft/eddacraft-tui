import { describe, expect, it } from 'vitest';
import type { EvaluationContext } from '@eddacraft/anvil-contracts';
import { resolveFlag } from '@eddacraft/anvil-runtime/feature-flags';
import { DOCS_ACCESS_FLAG, canonicalAccountTier } from '../src/index.js';

// FLAGCAT-003 audience-value reconciliation: the manifest targets canonical
// plan-* audience ids; surfaces receive bare tier claims. These tests lock the
// mapping AND the end-to-end docs.access resolution so a regression in either
// (e.g. a manifest targeting change or a mapping change) fails loudly.

describe('canonicalAccountTier', () => {
  it.each([
    ['free', 'plan-free'],
    ['beta', 'plan-beta'],
    ['pro', 'plan-pro'],
    ['enterprise', 'plan-enterprise'],
  ])('maps bare "%s" to "%s"', (bare, canonical) => {
    expect(canonicalAccountTier(bare)).toBe(canonical);
  });

  it('passes an already-canonical plan id through unchanged (idempotent)', () => {
    expect(canonicalAccountTier('plan-beta')).toBe('plan-beta');
  });

  it('returns an unknown tier unchanged — never invents a plan-* id', () => {
    expect(canonicalAccountTier('platinum')).toBe('platinum');
    expect(canonicalAccountTier('admin')).toBe('admin');
    expect(canonicalAccountTier('')).toBe('');
  });
});

describe('docs.access resolves identically via the canonical mapping', () => {
  function ctx(tier: string | null): EvaluationContext {
    return {
      targetingKey: 'u-1',
      environment: { environment: 'production' },
      ...(tier !== null ? { audience: { accountTier: canonicalAccountTier(tier) } } : {}),
    };
  }

  it.each(['beta', 'pro', 'enterprise'])('grants a bare "%s" tier', (tier) => {
    expect(resolveFlag(DOCS_ACCESS_FLAG, ctx(tier)).variant).toBe('enabled');
  });

  it('denies free, unknown, and missing tiers (fail closed)', () => {
    expect(resolveFlag(DOCS_ACCESS_FLAG, ctx('free')).variant).toBe('disabled');
    expect(resolveFlag(DOCS_ACCESS_FLAG, ctx('platinum')).variant).toBe('disabled');
    expect(resolveFlag(DOCS_ACCESS_FLAG, ctx(null)).variant).toBe('disabled');
  });
});
