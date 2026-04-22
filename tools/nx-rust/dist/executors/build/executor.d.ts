import type { ExecutorContext } from '@nx/devkit';
import type { BaseCargoOptions } from '../../models/base-options';
export interface BuildExecutorSchema extends BaseCargoOptions {
  lib?: boolean;
  bin?: string | string[];
  bins?: boolean;
  example?: string | string[];
  examples?: boolean;
  'all-targets'?: boolean;
}
export default function buildExecutor(
  options: BuildExecutorSchema,
  context: ExecutorContext
): Promise<{
  success: boolean;
}>;
//# sourceMappingURL=executor.d.ts.map
