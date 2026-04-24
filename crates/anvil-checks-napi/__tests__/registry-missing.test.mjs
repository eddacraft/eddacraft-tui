// Council review C1 (2026-04-24): the napi binding used to silently
// return an empty catalogue / zero-warning scan when the compiled
// pattern registry couldn't be found or parsed. In a long-lived editor
// host that manifested as "diagnostics quietly stop working" with no
// signal. The fix routes every entry point through `load_registry_or_err`
// which turns the missing-registry case into a `GenericFailure` naming
// the fault.
//
// This test pins the failure path. `node --test` runs each `*.test.mjs`
// file in a child process, so setting `ANVIL_REGISTRY_PATH` here does
// not leak into `parity.test.mjs` or `pattern-registry.test.mjs`.
//
// Run via: `pnpm --filter @eddacraft/anvil-checks-native test`.

import { test } from 'node:test';
import assert from 'node:assert/strict';

// Point the Rust loader at a path that cannot exist. `resolve_registry_path`
// rejects non-existent paths, so the loader enters the "not found" branch
// and `load_registry_or_err` surfaces a `GenericFailure`. Must be set
// before `../index.js` is imported — the napi binding's static state is
// established on load.
process.env.ANVIL_REGISTRY_PATH = '/definitely/does/not/exist/anywhere/registry.json';

const { scanArtifactJson, getDefaultPatternsJson, getPatternJson } = await import(
  '../index.js'
);

test('getDefaultPatternsJson throws when registry is unavailable', () => {
  assert.throws(
    () => getDefaultPatternsJson(),
    /anvil scanner registry unavailable/,
    'missing registry must be a loud error, not a silent empty catalogue'
  );
});

test('getDefaultPatternsJson error includes a remediation hint', () => {
  try {
    getDefaultPatternsJson();
    assert.fail('expected throw');
  } catch (err) {
    assert.match(
      err.message,
      /anvil doctor|registry\.json/,
      'error must point the user at how to recover'
    );
  }
});

test('getPatternJson throws when registry is unavailable (not null-on-miss)', () => {
  // A null return would be indistinguishable from "id not found in a
  // healthy registry" — which silently downgrades the caller's error
  // handling. The registry-unavailable case must be distinct.
  assert.throws(
    () => getPatternJson('AP-001'),
    /anvil scanner registry unavailable/,
    'unloadable registry must not masquerade as unknown-id'
  );
});

test('scanArtifactJson throws when registry is unavailable', () => {
  // Without the fix, an unloadable registry produced a scan with zero
  // patterns checked and zero warnings — indistinguishable from a clean
  // pass. This is the most dangerous silent-empty case because it
  // *looks* like enforcement is working.
  assert.throws(
    () =>
      scanArtifactJson(
        JSON.stringify({
          kind: 'source',
          reference: 'fixtures/sample.ts',
          content: 'const x: any = 42;\n',
        }),
        null
      ),
    /anvil scanner registry unavailable/,
    'missing registry must not produce a zero-warning "clean" scan'
  );
});
