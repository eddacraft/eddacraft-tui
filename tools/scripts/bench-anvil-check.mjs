import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { resolve } from 'node:path';

const workspaceRoot = resolve(new URL('.', import.meta.url).pathname, '..');
const cliEntry = resolve(workspaceRoot, 'cli/dist/index.js');

const files = process.argv.slice(2);
const targetFiles = files.length > 0 ? files : ['core/src/index.ts'];

if (!existsSync(cliEntry)) {
  console.error('CLI build output not found at cli/dist/index.js.');
  console.error('Build the CLI first, then re-run this script.');
  process.exit(1);
}

function runCheck(args) {
  const output = execFileSync('node', [cliEntry, 'check', ...args, '--json'], {
    cwd: workspaceRoot,
    encoding: 'utf8',
  });
  return JSON.parse(output);
}

const cold = runCheck(['--no-cache', ...targetFiles]);
const warm = runCheck(targetFiles);

console.log('Anvil check latency (ms):');
console.log(`  Cold: ${cold.executionTimeMs}`);
console.log(`  Warm: ${warm.executionTimeMs}`);
