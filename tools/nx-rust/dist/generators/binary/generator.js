"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.default = binaryGenerator;
const generator_1 = __importDefault(require("../crate/generator"));
/**
 * Alias for `nxrust:crate --bin`. Kept as a distinct generator so it shows up
 * in `nx list` with its own description and `x-type: application` metadata.
 */
async function binaryGenerator(tree, options) {
    return (0, generator_1.default)(tree, { ...options, bin: true });
}
//# sourceMappingURL=generator.js.map