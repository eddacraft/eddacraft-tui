'use strict';

const fs = require('node:fs');
const { syncBuiltinESMExports } = require('node:module');

const signalPath = process.env.ANVIL_CI_LOG_TEST_DELAY_SIGNAL;
const releasePath = process.env.ANVIL_CI_LOG_TEST_DELAY_RELEASE;

if (signalPath && releasePath) {
  const originalCloseSync = fs.closeSync;
  const originalExistsSync = fs.existsSync;
  const originalOpenSync = fs.openSync;
  const originalWriteFileSync = fs.writeFileSync;
  const originalWriteSync = fs.writeSync;
  const delayedFds = new Set();
  const sleepBuffer = new Int32Array(new SharedArrayBuffer(4));

  function isPendingPath(path) {
    const text = String(path).replaceAll('\\', '/');
    return text.includes('/anvil/ci-log-pending/');
  }

  fs.openSync = function patchedOpenSync(path, flags, ...rest) {
    const fd = originalOpenSync.call(fs, path, flags, ...rest);
    if (isPendingPath(path) && String(path).includes('.ci-log-pending-tmp-')) {
      delayedFds.add(fd);
    }
    return fd;
  };

  fs.writeFileSync = function patchedWriteFileSync(file, data, options) {
    const flag = typeof options === 'object' && options ? options.flag : undefined;
    const legacyFinalWrite =
      typeof file !== 'number' &&
      isPendingPath(file) &&
      String(file).endsWith('.md') &&
      flag === 'wx';
    const atomicTempWrite = typeof file === 'number' && delayedFds.has(file);
    if (!legacyFinalWrite && !atomicTempWrite) {
      return originalWriteFileSync.call(fs, file, data, options);
    }

    const encoding = typeof options === 'string' ? options : (options?.encoding ?? 'utf8');
    const bytes = Buffer.isBuffer(data) ? data : Buffer.from(String(data), encoding);
    const split = Math.max(1, bytes.indexOf(10) + 1);
    let ownedFd;
    const fd = atomicTempWrite
      ? file
      : (ownedFd = originalOpenSync.call(fs, file, flag, options?.mode));
    try {
      originalWriteSync.call(fs, fd, bytes.subarray(0, split));
      originalWriteFileSync.call(fs, signalPath, 'ready\n', 'utf8');
      const deadline = Date.now() + 10_000;
      while (!originalExistsSync.call(fs, releasePath)) {
        if (Date.now() >= deadline) throw new Error('timed out waiting to release delayed write');
        Atomics.wait(sleepBuffer, 0, 0, 20);
      }
      originalWriteSync.call(fs, fd, bytes.subarray(split));
    } finally {
      if (ownedFd !== undefined) originalCloseSync.call(fs, ownedFd);
    }
  };

  syncBuiltinESMExports();
}
