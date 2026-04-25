import { type ExecutorContext } from '@nx/devkit';
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
export default function releasePublishExecutor(options: ReleasePublishExecutorSchema, context: ExecutorContext): Promise<{
    success: boolean;
}>;
//# sourceMappingURL=executor.d.ts.map