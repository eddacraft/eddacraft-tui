import { randomUUID } from 'node:crypto';
import * as nodeFs from 'node:fs';
import { constants as fsConstants } from 'node:fs';
import { basename, dirname, join, resolve } from 'node:path';

const LOCK_POLL_INTERVAL_MS = 20;
const LOCK_WAIT_ARRAY = new Int32Array(new SharedArrayBuffer(4));

function temporaryPath(path, kind, uniqueId) {
  return join(dirname(path), `.${basename(path)}.anvil-${kind}-${uniqueId()}`);
}

function removeIfPresent(fileSystem, path) {
  fileSystem.rmSync(path, { force: true });
}

function syncFile(fileSystem, path) {
  let fd;
  try {
    fd = fileSystem.openSync(path, 'r');
    fileSystem.fsyncSync(fd);
    fileSystem.closeSync(fd);
    fd = undefined;
  } finally {
    if (fd !== undefined) fileSystem.closeSync(fd);
  }
}

function removePaths(fileSystem, paths) {
  for (const path of paths) {
    try {
      removeIfPresent(fileSystem, path);
    } catch {
      // Cleanup is best-effort while another operation failure is in flight.
    }
  }
}

function outputLockPath(outputPath) {
  const absolute = resolve(outputPath);
  return join(dirname(absolute), `.${basename(absolute)}.anvil-lock`);
}

function waitSynchronously(milliseconds) {
  Atomics.wait(LOCK_WAIT_ARRAY, 0, 0, milliseconds);
}

function releaseOutputLocks(fileSystem, locks) {
  const errors = [];
  for (const lock of locks.toReversed()) {
    try {
      const owner = JSON.parse(fileSystem.readFileSync(lock.path, 'utf8'));
      if (owner.token !== lock.token) {
        throw new Error(`Atomic output lock ownership changed: ${lock.path}`);
      }
      fileSystem.rmSync(lock.path);
    } catch (error) {
      errors.push(error);
    }
  }
  return errors;
}

function acquireOutputLocks(fileSystem, entries, lockTimeoutMs) {
  const paths = [...new Set(entries.map((entry) => outputLockPath(entry.path)))].sort();
  const acquired = [];
  const deadline = Date.now() + lockTimeoutMs;

  try {
    for (const path of paths) {
      for (;;) {
        const token = `${process.pid}-${randomUUID()}`;
        let fd;
        let ownsPath = false;
        try {
          fd = fileSystem.openSync(path, 'wx', 0o600);
          ownsPath = true;
          fileSystem.writeFileSync(
            fd,
            JSON.stringify({ pid: process.pid, token, acquiredAt: new Date().toISOString() }),
            'utf8'
          );
          fileSystem.fsyncSync(fd);
          fileSystem.closeSync(fd);
          fd = undefined;
          acquired.push({ path, token });
          break;
        } catch (error) {
          if (fd !== undefined) {
            try {
              fileSystem.closeSync(fd);
            } catch {
              // The lock acquisition failure remains authoritative.
            }
          }
          if (ownsPath) {
            try {
              removeIfPresent(fileSystem, path);
            } catch {
              // The lock acquisition failure remains authoritative.
            }
          }
          if (error.code !== 'EEXIST') throw error;
          if (Date.now() >= deadline) {
            const timeout = new Error(
              `Timed out waiting for atomic output lock ${path}; confirm its recorded owner is no longer running before removing it`,
              { cause: error }
            );
            timeout.code = 'ELOCKED';
            throw timeout;
          }
          waitSynchronously(Math.min(LOCK_POLL_INTERVAL_MS, Math.max(1, deadline - Date.now())));
        }
      }
    }
    return acquired;
  } catch (error) {
    const releaseErrors = releaseOutputLocks(fileSystem, acquired);
    if (releaseErrors.length > 0) {
      // oxlint-disable-next-line preserve-caught-error -- the caught error is both the first AggregateError member and its cause.
      throw new AggregateError(
        [error, ...releaseErrors],
        'Atomic output lock acquisition failed and acquired locks could not all be released',
        { cause: error }
      );
    }
    throw error;
  }
}

function restoreOldBatch(fileSystem, prepared, uniqueId, publishError) {
  const rollbackErrors = [];
  const rollbackTemps = [];

  for (const entry of prepared.toReversed()) {
    if (!entry.existed) {
      try {
        removeIfPresent(fileSystem, entry.path);
      } catch (error) {
        rollbackErrors.push(error);
      }
      continue;
    }

    const rollbackPath = temporaryPath(entry.path, 'restore', uniqueId);
    try {
      fileSystem.copyFileSync(entry.recoveryPath, rollbackPath, fsConstants.COPYFILE_EXCL);
      rollbackTemps.push(rollbackPath);
      syncFile(fileSystem, rollbackPath);
      fileSystem.renameSync(rollbackPath, entry.path);
    } catch (error) {
      rollbackErrors.push(error);
    }
  }

  removePaths(fileSystem, rollbackTemps);

  const recoveryPaths = prepared
    .filter((entry) => entry.recoveryPath)
    .map((entry) => entry.recoveryPath);
  if (rollbackErrors.length === 0) {
    removePaths(fileSystem, recoveryPaths);
    throw publishError;
  }

  const error = new AggregateError(
    [publishError, ...rollbackErrors],
    `Atomic output batch publish failed and rollback was incomplete; recovery copies retained: ${recoveryPaths.join(', ')}`,
    { cause: publishError }
  );
  error.recoveryPaths = recoveryPaths;
  throw error;
}

/**
 * Publish a set of generated files only after every member is safely staged.
 *
 * @param {Array<{path: string, content: string | Uint8Array}>} entries
 * @param {{fileSystem?: typeof nodeFs, uniqueId?: () => string, lockTimeoutMs?: number}} options
 */
export function atomicWriteBatchSync(
  entries,
  { fileSystem = nodeFs, uniqueId = randomUUID, lockTimeoutMs = 60_000 } = {}
) {
  const destinations = new Set();
  for (const entry of entries) {
    if (!entry || typeof entry.path !== 'string' || entry.path.length === 0) {
      throw new TypeError('Each atomic output must have a non-empty path');
    }
    if (destinations.has(entry.path)) {
      throw new Error(`Atomic output batch contains duplicate path: ${entry.path}`);
    }
    destinations.add(entry.path);
  }

  const locks = acquireOutputLocks(fileSystem, entries, lockTimeoutMs);
  let operationError;
  try {
    writeBatchWhileLocked(entries, { fileSystem, uniqueId });
  } catch (error) {
    operationError = error;
  }

  const releaseErrors = releaseOutputLocks(fileSystem, locks);
  if (operationError) {
    if (releaseErrors.length === 0) throw operationError;
    const error = new AggregateError(
      [operationError, ...releaseErrors],
      'Atomic output batch failed and its locks could not all be released',
      { cause: operationError }
    );
    if (Array.isArray(operationError.recoveryPaths)) {
      error.recoveryPaths = operationError.recoveryPaths;
    }
    throw error;
  }
  if (releaseErrors.length > 0) {
    throw new AggregateError(
      releaseErrors,
      'Atomic output batch published but its locks could not all be released'
    );
  }
}

function writeBatchWhileLocked(entries, { fileSystem, uniqueId }) {
  const staged = [];
  const prepared = [];
  try {
    for (const entry of entries) {
      const path = temporaryPath(entry.path, 'stage', uniqueId);
      let fd;
      let ownsPath = false;
      try {
        fd = fileSystem.openSync(path, 'wx', 0o644);
        ownsPath = true;
        fileSystem.writeFileSync(fd, entry.content, 'utf8');
        fileSystem.fsyncSync(fd);
        fileSystem.closeSync(fd);
        fd = undefined;
        staged.push({ ...entry, stagedPath: path });
      } catch (error) {
        if (fd !== undefined) {
          try {
            fileSystem.closeSync(fd);
          } catch {
            // The original staging failure remains authoritative.
          }
        }
        if (ownsPath) {
          try {
            removeIfPresent(fileSystem, path);
          } catch {
            // The original staging failure remains authoritative.
          }
        }
        throw error;
      }
    }

    for (const entry of staged) {
      const existed = fileSystem.existsSync(entry.path);
      if (!existed) {
        prepared.push({ ...entry, existed, recoveryPath: undefined });
        continue;
      }

      const recoveryPath = temporaryPath(entry.path, 'recovery', uniqueId);
      fileSystem.copyFileSync(entry.path, recoveryPath, fsConstants.COPYFILE_EXCL);
      prepared.push({ ...entry, existed, recoveryPath });
      syncFile(fileSystem, recoveryPath);
    }

    try {
      for (const entry of prepared) {
        fileSystem.renameSync(entry.stagedPath, entry.path);
      }
    } catch (error) {
      restoreOldBatch(fileSystem, prepared, uniqueId, error);
    }

    removePaths(fileSystem, prepared.map((entry) => entry.recoveryPath).filter(Boolean));
  } catch (error) {
    removePaths(
      fileSystem,
      staged.map((entry) => entry.stagedPath)
    );
    if (!Array.isArray(error.recoveryPaths)) {
      removePaths(fileSystem, prepared.map((entry) => entry.recoveryPath).filter(Boolean));
    }
    throw error;
  }
}
