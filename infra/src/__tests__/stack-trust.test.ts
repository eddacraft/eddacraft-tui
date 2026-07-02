import { describe, expect, it } from 'vitest';

// CIB-119: the trust decision is a single, deterministic predicate. Only the
// stacks named here may read production secrets or define production
// resources; everything else (dev, PR previews, typos) is untrusted.
import { isTrustedStack, untrustedSecretMarker } from '../../src/stack-trust.js';

describe('isTrustedStack', () => {
  it('trusts the prod stack', () => {
    expect(isTrustedStack('prod')).toBe(true);
  });

  it('does not trust the dev (PR preview) stack', () => {
    expect(isTrustedStack('dev')).toBe(false);
  });

  it('does not trust arbitrary or near-miss stack names', () => {
    expect(isTrustedStack('test-stack')).toBe(false);
    expect(isTrustedStack('production')).toBe(false);
    expect(isTrustedStack('prod2')).toBe(false);
    expect(isTrustedStack('')).toBe(false);
  });

  it('is case-sensitive — "PROD" is not the trusted stack', () => {
    expect(isTrustedStack('PROD')).toBe(false);
  });
});

describe('untrustedSecretMarker', () => {
  it('names the secret so a leaked marker is traceable', () => {
    expect(untrustedSecretMarker('anvil-api-database-url')).toContain('anvil-api-database-url');
  });

  it('is unmistakably a marker, not a plausible secret value', () => {
    expect(untrustedSecretMarker('token-pepper')).toBe('<untrusted-stack-secret:token-pepper>');
  });
});
