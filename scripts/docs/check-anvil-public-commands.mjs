#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import process from 'node:process';

const argv = process.argv.slice(2);
const ROOT = flagValue('--root')
  ? resolve(flagValue('--root'))
  : resolve(fileURLToPath(new URL('../..', import.meta.url)));
const ANVIL_BIN = flagValue('--anvil-bin') ?? 'anvil';
const files = markdownFiles(resolve(ROOT, 'docs/public/anvil'));
const commands = [];

for (const file of files) {
  let fenced = false;
  for (const [index, line] of readFileSync(file, 'utf8').split(/\r?\n/).entries()) {
    if (line.startsWith('```')) {
      fenced = !fenced;
      continue;
    }
    const fencedLine = line.trim();
    if (!fenced) continue;
    const yamlRun = /^run:\s+(anvil(?:\s+.*)?)$/.exec(fencedLine);
    const command = fencedLine.startsWith('anvil ') ? fencedLine : yamlRun?.[1];
    if (!command) continue;
    if (/^anvil\s+[0-9]/.test(command)) continue;
    commands.push({ file, line: index + 1, command });
  }
}

const failures = [];
for (const example of commands) {
  const words = shellWords(example.command);
  if (!words || words[0] !== 'anvil') {
    failures.push({ ...example, reason: 'unsupported shell syntax in command example' });
    continue;
  }
  const args = words.slice(1);
  if (args.length === 1 && (args[0] === '--help' || args[0] === '--version')) continue;
  const probeArgs = args.some((arg) => arg === '--help' || arg === '-h')
    ? args
    : [...args, '--help'];
  const result = spawnSync(ANVIL_BIN, probeArgs, {
    cwd: ROOT,
    encoding: 'utf8',
    env: { ...process.env, DO_NOT_TRACK: '1' },
  });
  if (result.status !== 0) {
    failures.push({
      ...example,
      reason: (result.stderr || result.stdout || `exit ${result.status}`).trim().split(/\r?\n/)[0],
    });
  }
}

for (const failure of failures) {
  process.stderr.write(
    `[public-commands] ERROR: ${failure.file.slice(ROOT.length + 1)}:${failure.line} — ${failure.command} — ${failure.reason}\n`
  );
}
process.stdout.write(
  `[public-commands] ${commands.length - failures.length}/${commands.length} fenced anvil commands parse against ${ANVIL_BIN}\n`
);
process.exit(failures.length === 0 ? 0 : 1);

function flagValue(name) {
  const index = argv.indexOf(name);
  return index >= 0 && index + 1 < argv.length ? argv[index + 1] : undefined;
}

function markdownFiles(root) {
  if (!existsSync(root)) return [];
  const found = [];
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    const path = resolve(root, entry.name);
    if (entry.isDirectory()) found.push(...markdownFiles(path));
    else if (entry.isFile() && /\.mdx?$/.test(entry.name)) found.push(path);
  }
  return found;
}

function shellWords(command) {
  if (/[|;&`$()]/.test(command)) return undefined;
  const words = [];
  let current = '';
  let quote;
  for (let index = 0; index < command.length; index += 1) {
    const character = command[index];
    if (quote) {
      if (character === quote) quote = undefined;
      else current += character;
    } else if (character === '"' || character === "'") {
      quote = character;
    } else if (/\s/.test(character)) {
      if (current) {
        words.push(current);
        current = '';
      }
    } else {
      current += character;
    }
  }
  if (quote) return undefined;
  if (current) words.push(current);
  return words;
}
