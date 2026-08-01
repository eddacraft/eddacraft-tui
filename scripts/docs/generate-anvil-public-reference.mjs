#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
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

const inputs = {
  registry: resolve(ROOT, 'patterns/compiled/registry.json'),
  cli: resolve(ROOT, 'crates/anvil-cli/src/main.rs'),
  clients: resolve(ROOT, 'crates/anvil-cli/src/activation/diagnostic.rs'),
  languages: resolve(ROOT, 'crates/anvil-kernel/src/parser/languages.rs'),
  dist: resolve(ROOT, 'dist-workspace.toml'),
};

for (const [name, path] of Object.entries(inputs)) {
  if (!existsSync(path)) fail(`missing ${name} source: ${path}`);
}

const release = latestPublicRelease();
const sourceRef = release ? `v${release}` : undefined;
const registry = JSON.parse(readProductSource(inputs.registry));
const commands = parseCommands(readProductSource(inputs.cli));
const clients = parseClients(readProductSource(inputs.clients));
const languages = parseLanguages(readProductSource(inputs.languages));
const targets = parseTargets(readProductSource(inputs.dist));
const ruleExtensions = new Set(
  registry.patterns.flatMap((pattern) => pattern.file_extensions ?? []).map((ext) => ext.slice(1))
);

const rendered = new Map([
  [resolve(ROOT, 'docs/public/anvil/reference/cli.md'), renderCli(commands)],
  [resolve(ROOT, 'docs/public/anvil/reference/rules.md'), renderRules(registry)],
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

if (CHECK) {
  if (stale > 0) fail(`${stale} generated reference file(s) need regeneration`);
  process.stdout.write(`[anvil-reference] 3 generated reference files are current\n`);
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
  const block = /pub enum McpClientId\s*\{([\s\S]*?)\n\}/.exec(source)?.[1];
  if (!block) fail('could not locate supported MCP clients');
  const clients = [...block.matchAll(/^\s*([A-Z][A-Za-z0-9]*),\s*$/gm)].map((match) => match[1]);
  if (clients.length === 0) fail('supported MCP client list is empty');
  return clients.map((client) => (client === 'ClaudeCode' ? 'Claude Code' : client));
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

function renderCli(commands) {
  const rows = commands
    .map(({ name, description }) => `| \`anvil ${name}\` | ${escapeCell(description)} |`)
    .join('\n');
  return (
    generatedHeader(
      'cli-reference',
      'CLI command reference',
      'Discover every public top-level anvil command.'
    ) +
    `# CLI command reference\n\n` +
    `This page is generated from the command definitions shipped with ${releaseLabel()}. ` +
    `Use \`anvil <command> --help\` for flags, examples, and subcommands for your installed version.\n\n` +
    `For a first installation, use the [quickstart](../quickstart.md).\n\n` +
    `## Daily ensure\n\n` +
    `With no subcommand, bare \`anvil\` runs the daily ensure surface: it turns protection on for an already-activated project (daemon + existing MCP entries). ` +
    `It does not install clients you skipped or rewrite configuration — use \`anvil start\` to activate or reconfigure.\n\n` +
    `| Command | Purpose |\n| --- | --- |\n` +
    `| \`anvil\` | Turn protection on for an already-activated project (daily ensure) |\n` +
    `${rows}\n`
  );
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
    `\`anvil start\` and \`anvil mcp install --client\` configure supported AI clients for pre-write validation. ` +
    `This public release documents **${clients.join('** and **')}** on the protection ladder; newer betas expand the install registry — run \`anvil mcp install --help\` on your binary for the full list. ` +
    `Other editors can use terminal checks and save-time watching; do not assume an editor extension is installed.\n`
  );
}

function generatedHeader(id, title, description) {
  return (
    `---\nid: ${id}\ntitle: ${title}\ndescription: ${description}\n---\n\n` +
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

function readProductSource(path) {
  if (!sourceRef) return readFileSync(path, 'utf8');
  const repoPath = relative(ROOT, path).replaceAll('\\', '/');
  const result = spawnSync('git', ['-C', ROOT, 'show', `${sourceRef}:${repoPath}`], {
    encoding: 'utf8',
  });
  if (result.status !== 0) {
    fail(`could not read ${repoPath} from the public release tag ${sourceRef}`);
  }
  return result.stdout;
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
