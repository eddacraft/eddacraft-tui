import { logger, type ExecutorContext } from '@nx/devkit';
import { cargoCommand } from '../../utils/cargo';

export interface ReleasePublishExecutorSchema {
  toolchain?: string;
  package?: string;
  registry?: string;
  token?: string;
  allowDirty?: boolean;
  dryRun?: boolean;
  noVerify?: boolean;
  args?: string | string[];
}

/**
 * Wraps `cargo publish`. Designed to be invoked via `nx release publish`
 * rather than directly, which is why it lives under `release-publish` and is
 * marked `hidden: true` in executors.json.
 */
export default async function releasePublishExecutor(
  options: ReleasePublishExecutorSchema,
  context: ExecutorContext
): Promise<{ success: boolean }> {
  const argv: string[] = [];

  if (options.toolchain && options.toolchain !== 'stable') {
    argv.push(`+${options.toolchain}`);
  }

  argv.push('publish');

  const pkg = options.package ?? context.projectName;
  if (pkg) argv.push('-p', pkg);

  if (options.registry) argv.push('--registry', options.registry);
  if (options.token) {
    logger.warn(
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

  return cargoCommand(...argv);
}
