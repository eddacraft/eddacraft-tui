## Save-time Trust Performance Check

Use this to verify the acceptance criterion:

- Cached `anvil check <file>` completes in < 2s
- Cold `anvil check <file>` completes in < 5s

### Build the CLI

If an Nx target exists:

```bash
nx run @anvil/cli:build
```

Otherwise:

```bash
pnpm --filter @anvil/cli build
```

### Run the benchmark

```bash
node scripts/bench-anvil-check.mjs core/src/index.ts
```

The script prints cold and warm execution times from the CLI JSON output.
