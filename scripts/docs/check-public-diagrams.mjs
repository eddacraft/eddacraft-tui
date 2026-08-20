#!/usr/bin/env node

import { readdir } from 'node:fs/promises';
import { extname, resolve } from 'node:path';

import { loadContract, validatePublicDiagrams } from './lib/public-diagrams.mjs';

const args = process.argv.slice(2);
const rootIndex = args.indexOf('--root');
const repoRoot =
  rootIndex >= 0 && args[rootIndex + 1]
    ? resolve(args[rootIndex + 1])
    : resolve(import.meta.dirname, '../..');
const json = args.includes('--json');
const contract = await loadContract(repoRoot);
const findings = await validatePublicDiagrams(repoRoot, contract);

let filesChecked = 0;
for (const family of contract.families) {
  filesChecked += await countDiagramFiles(resolve(repoRoot, family.root));
}

if (json) {
  process.stdout.write(
    `${JSON.stringify(
      {
        surface: 'public-diagrams',
        findings: findings.map(({ path: file, message }) => ({
          severity: 'ERROR',
          file,
          line: 1,
          message,
        })),
        summary: { errors: findings.length, warnings: 0, filesChecked },
      },
      null,
      2
    )}\n`
  );
} else {
  for (const finding of findings) {
    process.stdout.write(
      `[public-diagrams] ERROR: ${finding.path}:1 — ${finding.message} [${finding.code}]\n`
    );
  }
  process.stdout.write(
    `[public-diagrams] summary: ${findings.length} errors, 0 warnings, ${filesChecked} files checked\n`
  );
}

if (findings.length > 0) process.exitCode = 1;

async function countDiagramFiles(root) {
  let count = 0;
  async function visit(directory) {
    let entries;
    try {
      entries = await readdir(directory, { withFileTypes: true });
    } catch (error) {
      if (error.code === 'ENOENT') return;
      throw error;
    }
    for (const entry of entries) {
      const path = resolve(directory, entry.name);
      if (entry.isDirectory()) await visit(path);
      else if (['.drawio', '.svg'].includes(extname(path).toLowerCase())) count += 1;
    }
  }
  await visit(root);
  return count;
}
