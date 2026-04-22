/**
 * Entry point for the `nxrust` Nx plugin. Re-exports the project-graph
 * integration so `nx.json`'s `plugins: ["nxrust"]` wiring sees the
 * `createNodesV2` + `createDependencies` pair.
 */
export { createNodesV2, createDependencies } from './graph';
//# sourceMappingURL=index.d.ts.map