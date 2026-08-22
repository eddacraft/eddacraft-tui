import { execFile } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import process from 'node:process';
import { parseArgs, promisify } from 'node:util';
import { pathToFileURL } from 'node:url';

import { parseDocGovernance, ParseError } from '@eddacraft/anvil-docs-meta';
import globby from 'globby';

const DIAGRAM_DOCUMENT_PATTERNS = [
  'docs/**/*.md',
  'plans/specs/**/*.md',
  'apps/**/{README,ARCHITECTURE}.md',
  'crates/**/{README,ARCHITECTURE}.md',
  'packages/**/{README,ARCHITECTURE}.md',
  '!docs/**/archive/**',
  '!docs/public/**',
  '!docs/**/*.template.md',
];

const execFileAsync = promisify(execFile);
const MERMAID_RENDERER_VERSION = '11.16.0';
const TRUSTED_SANDBOX_PROBE = 'flowchart LR\n  SandboxProbeStart --> SandboxProbeEnd\n';
const TRUSTED_SANDBOX_LAUNCH_FAILURE =
  /^\[\d{4}\/\d{6}\.\d+:FATAL:content\/browser\/zygote_host\/zygote_host_impl_linux\.cc:\d+\] No usable sandbox! If you are running on Ubuntu 23\.10\+ or another Linux distro that has disabled unprivileged user namespaces with AppArmor, see https:\/\/chromium\.googlesource\.com\/chromium\/src\/\+\/main\/docs\/security\/apparmor-userns-restrictions\.md\. Otherwise see https:\/\/chromium\.googlesource\.com\/chromium\/src\/\+\/main\/docs\/linux\/suid_sandbox_development\.md for more information on developing with the \(older\) SUID sandbox\. If you want to live dangerously and need an immediate workaround, you can try using --no-sandbox\.$/mu;

function tableCells(line) {
  return line
    .split('|')
    .slice(1, -1)
    .map((cell) => cell.trim());
}

function parseFallbackGovernance(content, documentPath) {
  const rows = content.split(/\r?\n/u);
  const metadataHeader = rows.findIndex((row) =>
    /^\|\s*Type\s*\|\s*Authority\s*\|\s*Owner\s*\|\s*Status\s*\|\s*Freshness\s*\|$/u.test(row)
  );
  const relationsHeader = rows.findIndex((row) =>
    /^\|\s*Upstream\s*\|\s*Downstream\s*\|$/u.test(row)
  );
  if (metadataHeader === -1 || relationsHeader === -1) return null;

  const metadata = tableCells(rows[metadataHeader + 2] ?? '');
  const type = metadata[0];
  const isComponentArchitecture = type === 'Architecture';
  const isPlanSpec = type === 'Spec' && documentPath.startsWith('plans/specs/');
  if (!isComponentArchitecture && !isPlanSpec) return null;
  const upstreamCell = tableCells(rows[relationsHeader + 2] ?? '')[0] ?? '';

  return {
    status: metadata[3],
    upstreams: [...upstreamCell.matchAll(/`([^`]+)`/gu)].map((match) => match[1]),
  };
}

function normaliseRepoPath(value) {
  if (typeof value !== 'string' || value.length === 0 || path.posix.isAbsolute(value)) {
    return null;
  }

  const normalised = path.posix.normalize(value.replaceAll('\\', '/'));
  if (
    normalised === '.' ||
    normalised === '..' ||
    normalised.startsWith('../') ||
    normalised.includes('/../')
  ) {
    return null;
  }

  return normalised.replace(/^\.\//, '').replace(/\/$/, '');
}

function globToRegExp(pattern) {
  let expression = '^';

  for (let index = 0; index < pattern.length; index += 1) {
    const character = pattern[index];

    if (character === '*' && pattern[index + 1] === '*') {
      if (pattern[index + 2] === '/') {
        expression += '(?:.*/)?';
        index += 2;
      } else {
        expression += '.*';
        index += 1;
      }
      continue;
    }

    if (character === '*') {
      expression += '[^/]*';
      continue;
    }

    if (character === '?') {
      expression += '[^/]';
      continue;
    }

    expression += character.replace(/[|\\{}()[\]^$+?.]/g, '\\$&');
  }

  return new RegExp(`${expression}$`, 'u');
}

export function extractMermaidBlocks(content, documentPath) {
  const blocks = [];
  const fence = /^```mermaid[ \t]*\r?\n([\s\S]*?)^```[ \t]*$/gm;
  let match;

  while ((match = fence.exec(content)) !== null) {
    blocks.push({
      path: documentPath,
      index: blocks.length + 1,
      line: content.slice(0, match.index).split('\n').length,
      source: match[1].replaceAll('\r\n', '\n').replace(/\n$/, ''),
    });
  }

  return blocks;
}

export async function discoverDiagramDocuments({ root }) {
  const files = await globby(DIAGRAM_DOCUMENT_PATTERNS, { cwd: root, gitignore: true });
  const documents = [];

  for (const file of files.sort()) {
    const content = await readFile(path.resolve(root, file), 'utf8');
    let governance;
    let status;
    let upstreams;

    try {
      governance = parseDocGovernance(content, file);
      status = governance.metadata.status;
      upstreams = governance.sourceReferences
        .filter((reference) => reference.context === 'upstream')
        .map((reference) => reference.path);
    } catch (error) {
      if (error instanceof ParseError) {
        const fallback = parseFallbackGovernance(content, file);
        if (fallback === null) continue;
        ({ status, upstreams } = fallback);
      } else {
        throw error;
      }
    }

    if (status === 'Archived' || extractMermaidBlocks(content, file).length === 0) {
      continue;
    }

    documents.push({ path: file, upstreams: [...new Set(upstreams)], content });
  }

  return documents;
}

function rendererError(error) {
  return error?.stderr?.trim() || error?.stdout?.trim() || error?.message || String(error);
}

export async function verifyMermaidVersion({ root, execute = execFileAsync }) {
  const command = path.resolve(root, 'node_modules/.bin/mmdc');
  let result;

  try {
    result = await execute(command, ['--version']);
  } catch (error) {
    throw new Error(`Mermaid CLI version check failed: ${rendererError(error)}`, {
      cause: error,
    });
  }

  const stdout = typeof result?.stdout === 'string' ? result.stdout : '';
  const stderr = typeof result?.stderr === 'string' ? result.stderr : '';
  if (!/^11\.16\.0(?:\r?\n)?$/u.test(stdout) || stderr !== '') {
    throw new Error(`Mermaid CLI version check expected stdout-only ${MERMAID_RENDERER_VERSION}`);
  }

  return MERMAID_RENDERER_VERSION;
}

function isTrustedSandboxLaunchFailure(error) {
  const stderr = typeof error?.stderr === 'string' ? error.stderr : '';
  return TRUSTED_SANDBOX_LAUNCH_FAILURE.test(stderr);
}

async function detectMermaidSandboxMode({ command, execute }) {
  const directory = await mkdtemp(path.join(tmpdir(), 'anvil-mermaid-probe-'));
  const input = path.join(directory, 'probe.mmd');
  const output = path.join(directory, 'probe.svg');

  try {
    await writeFile(input, TRUSTED_SANDBOX_PROBE, 'utf8');
    try {
      await execute(command, ['--input', input, '--output', output, '--quiet']);
      return 'sandboxed';
    } catch (error) {
      if (isTrustedSandboxLaunchFailure(error)) {
        return 'conditional-no-sandbox-fallback';
      }
      throw new Error(rendererError(error), { cause: error });
    }
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

export function createMermaidRenderer({
  root,
  execute = execFileAsync,
  onFallback = () => {},
  onMode = () => {},
}) {
  const command = path.resolve(root, 'node_modules/.bin/mmdc');
  let sandboxModePromise;

  return async (block) => {
    sandboxModePromise ??= detectMermaidSandboxMode({ command, execute }).then((mode) => {
      onMode(mode);
      return mode;
    });
    const sandboxMode = await sandboxModePromise;
    const directory = await mkdtemp(path.join(tmpdir(), 'anvil-mermaid-'));
    const input = path.join(directory, 'diagram.mmd');
    const output = path.join(directory, 'diagram.svg');
    const args = ['--input', input, '--output', output, '--quiet'];

    try {
      await writeFile(input, `${block.source}\n`, 'utf8');
      if (sandboxMode === 'conditional-no-sandbox-fallback') {
        const config = path.join(directory, 'puppeteer.json');
        await writeFile(
          config,
          JSON.stringify({ args: ['--no-sandbox', '--disable-setuid-sandbox'] }),
          'utf8'
        );
        onFallback(block);
        args.push('--puppeteerConfigFile', config);
      }

      try {
        await execute(command, args);
      } catch (error) {
        throw new Error(rendererError(error), { cause: error });
      }
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  };
}

export async function validateMermaidBlocks({ blocks, render }) {
  const diagnostics = [];

  for (const block of blocks) {
    try {
      await render(block);
    } catch (error) {
      diagnostics.push({
        code: 'mermaid-render-failed',
        path: block.path,
        index: block.index,
        line: block.line,
        message: error instanceof Error ? error.message : String(error),
      });
    }
  }

  return diagnostics;
}

export async function checkDiagramImpact({ documents, changedPaths, render }) {
  const findings =
    changedPaths === undefined ? [] : classifyDiagramImpact({ documents, changedPaths });
  const affected = documents.filter((document) => {
    if (changedPaths === undefined) return true;
    const documentPath = normaliseRepoPath(document.path);

    return changedPaths.some((changedPath) => {
      const changed = normaliseRepoPath(changedPath);
      return (
        changed !== null &&
        (changed === documentPath ||
          document.upstreams.some((upstream) => matchesUpstream(changed, upstream)))
      );
    });
  });
  const blocks = affected.flatMap((document) =>
    extractMermaidBlocks(document.content, document.path)
  );

  return [...findings, ...(await validateMermaidBlocks({ blocks, render }))];
}

export function matchesUpstream(changedPath, upstream) {
  const changed = normaliseRepoPath(changedPath);
  const declared = normaliseRepoPath(upstream);
  if (changed === null || declared === null) {
    return false;
  }

  if (/[*?]/u.test(declared)) {
    return globToRegExp(declared).test(changed);
  }

  return changed === declared || changed.startsWith(`${declared}/`);
}

export function classifyDiagramImpact({ documents, changedPaths }) {
  const changed = changedPaths
    .map((changedPath) => normaliseRepoPath(changedPath))
    .filter((changedPath) => changedPath !== null);

  return documents.flatMap((document) => {
    if (
      extractMermaidBlocks(document.content, document.path).length === 0 ||
      changed.includes(normaliseRepoPath(document.path))
    ) {
      return [];
    }

    const upstream = document.upstreams.find((candidate) =>
      changed.some((changedPath) => matchesUpstream(changedPath, candidate))
    );

    return upstream === undefined
      ? []
      : [{ code: 'diagram-review-owed', path: document.path, upstream }];
  });
}

function lines(content) {
  return content
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter(Boolean);
}

function findingMessage(finding) {
  if (finding.code === 'diagram-review-owed') {
    return `declared upstream changed without owning diagram update: ${finding.upstream}`;
  }
  return finding.message;
}

export async function runDiagramImpactCli(
  argv = process.argv.slice(2),
  { executeVersion = execFileAsync, executeRenderer = execFileAsync } = {}
) {
  const { values } = parseArgs({
    args: argv,
    options: {
      root: { type: 'string' },
      since: { type: 'string' },
      'paths-file': { type: 'string' },
      json: { type: 'boolean', default: false },
    },
    allowPositionals: false,
  });
  const root = path.resolve(values.root ?? process.cwd());
  if (values.since && values['paths-file']) {
    throw new Error('choose only one of --since or --paths-file');
  }

  let changedPaths;
  if (values['paths-file']) {
    changedPaths = lines(await readFile(path.resolve(root, values['paths-file']), 'utf8'));
  } else if (values.since) {
    const result = await execFileAsync(
      'git',
      ['diff', '--name-only', '--diff-filter=ACDMR', `${values.since}...HEAD`],
      { cwd: root }
    );
    changedPaths = lines(result.stdout);
  }

  const rendererVersion = await verifyMermaidVersion({ root, execute: executeVersion });
  const documents = await discoverDiagramDocuments({ root });
  let rendererMode = 'not-probed';
  let rendered = 0;
  const renderer = createMermaidRenderer({
    root,
    execute: executeRenderer,
    onMode: (mode) => {
      rendererMode = mode;
    },
  });
  const findings = await checkDiagramImpact({
    documents,
    changedPaths,
    render: async (block) => {
      await renderer(block);
      rendered += 1;
    },
  });
  const summary = {
    errors: findings.length,
    documentsChecked: documents.length,
    fencesRendered: rendered,
    rendererVersion,
    rendererMode,
    mode: changedPaths === undefined ? 'corpus' : 'diff',
  };

  if (values.json) {
    process.stdout.write(
      `${JSON.stringify({ surface: 'diagram-impact', findings, summary }, null, 2)}\n`
    );
  } else {
    for (const finding of findings) {
      process.stdout.write(
        `[diagram-impact] ERROR: ${finding.path}:${finding.line ?? 1} — ${findingMessage(finding)}\n`
      );
    }
    process.stdout.write(
      `[diagram-impact] summary [${summary.mode}]: ${summary.errors} errors, ` +
        `${summary.documentsChecked} documents checked, ${summary.fencesRendered} fences rendered, ` +
        `Mermaid ${summary.rendererVersion} (${summary.rendererMode})\n`
    );
  }

  return findings.length > 0 ? 1 : 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  runDiagramImpactCli().then(
    (code) => {
      process.exitCode = code;
    },
    (error) => {
      process.stderr.write(`[diagram-impact] cannot run: ${error.message}\n`);
      process.exitCode = 2;
    }
  );
}
