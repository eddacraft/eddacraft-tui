import type { ExecutorContext } from '@nx/devkit';
import { cargoCommand } from '../../utils/cargo';

export interface FmtExecutorSchema {
  toolchain?: string;
  package?: string;
  check?: boolean;
  all?: boolean;
  args?: string | string[];
}

/**
 * `cargo fmt` has its own argv shape — `--package` is a cargo-level flag and
 * everything after `--` is forwarded to rustfmt. Implemented directly instead
 * of via `buildCargoArgs` so we don't accidentally forward unrelated fields.
 */
export default async function fmtExecutor(
  options: FmtExecutorSchema,
  context: ExecutorContext
): Promise<{ success: boolean }> {
  const argv: string[] = [];

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

  return cargoCommand(...argv);
}
