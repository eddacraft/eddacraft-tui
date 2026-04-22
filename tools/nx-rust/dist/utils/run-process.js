'use strict';
Object.defineProperty(exports, '__esModule', { value: true });
exports.runProcess = runProcess;
const node_child_process_1 = require('node:child_process');
/**
 * Spawn a process, inheriting stdio so cargo's colourised output and progress
 * bars surface unchanged through Nx. Returns `{ success }` with the exit code
 * normalised — 0 is success, anything else is failure.
 *
 * Child is tracked so SIGINT/SIGTERM on the parent propagates to cargo, and
 * the parent-process listeners are removed once the child exits so repeated
 * invocations don't leak handlers.
 */
function runProcess(command, ...args) {
  return new Promise((resolve) => {
    const child = (0, node_child_process_1.spawn)(command, args, {
      cwd: process.cwd(),
      env: process.env,
      stdio: 'inherit',
      shell: false,
      windowsHide: true,
    });
    const kill = () => {
      if (!child.killed) child.kill('SIGTERM');
    };
    // `nx run-many` can spawn many executors in parallel; each one adds three
    // listeners. Bump the cap so Node doesn't emit `MaxListenersExceeded`
    // warnings during the peak before any children exit.
    process.setMaxListeners(process.getMaxListeners() + 3);
    process.on('exit', kill);
    process.on('SIGINT', kill);
    process.on('SIGTERM', kill);
    const cleanup = () => {
      process.off('exit', kill);
      process.off('SIGINT', kill);
      process.off('SIGTERM', kill);
      process.setMaxListeners(Math.max(0, process.getMaxListeners() - 3));
    };
    child.on('error', (err) => {
      // eslint-disable-next-line no-console
      console.error(`Failed to spawn ${command}: ${err.message}`);
      cleanup();
      resolve({ success: false });
    });
    child.on('close', (code) => {
      cleanup();
      resolve({ success: code === 0 });
    });
  });
}
//# sourceMappingURL=run-process.js.map
