/**
 * MLP2-051c: TypeScript mirror of the `ProtectionClaim` wire shape.
 *
 * This module mirrors `crates/anvil-kernel-types/src/protection_claim.rs`
 * (spec §14 from
 * `plans/specs/2026-05-07-anvil-multilayer-protection-architecture.md`).
 * The Rust side is authoritative; if the two drift, the Rust side wins
 * and this file is updated to match. The cross-language parity tests in
 * `./types.test.ts` pin the wire string against a captured serde output
 * so renames or field-order changes fail loudly on either side.
 *
 * Hand-rolled rather than Zod-backed to match the MLP2-029 / MLP2-030
 * pattern already in this package — adding a Zod dependency just for
 * three small parsers would expand the surface area without giving the
 * driver-client anything it does not already have.
 *
 * The closed-set state enums are modelled as string-literal unions plus
 * an `ALL_*_STATES` exhaustivity list so a future variant addition is a
 * compile-time discovery (the parsers' `switch (state)` is exhaustive
 * against the union) and a runtime test failure (the conformance test
 * pins the list length and order).
 */

/**
 * Schema version pinned for the JSON wire shape. Forward-compat rule:
 * additions of optional fields ride this version; semantically breaking
 * changes (state-name renames, field-type changes, removed states) bump
 * the major component. The Rust deserialise path rejects any other
 * value at the type boundary, so a consumer holding a `ProtectionClaim`
 * is guaranteed to be on the current major.
 */
export const PROTECTION_CLAIM_SCHEMA_VERSION = 'anvil.protection-claim.v1' as const;

/**
 * Per-worktree protection-claim state from spec §14.2. Ten closed-set
 * variants. Tooling treats unknown variants as a hard error — there is
 * no silent fallthrough to a default.
 *
 * Mirrors `WorktreeClaimState` on the Rust side
 * (`crates/anvil-kernel-types/src/protection_claim.rs`).
 */
export type WorktreeClaimState =
  | 'unprotected'
  | 'warming'
  | 'pre-write-embedded'
  | 'pre-write-daemon'
  | 'save-time-only'
  | 'full'
  | 'degraded-protection'
  | 'cross-boundary-mixed'
  | 'multi-daemon-detected'
  | 'path-uncertain';

/**
 * Every worktree-state value in declaration order. Pinned by the
 * conformance test against `WorktreeClaimState::all()` on the Rust
 * side. Adding a variant in Rust without updating this list fails the
 * cross-language parity test.
 */
export const ALL_WORKTREE_CLAIM_STATES: readonly WorktreeClaimState[] = [
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
] as const;

/**
 * Per-surface protection-claim state from spec §14.1. Eight closed-set
 * variants. Mirrors `SurfaceClaimState` on the Rust side.
 */
export type SurfaceClaimState =
  | 'unbound'
  | 'attached'
  | 'participating'
  | 'embedded-fallback'
  | 'degraded'
  | 'cross-boundary-refused'
  | 'quarantined'
  | 'detached';

/** Every surface-state value in declaration order. */
export const ALL_SURFACE_CLAIM_STATES: readonly SurfaceClaimState[] = [
  'unbound',
  'attached',
  'participating',
  'embedded-fallback',
  'degraded',
  'cross-boundary-refused',
  'quarantined',
  'detached',
] as const;

/**
 * Single surface's claim entry — identifier plus state. Mirrors
 * `SurfaceClaim` on the Rust side. The `identifier` is opaque to this
 * contract; the daemon decides naming.
 */
export interface SurfaceClaim {
  readonly identifier: string;
  readonly state: SurfaceClaimState;
}

/**
 * Aggregate protection claim for a worktree. The wire shape `anvil
 * status --json`, the MCP `validate_write` response, and `anvil
 * doctor` all emit (via the MLP2-051a/-051b lanes). Consumers
 * deserialise this instead of pattern-matching strings.
 *
 * `schema_version` is pinned at [`PROTECTION_CLAIM_SCHEMA_VERSION`];
 * the parser rejects any other value so a consumer holding an instance
 * can rely on the current major.
 */
export interface ProtectionClaim {
  readonly schema_version: typeof PROTECTION_CLAIM_SCHEMA_VERSION;
  readonly worktree_state: WorktreeClaimState;
  readonly surfaces: readonly SurfaceClaim[];
}

const WORKTREE_STATE_SET: ReadonlySet<string> = new Set(ALL_WORKTREE_CLAIM_STATES);
const SURFACE_STATE_SET: ReadonlySet<string> = new Set(ALL_SURFACE_CLAIM_STATES);

function asObject(value: unknown, what: string): Record<string, unknown> {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    throw new TypeError(
      `${what} must be a JSON object, got ${
        value === null ? 'null' : Array.isArray(value) ? 'array' : typeof value
      }`
    );
  }
  return value as Record<string, unknown>;
}

/**
 * Parse a single `SurfaceClaim` entry. Throws `TypeError` describing
 * the first malformed field — callers that prefer a `Result`-style API
 * wrap this in a `try`.
 *
 * Forward-compat: unknown extra keys on the wire are silently dropped,
 * matching the Rust struct's lack of `#[serde(deny_unknown_fields)]`.
 */
export function parseSurfaceClaim(value: unknown): SurfaceClaim {
  const obj = asObject(value, 'SurfaceClaim');
  const identifier = obj.identifier;
  if (typeof identifier !== 'string') {
    throw new TypeError(`SurfaceClaim.identifier must be a string, got ${typeof identifier}`);
  }
  if (identifier.length === 0) {
    throw new TypeError('SurfaceClaim.identifier must be non-empty');
  }
  const state = obj.state;
  if (typeof state !== 'string' || !SURFACE_STATE_SET.has(state)) {
    throw new TypeError(
      `SurfaceClaim.state must be one of the spec §14.1 closed-set values, got ${JSON.stringify(
        state
      )}`
    );
  }
  return { identifier, state: state as SurfaceClaimState };
}

/**
 * Parse a `ProtectionClaim` wire payload. Mirrors the Rust
 * `ProtectionClaim` deserialise path including the schema-version
 * guard: any value other than [`PROTECTION_CLAIM_SCHEMA_VERSION`] is
 * rejected at the type boundary.
 */
export function parseProtectionClaim(value: unknown): ProtectionClaim {
  const obj = asObject(value, 'ProtectionClaim');

  const schemaVersion = obj.schema_version;
  if (typeof schemaVersion !== 'string') {
    throw new TypeError(
      `ProtectionClaim.schema_version must be a string, got ${typeof schemaVersion}`
    );
  }
  if (schemaVersion !== PROTECTION_CLAIM_SCHEMA_VERSION) {
    throw new TypeError(
      `unknown ProtectionClaim.schema_version: ${JSON.stringify(
        schemaVersion
      )} (expected ${JSON.stringify(PROTECTION_CLAIM_SCHEMA_VERSION)})`
    );
  }

  const worktreeState = obj.worktree_state;
  if (typeof worktreeState !== 'string' || !WORKTREE_STATE_SET.has(worktreeState)) {
    throw new TypeError(
      `ProtectionClaim.worktree_state must be one of the spec §14.2 closed-set values, got ${JSON.stringify(
        worktreeState
      )}`
    );
  }

  const surfaces = obj.surfaces;
  if (!Array.isArray(surfaces)) {
    throw new TypeError(
      `ProtectionClaim.surfaces must be an array, got ${
        surfaces === null ? 'null' : typeof surfaces
      }`
    );
  }

  const parsedSurfaces = surfaces.map(parseSurfaceClaim);
  if (worktreeState === 'full' && parsedSurfaces.length === 0) {
    throw new TypeError('worktree_state "full" requires at least one protecting surface');
  }

  return {
    schema_version: PROTECTION_CLAIM_SCHEMA_VERSION,
    worktree_state: worktreeState as WorktreeClaimState,
    surfaces: parsedSurfaces,
  };
}

/**
 * MCP response adapter: parse the optional `protection_claim` field
 * from an `anvil_validate_write` MCP tool response. MLP2-051b made the
 * field wire-additive — present when the daemon supplied a snapshot,
 * omitted (or explicitly `null`) otherwise. A pre-MLP2-051b driver
 * pinned to the older shape parses the new response cleanly because
 * this adapter returns `undefined` for the missing-field case rather
 * than throwing.
 *
 * Throws if the response envelope itself is malformed, or if the
 * `protection_claim` field is present but doesn't match the closed-set
 * contract — half-typed claims would defeat the whole point of the
 * closed-set vocabulary.
 *
 * @see crates/anvil-cli/src/mcp/tools/validate_write.rs (Rust producer)
 */
export function parseOptionalProtectionClaimFromValidateWrite(
  response: unknown
): ProtectionClaim | undefined {
  const obj = asObject(response, 'validate_write response');
  const raw = obj.protection_claim;
  if (raw === undefined || raw === null) {
    return undefined;
  }
  return parseProtectionClaim(raw);
}
