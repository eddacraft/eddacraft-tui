#!/usr/bin/env node
// Surface validator: documentation metadata.
//
// Verifies every governed Markdown document declares the DOCGOV-002
// five-column metadata table plus Upstream/Downstream relationships table
// (parsed by @eddacraft/anvil-docs-meta). Emits findings against the
// orchestrator's labelled-output contract.
//
// Out of scope: APS files, READMEs, ADRs (see docs/guides/documentation-governance.md
// for the native-format carve-outs), templates, and archived content.

import { readFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { parseArgs } from 'node:util';
import globby from 'globby';
import { parseDocGovernance, ParseError } from '@eddacraft/anvil-docs-meta';

const SURFACE = 'metadata';

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

// Globs: every docs/**/*.md governed file. APS/README/ADR have native formats;
// templates and any archive subdirectory under docs/ are intentionally
// excluded. `!docs/**/archive/**` covers both the top-level docs/archive/ and
// any nested archive directory (e.g. docs/plans/archive/) for future use.
//
// `docs/public/**` is the published Docusaurus docs-site source (apps/docs-site
// → docs.eddacraft.ai). Those pages own their own discovery layer and carry
// Docusaurus frontmatter, not the internal DOCGOV governance table — rendering
// that table after the H1 would surface it (and internal repo paths) on the
// public site. Public-docs freshness is governed by DOCSYNC release/version
// sync, so they are excluded from the internal metadata surface (DOCGOV-011).
const patterns = [
  'docs/**/*.md',
  '!docs/**/archive/**',
  '!docs/**/*.template.md',
  '!docs/**/README.md',
  '!docs/public/**',
];

const files = await globby(patterns, { cwd: root, gitignore: true });
files.sort();

let baseline = {};
if (!values['no-baseline'] && existsSync(baselinePath)) {
  try {
    const raw = await readFile(baselinePath, 'utf8');
    const parsed = JSON.parse(raw);
    baseline = parsed[SURFACE] ?? {};
  } catch (err) {
    process.stderr.write(`[${SURFACE}] failed to read baseline ${baselinePath}: ${err.message}\n`);
  }
}

const findings = [];
await Promise.all(
  files.map(async (relPath) => {
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
    try {
      parseDocGovernance(content, relPath);
    } catch (err) {
      if (err instanceof ParseError) {
        findings.push({
          severity: 'ERROR',
          file: relPath,
          line: err.lineNumber ?? 1,
          message: err.message,
        });
      } else {
        findings.push({
          severity: 'ERROR',
          file: relPath,
          line: 1,
          message: `unexpected parser failure: ${err.message}`,
        });
      }
    }
  })
);

// Apply baseline: downgrade matching ERROR findings to WARN with `[baselined]` prefix.
for (const finding of findings) {
  const fingerprints = baseline[finding.file];
  if (Array.isArray(fingerprints) && fingerprints.includes(finding.message)) {
    finding.severity = 'WARN';
    finding.message = `[baselined] ${finding.message}`;
  }
}

findings.sort((a, b) => a.file.localeCompare(b.file) || a.line - b.line);

const errors = findings.filter((f) => f.severity === 'ERROR').length;
const warnings = findings.filter((f) => f.severity === 'WARN').length;
const summary = { errors, warnings, filesChecked: files.length };

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
    `[${SURFACE}] summary: ${errors} errors, ${warnings} warnings, ${files.length} files checked\n`
  );
}

process.exit(errors > 0 ? 1 : 0);
