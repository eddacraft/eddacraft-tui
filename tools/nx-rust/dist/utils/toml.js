'use strict';
var __importDefault =
  (this && this.__importDefault) ||
  function (mod) {
    return mod && mod.__esModule ? mod : { default: mod };
  };
Object.defineProperty(exports, '__esModule', { value: true });
exports.parseCargoToml = parseCargoToml;
exports.readCargoTomlFromTree = readCargoTomlFromTree;
exports.stringifyCargoToml = stringifyCargoToml;
const j_toml_1 = __importDefault(require('@ltd/j-toml'));
/**
 * Parse a Cargo.toml string, preserving comments so generators don't mangle
 * hand-authored manifests on round-trip.
 */
function parseCargoToml(source) {
  return j_toml_1.default.parse(source, { x: { comment: true } });
}
/**
 * Read a project's Cargo.toml from the Nx Tree. Throws if the file is missing
 * — generators that call this always need it.
 */
function readCargoTomlFromTree(tree, relativePath) {
  const raw = tree.read(relativePath)?.toString();
  if (!raw) {
    throw new Error(`Cannot find Cargo.toml at ${relativePath}`);
  }
  return parseCargoToml(raw);
}
/**
 * Serialise a parsed Cargo.toml back to a string. `newlineAround: 'section'`
 * preserves the blank-line convention used by `cargo new`.
 */
function stringifyCargoToml(toml) {
  // @ltd/j-toml.stringify accepts any object shape at runtime; the exported
  // .Table type is a symbol-branded marker we don't need here.
  const result = j_toml_1.default.stringify(toml, { newlineAround: 'section' });
  return Array.isArray(result) ? result.join('\n') : result;
}
//# sourceMappingURL=toml.js.map
