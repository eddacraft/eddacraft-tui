import type { TargetConfiguration } from '@nx/devkit';

/**
 * Pre-fabricated `TargetConfiguration` blobs so generators don't duplicate
 * the executor + cache + outputs wiring. Each accepts optional option
 * overrides that get merged into the target's `options`.
 *
 * Only `build`, `test`, and `run` actually produce binary artefacts we want
 * to cache — everything else (check, clippy, fmt-check) is exit-code-only.
 * Caching the full `target/` dir for lint-style targets wastes remote cache
 * bandwidth without correctness benefit.
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
    outputs: BINARY_OUTPUTS,
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
