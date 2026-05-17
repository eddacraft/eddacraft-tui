/**
 * MLP2-051e: cross-surface protection-claim parity — TS leg.
 *
 * The Rust integration test in
 * `crates/anvil-cli/tests/protection_claim_cross_surface.rs` drives a
 * fixed `DaemonStatusV1` through every Rust render surface (CLI
 * status, CLI doctor, MCP shim) and pins the canonical claim bytes in
 * `crates/anvil-cli/tests/fixtures/status_v1/cross_surface/<case>.json`.
 *
 * This test reads those same fixture bytes from the TS driver-client
 * side and asserts that `parseProtectionClaim` produces a typed claim
 * matching the §14 closed-set values the Rust surfaces agreed on.
 *
 * If the fixture set drifts — file added, removed, or renamed — this
 * test fails with the missing case name, blocking a Rust-side change
 * that would otherwise silently break the TS driver-client contract.
 */
import { readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

import {
  PROTECTION_CLAIM_SCHEMA_VERSION,
  parseProtectionClaim,
  type ProtectionClaim,
  type SurfaceClaimState,
  type WorktreeClaimState,
} from './types.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Cross-surface fixtures live in the anvil-cli crate so the Rust
 * surfaces own the canonical bytes. The TS package reads them through
 * a workspace-relative path; both crates and packages share the same
 * monorepo root.
 */
const FIXTURES_DIR = resolve(
  __dirname,
  '../../../../crates/anvil-cli/tests/fixtures/status_v1/cross_surface'
);

interface ParityExpectation {
  readonly worktreeState: WorktreeClaimState;
  readonly surfaces: ReadonlyArray<readonly [string, SurfaceClaimState]>;
}

/**
 * Mirrors the `CASES` table in
 * `protection_claim_cross_surface.rs`. Update both sides together when
 * a new case is added — the Rust side regenerates the fixture, the TS
 * side updates the expectation.
 */
const EXPECTATIONS: Record<string, ParityExpectation> = {
  unprotected: {
    worktreeState: 'unprotected',
    surfaces: [],
  },
  'pre-write-daemon-single-session': {
    worktreeState: 'pre-write-daemon',
    surfaces: [['sess-alpha', 'participating']],
  },
  'pre-write-daemon-tagged-session': {
    worktreeState: 'pre-write-daemon',
    surfaces: [['claude/agent-7#1700000000', 'participating']],
  },
  'degraded-protection-all-fenced': {
    worktreeState: 'degraded-protection',
    surfaces: [['sess-fenced', 'quarantined']],
  },
  'degraded-protection-mixed-fence': {
    worktreeState: 'degraded-protection',
    surfaces: [
      ['sess-clean', 'participating'],
      ['sess-fenced', 'quarantined'],
    ],
  },
  'warming-ipc-draining': {
    worktreeState: 'warming',
    surfaces: [['sess-drain', 'detached']],
  },
};

function listFixtures(): string[] {
  return readdirSync(FIXTURES_DIR)
    .filter((name) => name.endsWith('.json'))
    .map((name) => name.replace(/\.json$/, ''))
    .sort();
}

function loadFixture(name: string): ProtectionClaim {
  const raw = readFileSync(resolve(FIXTURES_DIR, `${name}.json`), 'utf8');
  return parseProtectionClaim(JSON.parse(raw));
}

describe('MLP2-051e cross-surface ProtectionClaim parity (TS leg)', () => {
  it('parses every Rust-emitted fixture into the typed closed-set shape', () => {
    for (const [name, expectation] of Object.entries(EXPECTATIONS)) {
      const claim = loadFixture(name);
      expect(claim.schema_version, `${name}: schema_version`).toBe(PROTECTION_CLAIM_SCHEMA_VERSION);
      expect(claim.worktree_state, `${name}: worktree_state`).toBe(expectation.worktreeState);
      expect(claim.surfaces, `${name}: surface count`).toHaveLength(expectation.surfaces.length);
      expectation.surfaces.forEach(([identifier, state], i) => {
        expect(claim.surfaces[i]?.identifier, `${name}: surfaces[${i}].identifier`).toBe(
          identifier
        );
        expect(claim.surfaces[i]?.state, `${name}: surfaces[${i}].state`).toBe(state);
      });
    }
  });

  it('keeps the TS expectation table aligned with the fixture set on disk', () => {
    const onDisk = listFixtures();
    const declared = Object.keys(EXPECTATIONS).sort();
    expect(onDisk).toEqual(declared);
  });

  it('round-trips every fixture through JSON.stringify without losing fields', () => {
    for (const name of Object.keys(EXPECTATIONS)) {
      const claim = loadFixture(name);
      const reSerialised = JSON.stringify(claim);
      const reParsed = parseProtectionClaim(JSON.parse(reSerialised));
      expect(reParsed, `${name}: TS round-trip`).toEqual(claim);
    }
  });
});
