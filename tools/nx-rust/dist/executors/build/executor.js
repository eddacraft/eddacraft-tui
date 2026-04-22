"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.default = buildExecutor;
const build_command_1 = require("../../utils/build-command");
const cargo_1 = require("../../utils/cargo");
const BUILD_KEYS = ['lib', 'bin', 'bins', 'example', 'examples', 'all-targets'];
async function buildExecutor(options, context) {
    const args = (0, build_command_1.buildCargoArgs)('build', options, context, BUILD_KEYS);
    return (0, cargo_1.cargoCommand)(...args);
}
//# sourceMappingURL=executor.js.map