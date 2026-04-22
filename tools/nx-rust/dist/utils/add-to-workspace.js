'use strict';
var __importDefault =
  (this && this.__importDefault) ||
  function (mod) {
    return mod && mod.__esModule ? mod : { default: mod };
  };
Object.defineProperty(exports, '__esModule', { value: true });
exports.addToCargoWorkspace = addToCargoWorkspace;
const j_toml_1 = __importDefault(require('@ltd/j-toml'));
const devkit_1 = require('@nx/devkit');
const toml_1 = require('./toml');
/**
 * Add `projectPath` to the root `Cargo.toml`'s `[workspace.members]` array.
 *
 * If the root Cargo.toml has no `[workspace]` section, creates one. If the
 * member is already listed, logs and no-ops. Idempotent.
 */
function addToCargoWorkspace(tree, projectPath) {
  const rootPath = 'Cargo.toml';
  const existing = tree.read(rootPath)?.toString();
  const cleanPath = projectPath.replace(/^\.\//, '');
  if (!existing) {
    // Bootstrap a minimal workspace root if the consumer didn't run `init` yet.
    // `TOML.Section(...)` marks the object so j-toml emits `[workspace]`
    // rather than the dotted `workspace.resolver = ...` form.
    tree.write(
      rootPath,
      (0, toml_1.stringifyCargoToml)({
        workspace: j_toml_1.default.Section({ resolver: '2', members: [cleanPath] }),
      })
    );
    return;
  }
  const toml = (0, toml_1.parseCargoToml)(existing);
  toml.workspace ??= { members: [] };
  const members = (toml.workspace.members ??= []);
  if (isAlreadyMember(members, cleanPath)) {
    devkit_1.logger.info(`${cleanPath} is already a workspace member`);
    return;
  }
  toml.workspace.members = [...members, cleanPath];
  tree.write(rootPath, (0, toml_1.stringifyCargoToml)(toml));
}
/**
 * True if `cleanPath` is covered by any entry in `members`, respecting
 * simple `*` globs (e.g. `crates/*` matches `crates/foo`). We only need
 * single-segment matching — `**` in workspace members is unusual enough to
 * fall back to the literal add path.
 */
function isAlreadyMember(members, cleanPath) {
  for (const entry of members) {
    if (entry === cleanPath) return true;
    if (!entry.includes('*')) continue;
    // Translate a cargo-style glob to a regex: escape regex metacharacters,
    // then turn `*` into `[^/]+`.
    const pattern = '^' + entry.replace(/[.+^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '[^/]+') + '$';
    if (new RegExp(pattern).test(cleanPath)) return true;
  }
  return false;
}
//# sourceMappingURL=add-to-workspace.js.map
