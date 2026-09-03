import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import * as nodeFs from 'node:fs';
import { access, mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, dirname, join } from 'node:path';
import { test } from 'node:test';
import { promisify } from 'node:util';

import { atomicWriteBatchSync } from './lib/atomic-output-batch.mjs';

const execFileAsync = promisify(execFile);

function recordingFileSystem(events) {
  return {
    ...nodeFs,
    closeSync(fd) {
      events.push({ operation: 'close', fd });
      return nodeFs.closeSync(fd);
    },
    fsyncSync(fd) {
      events.push({ operation: 'fsync', fd });
      return nodeFs.fsyncSync(fd);
    },
    openSync(path, flags, mode) {
      const fd = nodeFs.openSync(path, flags, mode);
      events.push({ operation: 'open', path, flags, fd });
      return fd;
    },
    renameSync(from, to) {
      events.push({ operation: 'rename', from, to });
      return nodeFs.renameSync(from, to);
    },
    writeFileSync(fd, content, encoding) {
      events.push({ operation: 'write', fd });
      return nodeFs.writeFileSync(fd, content, encoding);
    },
  };
}

async function pathExists(path) {
  try {
    await access(path);
    return true;
  } catch {
    return false;
  }
}

test('stages and syncs the complete batch before publishing any output', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'atomic-output-batch-'));
  const events = [];
  const first = join(directory, 'first.md');
  const second = join(directory, 'second.md');

  try {
    atomicWriteBatchSync(
      [
        { path: first, content: 'first\n' },
        { path: second, content: 'second\n' },
      ],
      {
        fileSystem: recordingFileSystem(events),
        uniqueId: (() => {
          let id = 0;
          return () => `test-${++id}`;
        })(),
      }
    );

    assert.equal(await readFile(first, 'utf8'), 'first\n');
    assert.equal(await readFile(second, 'utf8'), 'second\n');

    const stagingOpens = events.filter(
      (event) => event.operation === 'open' && String(event.path).includes('.anvil-stage-')
    );
    assert.equal(stagingOpens.length, 2);
    assert.ok(stagingOpens.every((event) => event.flags === 'wx'));
    assert.deepEqual(
      stagingOpens.map((event) => dirname(event.path)),
      [directory, directory]
    );
    assert.deepEqual(
      stagingOpens.map((event) => basename(event.path)),
      ['.first.md.anvil-stage-test-1', '.second.md.anvil-stage-test-2']
    );

    const firstRename = events.findIndex((event) => event.operation === 'rename');
    assert.notEqual(firstRename, -1);
    for (const open of stagingOpens) {
      const fsync = events.findIndex(
        (event) => event.operation === 'fsync' && event.fd === open.fd
      );
      const close = events.findIndex(
        (event) => event.operation === 'close' && event.fd === open.fd
      );
      assert.ok(fsync > -1 && fsync < firstRename);
      assert.ok(close > fsync && close < firstRename);
    }

    assert.deepEqual(await readdir(directory), ['first.md', 'second.md']);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('exclusive staging never removes a pre-existing name collision', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'atomic-output-batch-'));
  const destination = join(directory, 'output.md');
  const collision = join(directory, '.output.md.anvil-stage-collision');

  try {
    await writeFile(collision, 'owned by another writer\n');

    assert.throws(
      () =>
        atomicWriteBatchSync([{ path: destination, content: 'new output\n' }], {
          uniqueId: () => 'collision',
        }),
      { code: 'EEXIST' }
    );

    assert.equal(await readFile(collision, 'utf8'), 'owned by another writer\n');
    await assert.rejects(readFile(destination, 'utf8'), { code: 'ENOENT' });
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('restores the complete old batch when a publish rename fails', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'atomic-output-batch-'));
  const first = join(directory, 'first.md');
  const previouslyAbsent = join(directory, 'new.md');
  const last = join(directory, 'last.md');
  const restored = [];
  const fileSystem = {
    ...nodeFs,
    renameSync(from, to) {
      if (to === last && basename(from).includes('.anvil-stage-')) {
        throw new Error('injected publish rename failure');
      }
      if (basename(from).includes('.anvil-restore-')) restored.push(to);
      return nodeFs.renameSync(from, to);
    },
  };

  try {
    await writeFile(first, 'old first\n');
    await writeFile(last, 'old last\n');

    assert.throws(
      () =>
        atomicWriteBatchSync(
          [
            { path: first, content: 'new first\n' },
            { path: previouslyAbsent, content: 'new file\n' },
            { path: last, content: 'new last\n' },
          ],
          {
            fileSystem,
            uniqueId: (() => {
              let id = 0;
              return () => `rollback-${++id}`;
            })(),
          }
        ),
      /injected publish rename failure/
    );

    assert.equal(await readFile(first, 'utf8'), 'old first\n');
    await assert.rejects(readFile(previouslyAbsent, 'utf8'), { code: 'ENOENT' });
    assert.equal(await readFile(last, 'utf8'), 'old last\n');
    assert.deepEqual(restored, [last, first]);
    assert.deepEqual(await readdir(directory), ['first.md', 'last.md']);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('leaves old outputs unchanged and cleans staging files after a write failure', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'atomic-output-batch-'));
  const existing = join(directory, 'existing.md');
  const previouslyAbsent = join(directory, 'new.md');
  let writes = 0;
  const fileSystem = {
    ...nodeFs,
    writeFileSync(fd, content, encoding) {
      writes += 1;
      if (writes === 2) throw new Error('injected staging write failure');
      return nodeFs.writeFileSync(fd, content, encoding);
    },
  };

  try {
    await writeFile(existing, 'old content\n');

    assert.throws(
      () =>
        atomicWriteBatchSync(
          [
            { path: existing, content: 'new content\n' },
            { path: previouslyAbsent, content: 'new file\n' },
          ],
          {
            fileSystem,
            uniqueId: (() => {
              let id = 0;
              return () => `write-${++id}`;
            })(),
          }
        ),
      /injected staging write failure/
    );

    assert.equal(await readFile(existing, 'utf8'), 'old content\n');
    await assert.rejects(readFile(previouslyAbsent, 'utf8'), { code: 'ENOENT' });
    assert.deepEqual(await readdir(directory), ['existing.md']);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('aggregates rollback failures and retains same-directory recovery evidence', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'atomic-output-batch-'));
  const first = join(directory, 'first.md');
  const previouslyAbsent = join(directory, 'new.md');
  const last = join(directory, 'last.md');
  const fileSystem = {
    ...nodeFs,
    renameSync(from, to) {
      if (to === last && basename(from).includes('.anvil-stage-')) {
        throw new Error('injected publish rename failure');
      }
      if (to === first && basename(from).includes('.anvil-restore-')) {
        throw new Error('injected rollback rename failure');
      }
      return nodeFs.renameSync(from, to);
    },
  };

  try {
    await writeFile(first, 'old first\n');
    await writeFile(last, 'old last\n');

    let failure;
    try {
      atomicWriteBatchSync(
        [
          { path: first, content: 'new first\n' },
          { path: previouslyAbsent, content: 'new file\n' },
          { path: last, content: 'new last\n' },
        ],
        {
          fileSystem,
          uniqueId: (() => {
            let id = 0;
            return () => `aggregate-${++id}`;
          })(),
        }
      );
    } catch (error) {
      failure = error;
    }

    assert.ok(failure instanceof AggregateError);
    assert.deepEqual(
      failure.errors.map((error) => error.message),
      ['injected publish rename failure', 'injected rollback rename failure']
    );
    assert.equal(failure.recoveryPaths.length, 2);
    assert.ok(failure.recoveryPaths.every((path) => dirname(path) === directory));
    assert.deepEqual(
      await Promise.all(failure.recoveryPaths.map((path) => readFile(path, 'utf8'))),
      ['old first\n', 'old last\n']
    );

    assert.equal(await readFile(first, 'utf8'), 'new first\n');
    await assert.rejects(readFile(previouslyAbsent, 'utf8'), { code: 'ENOENT' });
    assert.equal(await readFile(last, 'utf8'), 'old last\n');
    assert.ok(
      (await readdir(directory)).every(
        (name) => !name.includes('.anvil-stage-') && !name.includes('.anvil-restore-')
      )
    );
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('overlapping processes cannot publish a mixed batch', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'atomic-output-overlap-'));
  const worker = join(directory, 'writer.mjs');
  const helperUrl = new URL('./lib/atomic-output-batch.mjs', import.meta.url).href;
  const first = join(directory, 'first.md');
  const second = join(directory, 'second.md');
  const waitFor = async (path) => {
    const deadline = Date.now() + 10_000;
    while (!(await pathExists(path))) {
      if (Date.now() >= deadline) throw new Error(`timed out waiting for ${path}`);
      await new Promise((resolve) => setTimeout(resolve, 10));
    }
  };

  try {
    await writeFile(first, 'old first\n');
    await writeFile(second, 'old second\n');
    await writeFile(
      worker,
      `import * as fs from 'node:fs';
import { join } from 'node:path';

const { atomicWriteBatchSync } = await import(process.argv[2]);
const role = process.argv[3];
const directory = process.argv[4];
const first = join(directory, 'first.md');
const second = join(directory, 'second.md');
const sleeper = new Int32Array(new SharedArrayBuffer(4));
fs.writeFileSync(join(directory, role + '-ready'), '');
atomicWriteBatchSync(
  [
    { path: first, content: role + ' first\\n' },
    { path: second, content: role + ' second\\n' },
  ],
  {
    fileSystem: {
      ...fs,
      renameSync(from, to) {
        fs.renameSync(from, to);
        if (to === first && String(from).includes('.anvil-stage-')) {
          fs.writeFileSync(join(directory, role + '-first'), '');
          while (!fs.existsSync(join(directory, 'release-' + role))) {
            Atomics.wait(sleeper, 0, 0, 10);
          }
        }
      },
    },
  }
);
fs.writeFileSync(join(directory, role + '-done'), '');
`
    );

    const writerA = execFileAsync(process.execPath, [worker, helperUrl, 'a', directory]);
    await waitFor(join(directory, 'a-first'));
    const lockDirectories = (await readdir(directory))
      .filter((name) => name.endsWith('.anvil-lock'))
      .sort();

    const writerB = execFileAsync(process.execPath, [worker, helperUrl, 'b', directory]);
    await waitFor(join(directory, 'b-ready'));
    if (lockDirectories.length === 0) {
      await waitFor(join(directory, 'b-first'));
      await writeFile(join(directory, 'release-b'), '');
      await waitFor(join(directory, 'b-done'));
      await writeFile(join(directory, 'release-a'), '');
    } else {
      assert.equal(await pathExists(join(directory, 'b-first')), false);
      await writeFile(join(directory, 'release-a'), '');
      await waitFor(join(directory, 'a-done'));
      await waitFor(join(directory, 'b-first'));
      await writeFile(join(directory, 'release-b'), '');
    }

    await Promise.all([writerA, writerB]);
    assert.equal(await readFile(first, 'utf8'), 'b first\n');
    assert.equal(await readFile(second, 'utf8'), 'b second\n');
    assert.deepEqual(lockDirectories, ['.first.md.anvil-lock', '.second.md.anvil-lock']);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
