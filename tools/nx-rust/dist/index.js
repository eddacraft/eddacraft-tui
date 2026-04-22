"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.createDependencies = exports.createNodesV2 = void 0;
/**
 * Entry point for the `@eddacraft/nx-rust` Nx plugin. Re-exports the
 * project-graph integration so `nx.json`'s plugin wiring sees the
 * `createNodesV2` + `createDependencies` pair.
 */
var graph_1 = require("./graph");
Object.defineProperty(exports, "createNodesV2", { enumerable: true, get: function () { return graph_1.createNodesV2; } });
Object.defineProperty(exports, "createDependencies", { enumerable: true, get: function () { return graph_1.createDependencies; } });
//# sourceMappingURL=index.js.map