import { describe, expect, it } from 'vitest';

import {
  API_SCOPE_FLAGS,
  API_SCOPE_FLAG_PREFIX,
  API_SCOPE_NAMES,
  apiScopeFlagFor,
  defaultApiEvaluationContext,
  isApiScopeName,
  isScopeAllowed,
  resolveApiScope,
} from '../feature-flags.js';

describe('api.scope.* flag manifest', () => {
  it('exposes a flag for every allowed scope name', () => {
    for (const name of API_SCOPE_NAMES) {
      const flag = API_SCOPE_FLAGS[name];
      expect(flag.key).toBe(`${API_SCOPE_FLAG_PREFIX}${name}`);
      expect(flag.class).toBe('entitlement');
      expect(flag.defaultVariant).toBe('enabled');
      expect(flag.variants.map((v) => v.key).sort()).toEqual(['disabled', 'enabled']);
    }
  });

  it('keeps API_SCOPE_NAMES in sync with the manifest', () => {
    const fromManifest = Object.values(API_SCOPE_FLAGS)
      .map((f) => f.key.slice(API_SCOPE_FLAG_PREFIX.length))
      .sort();
    expect([...API_SCOPE_NAMES].sort()).toEqual(fromManifest);
  });

  it('recognises only manifest scopes as valid names', () => {
    for (const name of API_SCOPE_NAMES) {
      expect(isApiScopeName(name)).toBe(true);
    }
    expect(isApiScopeName('admin')).toBe(false);
    expect(isApiScopeName('')).toBe(false);
  });
});

describe('apiScopeFlagFor', () => {
  it('returns the matching flag for each allowed scope', () => {
    for (const name of API_SCOPE_NAMES) {
      expect(apiScopeFlagFor(name)?.key).toBe(`${API_SCOPE_FLAG_PREFIX}${name}`);
    }
  });

  it('returns undefined for unknown scopes', () => {
    expect(apiScopeFlagFor('admin')).toBeUndefined();
    expect(apiScopeFlagFor('api.scope.beta')).toBeUndefined();
  });
});

describe('resolveApiScope', () => {
  it('resolves allowed scopes to enabled by default', () => {
    for (const name of API_SCOPE_NAMES) {
      const result = resolveApiScope(name);
      expect(result?.allowed).toBe(true);
      expect(result?.details.variant).toBe('enabled');
      expect(result?.details.reason).toBe('default');
      expect(result?.details.flagKey).toBe(`${API_SCOPE_FLAG_PREFIX}${name}`);
    }
  });

  it('returns undefined for scopes with no backing flag', () => {
    expect(resolveApiScope('admin')).toBeUndefined();
  });

  it('honours local operator overrides', () => {
    const overrides = { local: { 'api.scope.internal': 'disabled' } };
    const result = resolveApiScope('internal', defaultApiEvaluationContext(), overrides);
    expect(result?.allowed).toBe(false);
    expect(result?.details.variant).toBe('disabled');
    expect(result?.details.reason).toBe('local_override');
  });

  it('exposes boolean-typed variant values via generic ResolutionDetails', () => {
    // Compile-time shape check — details.value must be boolean, not unknown.
    const result = resolveApiScope('beta');
    expect(result?.details.value).toBe(true);
    if (result) {
      const value: boolean | undefined = result.details.value;
      expect(value).toBe(true);
    }
  });
});

// -----------------------------------------------------------------------------
// isScopeAllowed behaviour — flag is now the sole authority. The FLAGM-005
// legacy-ALLOWED_SCOPES parity suite was retired in FLAGM-006; these checks
// cover manifest membership, unknown-scope rejection, and override-driven
// denial directly against the flag path.
// -----------------------------------------------------------------------------

describe('isScopeAllowed', () => {
  it('accepts every manifest scope at the default variant', () => {
    for (const scope of API_SCOPE_NAMES) {
      expect(isScopeAllowed(scope)).toBe(true);
    }
  });

  it('rejects unknown scopes', () => {
    for (const scope of ['', 'admin', 'api.scope.beta', 'BETA']) {
      expect(isScopeAllowed(scope)).toBe(false);
    }
  });

  it('denies a known scope when an operator override disables it', () => {
    const overrides = { local: { 'api.scope.internal': 'disabled' } };
    expect(isScopeAllowed('internal', defaultApiEvaluationContext(), overrides)).toBe(false);
  });
});
