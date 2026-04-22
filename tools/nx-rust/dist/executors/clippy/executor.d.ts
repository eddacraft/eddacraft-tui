import type { ExecutorContext } from '@nx/devkit';
import type { BaseCargoOptions } from '../../models/base-options';
export interface ClippyExecutorSchema extends BaseCargoOptions {
  'all-targets'?: boolean;
  fix?: boolean;
}
export default function clippyExecutor(
  options: ClippyExecutorSchema,
  context: ExecutorContext
): Promise<{
  success: boolean;
}>;
//# sourceMappingURL=executor.d.ts.map
