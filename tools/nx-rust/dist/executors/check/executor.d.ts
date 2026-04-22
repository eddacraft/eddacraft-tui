import type { ExecutorContext } from '@nx/devkit';
import type { BaseCargoOptions } from '../../models/base-options';
export interface CheckExecutorSchema extends BaseCargoOptions {
    'all-targets'?: boolean;
    tests?: boolean;
}
export default function checkExecutor(options: CheckExecutorSchema, context: ExecutorContext): Promise<{
    success: boolean;
}>;
//# sourceMappingURL=executor.d.ts.map