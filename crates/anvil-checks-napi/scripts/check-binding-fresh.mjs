// Pretest guard: refuse to run tests against a missing or stale .node binary.
//
// Local dev rebuilds the binding via `pnpm build`; CI downloads a fresh
// artefact into the package dir. Both paths produce a .node whose mtime
// is newer than every native build input. If a developer edits the binding
// (or bumps a dependency) without rebuilding, this script fires before
// `pnpm test` would silently load a stale binary and produce misleading
// results.
//
// The freshness baseline is the newest mtime across ALL inputs that can
// change the compiled `.node`, not just `src/`:
//   - `src/`        — the Rust sources of the binding.
//   - `Cargo.toml`  — dependency versions and feature flags.
//   - `build.rs`    — the build script (napi codegen inputs).
//   - workspace `Cargo.lock` — transitive registry / Rust-dependency pins,
//     so a `cargo update` or a sibling-crate bump that changes the binding
//     is caught even when no file under `src/` was touched.
// Previously only `src/` was considered, so a stale binding survived
// `Cargo.toml` / lockfile / dependency changes.

import { statSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const pkgDir = join(here, '..');
const srcDir = join(pkgDir, 'src');
// crates/anvil-checks-napi -> repo root (two levels up).
const workspaceRoot = join(pkgDir, '..', '..');

function newestMtime(dir) {
  let newest = 0;
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    const mtime = entry.isDirectory() ? newestMtime(full) : statSync(full).mtimeMs;
    if (mtime > newest) newest = mtime;
  }
  return newest;
}

// Mtime of a single file, or 0 if it does not exist (optional inputs such as
// `build.rs` must not hard-fail the guard when absent).
function fileMtime(path) {
  try {
    return statSync(path).mtimeMs;
  } catch (err) {
    // A genuinely absent optional input (e.g. a crate with no build.rs)
    // contributes nothing to the baseline. Any other stat failure
    // (permission, I/O) is a real problem that would silently weaken the
    // freshness check if treated as "0" — surface it instead.
    if (err && err.code === 'ENOENT') return 0;
    throw err;
  }
}

const nodeFiles = readdirSync(pkgDir).filter((f) => f.endsWith('.node'));
if (nodeFiles.length === 0) {
  console.error(
    'No .node binding found in ' +
      pkgDir +
      '. Run `pnpm --filter @eddacraft/anvil-checks-native build` first.'
  );
  process.exit(1);
}

// Use the OLDEST present binding, not the newest. A newer but unrelated
// `.node` (e.g. a leftover from another target left in the package dir)
// must not mask a stale binding that tests could actually load — guard on
// the worst case so any stale `.node` fires.
const bindingMtime = Math.min(...nodeFiles.map((f) => statSync(join(pkgDir, f)).mtimeMs));

const inputMtime = Math.max(
  newestMtime(srcDir),
  fileMtime(join(pkgDir, 'Cargo.toml')),
  fileMtime(join(pkgDir, 'build.rs')),
  fileMtime(join(workspaceRoot, 'Cargo.lock'))
);

if (bindingMtime < inputMtime) {
  const lagSec = ((inputMtime - bindingMtime) / 1000).toFixed(0);
  console.error(
    `.node binding is ${lagSec}s older than a native build input ` +
      '(src/, Cargo.toml, build.rs, or workspace Cargo.lock). Rebuild with ' +
      '`pnpm --filter @eddacraft/anvil-checks-native build` before running tests.'
  );
  process.exit(1);
}
