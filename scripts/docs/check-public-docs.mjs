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
const PUBLIC_ROOT = resolve(REPO_ROOT, 'docs/public');
const ANVIL_ROOT = resolve(REPO_ROOT, 'docs/public/anvil');
const BETA_ROOT = resolve(REPO_ROOT, 'docs/public/beta');
const APS_ROOT = resolve(REPO_ROOT, 'docs/public/aps');
// In-tree product sections declare the full governance triple (owner,
// upstream, verified_against). Copied sections (kindling, aps) attest
// owner plus verified_against as the imported product version; optional
// upstream must still resolve. edda-stack is in-tree (ADR-122).
const COPIED_SECTIONS = new Set(['kindling', 'aps']);
// Navigation authority now sits on the live hosts. `apps/docs-site` was the
// rollback host and carried the completeness sidebars; it was retired on
// 2026-07-08 (`847436623`) and deleted once the rollback window closed, so
// the live sidebars are the only ones that exist. `checkProductNavigation`
// (every page listed or `public_unlisted`) and `checkLiveAnvilNavigation`
// (required ids on, flag-gated ids off) now run against the same anvil file
// and are complementary rather than two-host cross-checks.
const ANVIL_SIDEBAR_PATH = resolve(REPO_ROOT, 'apps/anvil-docs-private/sidebars/anvil.ts');
const ANVIL_LIVE_SIDEBAR_PATH = ANVIL_SIDEBAR_PATH;
const APS_SIDEBAR_PATH = resolve(REPO_ROOT, 'apps/docs-public/sidebars/aps.ts');
const LIVE_REQUIRED_ANVIL_IDS = [
  'beta-testing-guide',
  'concepts/baseline',
  'concepts/boundaries',
  'concepts/evaluation-model',
  'concepts/glossary',
  'concepts/policy-model',
  'concepts/review-capsules',
  'guides/insights',
  'guides/save-time-validation',
  'integrations/agent-skills',
  'integrations/watch-output',
  'operations/git-hooks',
  'operations/telemetry',
  'operations/uninstall',
  'reference/checks',
  'reference/config',
  'reference/cli-reference',
  'reference/policy',
  'reference/rule-reference',
  'reference/support-reference',
  'reference/what-anvil-can-do',
];
const LIVE_FORBIDDEN_ANVIL_IDS = ['guides/dashboard'];
const INIT_ANVIL_YAML = `schema_version: "1.0.0"
planning_dir: "plans"
format: "yaml"
checks:
  - "secret-detection"
  - "import-boundaries"
  - "antipattern-scan"
`;
const CONFIG_CATALOGUE_REQUIRED = [
  'schema_version',
  'planning_dir',
  '`format`',
  '`checks`',
  'secret-detection',
  'import-boundaries',
  'antipattern-scan',
  'antipattern.exclude',
  'architecture.source',
  'gate.version',
  'gate.thresholds',
  'gate.global_config',
  'gate.checks',
  'public-api-expansion',
  'new-dependency-introduction',
  'cross-layer-violation',
  'privilege-expansion',
  'config show --json',
  'Not in this catalogue',
];
// De-duplicated: ANVIL_LIVE_SIDEBAR_PATH aliases ANVIL_SIDEBAR_PATH now that
// there is one host. The retired docs-site shell files (docusaurus.config.ts,
// src/pages/index.tsx, api/auth/{login,callback}.ts) are gone; the auth
// endpoints were already superseded by apps/docs-shell/app/auth/*/route.ts.
//
// The live hosts' own docusaurus configs are deliberately NOT added here.
// Scanning them would *expand* this gate rather than rehome it, and they
// carry pre-existing violations that need their own decision (product-name
// casing, and an internal repository name on a public footer link). Tracked
// separately; see the deletion PR that retired docs-site.
const SITE_SHELL_PATHS = [ANVIL_SIDEBAR_PATH, APS_SIDEBAR_PATH];

const findings = [];
const contentFiles = [
  ...markdownFiles(ANVIL_ROOT),
  ...markdownFiles(APS_ROOT),
  ...markdownFiles(BETA_ROOT).filter((path) => path.endsWith(`${sep}quickstart.md`)),
];
const contentFileSet = new Set(contentFiles);
const files = [...contentFiles, ...SITE_SHELL_PATHS.filter(existsSync)];

const internalPatterns = [
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
const anvilInternalPatterns = [
  {
    pattern:
      /(?:^|[\s`("'=/])(?:\.\.\/)*(?:plans|crates|packages|apps|scripts|\.github|\.claude|\.codex)\//,
    message: 'internal repository reference',
  },
];
const apsInternalPatterns = [
  {
    pattern: /(?:^|[\s`("'=/])(?:\.\.\/)*(?:cli\/src|lib\/rules|docs\/ai\/prompting)\//,
    message: 'internal repository reference',
  },
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

  // Governance keys legitimately name repository paths (ADR-119 D5) and are
  // never rendered, so only their lines are exempt from the leakage scan.
  // Other frontmatter (title, description) renders into built HTML and stays
  // under the newcomer trust contract.
  const exemptLines = governanceFrontmatterLines(lines);
  for (let index = 0; index < lines.length; index += 1) {
    if (exemptLines.has(index)) continue;
    const line = lines[index];
    if (productNamePattern.test(line)) {
      add(publicPath, index + 1, 'product name must be lowercase: anvil, eddacraft, or kindling');
    }
    const productPatterns = file.startsWith(`${ANVIL_ROOT}${sep}`)
      ? anvilInternalPatterns
      : file.startsWith(`${APS_ROOT}${sep}`)
        ? apsInternalPatterns
        : [];
    for (const rule of [...internalPatterns, ...productPatterns]) {
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

const governanceFiles = markdownFiles(PUBLIC_ROOT);
const copiedAttested = checkGovernance(governanceFiles);
const filesChecked = new Set([...files, ...governanceFiles]).size;

checkNavigation();
checkGeneratedReferences();
checkConfigCatalogueFixture();

findings.sort((a, b) =>
  a.file === b.file
    ? a.line - b.line || a.message.localeCompare(b.message)
    : a.file.localeCompare(b.file)
);

if (JSON_OUTPUT) {
  process.stdout.write(
    `${JSON.stringify({ surface: 'public-docs', findings, summary: { errors: findings.length, filesChecked, copiedAttested } }, null, 2)}\n`
  );
} else {
  for (const finding of findings) {
    process.stdout.write(
      `[public-docs] ERROR: ${finding.file}:${finding.line} — ${finding.message}\n`
    );
  }
  if (copiedAttested > 0) {
    process.stdout.write(
      `[public-docs] info: ${copiedAttested} copied-section page(s) attested against an imported product version\n`
    );
  }
  process.stdout.write(
    `[public-docs] summary: ${findings.length} errors, ${filesChecked} files checked\n`
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
  // Completeness: every public page must be listed or marked public_unlisted.
  // Live contract: required definition pages stay on nav and flag-gated
  // surfaces stay off it. Both now read the live host — a page deliberately
  // kept off nav (see LIVE_FORBIDDEN_ANVIL_IDS) must therefore carry
  // `public_unlisted: true`, which is what that frontmatter key means.
  checkProductNavigation(ANVIL_ROOT, ANVIL_SIDEBAR_PATH, 'anvilSidebar', 'anvil');
  checkLiveAnvilNavigation();
  checkProductNavigation(APS_ROOT, APS_SIDEBAR_PATH, 'apsSidebar', 'APS');
}

function checkLiveAnvilNavigation() {
  const livePath = ANVIL_LIVE_SIDEBAR_PATH;
  const publicPath = normalise(relative(REPO_ROOT, livePath));
  if (!existsSync(livePath)) {
    add(publicPath, 1, 'live anvil sidebar is missing');
    return;
  }

  // Skip the live-nav contract when this is not the real corpus. The check
  // asserts that named definition pages stay on nav; against a fixture that
  // ships two pages it would demand 21 ids that were never meant to exist.
  // Presence of the required documents is the honest signal — this previously
  // keyed off `apps/anvil-docs-private` existing, which stopped discriminating
  // once fixtures began writing the live sidebar there (the rollback host that
  // used to hold the fixture sidebar was deleted).
  const publishedForGuard = publishedAnvilDocumentIds();
  if (!LIVE_REQUIRED_ANVIL_IDS.some((id) => publishedForGuard.has(id))) return;

  let liveIds;
  try {
    liveIds = sidebarDocumentIds(readFileSync(livePath, 'utf8'), livePath, 'anvilSidebar');
  } catch (error) {
    add(publicPath, 1, `live anvil sidebar could not be read structurally: ${error.message}`);
    return;
  }

  const published = publishedAnvilDocumentIds();
  for (const id of liveIds) {
    if (!published.has(id)) {
      add(publicPath, 1, `live sidebar lists unknown document id ${id}`);
    }
  }
  for (const id of LIVE_REQUIRED_ANVIL_IDS) {
    if (!liveIds.has(id)) {
      add(publicPath, 1, `live sidebar is missing required document id ${id}`);
    }
  }
  for (const id of LIVE_FORBIDDEN_ANVIL_IDS) {
    if (liveIds.has(id)) {
      add(publicPath, 1, `live sidebar must not list ${id}`);
    }
  }
}

function publishedAnvilDocumentIds() {
  const ids = new Set();
  for (const file of markdownFiles(ANVIL_ROOT)) {
    const frontmatter = parseFrontmatter(readFileSync(file, 'utf8'));
    ids.add(documentId(file, frontmatter.id, ANVIL_ROOT));
  }
  return ids;
}

function checkProductNavigation(root, sidebarPath, sidebarKey, productLabel) {
  if (!existsSync(root)) return;
  if (!existsSync(sidebarPath)) {
    add(normalise(relative(REPO_ROOT, sidebarPath)), 1, `${productLabel} sidebar is missing`);
    return;
  }

  const sidebar = readFileSync(sidebarPath, 'utf8');
  let documentIds = new Set();
  try {
    documentIds = sidebarDocumentIds(sidebar, sidebarPath, sidebarKey);
  } catch (error) {
    add(
      normalise(relative(REPO_ROOT, sidebarPath)),
      1,
      `${productLabel} sidebar could not be read structurally: ${error.message}`
    );
  }

  for (const file of markdownFiles(root)) {
    const content = readFileSync(file, 'utf8');
    const frontmatter = parseFrontmatter(content);
    if (frontmatter.public_unlisted === 'true') continue;
    const id = documentId(file, frontmatter.id, root);
    if (!documentIds.has(id)) {
      add(
        normalise(relative(REPO_ROOT, file)),
        1,
        `public page ${id} is not present in the ${productLabel} sidebar and is not marked public_unlisted: true`
      );
    }
  }
}

function checkConfigCatalogueFixture() {
  const cataloguePath = resolve(ANVIL_ROOT, 'reference/config.md');
  const fixturePath = resolve(REPO_ROOT, 'scripts/docs/fixtures/anvil-init.yaml');
  const catalogueExists = existsSync(cataloguePath);
  const fixtureExists = existsSync(fixturePath);
  if (!catalogueExists && !fixtureExists) return;

  const catalogueRel = normalise(relative(REPO_ROOT, cataloguePath));
  const fixtureRel = normalise(relative(REPO_ROOT, fixturePath));

  if (!fixtureExists) {
    add(fixtureRel, 1, 'init-file fixture is missing');
    return;
  }
  if (!catalogueExists) {
    add(catalogueRel, 1, 'config field catalogue is missing');
    return;
  }

  const fixture = readFileSync(fixturePath, 'utf8');
  if (fixture !== INIT_ANVIL_YAML) {
    add(fixtureRel, 1, 'init-file fixture must match the file anvil init writes');
  }

  const catalogue = readFileSync(cataloguePath, 'utf8');
  for (const needle of CONFIG_CATALOGUE_REQUIRED) {
    if (!catalogue.includes(needle)) {
      add(catalogueRel, 1, `config catalogue is missing ${needle}`);
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

  if (existsSync(APS_ROOT)) {
    const apsChecker = resolve(REPO_ROOT, 'scripts/docs/check-aps-public-commands.mjs');
    if (!existsSync(apsChecker)) {
      add(normalise(relative(REPO_ROOT, apsChecker)), 1, 'APS public command checker is missing');
      return;
    }
    const apsResult = spawnSync(process.execPath, [apsChecker, '--root', REPO_ROOT], {
      cwd: REPO_ROOT,
      encoding: 'utf8',
    });
    if (apsResult.status !== 0) {
      const detail = (apsResult.stderr || apsResult.stdout || 'APS command examples are stale')
        .trim()
        .split(/\r?\n/)[0];
      add('docs/public/aps', 1, `APS command example is stale: ${detail}`);
    }
  }
}

function sidebarDocumentIds(source, sidebarPath, sidebarKey) {
  const sourceFile = ts.createSourceFile(
    sidebarPath,
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
  const sidebar = propertyValue(root, sidebarKey);
  const items = resolveExpression(sidebar, bindings);
  if (!items || !ts.isArrayLiteralExpression(items)) {
    throw new Error(`${sidebarKey} is not a statically readable array`);
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
  let listKey;
  for (const line of content.slice(4, end).split(/\r?\n/)) {
    const item = /^\s+-\s+(.*?)\s*$/.exec(line);
    if (item && listKey !== undefined) {
      values[listKey].push(item[1].replace(/^['"]|['"]$/g, ''));
      continue;
    }
    const match = /^([a-zA-Z0-9_-]+):\s*(.*?)\s*$/.exec(line);
    if (match) {
      if (match[2] === '') {
        listKey = match[1];
        values[listKey] = [];
      } else {
        listKey = undefined;
        values[match[1]] = match[2].replace(/^['"]|['"]$/g, '');
      }
    } else {
      listKey = undefined;
    }
  }
  return values;
}

function governanceFrontmatterLines(lines) {
  const exempt = new Set();
  if (lines[0] !== '---') return exempt;
  let inUpstreamList = false;
  for (let index = 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (line === '---') return exempt;
    if (inUpstreamList && /^\s+-\s/.test(line)) {
      exempt.add(index);
      continue;
    }
    inUpstreamList = false;
    if (/^(?:owner|verified_against):/.test(line)) {
      exempt.add(index);
    } else if (/^upstream:/.test(line)) {
      exempt.add(index);
      inUpstreamList = true;
    }
  }
  // Unterminated frontmatter: fail safe — scan everything.
  return new Set();
}

function checkGovernance(publicFiles) {
  let copiedAttested = 0;
  for (const file of publicFiles) {
    const publicPath = normalise(relative(REPO_ROOT, file));
    const section = publicPath.split('/')[2];
    const copied = COPIED_SECTIONS.has(section);
    const frontmatter = parseFrontmatter(readFileSync(file, 'utf8'));

    const owner = frontmatter.owner;
    if (owner === undefined || owner === '') {
      add(publicPath, 1, 'missing governance frontmatter: owner (uppercase module id)');
    } else if (typeof owner !== 'string' || !/^[A-Z][A-Z0-9]{1,15}$/.test(owner)) {
      add(publicPath, 1, `owner must be an uppercase module id: ${owner}`);
    }

    const upstream =
      frontmatter.upstream === undefined || Array.isArray(frontmatter.upstream)
        ? frontmatter.upstream
        : [frontmatter.upstream];
    if (upstream === undefined) {
      if (!copied) {
        add(publicPath, 1, 'missing governance frontmatter: upstream (declared source paths)');
      }
    } else if (upstream.length === 0) {
      add(publicPath, 1, 'upstream must list at least one repository path');
    } else {
      for (const entry of upstream) {
        if (entry.startsWith('/') || entry.split('/').includes('..')) {
          add(publicPath, 1, `upstream must be a relative repository path: ${entry}`);
        } else if (!existsSync(resolve(REPO_ROOT, entry))) {
          add(publicPath, 1, `upstream path does not exist: ${entry}`);
        }
      }
    }

    const version = frontmatter.verified_against;
    if (version === undefined) {
      add(publicPath, 1, 'missing governance frontmatter: verified_against (product version)');
    } else if (
      typeof version !== 'string' ||
      !/^[0-9]+\.[0-9]+\.[0-9]+(?:-[a-z0-9.-]+)?$/.test(version)
    ) {
      add(
        publicPath,
        1,
        `verified_against must be a bare product version like 0.9.4-beta: ${version}`
      );
    } else if (copied) {
      copiedAttested += 1;
    }
  }
  return copiedAttested;
}

function documentId(file, explicitId, root) {
  const rel = normalise(relative(root, file)).replace(/\.mdx?$/, '');
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
