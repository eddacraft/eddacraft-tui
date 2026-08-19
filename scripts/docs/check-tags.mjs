#!/usr/bin/env node
// Surface validator: APS tag vocabulary.
//
// Validates that every `**Tags:** ...` line inside plans/**/*.aps.md (live and
// archived) uses tags that exist in docs/governance/tags-catalogue.md and have
// a valid kebab-case syntax. Emits findings against the orchestrator's
// labelled-output contract. The catalogue is parsed live at runtime so a new
// tag is recognised the moment it's added to the catalogue file.

import { readFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { parseArgs } from 'node:util';
import globby from 'globby';
import { unified } from 'unified';
import remarkParse from 'remark-parse';
import remarkGfm from 'remark-gfm';
import { visit } from 'unist-util-visit';

const SURFACE = 'tags';
const TAG_PATTERN = /^[a-z0-9]+(-[a-z0-9]+)*$/;
const CATALOGUE_REL = 'docs/governance/tags-catalogue.md';

const { values } = parseArgs({
  options: {
    root: { type: 'string' },
    baseline: { type: 'string' },
    'no-baseline': { type: 'boolean', default: false },
    json: { type: 'boolean', default: false },
  },
  allowPositionals: false,
});

const root = resolve(values.root ?? process.cwd());
const baselinePath = resolve(root, values.baseline ?? 'docs/governance/docs-check.baseline.json');

let baseline = {};
if (!values['no-baseline'] && existsSync(baselinePath)) {
  try {
    const raw = await readFile(baselinePath, 'utf8');
    baseline = JSON.parse(raw)[SURFACE] ?? {};
  } catch (err) {
    process.stderr.write(`[${SURFACE}] failed to read baseline ${baselinePath}: ${err.message}\n`);
  }
}

// Parse the catalogue: collect every inline-code value found inside section
// tables under the `## Catalogue` heading. Each table row's first cell holds
// the approved tag as inline code; we treat any inlineCode token within those
// tables as an approved tag so future column reordering doesn't break us.
const cataloguePath = resolve(root, CATALOGUE_REL);
const approved = new Set();
try {
  const content = await readFile(cataloguePath, 'utf8');
  const ast = unified().use(remarkParse).use(remarkGfm).parse(content);

  let inCatalogue = false;
  for (const node of ast.children) {
    if (node.type === 'heading' && node.depth === 2) {
      const text = node.children
        .map((c) => (c.type === 'text' ? c.value : ''))
        .join('')
        .trim();
      inCatalogue = text.toLowerCase() === 'catalogue';
      continue;
    }
    if (!inCatalogue) continue;
    if (node.type !== 'table') continue;
    // Skip the header row (index 0); collect inline-code from data rows.
    for (let i = 1; i < node.children.length; i += 1) {
      visit(node.children[i], 'inlineCode', (codeNode) => {
        const tag = codeNode.value.trim();
        if (tag) approved.add(tag);
      });
    }
  }
} catch (err) {
  process.stdout.write(
    `[${SURFACE}] ERROR: ${CATALOGUE_REL}:1 — catalogue not found or unparseable: ${err.message}\n`
  );
  process.stdout.write(`[${SURFACE}] summary: 1 errors, 0 warnings, 0 files checked\n`);
  process.exit(1);
}

if (approved.size === 0) {
  process.stdout.write(
    `[${SURFACE}] ERROR: ${CATALOGUE_REL}:1 — catalogue contained zero approved tags; expected at least one inline-code tag under '## Catalogue'\n`
  );
  process.stdout.write(`[${SURFACE}] summary: 1 errors, 0 warnings, 0 files checked\n`);
  process.exit(1);
}

// Discover every APS file — live plans + archived modules. Tags travel with
// archived modules, so archived files must still validate cleanly.
const apsFiles = await globby(['plans/**/*.aps.md'], { cwd: root, gitignore: true });
apsFiles.sort();

const findings = [];

// One regex captures both bullet (`- **Tags:**`) and bare (`**Tags:**`) forms.
const TAG_LINE_RE = /^(?:- )?\*\*Tags:\*\*\s*(.+)$/;

await Promise.all(
  apsFiles.map(async (relPath) => {
    const absPath = resolve(root, relPath);
    let content;
    try {
      content = await readFile(absPath, 'utf8');
    } catch (err) {
      process.stderr.write(`[${SURFACE}] failed to read ${relPath}: ${err.message}\n`);
      findings.push({
        severity: 'ERROR',
        file: relPath,
        line: 1,
        message: `failed to read file: ${err.message}`,
      });
      return;
    }
    const lines = content.split('\n');
    for (let i = 0; i < lines.length; i += 1) {
      const match = lines[i].match(TAG_LINE_RE);
      if (!match) continue;
      const lineNumber = i + 1;
      const tags = match[1].split(',').map((t) => stripInlineCode(t.trim()));
      for (const tag of tags) {
        if (tag.length === 0) {
          findings.push({
            severity: 'ERROR',
            file: relPath,
            line: lineNumber,
            message: 'malformed tag "" (empty value in tag list)',
          });
          continue;
        }
        if (!TAG_PATTERN.test(tag)) {
          findings.push({
            severity: 'ERROR',
            file: relPath,
            line: lineNumber,
            message: `malformed tag "${tag}" (expected lowercase kebab-case, e.g. "agent" or "cross-platform")`,
          });
          continue;
        }
        if (!approved.has(tag)) {
          findings.push({
            severity: 'WARN',
            file: relPath,
            line: lineNumber,
            message: `unknown tag "${tag}"; add it to docs/governance/tags-catalogue.md or use an existing one`,
          });
        }
      }
    }
  })
);

// Apply baseline as a consumable multiset. Baseline regeneration stores one
// message per accepted occurrence; consuming that entry ensures a second
// identical finding in the same file remains an error instead of inheriting
// the first occurrence's allowance.
findings.sort((a, b) => a.file.localeCompare(b.file) || a.line - b.line);
const remainingBaseline = new Map(
  Object.entries(baseline)
    .filter(([, messages]) => Array.isArray(messages))
    .map(([file, messages]) => [file, [...messages]])
);
for (const finding of findings) {
  const fingerprints = remainingBaseline.get(finding.file);
  const index = fingerprints?.indexOf(finding.message) ?? -1;
  if (index >= 0) {
    fingerprints.splice(index, 1);
    finding.severity = 'WARN';
    finding.message = `[baselined] ${finding.message}`;
  }
}

const errors = findings.filter((f) => f.severity === 'ERROR').length;
const warnings = findings.filter((f) => f.severity === 'WARN').length;
const summary = { errors, warnings, filesChecked: apsFiles.length };

if (values.json) {
  const payload = {
    surface: SURFACE,
    findings: findings.map((f) => ({
      severity: f.severity,
      file: f.file,
      line: f.line,
      message: f.message,
    })),
    summary,
  };
  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
} else {
  for (const f of findings) {
    process.stdout.write(`[${SURFACE}] ${f.severity}: ${f.file}:${f.line} — ${f.message}\n`);
  }
  process.stdout.write(
    `[${SURFACE}] summary: ${errors} errors, ${warnings} warnings, ${apsFiles.length} files checked\n`
  );
}

// Tags are commonly rendered with backticks (e.g. `aps`) in APS files.
// Strip a single wrapping pair so the validator sees the bare value.
function stripInlineCode(value) {
  if (value.length >= 2 && value.startsWith('`') && value.endsWith('`')) {
    return value.slice(1, -1).trim();
  }
  return value;
}

process.exit(errors > 0 ? 1 : 0);
