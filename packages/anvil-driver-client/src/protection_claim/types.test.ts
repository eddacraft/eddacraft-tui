import { describe, expect, it } from 'vitest';

import {
  ALL_SURFACE_CLAIM_STATES,
  ALL_WORKTREE_CLAIM_STATES,
  PROTECTION_CLAIM_SCHEMA_VERSION,
  parseOptionalProtectionClaimFromValidateWrite,
  parseProtectionClaim,
  parseSurfaceClaim,
  type ProtectionClaim,
  type SurfaceClaim,
  type SurfaceClaimState,
  type WorktreeClaimState,
} from './types.js';

/**
 * MLP2-051c: cross-language parity for the ProtectionClaim wire shape.
 *
 * The reference JSON below is the byte-exact output of the Rust
 * `serde_json::to_string(&ProtectionClaim::new(WorktreeClaimState::Full,
 *   [SurfaceClaim{mcp-shim-claude, Participating},
 *    SurfaceClaim{editor-driver-vscode, Attached}]))` captured against the
 * `protection_claim_round_trips_through_json` test in
 * `crates/anvil-kernel-types/src/protection_claim.rs`. Keep the string
 * literal: any future Rust serde rename or field-order change must be a
 * deliberate, reviewed update on both sides.
 */
const RUST_EMITTED_FULL_CLAIM_JSON =
  '{"schema_version":"anvil.protection-claim.v1","worktree_state":"full","surfaces":[{"identifier":"mcp-shim-claude","state":"participating"},{"identifier":"editor-driver-vscode","state":"attached"}]}';

const RUST_EMITTED_WARMING_CLAIM_JSON =
  '{"schema_version":"anvil.protection-claim.v1","worktree_state":"warming","surfaces":[]}';

const RUST_EQUIVALENT_FULL_CLAIM: ProtectionClaim = {
  schema_version: 'anvil.protection-claim.v1',
  worktree_state: 'full',
  surfaces: [
    { identifier: 'mcp-shim-claude', state: 'participating' },
    { identifier: 'editor-driver-vscode', state: 'attached' },
  ],
};

describe('ProtectionClaim closed-set vocabulary', () => {
  it('pins the schema_version constant against the Rust mirror', () => {
    expect(PROTECTION_CLAIM_SCHEMA_VERSION).toBe('anvil.protection-claim.v1');
  });

  it('lists the ten spec §14.2 worktree states in declaration order', () => {
    // Pinned against `WorktreeClaimState::all()` in the Rust module so a
    // future variant addition fails this test (and forces the §14.2
    // canonical-string review) before drift can ship.
    expect(ALL_WORKTREE_CLAIM_STATES).toEqual<WorktreeClaimState[]>([
      'unprotected',
      'warming',
      'pre-write-embedded',
      'pre-write-daemon',
      'save-time-only',
      'full',
      'degraded-protection',
      'cross-boundary-mixed',
      'multi-daemon-detected',
      'path-uncertain',
    ]);
  });

  it('lists the eight spec §14.1 surface states in declaration order', () => {
    expect(ALL_SURFACE_CLAIM_STATES).toEqual<SurfaceClaimState[]>([
      'unbound',
      'attached',
      'participating',
      'embedded-fallback',
      'degraded',
      'cross-boundary-refused',
      'quarantined',
      'detached',
    ]);
  });

  it('treats pre-write-embedded as distinct from pre-write-daemon (spec §14.2)', () => {
    const embedded: WorktreeClaimState = 'pre-write-embedded';
    const daemon: WorktreeClaimState = 'pre-write-daemon';
    expect(embedded).not.toBe(daemon);
  });
});

describe('parseProtectionClaim (Rust → TS parity)', () => {
  it('round-trips the canonical Rust JSON shape losslessly', () => {
    const parsed = parseProtectionClaim(JSON.parse(RUST_EMITTED_FULL_CLAIM_JSON));
    expect(parsed).toEqual(RUST_EQUIVALENT_FULL_CLAIM);
  });

  it('accepts an empty surfaces array (Warming claim)', () => {
    const parsed = parseProtectionClaim(JSON.parse(RUST_EMITTED_WARMING_CLAIM_JSON));
    expect(parsed).toEqual<ProtectionClaim>({
      schema_version: 'anvil.protection-claim.v1',
      worktree_state: 'warming',
      surfaces: [],
    });
  });

  it('rejects a full claim with no protecting surfaces', () => {
    const wire = {
      schema_version: PROTECTION_CLAIM_SCHEMA_VERSION,
      worktree_state: 'full',
      surfaces: [],
    };
    expect(() => parseProtectionClaim(wire)).toThrow(/full.*surface/);
  });

  it('rejects an unknown schema_version (Rust enforces this at the type boundary)', () => {
    const wire = {
      schema_version: 'anvil.protection-claim.v999',
      worktree_state: 'full',
      surfaces: [],
    };
    expect(() => parseProtectionClaim(wire)).toThrow(/schema_version/);
  });

  it('rejects an unknown worktree_state', () => {
    const wire = {
      schema_version: 'anvil.protection-claim.v1',
      worktree_state: 'future-state',
      surfaces: [],
    };
    expect(() => parseProtectionClaim(wire)).toThrow(/worktree_state/);
  });

  it('rejects an unknown surface state', () => {
    const wire = {
      schema_version: 'anvil.protection-claim.v1',
      worktree_state: 'full',
      surfaces: [{ identifier: 's', state: 'future-surface' }],
    };
    expect(() => parseProtectionClaim(wire)).toThrow(/state/);
  });

  it('rejects null / non-object input', () => {
    expect(() => parseProtectionClaim(null)).toThrow(TypeError);
    expect(() => parseProtectionClaim(42)).toThrow(TypeError);
    expect(() => parseProtectionClaim('not-an-object')).toThrow(TypeError);
  });

  it('rejects each missing required field with a typed error mentioning it', () => {
    expect(() => parseProtectionClaim({ worktree_state: 'full', surfaces: [] } as unknown)).toThrow(
      /schema_version/
    );
    expect(() =>
      parseProtectionClaim({
        schema_version: PROTECTION_CLAIM_SCHEMA_VERSION,
        surfaces: [],
      } as unknown)
    ).toThrow(/worktree_state/);
    expect(() =>
      parseProtectionClaim({
        schema_version: PROTECTION_CLAIM_SCHEMA_VERSION,
        worktree_state: 'full',
      } as unknown)
    ).toThrow(/surfaces/);
  });

  it('drops unknown optional top-level fields (MLP2-052 additivity)', () => {
    // Mirrors Rust `additive_optional_top_level_field_deserialises_ok`: a
    // future v1.x field rides the v1 envelope. Known fields keep semantic
    // identity; the unknown field's data is not materialised.
    const extended = {
      ...JSON.parse(RUST_EMITTED_FULL_CLAIM_JSON),
      degraded_reasons: ['surface-drift', 'rule-pack-mismatch'],
      cross_boundary_token: 'future-token-abc123',
    };
    const parsed = parseProtectionClaim(extended);
    expect(parsed).toEqual(RUST_EQUIVALENT_FULL_CLAIM);
    expect(parsed).not.toHaveProperty('degraded_reasons');
    expect(parsed).not.toHaveProperty('cross_boundary_token');
  });

  it('drops unknown optional fields on per-surface entries (additivity composes)', () => {
    const wire = {
      schema_version: 'anvil.protection-claim.v1',
      worktree_state: 'save-time-only',
      surfaces: [
        {
          identifier: 'editor-driver-vscode',
          state: 'participating',
          last_evaluated_at: '2026-05-14T12:34:56Z',
        },
      ],
    };
    const parsed = parseProtectionClaim(wire);
    expect(parsed.surfaces).toHaveLength(1);
    expect(parsed.surfaces[0]).toEqual<SurfaceClaim>({
      identifier: 'editor-driver-vscode',
      state: 'participating',
    });
  });
});

describe('parseSurfaceClaim', () => {
  it('parses a single surface entry', () => {
    const surface = parseSurfaceClaim({
      identifier: 'mcp',
      state: 'participating',
    });
    expect(surface).toEqual<SurfaceClaim>({
      identifier: 'mcp',
      state: 'participating',
    });
  });

  it('rejects a non-object input', () => {
    expect(() => parseSurfaceClaim(null)).toThrow(TypeError);
    expect(() => parseSurfaceClaim('mcp')).toThrow(TypeError);
  });

  it('rejects missing identifier', () => {
    expect(() => parseSurfaceClaim({ state: 'participating' })).toThrow(/identifier/);
  });

  it('rejects empty-string identifier — daemon never assigns one', () => {
    expect(() => parseSurfaceClaim({ identifier: '', state: 'participating' })).toThrow(
      /identifier/
    );
  });

  it('rejects missing state', () => {
    expect(() => parseSurfaceClaim({ identifier: 'mcp' })).toThrow(/state/);
  });

  it('rejects each unknown state with a typed error', () => {
    expect(() => parseSurfaceClaim({ identifier: 'mcp', state: 'future-surface' })).toThrow(
      /state/
    );
  });
});

describe('parseOptionalProtectionClaimFromValidateWrite (MCP response adapter)', () => {
  /**
   * Captures the `validate_write` MCP response shape from
   * `crates/anvil-cli/src/mcp/tools/validate_write.rs`. The
   * `protection_claim` field is wire-additive: MLP2-051b emits it when
   * the daemon supplied a snapshot, omits it otherwise. A pre-MLP2-051b
   * driver pinned to the older shape MUST still parse the new response,
   * so this adapter returns `undefined` for the missing-field case
   * rather than throwing.
   */
  const VALIDATE_WRITE_RESPONSE_WITHOUT_CLAIM = {
    schema: 'anvil.mcp.validate-write.v1',
    decision: 'allow',
    summary: { error: 0, warn: 0 },
    diagnostics: [],
    correlation: {
      id: 'corr-1',
      surface: 'mcp',
      mode: 'preWrite',
      backend: 'embedded',
      daemonStatus: 'unavailable',
      path: 'src/lib.rs',
      enforcementMode: 'advise',
    },
  };

  it('returns undefined when the response omits protection_claim (pre-MLP2-051b parity)', () => {
    expect(
      parseOptionalProtectionClaimFromValidateWrite(VALIDATE_WRITE_RESPONSE_WITHOUT_CLAIM)
    ).toBeUndefined();
  });

  it('returns undefined when protection_claim is explicitly null', () => {
    expect(
      parseOptionalProtectionClaimFromValidateWrite({
        ...VALIDATE_WRITE_RESPONSE_WITHOUT_CLAIM,
        protection_claim: null,
      })
    ).toBeUndefined();
  });

  it('parses a daemon-served protection_claim into the typed shape', () => {
    const response = {
      ...VALIDATE_WRITE_RESPONSE_WITHOUT_CLAIM,
      protection_claim: JSON.parse(RUST_EMITTED_FULL_CLAIM_JSON),
    };
    expect(parseOptionalProtectionClaimFromValidateWrite(response)).toEqual(
      RUST_EQUIVALENT_FULL_CLAIM
    );
  });

  it('throws if protection_claim is present but malformed (closed-set invariant)', () => {
    const response = {
      ...VALIDATE_WRITE_RESPONSE_WITHOUT_CLAIM,
      protection_claim: {
        schema_version: 'anvil.protection-claim.v1',
        surfaces: [],
      },
    };
    expect(() => parseOptionalProtectionClaimFromValidateWrite(response)).toThrow(/worktree_state/);
  });

  it('rejects a non-object response envelope', () => {
    expect(() => parseOptionalProtectionClaimFromValidateWrite(null)).toThrow(TypeError);
    expect(() => parseOptionalProtectionClaimFromValidateWrite(42)).toThrow(TypeError);
  });
});

describe('parseProtectionClaim hostile inputs', () => {
  /**
   * `JSON.parse` defines `__proto__` as an own data property via
   * `CreateDataProperty` rather than invoking the prototype setter —
   * so a wire payload of `{"__proto__": {...}}` does NOT make the
   * inner object the prototype. Property lookups for `schema_version`
   * miss the inner object and fall through to `Object.prototype`
   * (where it isn't), so the parser rejects on the missing required
   * field. This test pins the structural defence; a future refactor
   * that switches to `Object.assign({}, raw)` or any permissive copy
   * step would defeat it and is the regression this test guards
   * against.
   */
  it('rejects a payload that hides schema_version under __proto__', () => {
    const hostile = JSON.parse(
      '{"__proto__":{"schema_version":"anvil.protection-claim.v1","worktree_state":"full","surfaces":[]}}'
    );
    expect(Object.hasOwn(hostile, '__proto__')).toBe(true);
    expect(Object.hasOwn(hostile, 'schema_version')).toBe(false);
    expect(() => parseProtectionClaim(hostile)).toThrow(/schema_version/);
  });

  it("rejects `surfaces: null` with a 'got null' diagnostic, not 'got object'", () => {
    const wire = {
      schema_version: PROTECTION_CLAIM_SCHEMA_VERSION,
      worktree_state: 'full' as const,
      surfaces: null,
    };
    expect(() => parseProtectionClaim(wire)).toThrow(/surfaces.*got null/);
  });

  it('rejects mixed-type surface entries — primitive in surfaces array', () => {
    const wire = {
      schema_version: PROTECTION_CLAIM_SCHEMA_VERSION,
      worktree_state: 'full' as const,
      surfaces: [{ identifier: 'ok', state: 'participating' }, 42],
    };
    expect(() => parseProtectionClaim(wire)).toThrow(/SurfaceClaim/);
  });

  it('rejects an array-shaped envelope (Array.isArray catches it before field access)', () => {
    // Without the `Array.isArray(value)` guard in `asObject`,
    // `typeof [] === 'object'` would pass and the parser would
    // access numeric-indexed fields. Pin the guard.
    expect(() =>
      parseProtectionClaim(['anvil.protection-claim.v1', 'full', []] as unknown)
    ).toThrow(/ProtectionClaim/);
  });
});
