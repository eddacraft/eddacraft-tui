"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.default = libraryGenerator;
const generator_1 = __importDefault(require("../crate/generator"));
/** Alias for `nxrust:crate` (library is the default). */
async function libraryGenerator(tree, options) {
    return (0, generator_1.default)(tree, { ...options, bin: false });
}
//# sourceMappingURL=generator.js.map