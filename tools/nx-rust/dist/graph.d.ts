import { type CreateDependencies, type CreateNodesV2 } from '@nx/devkit';
/**
 * Project-graph plugin: for every Cargo.toml in the workspace, materialise a
 * project with cargo-backed targets (build/check/clippy/fmt/test, plus `run`
 * for binary crates). Dependencies between workspace members and external
 * registry/git crates are published via `createDependencies`.
 *
 * Invariant: the Nx project name of a crate must equal its Cargo package
 * name. `inferProjectConfig` sets `name: pkg.name`, and `createDependencies`
 * assumes Nx has re-keyed the graph by that name. Workspaces that override
 * the name via a `project.json` will lose dependency edges from that project.
 */
export declare const createNodesV2: CreateNodesV2;
export declare const createDependencies: CreateDependencies;
/** Exposed for tests to reset the module-level metadata cache. */
export declare function __resetGraphCacheForTests(): void;
//# sourceMappingURL=graph.d.ts.map
