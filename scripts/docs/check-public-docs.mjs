#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { dirname, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import process from 'node:process';
import ts from 'typescript';

const argv = process.argv.slice(2);

function flagValue(name) {
  const index = argv.indexOf(name);
  return index >= 0 && index + 1 < argv.length ? argv[index + 1] : undefined;
}

const REPO_ROOT = flagValue('--root')
  ? resolve(flagValue('--root'))
  : fileURLToPath(new URL('../..', import.meta.url));
const JSON_OUTPUT = argv.includes('--json');
const SKIP_GENERATED = argv.includes('--skip-generated');
const ANVIL_ROOT = resolve(REPO_ROOT, 'docs/public/anvil');
const BETA_ROOT = resolve(REPO_ROOT, 'docs/public/beta');
const SIDEBAR_PATH = resolve(REPO_ROOT, 'apps/docs-site/sidebars/anvil.ts');
const SITE_SHELL_PATHS = [
  SIDEBAR_PATH,
  resolve(REPO_ROOT, 'apps/docs-site/docusaurus.config.ts'),
  resolve(REPO_ROOT, 'apps/docs-site/src/pages/index.tsx'),
  resolve(REPO_ROOT, 'apps/docs-site/api/auth/login.ts'),
  resolve(REPO_ROOT, 'apps/docs-site/api/auth/callback.ts'),
];

const findings = [];
const contentFiles = [
  ...markdownFiles(ANVIL_ROOT),
  ...markdownFiles(BETA_ROOT).filter((path) => path.endsWith(`${sep}quickstart.md`)),
];
const contentFileSet = new Set(contentFiles);
const files = [...contentFiles, ...SITE_SHELL_PATHS.filter(existsSync)];

const internalPatterns = [
  {
    pattern:
      /(?:^|[\s`("'=/])(?:\.\.\/)*(?:plans|crates|packages|apps|scripts|\.github|\.claude|\.codex)\//,
    message: 'internal repository reference',
  },
  {
    pattern: /(?:^|[\s`("'=/])(?:\.\.\/)*docs\/(?:architecture|guides|runbooks|specs)\//,
    message: 'internal documentation reference',
  },
  {
    pattern: /\b(?:ADR|DOCSYNC|DSITE|DOCGOV|CIB)-\d+\b/,
    message: 'internal work-item or decision reference',
  },
  { pattern: /\banvil-001\b/, message: 'internal repository name' },
];

const productNamePattern = /\b(?:Anvil|EddaCraft|Kindling)\b/;
const releaseDownloadPrefix = 'https://github.com/eddacraft/anvil/releases/';
const installChecks = [
  (line) =>
    releaseAssetPath(line, /^(?:latest\/download|download\/v[^/]+)\/eddacraft-anvil-installer\.sh/),
  (line) =>
    releaseAssetPath(
      line,
      /^(?:latest\/download|download\/v[^/]+)\/eddacraft-anvil-installer\.ps1/i
    ),
  (line) => /brew install eddacraft\/tap\/anvil/.test(line),
  (line) => /winget install eddacraft\.anvil/i.test(line),
  (line) => /scoop install anvil/.test(line),
];

for (const file of files) {
  const content = readFileSync(file, 'utf8');
  const lines = content.split(/\r?\n/);
  const publicPath = normalise(relative(REPO_ROOT, file));

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    if (productNamePattern.test(line)) {
      add(publicPath, index + 1, 'product name must be lowercase: anvil, eddacraft, or kindling');
    }
    for (const rule of internalPatterns) {
      if (rule.pattern.test(line)) add(publicPath, index + 1, rule.message);
    }
    if (contentFileSet.has(file) && !isCanonicalQuickstart(file)) {
      for (const isInstallProcedure of installChecks) {
        if (isInstallProcedure(line)) {
          add(
            publicPath,
            index + 1,
            'duplicates the canonical install procedure in anvil/quickstart'
          );
          break;
        }
      }
    }
  }
}

checkNavigation();
checkGeneratedReferences();

findings.sort((a, b) =>
  a.file === b.file
    ? a.line - b.line || a.message.localeCompare(b.message)
    : a.file.localeCompare(b.file)
);

if (JSON_OUTPUT) {
  process.stdout.write(
    `${JSON.stringify({ surface: 'public-docs', findings, summary: { errors: findings.length, filesChecked: files.length } }, null, 2)}\n`
  );
} else {
  for (const finding of findings) {
    process.stdout.write(
      `[public-docs] ERROR: ${finding.file}:${finding.line} — ${finding.message}\n`
    );
  }
  process.stdout.write(
    `[public-docs] summary: ${findings.length} errors, ${files.length} files checked\n`
  );
}

process.exit(findings.length === 0 ? 0 : 1);

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

function checkNavigation() {
  if (!existsSync(SIDEBAR_PATH)) {
    add(normalise(relative(REPO_ROOT, SIDEBAR_PATH)), 1, 'anvil sidebar is missing');
    return;
  }

  const sidebar = readFileSync(SIDEBAR_PATH, 'utf8');
  let documentIds = new Set();
  try {
    documentIds = sidebarDocumentIds(sidebar);
  } catch (error) {
    add(
      normalise(relative(REPO_ROOT, SIDEBAR_PATH)),
      1,
      `anvil sidebar could not be read structurally: ${error.message}`
    );
  }

  for (const file of markdownFiles(ANVIL_ROOT)) {
    const content = readFileSync(file, 'utf8');
    const frontmatter = parseFrontmatter(content);
    if (frontmatter.public_unlisted === 'true') continue;
    const id = documentId(file, frontmatter.id);
    if (!documentIds.has(id)) {
      add(
        normalise(relative(REPO_ROOT, file)),
        1,
        `public page ${id} is not present in the anvil sidebar and is not marked public_unlisted: true`
      );
    }
  }
}

function checkGeneratedReferences() {
  if (SKIP_GENERATED) return;
  const generator = resolve(REPO_ROOT, 'scripts/docs/generate-anvil-public-reference.mjs');
  if (!existsSync(generator)) {
    add(
      normalise(relative(REPO_ROOT, generator)),
      1,
      'generated public reference checker is missing'
    );
    return;
  }
  const result = spawnSync(process.execPath, [generator, '--check'], {
    cwd: REPO_ROOT,
    encoding: 'utf8',
  });
  if (result.status !== 0) {
    const detail = (result.stderr || result.stdout || 'generated reference is stale')
      .trim()
      .split(/\r?\n/)[0];
    add('docs/public/anvil/reference', 1, `generated public reference is stale: ${detail}`);
  }
}

function sidebarDocumentIds(source) {
  const sourceFile = ts.createSourceFile(
    SIDEBAR_PATH,
    source,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TS
  );
  if (sourceFile.parseDiagnostics.length > 0) {
    throw new Error(
      ts.flattenDiagnosticMessageText(sourceFile.parseDiagnostics[0].messageText, ' ')
    );
  }

  const bindings = new Map();
  let exported;
  for (const statement of sourceFile.statements) {
    if (ts.isVariableStatement(statement)) {
      for (const declaration of statement.declarationList.declarations) {
        if (ts.isIdentifier(declaration.name) && declaration.initializer) {
          bindings.set(declaration.name.text, declaration.initializer);
        }
      }
    } else if (ts.isExportAssignment(statement) && !statement.isExportEquals) {
      exported = statement.expression;
    }
  }

  const root = resolveExpression(exported, bindings);
  if (!root || !ts.isObjectLiteralExpression(root)) {
    throw new Error('default export is not a statically readable object');
  }
  const sidebar = propertyValue(root, 'anvilSidebar');
  const items = resolveExpression(sidebar, bindings);
  if (!items || !ts.isArrayLiteralExpression(items)) {
    throw new Error('anvilSidebar is not a statically readable array');
  }

  const ids = new Set();
  collectDocumentIds(items, bindings, ids);
  return ids;
}

function collectDocumentIds(array, bindings, ids) {
  for (const rawItem of array.elements) {
    const item = resolveExpression(rawItem, bindings);
    if (isStaticString(item)) {
      ids.add(item.text);
      continue;
    }
    if (!item || !ts.isObjectLiteralExpression(item)) {
      throw new Error('sidebar item is not a statically readable string or object');
    }

    const type = staticString(propertyValue(item, 'type'), bindings);
    if (type === 'doc') {
      const id = staticString(propertyValue(item, 'id'), bindings);
      if (!id) throw new Error('doc sidebar item has no static id');
      ids.add(id);
    } else if (type === 'category') {
      const nested = resolveExpression(propertyValue(item, 'items'), bindings);
      if (!nested || !ts.isArrayLiteralExpression(nested)) {
        throw new Error('category sidebar item has no static items array');
      }
      collectDocumentIds(nested, bindings, ids);
    }
  }
}

function resolveExpression(expression, bindings, seen = new Set()) {
  if (!expression) return undefined;
  if (
    ts.isParenthesizedExpression(expression) ||
    ts.isAsExpression(expression) ||
    ts.isSatisfiesExpression(expression) ||
    ts.isTypeAssertionExpression(expression)
  ) {
    return resolveExpression(expression.expression, bindings, seen);
  }
  if (ts.isIdentifier(expression)) {
    if (seen.has(expression.text)) throw new Error(`cyclic sidebar binding: ${expression.text}`);
    const value = bindings.get(expression.text);
    if (!value) return expression;
    return resolveExpression(value, bindings, new Set([...seen, expression.text]));
  }
  return expression;
}

function propertyValue(object, name) {
  for (const property of object.properties) {
    if (!ts.isPropertyAssignment(property)) continue;
    const key = property.name;
    if ((ts.isIdentifier(key) || isStaticString(key)) && key.text === name) {
      return property.initializer;
    }
  }
  return undefined;
}

function staticString(expression, bindings) {
  const resolved = resolveExpression(expression, bindings);
  return isStaticString(resolved) ? resolved.text : undefined;
}

function isStaticString(expression) {
  return Boolean(
    expression && (ts.isStringLiteral(expression) || ts.isNoSubstitutionTemplateLiteral(expression))
  );
}

function parseFrontmatter(content) {
  if (!content.startsWith('---\n')) return {};
  const end = content.indexOf('\n---', 4);
  if (end < 0) return {};
  const values = {};
  for (const line of content.slice(4, end).split(/\r?\n/)) {
    const match = /^([a-zA-Z0-9_-]+):\s*(.*?)\s*$/.exec(line);
    if (match) values[match[1]] = match[2].replace(/^['"]|['"]$/g, '');
  }
  return values;
}

function documentId(file, explicitId) {
  const rel = normalise(relative(ANVIL_ROOT, file)).replace(/\.mdx?$/, '');
  const directory = normalise(dirname(rel));
  const leaf = explicitId || rel.split('/').at(-1);
  return directory === '.' ? leaf : `${directory}/${leaf}`;
}

function isCanonicalQuickstart(file) {
  const relativePath = normalise(relative(ANVIL_ROOT, file));
  return relativePath === 'quickstart.md' || relativePath === 'integrations/github.md';
}

function releaseAssetPath(line, assetPattern) {
  const start = line.indexOf(releaseDownloadPrefix);
  if (start < 0) return false;
  return assetPattern.test(line.slice(start + releaseDownloadPrefix.length));
}

function add(file, line, message) {
  findings.push({ severity: 'ERROR', file, line, message });
}

function normalise(path) {
  return path.split(sep).join('/');
}
