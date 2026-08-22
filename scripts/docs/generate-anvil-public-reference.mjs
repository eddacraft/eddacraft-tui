#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import process from 'node:process';

const argv = process.argv.slice(2);

function flagValue(name) {
  const index = argv.indexOf(name);
  return index >= 0 && index + 1 < argv.length ? argv[index + 1] : undefined;
}

const ROOT = flagValue('--root')
  ? resolve(flagValue('--root'))
  : resolve(fileURLToPath(new URL('../..', import.meta.url)));
const CHECK = argv.includes('--check');
const UPDATE_HELP_SNAPSHOTS = argv.includes('--update-help-snapshots');
const ANVIL_BIN = flagValue('--anvil-bin') ?? 'anvil';

const PLANLESS_CHECK_NAMES = ['secret-detection', 'antipattern-scan'];

const SCOPED_COMMANDS = [
  { name: 'start', path: 'crates/anvil-cli/src/commands/start.rs', args: 'StartArgs' },
  { name: 'check', path: 'crates/anvil-cli/src/commands/check.rs', args: 'CheckArgs' },
  { name: 'gate', path: 'crates/anvil-cli/src/commands/gate.rs', args: 'GateArgs' },
  { name: 'config', path: 'crates/anvil-cli/src/commands/config.rs', args: 'ConfigArgs' },
  { name: 'watch', path: 'crates/anvil-cli/src/commands/watch.rs', args: 'WatchArgs' },
  { name: 'doctor', path: 'crates/anvil-cli/src/commands/doctor.rs', args: 'DoctorArgs' },
  { name: 'init', path: 'crates/anvil-cli/src/commands/init.rs', args: 'InitArgs' },
  { name: 'policy', path: 'crates/anvil-cli/src/commands/policy/mod.rs', args: 'PolicyArgs' },
];
const HELP_SNAPSHOT_COMMANDS = [
  'check',
  'gate',
  'policy',
  'config',
  'start',
  'watch',
  'doctor',
  'init',
];
const REQUIRED_HELP_SNAPSHOTS = ['check', 'gate', 'policy', 'config'];
const HELP_SNAPSHOT_DIR = resolve(ROOT, 'scripts/docs/fixtures/anvil-cli-help');
const GLOBAL_FLAG_NAMES = ['json', 'no-tui', 'verbose', 'anvil-home', 'touch-project-state'];
const SNAPSHOT_SKIP_FLAGS = new Set(['help', 'version', ...GLOBAL_FLAG_NAMES]);
const SNAPSHOT_SKIP_COMMANDS = new Set(['help']);

// Surface-check flag ids are an explicit generator table, not comments from
// check_catalog.rs. Each flag_id must exist in flags/manifest.json.
const SURFACE_CHECK_FLAGS = [
  {
    canonical_name: 'sql-migrations',
    flag_id: 'track.surface.sql',
    session_opt_out: 'ANVIL_TRACK_SURFACE_SQL=0',
  },
  {
    canonical_name: 'github-actions',
    flag_id: 'track.surface.gha',
    session_opt_out: 'ANVIL_TRACK_SURFACE_GHA=0',
  },
  {
    canonical_name: 'dockerfile',
    flag_id: 'track.surface.dock',
    session_opt_out: 'ANVIL_TRACK_SURFACE_DOCK=0',
  },
  {
    canonical_name: 'shell-scripts',
    flag_id: 'track.surface.sh',
    session_opt_out: 'ANVIL_TRACK_SURFACE_SH=0',
  },
];

const inputs = {
  registry: resolve(ROOT, 'patterns/compiled/registry.json'),
  cli: resolve(ROOT, 'crates/anvil-cli/src/main.rs'),
  start: resolve(ROOT, 'crates/anvil-cli/src/commands/start.rs'),
  check: resolve(ROOT, 'crates/anvil-cli/src/commands/check.rs'),
  gate: resolve(ROOT, 'crates/anvil-cli/src/commands/gate.rs'),
  config: resolve(ROOT, 'crates/anvil-cli/src/commands/config.rs'),
  watch: resolve(ROOT, 'crates/anvil-cli/src/commands/watch.rs'),
  doctor: resolve(ROOT, 'crates/anvil-cli/src/commands/doctor.rs'),
  init: resolve(ROOT, 'crates/anvil-cli/src/commands/init.rs'),
  policy: resolve(ROOT, 'crates/anvil-cli/src/commands/policy/mod.rs'),
  checkCatalog: resolve(ROOT, 'crates/anvil-cli/src/commands/check_catalog.rs'),
  flagManifest: resolve(ROOT, 'flags/manifest.json'),
  // Full MCP install registry (MCPX / ADR-106), not the two-client v1
  // protection-ladder enum in diagnostic.rs.
  clients: resolve(ROOT, 'crates/anvil-cli/src/activation/agent_registry.rs'),
  languages: resolve(ROOT, 'crates/anvil-kernel/src/parser/languages.rs'),
  dist: resolve(ROOT, 'dist-workspace.toml'),
};

const release = latestPublicRelease();
const sourceTag = release ? `v${release}` : undefined;
const sourceTagRef = sourceTag ? `refs/tags/${sourceTag}` : undefined;
const sourceRefResolved = sourceTagRef ? resolvesToCommit(sourceTagRef, sourceTag) : false;

if (!sourceRefResolved) {
  for (const [name, path] of Object.entries(inputs)) {
    if (!existsSync(path)) fail(`missing ${name} workspace source: ${path}`);
  }
  if (sourceTag) {
    process.stderr.write(
      `[anvil-reference] public release ref ${sourceTag} does not resolve; using workspace tree for all product inputs\n`
    );
  }
}

const registry = JSON.parse(readProductSource(inputs.registry));
const cliSource = readProductSource(inputs.cli);
const commands = parseCommands(cliSource);
const globalFlags = parseArgsStruct(cliSource, 'GlobalArgs')?.flags ?? [];
const scopedCommands = parseScopedCommands();
const exitCodes = parseExitCodes(cliSource);
const clients = parseClients(readProductSource(inputs.clients));
const languages = parseLanguages(readProductSource(inputs.languages));
const targets = parseTargets(readProductSource(inputs.dist));
const checkCatalogSource = readProductSource(inputs.checkCatalog);
const checkDefinitions = parseCheckDefinitions(checkCatalogSource);
const initDefaultChecks = parseStringSlice(checkCatalogSource, 'DEFAULT_INIT_CHECKS');
const planlessChecks = parseStringSlice(
  readProductSource(inputs.check),
  'PLANLESS_ELIGIBLE_CHECKS'
);
const surfaceFlags = resolveSurfaceCheckFlags(
  checkDefinitions,
  JSON.parse(readProductSource(inputs.flagManifest))
);
const ruleExtensions = new Set(
  registry.patterns.flatMap((pattern) => pattern.file_extensions ?? []).map((ext) => ext.slice(1))
);

const rendered = new Map([
  [
    resolve(ROOT, 'docs/public/anvil/reference/cli.md'),
    renderCli(commands, globalFlags, scopedCommands, exitCodes),
  ],
  [resolve(ROOT, 'docs/public/anvil/reference/rules.md'), renderRules(registry)],
  [
    resolve(ROOT, 'docs/public/anvil/reference/checks.md'),
    renderChecks(checkDefinitions, initDefaultChecks, planlessChecks, surfaceFlags),
  ],
  [
    resolve(ROOT, 'docs/public/anvil/reference/support.md'),
    renderSupport(languages, targets, ruleExtensions, clients),
  ],
]);
const outputs = new Map(
  [...rendered].map(([path, content]) => [path, formatMarkdown(path, content)])
);

let stale = 0;
for (const [path, content] of outputs) {
  if (CHECK) {
    if (!existsSync(path) || readFileSync(path, 'utf8') !== content) {
      process.stderr.write(`[anvil-reference] stale: ${path.slice(ROOT.length + 1)}\n`);
      stale += 1;
    }
  } else {
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, content);
    process.stdout.write(`[anvil-reference] wrote ${path.slice(ROOT.length + 1)}\n`);
  }
}

if (UPDATE_HELP_SNAPSHOTS) {
  writeHelpSnapshots();
}

if (CHECK) {
  if (stale > 0) fail(`${stale} generated reference file(s) need regeneration`);
  checkHelpSnapshots(outputs.get(resolve(ROOT, 'docs/public/anvil/reference/cli.md')) ?? '');
  process.stdout.write(`[anvil-reference] ${outputs.size} generated reference files are current\n`);
}

function parseStringSlice(source, name) {
  const match = new RegExp(`const ${name}:\\s*&\\[&str\\]\\s*=\\s*&\\[([\\s\\S]*?)\\];`).exec(
    source
  );
  if (!match) fail(`could not locate ${name}`);
  return [...match[1].matchAll(/"([^"]+)"/g)].map((item) => item[1]);
}

function parseCheckDefinitions(source) {
  const start = source.indexOf('pub(crate) const CHECK_DEFINITIONS: &[CheckDefinition] = &[');
  if (start < 0) fail('could not locate CHECK_DEFINITIONS');
  const end = source.indexOf('\n];', start);
  if (end < 0) fail('could not locate the end of CHECK_DEFINITIONS');
  const blocks = source
    .slice(start, end)
    .split(/CheckDefinition\s*\{/)
    .slice(1);
  if (blocks.length === 0) fail('CHECK_DEFINITIONS contained no entries');
  return blocks.map((block, index) => {
    const stringField = (name) => {
      const match = new RegExp(`${name}:\\s*"([^"]*)"`).exec(block);
      if (!match) fail(`CHECK_DEFINITIONS[${index}] missing ${name}`);
      return match[1];
    };
    const boolField = (name) => {
      const match = new RegExp(`${name}:\\s*(true|false)`).exec(block);
      if (!match) fail(`CHECK_DEFINITIONS[${index}] missing ${name}`);
      return match[1] === 'true';
    };
    const listField = (name) => {
      const match = new RegExp(`${name}:\\s*&\\[([\\s\\S]*?)\\]`).exec(block);
      if (!match) fail(`CHECK_DEFINITIONS[${index}] missing ${name}`);
      return [...match[1].matchAll(/"([^"]*)"/g)].map((item) => item[1]);
    };
    return {
      stable_id: stringField('stable_id'),
      canonical_name: stringField('canonical_name'),
      aliases: listField('aliases'),
      description: stringField('description'),
      init_enabled: boolField('init_enabled'),
      init_visible: boolField('init_visible'),
      gate_supported: boolField('gate_supported'),
      gate_config_supported: boolField('gate_config_supported'),
    };
  });
}

function resolveSurfaceCheckFlags(definitions, manifest) {
  const knownFlags = new Set((manifest.flags ?? []).map((flag) => flag.key));
  for (const row of SURFACE_CHECK_FLAGS) {
    if (!knownFlags.has(row.flag_id)) {
      fail(`SURFACE_CHECK_FLAGS cites unknown flag ${row.flag_id}`);
    }
  }
  const byName = new Map(SURFACE_CHECK_FLAGS.map((row) => [row.canonical_name, row]));
  const missing = definitions
    .filter((definition) => !definition.gate_config_supported)
    .filter((definition) => !byName.has(definition.canonical_name))
    .map((definition) => definition.canonical_name);
  if (missing.length > 0) {
    fail(
      `gate_config_supported:false checks missing SURFACE_CHECK_FLAGS rows: ${missing.join(', ')}`
    );
  }
  return byName;
}

function parseCommands(source) {
  const start = source.indexOf('enum Commands {');
  const end = source.indexOf('\n}\n\n/// Canonical stable name', start);
  if (start < 0 || end < 0) fail('could not locate the top-level Commands enum');

  const commands = [];
  let docs = [];
  let explicitName;
  let hidden = false;
  let nesting = 0;
  for (const line of source.slice(start, end).split(/\r?\n/).slice(1)) {
    if (nesting > 0) {
      nesting += delimiterBalance(line);
      continue;
    }
    const comment = /^\s*\/\/\/\s?(.*)$/.exec(line);
    if (comment) {
      docs.push(comment[1]);
      continue;
    }
    const attribute = /^\s*#\[command\((.*)\)\]/.exec(line);
    if (attribute) {
      explicitName = /name\s*=\s*"([^"]+)"/.exec(attribute[1])?.[1];
      hidden = /\bhide\s*=\s*true\b/.test(attribute[1]);
      continue;
    }
    if (/^\s*$/.test(line)) continue;
    const variant = /^\s*([A-Z][A-Za-z0-9]*)\s*(?:\(|\{|,)/.exec(line);
    if (!variant) fail(`could not parse Commands enum line: ${line.trim()}`);
    if (!hidden) {
      const description = docs.filter(Boolean).join(' ').split(/\.\s/)[0].replace(/\.$/, '');
      const name = explicitName ?? kebabCase(variant[1]);
      commands.push({
        name,
        description: publicCommandDescription(
          name,
          lowerBrands(description || 'See `anvil help` for command details')
        ),
      });
    }
    docs = [];
    explicitName = undefined;
    hidden = false;
    nesting += delimiterBalance(line);
  }
  return commands.sort((a, b) => a.name.localeCompare(b.name));
}

function delimiterBalance(line) {
  const structural = line.replace(/"(?:\\.|[^"\\])*"/g, '').replace(/\/\/.*$/, '');
  return [...structural].reduce((depth, character) => {
    if (character === '(' || character === '{') return depth + 1;
    if (character === ')' || character === '}') return depth - 1;
    return depth;
  }, 0);
}

function parseClients(source) {
  // Prefer display_name from the ADR-106 agent registry; fall back to the
  // enum variants when the registry table is unavailable.
  const displayNames = [...source.matchAll(/display_name:\s*"([^"]+)"/g)].map((match) => match[1]);
  if (displayNames.length > 0) return displayNames;

  const block = /pub enum AgentClientId\s*\{([\s\S]*?)\n\}/.exec(source)?.[1];
  if (!block) fail('could not locate supported MCP clients (AgentClientId)');
  const clients = [...block.matchAll(/^\s*(?:#\[[^\]]*\]\s*)*([A-Z][A-Za-z0-9]*),?\s*$/gm)].map(
    (match) => match[1]
  );
  if (clients.length === 0) fail('supported MCP client list is empty');
  return clients.map(clientDisplayName);
}

function clientDisplayName(variant) {
  // Must match AgentClient.display_name in agent_registry.rs (enum fallback only).
  const names = {
    ClaudeCode: 'Claude Code',
    Cursor: 'Cursor',
    Codex: 'Codex',
    OpenCode: 'OpenCode',
    GeminiCli: 'Gemini CLI',
    Antigravity: 'Antigravity',
    OpenClaw: 'OpenClaw',
    VsCode: 'VS Code',
    CopilotCli: 'GitHub Copilot CLI',
    Grok: 'Grok Build',
    Warp: 'Warp',
    Zed: 'Zed',
  };
  return names[variant] ?? variant;
}

function parseScopedCommands() {
  const details = new Map();
  for (const spec of SCOPED_COMMANDS) {
    const path = resolve(ROOT, spec.path);
    const source = readOptionalProductSource(path);
    if (!source) continue;
    const parsed = parseArgsStruct(source, spec.args, path) ?? emptyCommandSurface();
    if (spec.name === 'start' && parsed.flags.length === 0) {
      fail('StartArgs produced no public flags');
    }
    details.set(spec.name, parsed);
  }
  return details;
}

function emptyCommandSurface() {
  return { flags: [], arguments: [], subcommands: [] };
}

function parseArgsStruct(source, typeName, filePath) {
  const body = extractTypeBody(source, 'struct', typeName);
  if (!body) return null;
  return parseArgsBody(body, source, filePath);
}

function extractTypeBody(source, kind, name) {
  const match = new RegExp(`\\b${kind}\\s+${name}\\s*\\{`).exec(source);
  if (!match) return null;
  const brace = match.index + match[0].length - 1;
  let depth = 0;
  let inString = false;
  let escaped = false;
  for (let index = brace; index < source.length; index += 1) {
    const character = source[index];
    if (inString) {
      if (escaped) escaped = false;
      else if (character === '\\') escaped = true;
      else if (character === '"') inString = false;
      continue;
    }
    if (character === '"') {
      inString = true;
      continue;
    }
    if (character === '{') depth += 1;
    else if (character === '}') {
      depth -= 1;
      if (depth === 0) return source.slice(brace + 1, index);
    }
  }
  fail(`could not locate the end of ${kind} ${name}`);
}

function parseArgsBody(body, source, filePath) {
  const flags = [];
  const args = [];
  let subcommands = [];
  let docs = [];
  let argAttr = '';
  let commandAttr = '';

  const flush = (fieldName, fieldType) => {
    const description = firstSentence(docs);
    const commandBody = commandAttr;
    const parsedArg = parseArgAttribute(argAttr);
    docs = [];
    argAttr = '';
    commandAttr = '';
    if (!fieldName) return;
    if (/\bsubcommand\b/.test(commandBody)) {
      const typeName = rustTypeName(fieldType);
      subcommands = parseSubcommandEnum(source, typeName, filePath);
      return;
    }
    if (/\bflatten\b/.test(commandBody) || parsedArg.hidden) return;
    if (parsedArg.hasLong) {
      flags.push({
        flag: `--${parsedArg.longName ?? fieldName.replaceAll('_', '-')}`,
        description: lowerBrands(description || `See \`anvil --help\``),
      });
      return;
    }
    args.push({
      name: parsedArg.valueName ?? fieldName.toUpperCase(),
      description: lowerBrands(description || 'Positional argument'),
    });
  };

  for (const line of splitClapLines(body)) {
    const comment = /^\/\/\/\s?(.*)$/.exec(line);
    if (comment) {
      docs.push(comment[1]);
      continue;
    }
    const attribute = /^#\[(arg|command)\((.*)\)\]$/.exec(line);
    if (attribute) {
      if (attribute[1] === 'arg') argAttr += (argAttr ? ' ' : '') + attribute[2];
      else commandAttr += (commandAttr ? ' ' : '') + attribute[2];
      continue;
    }
    const field = /^(?:pub(?:\([^)]+\))?\s+)?([a-z][a-z0-9_]*)\s*:\s*(.+?),?\s*$/.exec(line);
    if (field) {
      flush(field[1], field[2].replace(/,$/, ''));
      continue;
    }
  }

  return { flags, arguments: args, subcommands };
}

function parseSubcommandEnum(source, typeName, filePath) {
  const body = extractTypeBody(source, 'enum', typeName);
  if (!body) return [];
  const commands = [];
  const lines = body.split(/\r?\n/);
  let index = 0;
  while (index < lines.length) {
    let line = lines[index].trim();
    if (!line || (line.startsWith('//') && !line.startsWith('///'))) {
      index += 1;
      continue;
    }
    const docs = [];
    let explicitName;
    let hidden = false;
    while (index < lines.length) {
      line = lines[index].trim();
      const comment = /^\/\/\/\s?(.*)$/.exec(line);
      if (comment) {
        docs.push(comment[1]);
        index += 1;
        continue;
      }
      const attribute = /^#\[command\((.*)\)\]$/.exec(line);
      if (attribute) {
        explicitName = /name\s*=\s*"([^"]+)"/.exec(attribute[1])?.[1];
        hidden = /\bhide\s*=\s*true\b/.test(attribute[1]);
        index += 1;
        continue;
      }
      break;
    }
    if (index >= lines.length) break;
    line = lines[index].trim();
    if (!line || line.startsWith('//')) {
      index += 1;
      continue;
    }
    const tuple = /^([A-Z][A-Za-z0-9]*)\s*\(([^)]*)\)\s*,?\s*$/.exec(line);
    const unit = /^([A-Z][A-Za-z0-9]*)\s*,?\s*$/.exec(line);
    const structStart = /^([A-Z][A-Za-z0-9]*)\s*\{/.exec(line);
    let variantName;
    let nested = emptyCommandSurface();
    if (tuple) {
      variantName = tuple[1];
      nested = resolveNestedArgs(tuple[2].trim(), source, filePath);
      index += 1;
    } else if (structStart) {
      variantName = structStart[1];
      const openLine = lines[index];
      const open = openLine.indexOf('{');
      let depth = delimiterBalance(openLine.slice(open));
      const chunks = [openLine.slice(open + 1)];
      index += 1;
      while (index < lines.length && depth > 0) {
        chunks.push(lines[index]);
        depth += delimiterBalance(lines[index]);
        index += 1;
      }
      const structBody = chunks.join('\n').replace(/\}\s*,?\s*$/, '');
      nested = parseArgsBody(structBody, source, filePath);
    } else if (unit) {
      variantName = unit[1];
      index += 1;
    } else {
      fail(`could not parse ${typeName} variant: ${line}`);
    }
    if (hidden) continue;
    const name = explicitName ?? kebabCase(variantName);
    commands.push({
      name,
      description: lowerBrands(firstSentence(docs) || `See \`anvil ${name} --help\``),
      flags: nested.flags,
      arguments: nested.arguments,
      subcommands: nested.subcommands,
    });
  }
  return commands;
}

function resolveNestedArgs(typeExpr, source, filePath) {
  const parts = typeExpr.split('::').map((part) => part.trim());
  if (parts.length === 1) {
    return parseArgsStruct(source, parts[0], filePath) ?? emptyCommandSurface();
  }
  if (parts.length === 2 && filePath) {
    const nestedPath = resolve(dirname(filePath), `${parts[0]}.rs`);
    const nestedSource = readOptionalProductSource(nestedPath);
    if (!nestedSource) return emptyCommandSurface();
    return parseArgsStruct(nestedSource, parts[1], nestedPath) ?? emptyCommandSurface();
  }
  return emptyCommandSurface();
}

function parseArgAttribute(body) {
  const explicitLong = /long\s*=\s*"([^"]+)"/.exec(body)?.[1];
  return {
    longName: explicitLong,
    hasLong: Boolean(explicitLong) || /(^|[^a-zA-Z])long([^a-zA-Z]|$)/.test(body),
    hidden: /\bhide\s*=\s*true\b/.test(body),
    valueName: /value_name\s*=\s*"([^"]+)"/.exec(body)?.[1],
  };
}

function splitClapLines(body) {
  const lines = [];
  let buffer = '';
  let depth = 0;
  for (const line of body.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!buffer && trimmed.startsWith('#[')) {
      buffer = trimmed;
      depth = delimiterBalance(trimmed);
      if (depth === 0) {
        lines.push(buffer);
        buffer = '';
      }
      continue;
    }
    if (buffer) {
      buffer += trimmed;
      depth += delimiterBalance(trimmed);
      if (depth <= 0) {
        lines.push(buffer);
        buffer = '';
        depth = 0;
      }
      continue;
    }
    if (trimmed) lines.push(trimmed);
  }
  if (buffer) lines.push(buffer);
  return lines;
}

function firstSentence(docs) {
  const text = docs
    .filter(Boolean)
    .join(' ')
    .replace(/\b(e\.g|i\.e)\./gi, '$1\u2024')
    .trim();
  return text.split(/\.\s/)[0].replace(/\.$/, '').replaceAll('\u2024', '.').trim();
}

function rustTypeName(fieldType) {
  return fieldType.replace(/,$/, '').trim();
}

function readOptionalProductSource(path) {
  if (!sourceRefResolved) {
    if (!existsSync(path)) return undefined;
    return readFileSync(path, 'utf8');
  }
  const repoPath = relative(ROOT, path).replaceAll('\\', '/');
  const result = spawnSync('git', ['-C', ROOT, 'show', `${sourceTagRef}:${repoPath}`], {
    encoding: 'utf8',
  });
  return result.status === 0 ? result.stdout : undefined;
}

function writeHelpSnapshots() {
  mkdirSync(HELP_SNAPSHOT_DIR, { recursive: true });
  for (const command of HELP_SNAPSHOT_COMMANDS) {
    const result = spawnSync(ANVIL_BIN, [command, '--help'], {
      encoding: 'utf8',
      env: { ...process.env, DO_NOT_TRACK: '1' },
    });
    const text = `${result.stdout || ''}${result.stderr || ''}`;
    if (!text.includes('Usage:')) {
      fail(
        `could not capture ${ANVIL_BIN} ${command} --help: ${(result.stderr || text).trim() || `exit ${result.status}`}`
      );
    }
    writeFileSync(helpSnapshotPath(command), text);
    process.stdout.write(`[anvil-reference] wrote help snapshot for ${command}\n`);
  }
}

function checkHelpSnapshots(page) {
  if (!existsSync(HELP_SNAPSHOT_DIR)) return;
  const present = new Set(
    readdirSync(HELP_SNAPSHOT_DIR)
      .filter((name) => name.endsWith('.txt'))
      .map((name) => name.replace(/\.txt$/, ''))
  );
  if (present.size === 0) return;
  let mismatches = 0;
  for (const command of REQUIRED_HELP_SNAPSHOTS) {
    if (present.has(command)) continue;
    process.stderr.write(`[anvil-reference] missing help snapshot: ${command}.txt\n`);
    mismatches += 1;
  }
  for (const command of [...present].sort()) {
    const path = helpSnapshotPath(command);
    const snapshot = parseHelpSnapshot(readFileSync(path, 'utf8'));
    const missingFlags = [...snapshot.flags].filter(
      (flag) => !SNAPSHOT_SKIP_FLAGS.has(flag) && !pageIncludesFlag(page, command, flag)
    );
    const missingCommands = snapshot.commands.filter(
      (name) => !SNAPSHOT_SKIP_COMMANDS.has(name) && !page.includes(`anvil ${command} ${name}`)
    );
    if (missingFlags.length > 0 || missingCommands.length > 0) {
      process.stderr.write(
        `[anvil-reference] help-snapshot drift for ${command}: missing flags [${missingFlags.join(', ')}] missing subcommands [${missingCommands.join(', ')}]\n`
      );
      mismatches += 1;
    }
  }
  if (mismatches > 0) fail(`${mismatches} CLI help snapshot(s) disagree with the generated page`);
}

function helpSnapshotPath(command) {
  return resolve(HELP_SNAPSHOT_DIR, `${command}.txt`);
}

function pageIncludesFlag(page, command, flag) {
  return commandSection(page, command).includes(`\`--${flag}\``);
}

function commandSection(page, command) {
  const start = page.indexOf(`### \`anvil ${command}\``);
  if (start < 0) return '';
  const rest = page.slice(start);
  const next = rest.slice(4).search(/\n### `/);
  return next < 0 ? rest : rest.slice(0, next + 4);
}

function parseHelpSnapshot(text) {
  const options = extractHelpSection(text, 'Options');
  const commands = extractHelpSection(text, 'Commands');
  const flags = new Set();
  for (const match of options.matchAll(/^\s*(?:-[a-zA-Z],\s*)?--([a-z0-9][a-z0-9-]*)/gm)) {
    flags.add(match[1]);
  }
  const names = [];
  for (const match of commands.matchAll(/^\s{2}([a-z][a-z0-9-]*)(?:\s|$)/gm)) names.push(match[1]);
  return { flags, commands: names };
}

function extractHelpSection(text, heading) {
  const start = text.search(new RegExp(`^${heading}:\\s*$`, 'm'));
  if (start < 0) return '';
  const rest = text.slice(start + heading.length + 2);
  const stop = rest.search(
    /^(?:Arguments|Options|Commands|EXIT CODES|WHEN TO USE|COMMON FLAGS|LEARN MORE|Behaviour):/m
  );
  return stop < 0 ? rest : rest.slice(0, stop);
}

function parseExitCodes(source) {
  const codes = [];
  for (const match of source.matchAll(/pub const (EXIT_[A-Z0-9_]+):\s*u8\s*=\s*(\d+);/g)) {
    codes.push({ name: match[1], code: Number(match[2]) });
  }
  if (codes.length === 0) fail('could not locate EXIT_* constants in main.rs');
  return codes;
}

function parseLanguages(source) {
  const start = source.indexOf('pub fn from_path');
  const functionSource = start >= 0 ? source.slice(start) : '';
  const end = /\n\s*_ => None,/.exec(functionSource)?.index ?? -1;
  if (start < 0 || end < 0) fail('could not locate Language::from_path');
  const byLanguage = new Map();
  for (const line of functionSource.slice(0, end).split(/\r?\n/)) {
    const match = /^\s*((?:"[^"]+"(?:\s*\|\s*)?)+)\s*=>\s*Some\(Self::([A-Za-z0-9]+)\)/.exec(line);
    if (!match) continue;
    const extensions = [...match[1].matchAll(/"([^"]+)"/g)].map((item) => item[1]);
    const current = byLanguage.get(match[2]) ?? [];
    byLanguage.set(match[2], [...current, ...extensions]);
  }
  return [...byLanguage].map(([variant, extensions]) => ({
    name: languageName(variant),
    extensions,
  }));
}

function parseTargets(source) {
  const block = /targets\s*=\s*\[([\s\S]*?)\]/.exec(source)?.[1];
  if (!block) fail('could not locate release targets');
  return [...block.matchAll(/"([^"]+)"/g)].map((match) => match[1]);
}

function renderCli(commands, globalFlags, scoped, exitCodes) {
  const rows = commands
    .map(({ name, description }) => `| \`anvil ${name}\` | ${escapeCell(description)} |`)
    .join('\n');
  const globalRows = renderFlagRows(globalFlags);
  const scopedSections = SCOPED_COMMANDS.map((spec) =>
    renderScopedCommand(spec.name, scoped.get(spec.name), commands)
  )
    .filter(Boolean)
    .join('\n');
  const exitRows = exitCodes
    .map(({ name, code }) => {
      const meaning = exitCodeMeaning(name, code);
      return `| \`${code}\` | ${escapeCell(meaning)} |`;
    })
    .join('\n');
  return (
    generatedHeader(
      'cli-reference',
      'CLI command reference',
      'Discover every public top-level anvil command, plus flags and subcommands for the daily set.'
    ) +
    `# CLI command reference\n\n` +
    `This page is generated from the command definitions shipped with ${releaseLabel()}. ` +
    `Global flags appear once. Hidden clap commands are unpublished. ` +
    `Flags and subcommands below cover the daily set (\`start\`, \`check\`, \`gate\`, \`config\`, \`watch\`, \`doctor\`, \`init\`, \`policy\`). ` +
    `Use \`anvil <command> --help\` for other commands and for examples on your installed version.\n\n` +
    `For a first installation, use the [quickstart](../quickstart.md).\n\n` +
    `## Daily ensure\n\n` +
    `With no subcommand, bare \`anvil\` runs the daily ensure surface: it turns protection on for an already-activated project (daemon + existing MCP entries). ` +
    `It does not install clients you skipped or rewrite configuration — use \`anvil start\` to activate or reconfigure.\n\n` +
    `| Command | Purpose |\n| --- | --- |\n` +
    `| \`anvil\` | Turn protection on for an already-activated project (daily ensure) |\n` +
    `${rows}\n\n` +
    `## Global flags\n\n` +
    `These flags are available on every command. They are not repeated in the per-command tables.\n\n` +
    `| Flag | Purpose |\n| --- | --- |\n` +
    `${globalRows}\n\n` +
    `## Command flags and subcommands\n\n` +
    scopedSections +
    `## Exit codes\n\n` +
    `Stable process exit codes used by the CLI. Scripts should gate on these values rather than parsing human-readable prose.\n\n` +
    `| Code | Meaning |\n| --- | --- |\n` +
    `${exitRows}\n\n` +
    `### Authentication-required behaviour\n\n` +
    `- **Action commands** (\`anvil start\`, bare \`anvil\`, \`anvil init\`, \`anvil gate\`, \`anvil check\`, \`anvil watch\`, and other gated mutating surfaces) exit **\`3\`** when authentication is required, so \`&&\` chains and script preflights stop at an unauthenticated or unactivated repo.\n` +
    `- **Read-only status** (\`anvil status\`) exits **\`0\`** when authentication is required and reports an informational \`authRequired\` envelope under \`--json\`. Auth-required is the expected answer on that state probe, not a failure.\n` +
    `- Auth state probes such as \`anvil auth whoami\` exit **\`3\`** so scripts can detect a missing login without treating it as a generic error \`1\`.\n` +
    `- Read-only activation probes (\`anvil start --verify\`, \`anvil status --verify\`) bypass the pre-dispatch auth wall entirely.\n\n` +
    `When \`--json\` is set, action-command auth-required responses use an informational envelope (\`state: "authRequired"\`, \`next\`, optional \`earlyAccessUrl\`) on stdout while still exiting \`3\`.\n`
  );
}

function renderScopedCommand(name, surface, commands) {
  if (!surface) return '';
  const summary = commands.find((command) => command.name === name)?.description;
  const heading = `### \`anvil ${name}\`\n\n`;
  const purpose = summary ? `${escapeCell(summary)}.\n\n` : '';
  const argumentTable = renderNamedRows('Argument', surface.arguments, (item) => [
    item.name,
    item.description,
  ]);
  const flagTable = renderNamedRows('Flag', surface.flags, (item) => [item.flag, item.description]);
  const subcommandTable = renderNamedRows('Subcommand', surface.subcommands, (item) => [
    `anvil ${name} ${item.name}`,
    item.description,
  ]);
  const nested = surface.subcommands
    .map((item) => renderNestedSurface(`anvil ${name} ${item.name}`, item))
    .join('');
  const extra =
    name === 'start'
      ? `Interactive \`anvil start\` offers every installable MCP client (unticked by default). ` +
        `Scripted multi-client install uses \`--mcp-client <id>\` (repeatable), \`--all-mcp-clients\`, and \`--mcp-scope global|project\`. ` +
        `Discover client ids with \`anvil mcp install --help\`.\n\n`
      : '';
  if (!argumentTable && !flagTable && !subcommandTable && !nested && !extra) return '';
  return heading + purpose + argumentTable + flagTable + subcommandTable + nested + extra;
}

function renderNestedSurface(title, surface) {
  const argumentTable = renderNamedRows('Argument', surface.arguments, (item) => [
    item.name,
    item.description,
  ]);
  const flagTable = renderNamedRows('Flag', surface.flags, (item) => [item.flag, item.description]);
  const subcommandTable = renderNamedRows('Subcommand', surface.subcommands, (item) => [
    item.name,
    item.description,
  ]);
  if (!argumentTable && !flagTable && !subcommandTable) return '';
  return `#### \`${title}\`\n\n` + argumentTable + flagTable + subcommandTable;
}

function renderFlagRows(flags) {
  return flags
    .map(({ flag, description }) => `| \`${flag}\` | ${escapeCell(description)} |`)
    .join('\n');
}

function renderNamedRows(header, items, cells) {
  if (!items || items.length === 0) return '';
  const rows = items
    .map((item) => {
      const [name, description] = cells(item);
      return `| \`${name}\` | ${escapeCell(description)} |`;
    })
    .join('\n');
  return `| ${header} | Purpose |\n| --- | --- |\n${rows}\n\n`;
}

function exitCodeMeaning(name, code) {
  const meanings = {
    EXIT_OK: 'Success',
    EXIT_ERROR: 'General error (recoverable user-action condition)',
    EXIT_GATE_FAIL: 'Gate failure — fail-fast for CI and scripted gates',
    EXIT_AUTH_REQUIRED:
      'Authentication required on an action command or auth probe (not used by read-only `status`, which exits 0 with an informational envelope)',
    EXIT_CONFIG_ERROR: 'Configuration error',
    EXIT_CROSS_BOUNDARY:
      'Surface and daemon on different OS instances, or cross-boundary mixed configuration (reserved / future emission)',
    EXIT_DAEMON_DOWN:
      'Daemon not running and embedded fallback unavailable (reserved / future emission)',
    EXIT_VERSION_MISMATCH:
      'CLI or hook protocol version mismatch with the daemon (reserved / future emission)',
    EXIT_DISCOVERY_FAILED: 'Runtime discovery failed (reserved / future emission)',
  };
  return meanings[name] ?? `${name} (${code})`;
}

function renderRules(registry) {
  const patterns = registry.patterns
    .filter((pattern) => pattern.enabled !== false)
    .sort((a, b) =>
      a.family === b.family ? a.id.localeCompare(b.id) : a.family.localeCompare(b.family)
    );
  const rows = patterns
    .map((pattern) => {
      const appliesTo = pattern.file_extensions ?? pattern.targets ?? [];
      return `| \`${pattern.id}\` | ${escapeCell(pattern.title)} | ${pattern.family} | ${pattern.severity} | ${appliesTo.join(', ')} |`;
    })
    .join('\n');
  return (
    generatedHeader(
      'rule-reference',
      'Compiled pattern catalogue',
      'Look up source-pattern rules compiled into anvil.'
    ) +
    `# Compiled pattern catalogue\n\n` +
    `These rules are the body of the \`antipattern-scan\` check, not the list of anvil checks. ` +
    `See the [check catalogue](checks.md) for every shipped check.\n\n` +
    `This catalogue covers source-pattern rules in the compiled registry shipped with ${releaseLabel()}. ` +
    `Secrets, architecture, policy, command-safety, and other gate checks have separate engines and are not listed here. ` +
    `The registry contains ` +
    `**${patterns.length} enabled rules across ${new Set(patterns.map((pattern) => pattern.family)).size} families**.\n\n` +
    `Rule IDs are stable identifiers you may see in terminal or machine-readable output. ` +
    `A warning describes a finding; it does not automatically mean a command failed.\n\n` +
    `| Rule | What it detects | Family | Default severity | Applies to |\n` +
    `| --- | --- | --- | --- | --- |\n${rows}\n`
  );
}

function renderChecks(definitions, initDefaultChecks, planlessChecks, surfaceFlags) {
  if (
    planlessChecks.length !== PLANLESS_CHECK_NAMES.length ||
    PLANLESS_CHECK_NAMES.some((name, index) => planlessChecks[index] !== name)
  ) {
    fail(`PLANLESS_ELIGIBLE_CHECKS drifted from generator pin: ${planlessChecks.join(', ')}`);
  }
  const planlessSet = new Set(planlessChecks);
  const initList = initDefaultChecks.map((name) => `\`${name}\``).join(', ');
  const planlessList = planlessChecks.map((name) => `\`${name}\``).join(' and ');
  const sections = definitions
    .map((definition) => renderCheckSection(definition, planlessSet, surfaceFlags))
    .join('\n');
  return (
    generatedHeader(
      'checks',
      'Check catalogue',
      'Look up every shipped anvil check, including the planless pair and flag-driven surfaces.'
    ) +
    `# Check catalogue\n\n` +
    `This catalogue is generated from the shipped check definitions. It is the complete engine list; ` +
    `[what anvil can do](what-anvil-can-do.md) stays a 12-row index.\n\n` +
    `- **Planless \`anvil check\` pair:** ${planlessList}. Every other engine is ignored by \`anvil check\`, even if it appears in \`checks:\`.\n` +
    `- **Init-default checks:** ${initList}.\n` +
    `- **Surface checks** (\`sql-migrations\`, \`github-actions\`, \`dockerfile\`, \`shell-scripts\`) are shipped-with-flag-status: default-on in \`anvil gate\`, not list-editable via \`checks:\`, and warn-only unless \`--fail-on-warnings\`.\n\n` +
    `Read [how anvil evaluates a project](../concepts/evaluation-model.md) for check versus scan versus gate.\n\n` +
    sections
  );
}

function renderCheckSection(definition, planlessSet, surfaceFlags) {
  const surface = surfaceFlags.get(definition.canonical_name);
  const aliases =
    definition.aliases.length > 0
      ? definition.aliases.map((alias) => `\`${alias}\``).join(', ')
      : 'none';
  const selection = surface
    ? `feature flag \`${surface.flag_id}\` (session opt-out \`${surface.session_opt_out}\`)`
    : '`.anvil` `checks:` list';
  const checkCommand = planlessSet.has(definition.canonical_name)
    ? 'runs'
    : '**ignored** (planless pair only)';
  const warnOnly = surface
    ? `Warn-only in \`anvil gate\` unless \`--fail-on-warnings\` or \`ANVIL_FAIL_ON_WARNINGS\`. Session opt-out: \`${surface.session_opt_out}\`.\n`
    : 'Follows the engine severity and gate thresholds.\n';
  const configure = surface
    ? `Surface checks cannot be enabled or disabled through the \`checks:\` list.\n`
    : `Select with top-level \`checks:\`, \`--only-checks\`, or \`--skip-checks\`.\n`;
  const related = ['- Model: [How anvil evaluates a project](../concepts/evaluation-model.md)'];
  if (definition.canonical_name === 'antipattern-scan') {
    related.push('- Rules body: [Compiled pattern catalogue](rules.md)');
  }
  if (definition.canonical_name === 'import-boundaries') {
    related.push('- Boundaries: [Architecture boundaries](../concepts/boundaries.md)');
  }
  if (definition.canonical_name === 'policy') {
    related.push('- Packs: [Policy model](../concepts/policy-model.md)');
    related.push('- Commands: [Policy command reference](policy.md)');
  }
  return (
    `## \`${definition.canonical_name}\`\n\n` +
    `${escapeCell(definition.description)}.\n\n` +
    `| Field | Value |\n| --- | --- |\n` +
    `| Stable ID | \`${definition.stable_id}\` |\n` +
    `| Canonical name | \`${definition.canonical_name}\` |\n` +
    `| Aliases | ${aliases} |\n` +
    `| Init enabled / visible | ${definition.init_enabled ? 'enabled' : 'not enabled'} / ${definition.init_visible ? 'visible' : 'hidden'} |\n` +
    `| Gate / gate-config | ${definition.gate_supported ? 'yes' : 'no'} / ${definition.gate_config_supported ? 'yes' : 'no'} |\n` +
    `| Selection | ${selection} |\n` +
    `| \`anvil check\` | ${checkCommand} |\n\n` +
    `### What it evaluates\n\n${escapeCell(definition.description)}.\n\n` +
    `### Findings / warn-only\n\n${warnOnly}\n` +
    `### Configure\n\n${configure}\n` +
    `### Related\n\n${related.join('\n')}\n`
  );
}

function renderSupport(languages, targets, ruleExtensions, clients) {
  const platformRows = targets
    .map((target) => `| ${platformName(target)} | \`${target}\` |`)
    .join('\n');
  const languageRows = languages
    .flatMap(({ name, extensions }) => {
      const covered = extensions.filter((extension) => ruleExtensions.has(extension));
      const structural = extensions.filter((extension) => !ruleExtensions.has(extension));
      return [
        covered.length > 0
          ? `| ${name} | ${covered.map((ext) => `\`.${ext}\``).join(', ')} | Compiled patterns available |`
          : undefined,
        structural.length > 0
          ? `| ${name} | ${structural.map((ext) => `\`.${ext}\``).join(', ')} | Parsing and structure only |`
          : undefined,
      ].filter(Boolean);
    })
    .join('\n');
  return (
    generatedHeader(
      'support-reference',
      'Supported platforms and languages',
      'Check where anvil runs and what it can parse.'
    ) +
    `# Supported platforms and languages\n\n` +
    `This page is generated from the release targets, parser mappings, and compiled rule registry shipped with ${releaseLabel()}. ` +
    `“Parsing and structure only” means anvil can build structural evidence for the language; it does not promise the same specialised rule depth as a language with compiled rules.\n\n` +
    `## Configured release targets\n\n` +
    `These targets are configured for release builds. Check the assets attached to your chosen ` +
    `[GitHub release](https://github.com/eddacraft/anvil/releases) before assuming that every target is present in every beta.\n\n` +
    `| Platform | Release target |\n| --- | --- |\n${platformRows}\n\n` +
    `## Language coverage\n\n| Language | File extensions | Current depth |\n| --- | --- | --- |\n${languageRows}\n\n` +
    `## AI clients\n\n` +
    `\`anvil start\` and \`anvil mcp install --client <id>\` configure supported AI clients for MCP-backed pre-write validation. ` +
    `The install registry currently includes **${clients.join('**, **')}** (${clients.length} clients). ` +
    `Interactive \`anvil start\` offers every registry client (unticked by default); scripted installs use \`anvil start --mcp-client <id>\`, \`anvil start --all-mcp-clients\`, or \`anvil mcp install --client <id>\`. ` +
    `Client ids and scope notes change with the binary — always run \`anvil mcp install --help\` on your install for the authoritative list. ` +
    `Other editors can use terminal checks and save-time watching; do not assume an editor extension is installed.\n`
  );
}

function generatedHeader(id, title, description) {
  // Governance frontmatter (ADR-119 D5 / DOCFRESH-005): each generated page
  // declares the generator inputs it renders plus the generator itself, and
  // is verified against the release its content is generated from.
  const governance = {
    'cli-reference': {
      owner: 'CLICT',
      sources: [
        inputs.cli,
        inputs.start,
        inputs.check,
        inputs.gate,
        inputs.config,
        inputs.watch,
        inputs.doctor,
        inputs.init,
        inputs.policy,
      ],
    },
    'rule-reference': { owner: 'DOCSYNC', sources: [inputs.registry] },
    checks: {
      owner: 'DOCDEF',
      sources: [inputs.checkCatalog, inputs.check, inputs.flagManifest],
    },
    'support-reference': {
      owner: 'DOCSYNC',
      sources: [inputs.languages, inputs.dist, inputs.clients, inputs.registry],
    },
  }[id];
  const lines = [`id: ${id}`, `title: ${title}`, `description: ${description}`];
  if (governance) {
    if (!release) {
      // Fail here rather than emitting a page check-public-docs.mjs will
      // reject later for a missing verified_against.
      fail(
        'cannot emit governance frontmatter: no public release found in docs/public/anvil/releases/changelog.md'
      );
    }
    lines.push(`owner: ${governance.owner}`, 'upstream:');
    for (const source of governance.sources) {
      lines.push(`  - ${relative(ROOT, source).split('\\').join('/')}`);
    }
    lines.push('  - scripts/docs/generate-anvil-public-reference.mjs');
    lines.push(`verified_against: ${release}`);
  }
  return (
    `---\n${lines.join('\n')}\n---\n\n` +
    `<!-- Generated from shipped product sources. Do not edit by hand. -->\n\n`
  );
}

function latestPublicRelease() {
  const changelog = resolve(ROOT, 'docs/public/anvil/releases/changelog.md');
  if (!existsSync(changelog)) return undefined;
  const match = /^##\s+([0-9]+\.[0-9]+\.[0-9]+(?:-[a-z0-9.-]+)?)(?:\s|$)/m.exec(
    readFileSync(changelog, 'utf8')
  );
  if (!match) fail('could not determine the latest public release from the changelog');
  return match[1];
}

function resolvesToCommit(ref, displayRef) {
  const result = spawnSync(
    'git',
    ['-C', ROOT, 'rev-parse', '--verify', '--quiet', `${ref}^{commit}`],
    {
      encoding: 'utf8',
    }
  );
  if (result.status === 0) return true;
  if (result.status === 1) return false;
  const detail = (result.stderr || result.stdout || `git exited ${result.status}`).trim();
  fail(`could not resolve public release ref ${displayRef}: ${detail}`);
}

function readProductSource(path) {
  if (!sourceRefResolved) return readFileSync(path, 'utf8');
  const repoPath = relative(ROOT, path).replaceAll('\\', '/');
  const result = spawnSync('git', ['-C', ROOT, 'show', `${sourceTagRef}:${repoPath}`], {
    encoding: 'utf8',
  });
  if (result.status === 0) {
    return result.stdout;
  }
  const detail = (result.stderr || result.stdout || `git show exited ${result.status}`).trim();
  fail(`could not read ${repoPath} from resolved public release ref ${sourceTag}: ${detail}`);
}

function releaseLabel() {
  return release ? `anvil ${release}` : 'this anvil source tree';
}

function languageName(variant) {
  const names = {
    TypeScript: 'TypeScript',
    Tsx: 'TypeScript JSX',
    JavaScript: 'JavaScript',
    Jsx: 'JavaScript JSX',
    CSharp: 'C#',
    Cpp: 'C++',
    Wat: 'WebAssembly text',
  };
  return names[variant] ?? variant;
}

function publicCommandDescription(name, sourceDescription) {
  const explanations = {
    admin: 'Manage service approvals and users (administrators only)',
    'audit-chain': 'Check commits that bypassed protection for missing evidence',
    baseline: 'Manage the record of findings accepted when anvil was introduced',
    capsule: 'Package review evidence for a commit range into a portable file',
    dashboard: 'Open a native read-only dashboard over local anvil state (flag-gated)',
    edda: 'Inspect durable local memory records used by eddacraft workflows',
    ember: 'Inspect proposed memory records before they become durable records',
    exception: 'Manage recorded policy exceptions',
    'gate-config': 'Set which checks and thresholds a gate uses',
    gctx: 'Control whether graph-context snippets may leave the local machine',
    hook: 'Run Git-hook operations; normally invoked by anvil-managed hooks',
    intercept: 'Manage the local process that protects supported AI-assisted writes',
    kindling: 'Inspect the local command-usage record used for activity insights',
    'l4-validate': 'Validate a commit range against policy in continuous integration',
    mcp: 'Manage Model Context Protocol (MCP) connections for supported AI clients',
    'mcp-config': 'Print MCP configuration for a supported AI client',
    plan: 'Inspect planning files written in APS, a Markdown-based plan format',
    validate: 'Validate a planning file written in APS format',
    workspace: 'Control which project folders the local protection process may access',
  };
  return explanations[name] ?? sourceDescription;
}

function platformName(target) {
  if (target.includes('apple-darwin'))
    return target.startsWith('aarch64') ? 'macOS (Apple silicon)' : 'macOS (Intel)';
  if (target.includes('windows'))
    return target.startsWith('aarch64') ? 'Windows (Arm64)' : 'Windows (x64)';
  if (target.includes('linux'))
    return target.startsWith('aarch64') ? 'Linux (Arm64)' : 'Linux (x64)';
  return target;
}

function kebabCase(value) {
  return value.replace(/([a-z0-9])([A-Z])/g, '$1-$2').toLowerCase();
}

function lowerBrands(value) {
  return value
    .replace(/\bAnvil\b/g, 'anvil')
    .replace(/\bEddaCraft\b/g, 'eddacraft')
    .replace(/\bKindling\b/g, 'kindling');
}

function formatMarkdown(path, content) {
  const formatter = fileURLToPath(new URL('../../node_modules/.bin/oxfmt', import.meta.url));
  if (!existsSync(formatter)) return content;
  const result = spawnSync(formatter, [`--stdin-filepath=${path}`], {
    cwd: ROOT,
    input: content,
    encoding: 'utf8',
  });
  if (result.status !== 0 || !result.stdout) {
    fail(
      `could not format generated reference: ${(result.stderr || 'unknown formatter error').trim()}`
    );
  }
  return result.stdout;
}

function escapeCell(value) {
  return lowerBrands(String(value))
    .replace(/\\/g, '\\\\')
    .replace(/\|/g, '\\|')
    .replace(/\s+/g, ' ')
    .trim();
}

function fail(message) {
  process.stderr.write(`[anvil-reference] ${message}\n`);
  process.exit(1);
}
