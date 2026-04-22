import type { ExecutorContext } from '@nx/devkit';
import type { BaseCargoOptions } from '../../models/base-options';
export interface TestExecutorSchema extends BaseCargoOptions {
    doc?: boolean;
    lib?: boolean;
    bin?: string | string[];
    bins?: boolean;
    test?: string | string[];
    tests?: boolean;
    'all-targets'?: boolean;
    'no-run'?: boolean;
    'no-fail-fast'?: boolean;
}
export default function testExecutor(options: TestExecutorSchema, context: ExecutorContext): Promise<{
    success: boolean;
}>;
//# sourceMappingURL=executor.d.ts.map