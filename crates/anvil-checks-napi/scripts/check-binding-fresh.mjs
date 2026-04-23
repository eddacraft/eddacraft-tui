// Pretest guard: refuse to run tests against a missing or stale .node binary.
//
// Local dev rebuilds the binding via `pnpm build`; CI downloads a fresh
// artefact into the package dir. Both paths produce a .node whose mtime
// is newer than src/lib.rs. If a developer edits the binding without
// rebuilding, this script fires before pnpm test would silently load a
// stale binary and produce misleading results.

import { statSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const pkgDir = join(here, '..');
const srcDir = join(pkgDir, 'src');

function newestMtime(dir) {
  let newest = 0;
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    const mtime = entry.isDirectory()
      ? newestMtime(full)
      : statSync(full).mtimeMs;
    if (mtime > newest) newest = mtime;
  }
  return newest;
}

const nodeFiles = readdirSync(pkgDir).filter((f) => f.endsWith('.node'));
if (nodeFiles.length === 0) {
  console.error(
    'No .node binding found in ' +
      pkgDir +
      '. Run `pnpm --filter @eddacraft/anvil-checks-native build` first.',
  );
  process.exit(1);
}

const bindingMtime = Math.max(
  ...nodeFiles.map((f) => statSync(join(pkgDir, f)).mtimeMs),
);
const srcMtime = newestMtime(srcDir);

if (bindingMtime < srcMtime) {
  const lagSec = ((srcMtime - bindingMtime) / 1000).toFixed(0);
  console.error(
    `.node binding is ${lagSec}s older than src/. Rebuild with ` +
      '`pnpm --filter @eddacraft/anvil-checks-native build` before running tests.',
  );
  process.exit(1);
}
