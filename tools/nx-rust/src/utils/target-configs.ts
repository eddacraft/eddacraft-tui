import type { TargetConfiguration } from '@nx/devkit';

/**
 * Pre-fabricated `TargetConfiguration` blobs so generators don't duplicate
 * the executor + cache + outputs wiring. Each accepts optional option
 * overrides that get merged into the target's `options`.
 *
 * Only `build` actually produces binary artefacts we want to cache.
 * `test` is exit-code-only — cargo test reuses the workspace `target/` dir
 * that `build` already populates, and snapshotting the full 36 GB dir into
 * `.nx/cache` (and pushing it to the remote cache) for every per-crate test
 * target dominated `pnpm test` wall-clock with disk I/O. `check`, `clippy`,
 * and `fmt-check` are exit-code-only for the same reason.
 */

type AnyOpts = Record<string, unknown>;

const BINARY_OUTPUTS = ['{options.target-dir}', '{workspaceRoot}/target'];

export function buildTargetConfig(options: AnyOpts = {}): TargetConfiguration {
  return {
    executor: '@eddacraft/nx-rust:build',
    cache: true,
    outputs: BINARY_OUTPUTS,
    options,
    configurations: {
      production: { release: true },
    },
  };
}

export function checkTargetConfig(options: AnyOpts = {}): TargetConfiguration {
  return {
    executor: '@eddacraft/nx-rust:check',
    cache: true,
    outputs: [],
    options,
  };
}

export function clippyTargetConfig(options: AnyOpts = {}): TargetConfiguration {
  return {
    executor: '@eddacraft/nx-rust:clippy',
    cache: true,
    outputs: [],
    options,
  };
}

/**
 * Reformatting target — rewrites source files in place, so it is NOT safely
 * cacheable. Pair with `fmtCheckTargetConfig` for CI lint runs.
 */
export function fmtTargetConfig(options: AnyOpts = {}): TargetConfiguration {
  return {
    executor: '@eddacraft/nx-rust:fmt',
    options,
  };
}

/**
 * Lint-only formatter target — runs `cargo fmt --check`, safe to cache by
 * exit code. Use this in `nx run-many --target=fmt-check` CI gates.
 */
export function fmtCheckTargetConfig(options: AnyOpts = {}): TargetConfiguration {
  return {
    executor: '@eddacraft/nx-rust:fmt',
    cache: true,
    outputs: [],
    options: { check: true, ...options },
  };
}

export function testTargetConfig(options: AnyOpts = {}): TargetConfiguration {
  return {
    executor: '@eddacraft/nx-rust:test',
    cache: true,
    outputs: [],
    options,
    configurations: {
      production: { release: true },
    },
  };
}

export function runTargetConfig(options: AnyOpts = {}): TargetConfiguration {
  return {
    executor: '@eddacraft/nx-rust:run',
    options,
    configurations: {
      production: { release: true },
    },
  };
}
