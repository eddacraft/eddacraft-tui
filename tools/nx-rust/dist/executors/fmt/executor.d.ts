import type { ExecutorContext } from '@nx/devkit';
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
export default function fmtExecutor(
  options: FmtExecutorSchema,
  context: ExecutorContext
): Promise<{
  success: boolean;
}>;
//# sourceMappingURL=executor.d.ts.map
