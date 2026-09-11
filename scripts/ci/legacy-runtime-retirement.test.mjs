import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const root = fileURLToPath(new URL('../../', import.meta.url));
const read = (path) => readFileSync(resolve(root, path), 'utf8');
const json = (path) => JSON.parse(read(path));

test('retired JS runtime implementations cannot enter a build or source package', () => {
  for (const path of [
    'packages/anvil/runtime/src/watch',
    'packages/anvil/runtime/src/concurrency',
    'packages/anvil/runtime/src/cache',
  ]) {
    assert.equal(existsSync(resolve(root, path)), false, path);
  }
  const ember = resolve(root, 'packages/edda-stack/src/ember');
  assert.deepEqual(readdirSync(ember), ['README.md']);
});

test('published entry points cannot resolve retired JS services', () => {
  const runtime = json('packages/anvil/runtime/package.json');
  assert.deepEqual(Object.keys(runtime.exports).sort(), ['.', './feature-flags']);
  assert.deepEqual(runtime.exports['./feature-flags'], {
    types: './dist/feature-flags/index.d.ts',
    import: './dist/feature-flags/index.js',
  });
  assert.equal(
    existsSync(resolve(root, 'packages/anvil/runtime/src/feature-flags/index.ts')),
    true
  );
  assert.equal(
    read('packages/anvil/runtime/src/index.ts')
      .replace(/\/\*[\s\S]*?\*\//g, '')
      .trim(),
    ''
  );
  const packageOverview = read('packages/anvil/README.md');
  assert.doesNotMatch(packageOverview, /FileCache|runtime - Orchestration and I\/O/);
  assert.match(packageOverview, /@eddacraft\/anvil-runtime\/feature-flags/);
  const stack = json('packages/edda-stack/package.json');
  assert.equal(stack.exports['./ember'], undefined);
  assert.equal(stack.dependencies['better-sqlite3'], undefined);
  assert.doesNotMatch(read('packages/edda-stack/src/index.ts'), /from ['"].*ember\//);
});

test('Ember is a linked, unlisted, default-off Rust-only opt-in', () => {
  const flag = json('flags/manifest.json').flags.find((entry) => entry.key === 'ember.enabled');
  assert.ok(flag);
  assert.equal(flag.defaultVariant, 'disabled');
  assert.equal(flag.variants.find((entry) => entry.key === 'disabled').value, false);
  assert.equal(flag.class, 'rollout');
  assert.equal(flag.createdFor, 'EMBERRS-001');
  assert.ok(flag.expiryOrReviewDate);
  assert.deepEqual(flag.targeting ?? [], []);
  assert.deepEqual(flag.controlsProductFeatures, ['ember']);
  const catalogue = json('flags/surfaces.json');
  assert.deepEqual(catalogue.productFeatures.find((entry) => entry.key === 'ember').flagLinkage, {
    disposition: 'linked',
    flagKeys: ['ember.enabled'],
  });
  assert.equal(catalogue.deliverySurfaces.find((entry) => entry.key === 'cli.ember').listed, false);
});
