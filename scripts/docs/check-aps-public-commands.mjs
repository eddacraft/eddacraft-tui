#!/usr/bin/env node

import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import process from 'node:process';

const argv = process.argv.slice(2);
const SCRIPT_ROOT = fileURLToPath(new URL('../..', import.meta.url));
const ROOT = flagValue('--root') ? resolve(flagValue('--root')) : SCRIPT_ROOT;
const CONTRACT_PATH = flagValue('--contract')
  ? resolve(flagValue('--contract'))
  : resolve(SCRIPT_ROOT, 'scripts/docs/aps-public-cli-contract.json');
const contract = JSON.parse(readFileSync(CONTRACT_PATH, 'utf8'));
const files = markdownFiles(resolve(ROOT, 'docs/public/aps'));
const examples = fencedCommands(files);
const failures = [];
const globalOptions = new Set(contract.globalOptions);

for (const example of examples) validate(example);

for (const failure of failures) {
  process.stderr.write(
    `[aps-public-commands] ERROR: ${relative(ROOT, failure.file)}:${failure.line} — ${failure.command} — ${failure.reason}\n`
  );
}
process.stdout.write(
  `[aps-public-commands] ${examples.length - failures.length}/${examples.length} fenced APS commands match ${contract.source.repository} ${contract.source.version} at ${contract.source.commit.slice(0, 8)}\n`
);
process.exit(failures.length === 0 ? 0 : 1);

function validate(example) {
  const words = shellWords(example.command);
  if (!words || words[0] !== 'aps') {
    failures.push({ ...example, reason: 'unsupported shell syntax in command example' });
    return;
  }
  if (words.length === 1) return;

  const first = words[1];
  if (first.startsWith('-')) {
    for (const token of words.slice(1).filter((word) => word.startsWith('-'))) {
      const option = token.split('=', 1)[0];
      if (!globalOptions.has(option)) {
        failures.push({ ...example, reason: `aps does not accept ${option} without a command` });
        return;
      }
    }
    return;
  }

  const commandOptions = contract.commands[first];
  if (!commandOptions) {
    failures.push({ ...example, reason: `unknown command '${first}'` });
    return;
  }
  const allowed = new Set([...globalOptions, ...commandOptions]);
  for (const token of words.slice(2).filter((word) => word.startsWith('-'))) {
    const option = token.split('=', 1)[0];
    if (!allowed.has(option)) {
      failures.push({ ...example, reason: `${first} does not accept ${option}` });
      return;
    }
  }
}

function fencedCommands(paths) {
  const found = [];
  for (const file of paths) {
    let fenced = false;
    for (const [index, line] of readFileSync(file, 'utf8').split(/\r?\n/).entries()) {
      if (line.startsWith('```')) {
        fenced = !fenced;
        continue;
      }
      if (!fenced) continue;
      const trimmed = line.trim();
      const yamlRun = /^run:\s+(aps(?:\s+.*)?)$/.exec(trimmed);
      const command = trimmed.startsWith('aps ') || trimmed === 'aps' ? trimmed : yamlRun?.[1];
      if (command) found.push({ file, line: index + 1, command });
    }
  }
  return found;
}

function shellWords(command) {
  if (/[|;&`$()]/.test(command)) return undefined;
  const words = [];
  let current = '';
  let quote;
  for (const character of command) {
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

function flagValue(name) {
  const index = argv.indexOf(name);
  return index >= 0 && index + 1 < argv.length ? argv[index + 1] : undefined;
}
