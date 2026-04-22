'use strict';
Object.defineProperty(exports, '__esModule', { value: true });
exports.normalizeOptions = normalizeOptions;
const devkit_1 = require('@nx/devkit');
const snake_case_1 = require('./snake-case');
/**
 * Normalise a generator's raw input into the everything-you-need shape
 * downstream code (template rendering, `addProjectConfiguration`, etc.)
 * actually wants.
 *
 * Layout: crates live at `<directory>/<name>`, defaulting to `crates/<name>`.
 * This matches the anvil-001 layout and is the commonest Cargo convention.
 */
// Cargo accepts `[a-zA-Z0-9_-]` package names starting with a letter. Enforce
// the same rule at generator time so we don't write manifests cargo rejects.
const CARGO_NAME_RE = /^[a-zA-Z][a-zA-Z0-9_-]*$/;
function normalizeOptions(_tree, options) {
  const cargoName = options.name.trim();
  if (!cargoName) {
    throw new Error('Generator requires a non-empty `name`.');
  }
  if (!CARGO_NAME_RE.test(cargoName)) {
    throw new Error(
      `Invalid Cargo package name: "${cargoName}". ` +
        'Names must start with a letter and contain only letters, digits, `-`, or `_`.'
    );
  }
  const libName = (0, snake_case_1.toSnakeCase)(cargoName);
  const directory = options.directory ?? 'crates';
  const projectDirectory = cargoName;
  const projectRoot = (0, devkit_1.joinPathFragments)(directory, projectDirectory);
  const projectName = cargoName;
  const parsedTags = options.tags
    ? options.tags
        .split(',')
        .map((t) => t.trim())
        .filter(Boolean)
    : [];
  const edition = options.edition ?? '2021';
  const { edition: _ignored, ...rest } = options;
  void _ignored;
  return {
    ...rest,
    projectName,
    projectRoot,
    projectDirectory,
    cargoName,
    libName,
    edition,
    parsedTags,
  };
}
//# sourceMappingURL=normalize-options.js.map
