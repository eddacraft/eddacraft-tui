"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.default = clippyExecutor;
const build_command_1 = require("../../utils/build-command");
const cargo_1 = require("../../utils/cargo");
const CLIPPY_KEYS = ['all-targets', 'fix'];
async function clippyExecutor(options, context) {
    const args = (0, build_command_1.buildCargoArgs)('clippy', options, context, CLIPPY_KEYS);
    return (0, cargo_1.cargoCommand)(...args);
}
//# sourceMappingURL=executor.js.map