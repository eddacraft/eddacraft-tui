import type { ExecutorContext } from '@nx/devkit';
import type { BaseCargoOptions } from '../../models/base-options';
export interface RunExecutorSchema extends BaseCargoOptions {
    bin?: string;
    example?: string;
}
export default function runExecutor(options: RunExecutorSchema, context: ExecutorContext): Promise<{
    success: boolean;
}>;
//# sourceMappingURL=executor.d.ts.map