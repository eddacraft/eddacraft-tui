"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.default = testExecutor;
const build_command_1 = require("../../utils/build-command");
const cargo_1 = require("../../utils/cargo");
const TEST_KEYS = [
    'doc',
    'lib',
    'bin',
    'bins',
    'test',
    'tests',
    'all-targets',
    'no-run',
    'no-fail-fast',
];
async function testExecutor(options, context) {
    const args = (0, build_command_1.buildCargoArgs)('test', options, context, TEST_KEYS);
    return (0, cargo_1.cargoCommand)(...args);
}
//# sourceMappingURL=executor.js.map