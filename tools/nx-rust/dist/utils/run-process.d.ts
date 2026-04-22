/**
 * Spawn a process, inheriting stdio so cargo's colourised output and progress
 * bars surface unchanged through Nx. Returns `{ success }` with the exit code
 * normalised — 0 is success, anything else is failure.
 *
 * Child is tracked so SIGINT/SIGTERM on the parent propagates to cargo, and
 * the parent-process listeners are removed once the child exits so repeated
 * invocations don't leak handlers.
 */
export declare function runProcess(command: string, ...args: string[]): Promise<{
    success: boolean;
}>;
//# sourceMappingURL=run-process.d.ts.map