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
export declare function buildTargetConfig(options?: AnyOpts): TargetConfiguration;
export declare function checkTargetConfig(options?: AnyOpts): TargetConfiguration;
export declare function clippyTargetConfig(options?: AnyOpts): TargetConfiguration;
/**
 * Reformatting target — rewrites source files in place, so it is NOT safely
 * cacheable. Pair with `fmtCheckTargetConfig` for CI lint runs.
 */
export declare function fmtTargetConfig(options?: AnyOpts): TargetConfiguration;
/**
 * Lint-only formatter target — runs `cargo fmt --check`, safe to cache by
 * exit code. Use this in `nx run-many --target=fmt-check` CI gates.
 */
export declare function fmtCheckTargetConfig(options?: AnyOpts): TargetConfiguration;
export declare function testTargetConfig(options?: AnyOpts): TargetConfiguration;
export declare function runTargetConfig(options?: AnyOpts): TargetConfiguration;
export {};
//# sourceMappingURL=target-configs.d.ts.map