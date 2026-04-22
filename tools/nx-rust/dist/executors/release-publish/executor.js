'use strict';
Object.defineProperty(exports, '__esModule', { value: true });
exports.default = releasePublishExecutor;
const devkit_1 = require('@nx/devkit');
const cargo_1 = require('../../utils/cargo');
/**
 * Wraps `cargo publish`. Designed to be invoked via `nx release publish`
 * rather than directly, which is why it lives under `release-publish` and is
 * marked `hidden: true` in executors.json.
 */
async function releasePublishExecutor(options, context) {
  const argv = [];
  if (options.toolchain && options.toolchain !== 'stable') {
    argv.push(`+${options.toolchain}`);
  }
  argv.push('publish');
  const pkg = options.package ?? context.projectName;
  if (pkg) argv.push('-p', pkg);
  if (options.registry) argv.push('--registry', options.registry);
  if (options.token) {
    devkit_1.logger.warn(
      'release-publish: using inline `token` option leaks the secret into process listings; ' +
        'prefer the CARGO_REGISTRY_TOKEN environment variable.'
    );
    argv.push('--token', options.token);
  }
  if (options.allowDirty) argv.push('--allow-dirty');
  if (options.dryRun) argv.push('--dry-run');
  if (options.noVerify) argv.push('--no-verify');
  if (options.args !== undefined) {
    if (Array.isArray(options.args)) {
      for (const a of options.args) argv.push(String(a));
    } else {
      argv.push(String(options.args));
    }
  }
  return (0, cargo_1.cargoCommand)(...argv);
}
//# sourceMappingURL=executor.js.map
