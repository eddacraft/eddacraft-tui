"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.createDependencies = exports.createNodesV2 = void 0;
exports.__resetGraphCacheForTests = __resetGraphCacheForTests;
const devkit_1 = require("@nx/devkit");
const project_graph_1 = require("nx/src/config/project-graph");
const node_fs_1 = require("node:fs");
const node_path_1 = require("node:path");
const cargo_1 = require("./utils/cargo");
const target_configs_1 = require("./utils/target-configs");
/**
 * Glob that matches any Cargo.toml in the workspace. Nx invokes
 * `createNodesV2` with every matching file — the glob is just the filter.
 */
const CARGO_GLOB = '**/Cargo.toml';
/**
 * Cache `cargo metadata` per workspace root — Nx calls `createNodesV2` once per
 * matched Cargo.toml, and would otherwise spawn cargo N times per graph
 * recompute. Invalidated by Cargo.lock mtime so edits to the workspace still
 * get picked up.
 */
const metadataCache = new Map();
function computeCached(workspaceRoot) {
    let currentMtime = 0;
    try {
        currentMtime = (0, node_fs_1.statSync)((0, node_path_1.join)(workspaceRoot, 'Cargo.lock')).mtimeMs;
    }
    catch {
        // No lockfile (e.g. fresh workspace) — fall through with mtime 0.
    }
    const cached = metadataCache.get(workspaceRoot);
    if (cached && cached.mtime === currentMtime) {
        return cached.result;
    }
    const result = computeGraph(workspaceRoot);
    metadataCache.set(workspaceRoot, { mtime: currentMtime, result });
    return result;
}
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
exports.createNodesV2 = [
    CARGO_GLOB,
    async (configFilePaths, options, context) => {
        const computed = computeCached(context.workspaceRoot);
        // `externalNodes` is a single graph-wide payload — attaching the same map
        // to every file result works (Nx dedupes by key) but wastes IPC. Emit it
        // with the first file we actually produce and blank on the rest.
        let externalsEmitted = false;
        return (0, devkit_1.createNodesFromFiles)(async (configFile) => {
            const projects = pickProjectForConfigFile(computed.projects, configFile);
            if (!projects) {
                return { projects: {}, externalNodes: {} };
            }
            const externalNodes = externalsEmitted ? {} : computed.externalNodes;
            externalsEmitted = true;
            return { projects, externalNodes };
        }, configFilePaths, options, context);
    },
];
const createDependencies = (_opts, ctx) => {
    const { projects, externalNodes, workspaceRoot } = ctx;
    const { metadata } = computeCached(workspaceRoot);
    if (!metadata)
        return [];
    const out = [];
    for (const pkg of metadata.packages) {
        // Nx re-keys projects by `name` after `createNodesV2` emits them keyed by
        // root; the lookup below relies on that transform.
        if (!projects[pkg.name])
            continue;
        for (const dep of pkg.dependencies) {
            // Dev deps shouldn't retrigger rebuilds of downstream projects.
            if (dep.kind === 'dev')
                continue;
            if (projects[dep.name]) {
                out.push(makeDependency(pkg, dep.name, workspaceRoot));
                continue;
            }
            const externalName = `cargo:${dep.name}`;
            if (externalNodes?.[externalName]) {
                out.push(makeDependency(pkg, externalName, workspaceRoot));
            }
        }
    }
    return out;
};
exports.createDependencies = createDependencies;
function computeGraph(workspaceRoot) {
    const metadata = (0, cargo_1.cargoMetadata)(workspaceRoot);
    if (!metadata) {
        return { projects: {}, externalNodes: {}, metadata: null };
    }
    const projects = {};
    const externalNodes = {};
    const versionByPackage = indexVersions(metadata);
    for (const pkg of metadata.packages) {
        if ((0, cargo_1.isExternal)(pkg, workspaceRoot))
            continue;
        const root = (0, devkit_1.normalizePath)((0, node_path_1.dirname)((0, node_path_1.relative)(workspaceRoot, pkg.manifest_path)));
        projects[root] = inferProjectConfig(pkg, root);
        // Only create external nodes for DIRECT deps of workspace members. If we
        // scanned every package's deps, transitive registry crates would show up
        // as graph nodes the workspace doesn't actually depend on.
        for (const dep of pkg.dependencies) {
            if (dep.kind === 'dev')
                continue;
            if (!(0, cargo_1.isExternal)(dep, workspaceRoot))
                continue;
            const name = `cargo:${dep.name}`;
            if (externalNodes[name])
                continue;
            externalNodes[name] = {
                type: 'cargo',
                name: name,
                data: {
                    packageName: dep.name,
                    version: versionByPackage.get(dep.name) ?? dep.req ?? '0.0.0',
                },
            };
        }
    }
    return { projects, externalNodes, metadata };
}
/**
 * Build a default project configuration from a cargo package. We infer
 * library vs. application from the crate's targets — a package with any
 * `bin` target is treated as an application and gets a `run` target wired
 * up. Consumers can still override everything via project.json.
 */
function inferProjectConfig(pkg, root) {
    const hasBin = pkg.targets.some((t) => t.kind.includes('bin'));
    const isPrivate = pkg.publish?.length === 0;
    const targets = {
        build: (0, target_configs_1.buildTargetConfig)(),
        check: (0, target_configs_1.checkTargetConfig)(),
        clippy: (0, target_configs_1.clippyTargetConfig)(),
        // `fmt` rewrites files (uncached); `fmt-check` is the lint mode that
        // caches safely because its output is just an exit status.
        fmt: (0, target_configs_1.fmtTargetConfig)(),
        'fmt-check': (0, target_configs_1.fmtCheckTargetConfig)(),
        test: (0, target_configs_1.testTargetConfig)(),
    };
    if (hasBin) {
        targets.run = (0, target_configs_1.runTargetConfig)();
    }
    if (!isPrivate) {
        targets['nx-release-publish'] = {
            dependsOn: ['^nx-release-publish'],
            executor: '@eddacraft/nx-rust:release-publish',
            options: {},
        };
    }
    return {
        root,
        name: pkg.name,
        projectType: hasBin ? 'application' : 'library',
        sourceRoot: `${root}/src`,
        targets,
    };
}
function pickProjectForConfigFile(projects, configFile) {
    const dir = (0, devkit_1.normalizePath)((0, node_path_1.dirname)(configFile));
    const match = projects[dir];
    return match ? { [dir]: match } : null;
}
function indexVersions(metadata) {
    const out = new Map();
    for (const pkg of metadata.packages) {
        if (!out.has(pkg.name))
            out.set(pkg.name, pkg.version);
    }
    return out;
}
function makeDependency(pkg, targetName, workspaceRoot) {
    const normalizedRoot = (0, devkit_1.normalizePath)(workspaceRoot);
    const manifest = (0, devkit_1.normalizePath)(pkg.manifest_path);
    const sourceFile = manifest.startsWith(`${normalizedRoot}/`)
        ? manifest.slice(normalizedRoot.length + 1)
        : manifest;
    return {
        type: project_graph_1.DependencyType.static,
        source: pkg.name,
        target: targetName,
        sourceFile,
    };
}
/** Exposed for tests to reset the module-level metadata cache. */
function __resetGraphCacheForTests() {
    metadataCache.clear();
}
//# sourceMappingURL=graph.js.map