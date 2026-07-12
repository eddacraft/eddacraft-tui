import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const check = process.argv.includes('--check');
const projectRoot = resolve(import.meta.dirname, '..');
const generatedRoot = resolve(projectRoot, 'src/api/generated');
const temporaryRoot = check ? mkdtempSync(join(tmpdir(), 'anvil-dashboard-api-')) : generatedRoot;
const openapiPath = join(temporaryRoot, 'openapi.json');
const typesPath = join(temporaryRoot, 'openapi.d.ts');
mkdirSync(temporaryRoot, { recursive: true });

function run(command, args) {
  const result = spawnSync(command, args, { cwd: projectRoot, stdio: 'inherit' });
  if (result.status !== 0) process.exit(result.status ?? 1);
}

try {
  run('cargo', [
    'run',
    '--quiet',
    '--manifest-path',
    '../../Cargo.toml',
    '-p',
    'eddacraft-anvil-dashboard-server',
    '--bin',
    'export-openapi',
    '--',
    openapiPath,
  ]);
  run('pnpm', ['exec', 'openapi-typescript', openapiPath, '--output', typesPath]);
  run('pnpm', ['exec', 'oxfmt', '--write', openapiPath, typesPath]);

  if (check) {
    for (const file of ['openapi.json', 'openapi.d.ts']) {
      const expected = readFileSync(join(generatedRoot, file));
      const actual = readFileSync(join(temporaryRoot, file));
      if (!expected.equals(actual)) {
        console.error(`Generated dashboard API drift detected in ${file}. Run pnpm generate:api.`);
        process.exitCode = 1;
      }
    }
  }
} finally {
  if (check) rmSync(temporaryRoot, { force: true, recursive: true });
}
