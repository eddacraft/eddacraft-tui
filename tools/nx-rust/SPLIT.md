# Splitting `@eddacraft/nx-rust` Back Out

> **Vendored from [eddacraft/nxrust](https://github.com/EddaCraft/nxrust) at
> commit `646231204d7972de22f55b670b7e2cfabb4d5d0e`.** Update this header on
> every re-sync (see §Sync invariants below) so the upstream pin is verifiable.

This package is **vendored** into `anvil-001`. The original intent (per
`plans/modules/nx-rust-plugin.aps.md`) was an in-repo Nx plugin linked via the
pnpm workspace protocol; the upstream standalone repo exists but is unpublished.
This vendor copy unblocks CI without forcing a publish flow.

If you ever want to extract this back into a standalone published package
(publish to npm, share with other monorepos, give it independent release
cadence), the path is short — but it requires reverting the anvil-specific
divergences listed below.

## Anvil-specific divergences from upstream

These differ from upstream and **must be reverted** at split time:

- **`package.json` `name`** — vendored as `@eddacraft/nx-rust` (matches the APS
  plan and removes the npm-name-squat surface). Upstream is `nxrust` (unscoped,
  publish-ready).
- **`package.json` `license`** — vendored as `PROPRIETARY` (matches
  `tools/generators/` convention; we have no need to ship Apache-2.0 licence
  text inside anvil's monorepo). Upstream is `Apache-2.0`.
- **`LICENSE` file** — removed in the vendor copy (PROPRIETARY packages inside
  anvil don't ship a LICENSE file). Upstream keeps the Apache-2.0 text.
- **`package.json` `publishConfig`** — removed in the vendor copy (we're not
  publishing this from anvil's tree). Upstream keeps `{ "access": "public" }`.
- **`package.json` `files` array** — `"LICENSE"` entry removed (no LICENSE file
  to ship).
- **`package.json` devDependencies** — `typescript` bumped to `~6.0.3` and
  `vitest` bumped to `^4.1.4` to match anvil's root workspace versions and
  prevent dual-version trees in the lockfile. Upstream pins `^5.6.0` and
  `^2.1.0` respectively.
- **`.gitignore`** — anvil's root `.gitignore` excludes `dist` project-wide, so
  the vendored `.gitignore` adds `!/dist/` (rooted) to re-include only the
  package-level dist. dist/ **is committed** in this vendor copy because Nx
  loads plugins at process startup, before any install-time build script could
  fire. Upstream `eddacraft/nxrust` keeps `dist/` ignored and rebuilt per
  release.
- **Executor namespace renamed** — every `'nxrust:<name>'` executor reference in
  `src/` (target-configs.ts, graph.ts, schema.json titles, doc comments) is
  rewritten to `'@eddacraft/nx-rust:<name>'`. Required because Nx looks up
  executors by their plugin's package name. `generators.json` `name` field
  follows the same rename. Upstream uses `nxrust:<name>` everywhere.
- **`chalk` removed** — vendor copy of `src/utils/cargo.ts` replaces the single
  `chalk.dim(...)` call with inline ANSI escapes (`\x1b[2m`/`\x1b[22m`). The
  package's `chalk@^4.1.2` dep was being shadowed by chalk@5 hoisted at the
  workspace root, breaking chalk's CJS default-import interop under the Nx
  plugin loader (`Cannot read properties of undefined (reading 'dim')`).
  Removing the dep entirely is simpler than dedupe-and-pin, and the single log
  line doesn't justify the dependency. Upstream keeps chalk and its single
  call site.
- **`tsconfig.json`** — `moduleResolution: "node"` (the deprecated alias)
  rewritten to `moduleResolution: "node10"` and `ignoreDeprecations: "6.0"`
  added so the bumped TS 6 compiler doesn't error on the deprecation warning.
  Upstream's tsconfig predates TS 6.

## Other anvil-specific differences (no revert needed)

- **`pnpm-lock.yaml`** — upstream has its own lockfile; not vendored.
  Dependencies resolve through anvil's root lockfile.
- **`bin/`, `lib/`, `aps-planning/`, `plans/`, `.claude/`, `.envrc`** — upstream
  development infrastructure (an APS planning skill, dev CLI helpers). Not
  vendored — they would collide with anvil's equivalents. When syncing source,
  remember these directories exist upstream and are intentionally not mirrored
  here.

## Sync invariants — fields to audit on every sync

When merging upstream changes into this vendor copy (`rsync` direction: upstream
→ vendor), audit these fields after the rsync to confirm the divergences above
are still in place:

- `package.json` `name` (must stay `@eddacraft/nx-rust`)
- `package.json` `license` (must stay `PROPRIETARY`)
- `package.json` `files` (must NOT include `"LICENSE"`)
- `package.json` `publishConfig` (must NOT be present)
- `package.json` `dependencies.chalk` (must NOT be present)
- `package.json` `devDependencies.typescript` (must stay `~6.0.3`)
- `package.json` `devDependencies.vitest` (must stay `^4.1.4`)
- `package.json` `scripts.prepare` — if upstream introduces one (e.g. for npm
  publish lifecycle), decide whether to keep it. pnpm runs `prepare` on
  workspace packages erratically; the original vendor intentionally has no
  `prepare`.
- `generators.json` `name` field (must stay `@eddacraft/nx-rust`, not `nxrust`)
- `src/utils/cargo.ts` (must NOT import `chalk`; uses inline ANSI escapes)
- Every `'nxrust:<name>'` literal in `src/` (must read `'@eddacraft/nx-rust:<name>'`
  in target-configs.ts, graph.ts, schema.json `title` fields, and doc
  comments)
- `tsconfig.json` `moduleResolution` (must stay `"node10"`, not `"node"`) and
  `ignoreDeprecations: "6.0"` line (must be present)
- `LICENSE` (must NOT be present in the vendor copy)
- `.gitignore` `!/dist/` line (must stay; not present upstream)
- The "Vendored from … at commit `<sha>`" header at the top of this file (must
  be updated to the synced upstream SHA)

If any of these snap back to upstream values during rsync, fix them before
committing the sync.

## Splitting back out

When the time comes (e.g. a second monorepo wants to consume it, or
`@eddacraft/nx-rust` needs an independent release cadence):

1. **Revert anvil-specific package.json changes**, restoring upstream
   identifiers:

   ```diff
   -"name": "@eddacraft/nx-rust",
   +"name": "nxrust",
   -"license": "PROPRIETARY",
   +"license": "Apache-2.0",
    "files": [
      "dist/**/*.js",
      ...
   +  "LICENSE",
      "README.md",
      "CHANGELOG.md"
    ],
   +"publishConfig": { "access": "public" },
    "devDependencies": {
      ...
   -  "typescript": "~6.0.3",
   -  "vitest": "^4.1.4"
   +  "typescript": "^5.6.0",
   +  "vitest": "^2.1.0"
    }
   ```

2. **Restore the LICENSE file** (Apache-2.0 text, copied from upstream).

3. **Sync src/CHANGELOG.md/README.md back upstream** (this picks up any
   anvil-side fixes):

   ```sh
   REPO_ROOT=$(git rev-parse --show-toplevel)
   rsync -a --delete \
     --exclude=node_modules --exclude=.nx \
     --exclude=.gitignore --exclude=SPLIT.md \
     "$REPO_ROOT/tools/nx-rust/" \
     ~/Projects/src/nxrust/
   ```

   `.gitignore` and `SPLIT.md` are anvil-only and excluded.

4. **Publish from upstream**:

   ```sh
   cd ~/Projects/src/nxrust
   pnpm install
   pnpm build
   pnpm publish --access public
   ```

5. **Switch anvil to the published version**:

   ```diff
    "devDependencies": {
   -  "@eddacraft/nx-rust": "workspace:*"
   +  "nxrust": "^0.1.0"
    }
   ```

   And in `nx.json`:

   ```diff
    {
   -  "plugin": "@eddacraft/nx-rust"
   +  "plugin": "nxrust"
    }
   ```

6. **Delete the vendor copy**:

   ```sh
   git rm -r tools/nx-rust
   pnpm install   # regenerates lockfile against the registry version
   ```

7. **Commit, PR, done.** Don't forget to remove the
   `--exclude=@eddacraft/nx-rust` flags from the root `package.json` `build`,
   `test`, `test:coverage:ts`, and `typecheck` scripts (they were added so
   anvil's CI doesn't run the vendored package's own pipeline; once the package
   is external, those exclusions are stale).

## Local development on this vendor copy

If you only need to iterate from inside this repo:

```sh
cd tools/nx-rust
pnpm build       # rebuilds dist/ (CI verifies this matches src/)
cd "$(git rev-parse --show-toplevel)"
pnpm install     # picks up the rebuilt workspace package
```

If you prefer to develop in the upstream repo and have changes reflect here
without a copy step, use `pnpm link` from the upstream checkout into this repo.
That gives you the dev-loop without changing the committed dependency.

## CI guard

Anvil's `lint` job runs `pnpm --filter @eddacraft/nx-rust build` and fails if
`git diff tools/nx-rust/dist/` reports a delta. This catches the case where
someone edits `src/` and forgets to rebuild `dist/`. If you see that failure,
run `cd tools/nx-rust && pnpm build` and commit the resulting dist diff.
