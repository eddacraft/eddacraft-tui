# @eddacraft/anvil-checks-native

Node-API binding for the authoritative Anvil scanner crate
(`crates/anvil-checks`). Replaces the duplicate TypeScript scanner in
the VSCode extension and MCP server (see ADR-026 and the `TSRET`
module).

## Status

TSRET-001 spike — JSON-in / JSON-out wrapper around `scan_artifact`.
TSRET-002 in progress — cross-platform CI build matrix landed
(`.github/workflows/napi.yml`); publishing remains gated by
`private: true` until the publish checklist is cleared.

### Publish checklist (TSRET-002)

Before tagging `napi-v*`:

1. Flip `private: true` → remove the field in this `package.json`.
2. Confirm `@eddacraft` npm scope ownership and that the package name
   is reserved (publish a `0.0.0-placeholder` if not).
3. Confirm the `NPM_TOKEN` secret exists on the repo with publish
   rights to `@eddacraft/anvil-checks-native*`.
4. Decide whether to enable npm provenance — if yes, add
   `id-token: write` to the publish job permissions and pass
   `--provenance` through to `napi pre-publish`.
5. Run an out-of-band install test on aarch64-linux and x86_64-darwin
   (no native runner in the test matrix).

## Build

```bash
pnpm --filter @eddacraft/anvil-checks-native build
```

Generates `anvil-checks-native.<platform>.node`, `index.js`, and
`index.d.ts` in this directory. The latter two are regenerated on
every build and are gitignored.

## Test

```bash
pnpm --filter @eddacraft/anvil-checks-native test
```

The test exercises the JS↔Rust wire round-trip on a fixture
(`fixtures/sample.ts`), asserts the expected rule IDs fire, exercises
the options filter and error path, and prints cold/warm call timings.

**Note on parity:** the binding's JSON shape is *not* identical to
`anvil check --json`. The CLI emits an aggregate `CheckOutput` over
many files with warnings projected through a narrow `JsonWarning`
(flat `file`/`line`, ~9 fields). This binding emits a per-artifact
`ScanResultOutput` with the full `Warning` struct (~17 fields, nested
`location`). Warning *content* is parity by construction (same
`scan_artifact` call); the *envelope* is deliberately different.
Adding a CLI-diff golden snapshot is a TSRET-003 prerequisite.
