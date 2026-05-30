#!/usr/bin/env node
// Surface validator: internal link integrity.
//
// Resolves every internal Markdown link in docs/**/*.md and plans/**/*.md to
// a real file (and, when anchored, a real heading). External URLs, mail/tel
// schemes, freshness anchors (which are inline code, not links), and links
// inside fenced code blocks are intentionally out of scope — the remark AST
// visits only link/image/linkReference/imageReference nodes which already
// skip code blocks.

import { readFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { resolve, dirname, isAbsolute, relative as relPath } from 'node:path';
import { parseArgs } from 'node:util';
import globby from 'globby';
import { unified } from 'unified';
import remarkParse from 'remark-parse';
import remarkGfm from 'remark-gfm';
import { visit } from 'unist-util-visit';

const SURFACE = 'links';
const EXTERNAL_RE = /^(?:https?:|mailto:|tel:|ftp:|data:)/i;

// decodeURIComponent throws a URIError on malformed percent escapes (e.g.
// `%zz` or a lone `%`). A bad link must surface as a labelled ERROR finding,
// never an uncaught crash that aborts the whole surface (DOCGOV-012). The
// sentinel lets resolveLink distinguish "decode failed" from a valid decode.
const DECODE_FAILED = Symbol('decode-failed');
function safeDecode(value) {
  try {
    return decodeURIComponent(value);
  } catch {
    return DECODE_FAILED;
  }
}

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

const files = await globby(['docs/**/*.md', 'plans/**/*.md'], { cwd: root, gitignore: true });
files.sort();

// Cache parsed-heading results per file path; the headings of a target file
// are needed both when validating links from that file and when other files
// link into it.
const headingCache = new Map();

async function getHeadings(absPath) {
  if (headingCache.has(absPath)) return headingCache.get(absPath);
  let slugs;
  try {
    const content = await readFile(absPath, 'utf8');
    slugs = extractHeadingSlugs(content);
  } catch {
    slugs = null; // unreadable target → caller will treat as missing
  }
  headingCache.set(absPath, slugs);
  return slugs;
}

function extractHeadingSlugs(content) {
  const ast = unified().use(remarkParse).use(remarkGfm).parse(content);
  const counts = new Map();
  const slugs = new Set();
  visit(ast, 'heading', (node) => {
    const text = headingText(node).trim();
    const baseSlug = slugify(text);
    if (!baseSlug) return;
    const seen = counts.get(baseSlug) ?? 0;
    counts.set(baseSlug, seen + 1);
    slugs.add(seen === 0 ? baseSlug : `${baseSlug}-${seen}`);
  });
  return slugs;
}

function headingText(node) {
  let out = '';
  visit(node, (child) => {
    if (child.type === 'text' || child.type === 'inlineCode') out += child.value;
  });
  return out;
}

// GitHub-style heading slug rule: lowercase, drop non `[a-z0-9 -]`, replace
// runs of whitespace with `-`, collapse repeated `-`, trim.
function slugify(text) {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9\s-]/g, '')
    .trim()
    .replace(/\s+/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-+|-+$/g, '');
}

const findings = [];

await Promise.all(
  files.map(async (relFile) => {
    const absFile = resolve(root, relFile);
    let content;
    try {
      content = await readFile(absFile, 'utf8');
    } catch (err) {
      process.stderr.write(`[${SURFACE}] failed to read ${relFile}: ${err.message}\n`);
      findings.push({
        severity: 'ERROR',
        file: relFile,
        line: 1,
        message: `failed to read file: ${err.message}`,
      });
      return;
    }

    const ast = unified().use(remarkParse).use(remarkGfm).parse(content);

    // Collect link-reference definitions for resolving `[text][label]` forms.
    const definitions = new Map();
    visit(ast, 'definition', (node) => {
      definitions.set(node.identifier, { url: node.url, line: node.position?.start.line ?? 1 });
    });

    // Walk link / image / reference nodes only (skips fenced code by design).
    const checks = [];
    visit(ast, (node) => {
      if (node.type === 'link' || node.type === 'image') {
        checks.push({ url: node.url, line: node.position?.start.line ?? 1 });
      } else if (node.type === 'linkReference' || node.type === 'imageReference') {
        const def = definitions.get(node.identifier);
        if (!def) {
          findings.push({
            severity: 'WARN',
            file: relFile,
            line: node.position?.start.line ?? 1,
            message: `reference label "${node.identifier}" has no matching definition`,
          });
          return;
        }
        checks.push({ url: def.url, line: node.position?.start.line ?? 1 });
      }
    });

    for (const check of checks) {
      const result = await resolveLink(check.url, absFile);
      if (result) {
        findings.push({
          severity: 'ERROR',
          file: relFile,
          line: check.line,
          message: result,
        });
      }
    }
  })
);

async function resolveLink(href, absFile) {
  if (!href) return null;
  if (EXTERNAL_RE.test(href)) return null;
  if (href.startsWith('#')) {
    const slugs = await getHeadings(absFile);
    if (!slugs) return `broken anchor "${href}" — current file unreadable for heading parse`;
    const anchor = safeDecode(href.slice(1));
    if (anchor === DECODE_FAILED) {
      return `malformed link "${href}" — invalid percent-encoding in anchor`;
    }
    if (!slugs.has(anchor)) {
      return `broken anchor "${href}" — heading "#${anchor}" not found in current file`;
    }
    return null;
  }

  const hashIdx = href.indexOf('#');
  const rawPath = hashIdx === -1 ? href : href.slice(0, hashIdx);
  let anchor = null;
  if (hashIdx !== -1) {
    anchor = safeDecode(href.slice(hashIdx + 1));
    if (anchor === DECODE_FAILED) {
      return `malformed link "${href}" — invalid percent-encoding in anchor`;
    }
  }

  let targetAbs;
  if (rawPath.startsWith('/')) {
    targetAbs = resolve(root, `.${rawPath}`);
  } else {
    const decoded = safeDecode(rawPath);
    if (decoded === DECODE_FAILED) {
      return `malformed link "${href}" — invalid percent-encoding in path`;
    }
    targetAbs = resolve(dirname(absFile), decoded);
  }

  // Reject paths escaping the repo root — those are out of scope.
  const rel = relPath(root, targetAbs);
  if (rel.startsWith('..') || isAbsolute(rel)) return null;

  if (!existsSync(targetAbs)) {
    return `broken link "${href}" — target file does not exist`;
  }

  if (!anchor) return null;

  // Anchor resolution only applies to Markdown targets.
  if (!targetAbs.endsWith('.md')) return null;

  const slugs = await getHeadings(targetAbs);
  if (!slugs) return `broken anchor "${href}" — target file unreadable for heading parse`;
  if (!slugs.has(anchor)) {
    return `broken anchor "${href}" — file exists but heading "#${anchor}" not found`;
  }
  return null;
}

// Apply baseline (fingerprint = message text without line number).
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
