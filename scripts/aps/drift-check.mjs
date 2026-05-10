#!/usr/bin/env node
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { basename, join, relative } from 'node:path';

const args = process.argv.slice(2);

function usage() {
  console.log(`Usage: scripts/aps/drift-check.mjs [--root PATH] [--changed-files PATH] [--release-record PATH] [--json]

Warning-mode APS/repo/release drift checks. Findings are advisory and exit 0.`);
}

let root = process.cwd();
let changedFilesPath = '';
let releaseRecordPath = '';
let json = false;

for (let i = 0; i < args.length; i += 1) {
  const arg = args[i];
  if (arg === '--help' || arg === '-h') {
    usage();
    process.exit(0);
  }
  if (arg === '--json') {
    json = true;
    continue;
  }
  if (arg === '--root' || arg === '--changed-files' || arg === '--release-record') {
    const value = args[i + 1];
    if (!value || value.startsWith('--')) {
      console.error(`drift-check.mjs: ${arg} requires a value`);
      process.exit(2);
    }
    if (arg === '--root') root = value;
    if (arg === '--changed-files') changedFilesPath = value;
    if (arg === '--release-record') releaseRecordPath = value;
    i += 1;
    continue;
  }
  console.error(`drift-check.mjs: unknown argument: ${arg}`);
  process.exit(2);
}

const findings = [];

function addFinding(code, message, details = {}) {
  findings.push({ severity: 'warning', code, message, ...details });
}

function readText(path) {
  return readFileSync(path, 'utf8');
}

function readJson(path) {
  return JSON.parse(readText(path));
}

function listModulePaths() {
  const modulesDir = join(root, 'plans/modules');
  if (!existsSync(modulesDir)) return [];
  return readdirSync(modulesDir)
    .filter((name) => name.endsWith('.aps.md'))
    .map((name) => join(modulesDir, name));
}

function normalisePath(path) {
  return path.replaceAll('\\', '/').replace(/^\.\//, '');
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function extractModule(path) {
  const text = readText(path);
  const id = text.match(/^\|\s*([A-Z][A-Z0-9-]*)\s*\|.*?\|\s*(\d+)\/(\d+)\s*\|\s*$/m);
  const slug = basename(path, '.aps.md');
  const items = [];
  const headingPattern = /^###\s+([A-Z][A-Z0-9]*-\d{3})(?::|\s+[—-])\s+(.+)$/gm;
  const headings = [...text.matchAll(headingPattern)];

  headings.forEach((heading, index) => {
    const start = heading.index ?? 0;
    const end =
      index + 1 < headings.length ? (headings[index + 1].index ?? text.length) : text.length;
    const block = text.slice(start, end);
    const status = block.match(/^- \*\*Status:\*\*\s+(.+)$/m)?.[1]?.trim() ?? 'Unknown';
    const filesBlock = block.match(/^- \*\*Files:\*\*\s+([^\n]+(?:\n\s{2,}[^\n]+)*)/m)?.[1] ?? '';
    const files = filesBlock
      .split(/,|\n/)
      .map((entry) =>
        entry
          .replace(/`/g, '')
          .replace(/\s+when implemented$/, '')
          .trim()
      )
      .filter(Boolean)
      .map(normalisePath);
    items.push({
      id: heading[1],
      title: heading[2].trim(),
      status,
      block,
      files,
      moduleSlug: slug,
    });
  });

  return {
    id: id?.[1] ?? slug.toUpperCase(),
    progressDone: id ? Number(id[2]) : null,
    progressTotal: id ? Number(id[3]) : null,
    path,
    items,
  };
}

const modules = listModulePaths().map(extractModule);
const items = modules.flatMap((module) => module.items);

for (const module of modules) {
  if (module.progressDone === null || module.progressTotal === null) continue;
  if (module.items.length === 0) continue;
  const completeCount = module.items.filter((item) => item.status === 'Complete').length;
  if (completeCount !== module.progressDone || module.items.length !== module.progressTotal) {
    addFinding(
      'aps-progress-mismatch',
      `${relative(root, module.path)} progress is ${module.progressDone}/${module.progressTotal}, but tasks count as ${completeCount}/${module.items.length}.`,
      {
        module: module.id,
      }
    );
  }
}

const indexPath = join(root, 'plans/index.aps.md');
if (existsSync(indexPath)) {
  const indexText = readText(indexPath);
  for (const module of modules) {
    const indexProgress = indexText.match(
      new RegExp(`\\b${escapeRegExp(module.id)}\\b[^\\n]*?(\\d+)\\/(\\d+)`)
    );
    if (!indexProgress || module.progressDone === null || module.progressTotal === null) continue;
    const done = Number(indexProgress[1]);
    const total = Number(indexProgress[2]);
    if (done !== module.progressDone || total !== module.progressTotal) {
      addFinding(
        'aps-index-progress-mismatch',
        `plans/index.aps.md lists ${module.id} ${done}/${total}, but the module header is ${module.progressDone}/${module.progressTotal}.`,
        {
          module: module.id,
        }
      );
    }
  }
}

for (const item of items.filter((entry) => entry.status === 'Complete')) {
  if (!/^- \*\*Validation:\*\*/m.test(item.block) || !/validation\s+passed/i.test(item.block)) {
    addFinding(
      'aps-complete-without-validation-evidence',
      `${item.id} is Complete without explicit validation evidence.`,
      {
        apsItem: item.id,
      }
    );
  }
}

if (changedFilesPath) {
  const changedFiles = readText(changedFilesPath)
    .split(/\r?\n/)
    .map((entry) => normalisePath(entry.trim()))
    .filter(Boolean);
  const trackedPatterns = items.flatMap((item) =>
    item.files.map((file) => ({ file, item: item.id }))
  );
  for (const file of changedFiles) {
    if (file.startsWith('plans/') || file.startsWith('.changeset/')) continue;
    const referenced = trackedPatterns.some(({ file: pattern }) => {
      if (pattern.endsWith('/**')) return file.startsWith(pattern.slice(0, -3));
      if (pattern.endsWith('/')) return file.startsWith(pattern);
      if (pattern.includes('<date>')) return file.startsWith(pattern.split('<date>')[0]);
      return file === pattern || file.startsWith(`${pattern}/`);
    });
    if (!referenced) {
      addFinding(
        'changed-file-without-aps-reference',
        `${file} is changed but no active APS Files field references it.`,
        {
          path: file,
        }
      );
    }
  }
}

let releaseRecord = null;
if (releaseRecordPath) {
  releaseRecord = readJson(releaseRecordPath);
  if (
    releaseRecord.version &&
    releaseRecord.source?.tag &&
    releaseRecord.version !== releaseRecord.source.tag
  ) {
    addFinding(
      'release-version-tag-mismatch',
      `Release record version ${releaseRecord.version} does not match source tag ${releaseRecord.source.tag}.`
    );
  }
  if (releaseRecord.source?.tag) {
    const packageJsonPath = join(root, 'package.json');
    if (existsSync(packageJsonPath)) {
      const packageVersion = readJson(packageJsonPath).version;
      if (packageVersion && `v${packageVersion}` !== releaseRecord.source.tag) {
        addFinding(
          'package-version-tag-mismatch',
          `package.json version v${packageVersion} does not match release tag ${releaseRecord.source.tag}.`
        );
      }
    }
  }
  if (releaseRecord.lifecycleState === 'published') {
    const artifacts = Array.isArray(releaseRecord.artifacts) ? releaseRecord.artifacts : [];
    if (artifacts.length === 0) {
      addFinding('release-missing-artifacts', 'Published release record has no artifacts.');
    }
    artifacts.forEach((artifact) => {
      if (!artifact.name || !artifact.url || (!artifact.sha256 && !artifact.integrityRef)) {
        addFinding(
          'release-artifact-missing-integrity',
          `Published artifact ${artifact.name ?? '<unnamed>'} is missing required location or integrity metadata.`
        );
      }
    });
  }
  if (releaseRecord.lifecycleState === 'candidate') {
    const recordItems = new Set((releaseRecord.aps?.items ?? []).map((item) => item.id));
    for (const item of items.filter((entry) => entry.status === 'Merged')) {
      if (!recordItems.has(item.id)) {
        addFinding(
          'candidate-missing-merged-aps-item',
          `Candidate release record does not include merged APS item ${item.id}.`,
          {
            apsItem: item.id,
          }
        );
      }
    }
  }
}

const publishedItems = new Set(
  releaseRecord?.lifecycleState === 'published'
    ? (releaseRecord.aps?.items ?? []).map((item) => item.id)
    : []
);
for (const item of items.filter((entry) => entry.status === 'Released/Shipped')) {
  if (!publishedItems.has(item.id)) {
    addFinding(
      'shipped-aps-without-release-record',
      `${item.id} is Released/Shipped without a matching published release record item.`,
      {
        apsItem: item.id,
      }
    );
  }
}

const result = { advisory: true, enforcement: 'none', findingCount: findings.length, findings };

if (json) {
  console.log(JSON.stringify(result));
} else {
  console.log('APS drift check');
  console.log('Advisory: true (enforcement: none)');
  if (findings.length === 0) {
    console.log('No drift warnings detected.');
  } else {
    for (const finding of findings) {
      console.log(`- [${finding.code}] ${finding.message}`);
    }
  }
}
