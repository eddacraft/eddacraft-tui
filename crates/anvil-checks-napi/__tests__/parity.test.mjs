// TSRET-001 spike test.
//
// Asserts the JS↔Rust wire round-trip works (artifact in, warning list out
// matching what the underlying scan_artifact call produces) and records
// cold/warm call timings.
//
// NOTE: this is not a CLI-diff parity test. The binding emits a per-artifact
// `ScanResultOutput` that's deliberately distinct from the CLI's aggregate
// `CheckOutput` shape (see lib.rs crate-level doc comment). Warning content
// is parity by construction; envelope is not. A golden-snapshot diff against
// CLI output is a TSRET-003 prerequisite.
//
// Run via: `pnpm --filter @eddacraft/anvil-checks-native test`.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { performance } from 'node:perf_hooks';

const here = dirname(fileURLToPath(import.meta.url));
const fixturePath = join(here, '..', 'fixtures', 'sample.ts');
const fixtureContent = readFileSync(fixturePath, 'utf8');

const { scanArtifactJson, version } = await import('../index.js');

function napiScan() {
  return JSON.parse(
    scanArtifactJson(
      JSON.stringify({
        kind: 'source',
        // Use a stable reference so the JSON diff isn't affected by the
        // absolute fixture path.
        reference: 'fixtures/sample.ts',
        content: fixtureContent,
      }),
      null,
    ),
  );
}

test('napi binding loads', () => {
  assert.equal(typeof version, 'function');
  assert.match(version(), /^\d+\.\d+\.\d+/);
});

test('napi scan produces warnings on the fixture', () => {
  const result = napiScan();
  assert.equal(result.file, 'fixtures/sample.ts');
  assert.equal(result.artifactType, 'source');
  assert.ok(
    result.warnings.length >= 2,
    `expected >= 2 warnings, got ${result.warnings.length}`,
  );
  // AP-003 (any usage) and DD-001 (untracked TODO) must both fire on the
  // fixture; AP-001 (eslint-disable) is opt-in so we don't assert it here.
  const ids = new Set(result.warnings.map((w) => w.id));
  assert.ok(ids.has('AP-003'), `AP-003 missing from ${[...ids].join(',')}`);
  assert.ok(ids.has('DD-001'), `DD-001 missing from ${[...ids].join(',')}`);
});

test('napi options are accepted and filter rules', () => {
  const result = JSON.parse(
    scanArtifactJson(
      JSON.stringify({
        kind: 'source',
        reference: 'fixtures/sample.ts',
        content: fixtureContent,
      }),
      JSON.stringify({ patterns: ['AP-003'], includeOptIn: true }),
    ),
  );
  assert.deepEqual(result.patternsChecked, ['AP-003']);
  assert.ok(result.warnings.every((w) => w.id === 'AP-003'));
});

test('napi rejects unknown artifact kind', () => {
  assert.throws(
    () =>
      scanArtifactJson(
        JSON.stringify({ kind: 'invalid', reference: 'x', content: '' }),
        null,
      ),
    /unknown artifact kind/,
  );
});

test('cold and warm call timings (informational)', () => {
  // 200 iterations is enough to dwarf timer noise on the small fixture
  // without making the test slow. Numbers print to stdout for the spike's
  // record; no assertion — that lives in TSRET-002 once we have a target.
  const t0 = performance.now();
  napiScan();
  const cold = performance.now() - t0;

  const iterations = 200;
  const start = performance.now();
  for (let i = 0; i < iterations; i++) napiScan();
  const warmAvg = (performance.now() - start) / iterations;

  console.log(
    `[TSRET-001] cold call: ${cold.toFixed(3)}ms; warm avg over ${iterations} calls: ${warmAvg.toFixed(4)}ms`,
  );
});
