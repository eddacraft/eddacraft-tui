import type { EvaluationContext } from '@eddacraft/anvil-contracts';
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

// Day-1 parity mirror of the pre-FLAGM-005 constant. FLAGM-006 retires it.
const LEGACY_ALLOWED_SCOPES = ['beta', 'preview', 'internal'] as const;

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

  it('recognises only the manifest scopes as valid names', () => {
    for (const name of LEGACY_ALLOWED_SCOPES) {
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
// FLAGM-005 parity — flag-backed path must agree with the legacy constant list
// on day one. Three design-spec cases: enabled, disabled, default.
// -----------------------------------------------------------------------------

function legacyAccepts(scope: string): boolean {
  return (LEGACY_ALLOWED_SCOPES as readonly string[]).includes(scope);
}

interface ParityCase {
  readonly name: string;
  readonly scope: string;
  readonly context?: EvaluationContext;
}

const PARITY_CASES: readonly ParityCase[] = [
  // enabled: known scope resolves to the flag's enabled default
  { name: 'enabled: beta', scope: 'beta' },
  // disabled: unknown scope has no flag and fails the legacy membership
  { name: 'disabled: admin (unknown)', scope: 'admin' },
  // default: known scope resolves to the flag's default variant (enabled)
  { name: 'default: preview', scope: 'preview' },
];

describe('FLAGM-005 parity: api.scope.* vs legacy ALLOWED_SCOPES', () => {
  for (const parity of PARITY_CASES) {
    it(`${parity.name}: legacy and flag decisions agree`, () => {
      const legacy = legacyAccepts(parity.scope);
      const flag = isScopeAllowed(parity.scope, parity.context);
      expect(flag).toBe(legacy);
    });
  }

  it('sweeps every legacy scope string and confirms the flag path agrees', () => {
    for (const scope of LEGACY_ALLOWED_SCOPES) {
      expect(isScopeAllowed(scope)).toBe(true);
      expect(legacyAccepts(scope)).toBe(true);
    }
  });

  it('rejects unknown scopes on both paths', () => {
    for (const scope of ['', 'admin', 'api.scope.beta', 'BETA']) {
      expect(isScopeAllowed(scope)).toBe(false);
      expect(legacyAccepts(scope)).toBe(false);
    }
  });

  // The flag path MUST diverge from the legacy constant when an operator
  // disables a scope via override — this is the migration's whole point.
  // Without this case the parity suite just confirms trivial agreement
  // against the day-1 default.
  it('diverges from legacy when an override disables a known scope', () => {
    const overrides = { local: { 'api.scope.internal': 'disabled' } };
    expect(isScopeAllowed('internal', defaultApiEvaluationContext(), overrides)).toBe(false);
    expect(legacyAccepts('internal')).toBe(true);
  });
});
