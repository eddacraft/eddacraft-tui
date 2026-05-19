// Council review C1 (2026-04-24) pinned: "missing registry must not
// silently degrade." The original protection lived in
// `load_registry_or_err`, which turned a `registry: None` from the
// Rust loader into a `GenericFailure` so callers couldn't accidentally
// rely on a zero-warning "clean" scan from an unloaded catalogue.
//
// Issue #1630 (PR #1725) makes that protection structural rather than
// runtime: `crates/anvil-checks/src/antipattern/registry_loader.rs`
// now embeds `patterns/compiled/registry.json` at compile time via
// `include_str!` and falls back to it whenever path-based resolution
// returns nothing. The catalogue is now atomic with the binary, so the
// "registry unavailable" failure mode the original C1 tests guarded
// against no longer exists for missing / unresolvable paths.
//
// This file pins the *new* contract:
//
//   1. A missing `ANVIL_REGISTRY_PATH` does NOT crash — the embedded
//      catalogue covers it (the user-visible regression #1630 closed).
//   2. The catalogue is non-empty and behaves like a real registry —
//      `getDefaultPatternsJson` returns patterns, `getPatternJson` can
//      look up a known id, `scanArtifactJson` produces real diagnostics
//      on a known-bad input (no silent zero-warning "clean" pass).
//   3. The C1 throw contract is preserved for the narrower case it
//      still applies to: a *malformed* override file. In that case the
//      Rust resolver returns `ResolvedPath::Found(path)`, parses, and
//      surfaces `registry: None` with a parse warning;
//      `load_registry_or_err` turns that into the documented throw.
//
// `node --test` runs each `*.test.mjs` file in a child process, so the
// `ANVIL_REGISTRY_PATH` we set here is isolated from `parity.test.mjs`
// and `pattern-registry.test.mjs`.
//
// Run via: `pnpm --filter @eddacraft/anvil-checks-native test`.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { randomUUID } from 'node:crypto';

// Point the Rust loader at a path that cannot exist. With #1630 the
// loader's `ResolvedPath::OverrideMissing` branch warns and falls back
// to the compile-time-embedded catalogue, so this test covers the
// production "stock install" path (where no override is set and no
// upward walk finds the workspace registry) without mutating CWD.
const missingRegistryPath = join(tmpdir(), `anvil-missing-registry-${randomUUID()}.json`);
assert.equal(
  existsSync(missingRegistryPath),
  false,
  'test precondition: missingRegistryPath must not exist'
);
process.env.ANVIL_REGISTRY_PATH = missingRegistryPath;

const { scanArtifactJson, getDefaultPatternsJson, getPatternJson } = await import('../index.js');

test('getDefaultPatternsJson returns the embedded catalogue when override is missing', () => {
  // #1630: missing override no longer throws; the embedded catalogue
  // covers it. The previous contract (throw with `anvil scanner registry
  // unavailable`) is now structurally enforced — the catalogue is atomic
  // with the binary, so "unavailable" doesn't happen for missing paths.
  const json = getDefaultPatternsJson();
  const patterns = JSON.parse(json);
  assert.ok(Array.isArray(patterns), 'patterns must be an array');
  assert.ok(patterns.length > 0, 'embedded catalogue must contain at least one default pattern');
});

test('getPatternJson can look up a known id from the embedded catalogue', () => {
  // AP-001 ("Broad eslint-disable") is a stable id in the workspace
  // registry; the embedded copy must carry it. A null return would
  // mean the embedded snapshot drifted from the source.
  const json = getPatternJson('AP-001');
  assert.ok(json, 'AP-001 must be present in the embedded catalogue');
  const pattern = JSON.parse(json);
  assert.equal(pattern.id, 'AP-001');
});

test('getPatternJson returns null for unknown ids (distinguishable from registry-missing)', () => {
  // Pre-#1630, "registry unavailable" and "id not present" were
  // disambiguated by throw-vs-null. Post-#1630 the registry is always
  // available, so "id not present" is the *only* null-return path.
  // Pin that — a healthy registry returns null for unknown ids; only a
  // parse-failed override throws (see `load_failures_on_a_malformed_*`
  // below).
  const result = getPatternJson('definitely-not-a-real-pattern-id');
  assert.equal(result, null);
});

test('scanArtifactJson produces real diagnostics on known-bad input', () => {
  // The most dangerous Council C1 case was "missing registry returns a
  // zero-warning clean scan." The embedded fallback closes this
  // structurally: a known-bad input (broad eslint-disable) must surface
  // warnings, proving the scan ran against a real catalogue.
  const resultJson = scanArtifactJson(
    JSON.stringify({
      kind: 'source',
      reference: 'fixtures/sample.ts',
      content: '/* eslint-disable */\nconst x = 42;\n',
    }),
    null
  );
  const result = JSON.parse(resultJson);
  assert.ok(
    Array.isArray(result.warnings) && result.warnings.length > 0,
    'embedded catalogue must produce diagnostics on a known-bad input; ' +
      'a zero-warning scan would indicate the registry never loaded'
  );
});

test('load failures on a malformed override still throw (C1 contract preserved)', () => {
  // The C1 throw contract still applies to the case where a real on-disk
  // override resolves but parses badly — the Rust loader returns
  // `registry: None` with a parse warning, and the napi wrapper turns
  // that into a `GenericFailure`. This narrower case is the one place a
  // user can still produce an unloadable registry, and it should surface
  // loudly rather than fall back silently.
  // `mkdtempSync` atomically creates a private dedicated directory
  // owned by this process — avoids the CodeQL "insecure temp file
  // creation" pattern that `tmpdir() + randomUUID()` triggers, even
  // though randomUUID provides ~128 bits of entropy in practice.
  const corruptDir = mkdtempSync(join(tmpdir(), 'anvil-corrupt-'));
  const corruptPath = join(corruptDir, 'registry.json');
  writeFileSync(corruptPath, '{ this is not valid json');
  process.env.ANVIL_REGISTRY_PATH = corruptPath;

  // The Rust loader caches per resolved path; this is a fresh path so
  // it will re-enter `parse_registry` and fail. The previous
  // `missingRegistryPath` cache entry remains for that path.
  assert.throws(
    () => getDefaultPatternsJson(),
    /anvil scanner registry unavailable/,
    'a malformed override must surface a loud parse error, not a silent fallback'
  );

  // Restore the original env so subsequent tests in this file (if any
  // later land) see the missing-path state.
  process.env.ANVIL_REGISTRY_PATH = missingRegistryPath;
});
