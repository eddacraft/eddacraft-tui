"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.default = checkExecutor;
const build_command_1 = require("../../utils/build-command");
const cargo_1 = require("../../utils/cargo");
const CHECK_KEYS = ['all-targets', 'tests'];
async function checkExecutor(options, context) {
    const args = (0, build_command_1.buildCargoArgs)('check', options, context, CHECK_KEYS);
    return (0, cargo_1.cargoCommand)(...args);
}
//# sourceMappingURL=executor.js.map