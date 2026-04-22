'use strict';
Object.defineProperty(exports, '__esModule', { value: true });
exports.default = fmtExecutor;
const cargo_1 = require('../../utils/cargo');
/**
 * `cargo fmt` has its own argv shape — `--package` is a cargo-level flag and
 * everything after `--` is forwarded to rustfmt. Implemented directly instead
 * of via `buildCargoArgs` so we don't accidentally forward unrelated fields.
 */
async function fmtExecutor(options, context) {
  const argv = [];
  if (options.toolchain && options.toolchain !== 'stable') {
    argv.push(`+${options.toolchain}`);
  }
  argv.push('fmt');
  if (options.all) {
    argv.push('--all');
  } else {
    const pkg = options.package ?? context.projectName;
    if (pkg) argv.push('-p', pkg);
  }
  if (options.check) {
    argv.push('--check');
  }
  if (options.args !== undefined) {
    argv.push('--');
    if (Array.isArray(options.args)) {
      for (const a of options.args) argv.push(String(a));
    } else {
      argv.push(String(options.args));
    }
  }
  return (0, cargo_1.cargoCommand)(...argv);
}
//# sourceMappingURL=executor.js.map
