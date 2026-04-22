# Splitting `nxrust` Back Out

This package is **vendored** into `anvil-001` from upstream
[eddacraft/nxrust](https://github.com/EddaCraft/nxrust). The original
intent (per `plans/modules/nx-rust-plugin.aps.md`) was an in-repo Nx
plugin linked via the pnpm workspace protocol; the upstream standalone
repo exists but is unpublished. This vendor copy unblocks CI without
forcing a publish flow.

If you ever want to extract `nxrust` back into a standalone package
(publish to npm, share with other monorepos, give it independent release
cadence), the path is intentionally short:

## Pre-vendor invariants — keep these aligned with upstream

To make a future split mechanical, the following fields stay **identical
to the upstream `eddacraft/nxrust` package** and are not bent to fit
anvil-specific conventions:

- `package.json` `name` (`nxrust` — unscoped, publish-ready)
- `package.json` `version` (currently `0.1.0`)
- `package.json` `license` (`Apache-2.0`)
- `package.json` `repository.url` (`https://github.com/EddaCraft/nxrust.git`)
- `package.json` `homepage` (same)
- `package.json` `publishConfig` (`{ "access": "public" }`)
- `package.json` `main`, `types`, `files` (publish-shape)
- `package.json` `dependencies`, `peerDependencies`, `devDependencies`
- `tsconfig.json`, `tsconfig.spec.json`, `vitest.config.ts` — local to
  the package, not inherited from anvil's root configs
- `tools/copy-assets.mjs` — the package's own build helper
- `README.md`, `LICENSE`, `CHANGELOG.md`

If you find yourself wanting to "fix up" any of these to match an
anvil-specific style — don't. Add a wrapper in anvil instead, or accept
the divergence as a one-way intentional fork.

## Anvil-specific differences from upstream

These differ from upstream and are flagged so a re-sync knows what to
revert:

- **`.gitignore`** — anvil's root `.gitignore` has `dist`, so the
  vendored `.gitignore` adds `!dist/` to re-include it. dist/ **is**
  committed in this vendor copy because Nx loads plugins at process
  startup, before any install-time build script could fire.
  Upstream `eddacraft/nxrust` keeps `dist/` ignored and rebuilt per
  release.
- **`pnpm-lock.yaml`** — upstream has its own lockfile; not vendored.
  Dependencies resolve through anvil's root lockfile.
- **`bin/`, `lib/`, `aps-planning/`, `plans/`, `.claude/`, `.envrc`** —
  upstream development infrastructure (an APS planning skill, dev CLI
  helpers). Not vendored — they would collide with anvil's equivalents.
  When syncing source, remember these directories exist upstream and
  are intentionally not mirrored here.

## Splitting back out

When the time comes (e.g. a second monorepo wants to consume it, or
`nxrust` needs an independent release cadence):

1. **Sync any anvil-side improvements back upstream**:

   ```sh
   rsync -a --delete \
     --exclude=node_modules --exclude=.nx \
     --exclude=.gitignore --exclude=SPLIT.md \
     ~/Projects/src/EddaCraft/anvil-001/tools/nx-rust/ \
     ~/Projects/src/nxrust/
   ```

   Excluded files: `.gitignore` (intentionally diverges), `SPLIT.md`
   (anvil-only). If `dist/` was edited here without rebuilding from
   source, rebuild upstream from the synced `src/` before committing
   there.

2. **Publish from upstream**:

   ```sh
   cd ~/Projects/src/nxrust
   pnpm install
   pnpm build
   pnpm publish --access public
   ```

3. **Switch anvil to the published version**:

   ```diff
    "devDependencies": {
   -  "nxrust": "workspace:*"
   +  "nxrust": "^0.1.0"
    }
   ```

4. **Delete the vendor copy**:

   ```sh
   git rm -r tools/nx-rust
   pnpm install   # regenerates lockfile against the registry version
   ```

5. **Commit, PR, done.**

## Local development on this vendor copy

If you only need to iterate on `nxrust` from inside this repo:

```sh
cd tools/nx-rust
pnpm build       # rebuilds dist/
cd ../..
pnpm install     # picks up the rebuilt workspace package
```

If you prefer to develop in the upstream repo and have changes reflect
here without a copy step, use `pnpm link` from the upstream checkout
into this repo. That gives you the dev-loop without changing the
committed dependency.
