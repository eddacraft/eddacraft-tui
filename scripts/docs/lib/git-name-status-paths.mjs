/**
 * Fail-closed collector for diagram-impact changed paths.
 *
 * `git diff --name-only` drops a rename's source endpoint. `--name-status -z`
 * keeps both endpoints and is NUL-safe. Unknown or truncated status records
 * abort rather than silently dropping a path.
 *
 * Status letters honoured here match `--diff-filter=ACDMR`. Copy and rename
 * records always carry two paths (source then destination).
 */

import { execFile } from 'node:child_process';
import { parseArgs, promisify } from 'node:util';
import { pathToFileURL } from 'node:url';

const execFileAsync = promisify(execFile);

const SINGLE_PATH_STATUS = /^[ADM]$/u;
const TWO_PATH_STATUS = /^[CR][0-9]*$/u;

const GIT_DIFF_PATH_ARGS = ['diff', '--name-status', '-z', '-M', '--diff-filter=ACDMR'];

export function parseGitNameStatusZ(stdout) {
  if (typeof stdout !== 'string' || stdout.length === 0) return [];

  const tokens = stdout.split('\0');
  if (tokens.at(-1) === '') tokens.pop();

  const paths = [];
  let index = 0;

  while (index < tokens.length) {
    const status = tokens[index];
    if (status === undefined || status.length === 0) {
      throw new Error('empty git name-status record');
    }

    if (TWO_PATH_STATUS.test(status)) {
      const source = tokens[index + 1];
      const destination = tokens[index + 2];
      if (!source || !destination) {
        throw new Error(`incomplete git name-status ${status} record`);
      }
      paths.push(source, destination);
      index += 3;
      continue;
    }

    if (SINGLE_PATH_STATUS.test(status)) {
      const changedPath = tokens[index + 1];
      if (!changedPath) {
        throw new Error(`incomplete git name-status ${status} record`);
      }
      paths.push(changedPath);
      index += 2;
      continue;
    }

    throw new Error(`unrecognised git name-status record: ${status}`);
  }

  return [...new Set(paths)];
}

export async function collectGitDiffPaths({ cwd, extraArgs = [], execute = execFileAsync } = {}) {
  let result;
  try {
    result = await execute('git', [...GIT_DIFF_PATH_ARGS, ...extraArgs], { cwd });
  } catch (error) {
    throw new Error(`git name-status collector failed: ${error?.message || String(error)}`, {
      cause: error,
    });
  }

  const stdout = typeof result?.stdout === 'string' ? result.stdout : '';
  return parseGitNameStatusZ(stdout);
}

export async function runGitNameStatusPathsCli(argv = process.argv.slice(2)) {
  const { values, positionals } = parseArgs({
    args: argv,
    options: {
      cwd: { type: 'string' },
    },
    allowPositionals: true,
    strict: true,
  });

  const paths = await collectGitDiffPaths({
    cwd: values.cwd,
    extraArgs: positionals,
  });
  if (paths.length > 0) {
    process.stdout.write(`${paths.join('\n')}\n`);
  }
  return 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  runGitNameStatusPathsCli().then(
    (code) => {
      process.exitCode = code;
    },
    (error) => {
      process.stderr.write(`[git-name-status-paths] cannot collect: ${error.message}\n`);
      process.exitCode = 2;
    }
  );
}
