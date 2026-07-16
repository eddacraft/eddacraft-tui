import { existsSync } from 'node:fs';
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import * as os from 'node:os';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { compilePatterns } from './compile.js';

const thisDir = path.dirname(fileURLToPath(import.meta.url));

function findWorkspaceRoot(start: string): string {
  let dir = start;
  for (;;) {
    if (existsSync(path.join(dir, 'pnpm-workspace.yaml'))) return dir;
    const parent = path.dirname(dir);
    if (parent === dir) return start;
    dir = parent;
  }
}

const workspaceRoot = findWorkspaceRoot(thisDir);
const patternsDir = path.join(workspaceRoot, 'patterns');

describe('compilePatterns — real patterns/ tree (golden)', () => {
  it('compiles the checked-in patterns with no errors', async () => {
    const result = await compilePatterns({
      patternsDir,
      referenceRoot: workspaceRoot,
    });

    expect(result.errors, JSON.stringify(result.errors, null, 2)).toEqual([]);
    expect(result.registry).not.toBeNull();
  });

  it('produces the expected families and a stable rule id set', async () => {
    const { registry } = await compilePatterns({
      patternsDir,
      referenceRoot: workspaceRoot,
    });
    if (!registry) throw new Error('registry missing');

    const familyIds = registry.families.map((f) => f.id);
    expect(familyIds).toEqual([
      'deferred-debt',
      'dynamic-execution',
      'error-visibility',
      'fragile-presentation',
      'guardrail-suppression',
      'python-reliability',
      'responsibility-laundering',
      'rust-reliability',
      'type-system-evasion',
      'unsafe-rendering',
      'weak-cryptography',
    ]);

    const ruleIds = registry.patterns.map((p) => p.id);
    expect(ruleIds).toContain('AP-001');
    expect(ruleIds).toContain('AP-007');
    expect(ruleIds).toContain('AP-008'); // dynamic-execution: eval(<dynamic>) — LANGTS-006
    expect(ruleIds).toContain('AP-009'); // dynamic-execution: new Function() — LANGTS-006
    expect(ruleIds).toContain('GS-001');
    expect(ruleIds).toContain('DD-001');
    expect(ruleIds).toContain('PY-001');
    expect(ruleIds).toContain('PY-007');
    expect(ruleIds).toContain('RL-001');
    expect(ruleIds).toContain('RS-001'); // rust-reliability: AST unwrap/expect — RSTLAN-003
    expect(ruleIds).toContain('RS-005'); // rust-reliability: regex todo!/unimplemented! — RSTLAN-003
    expect(ruleIds).toContain('AP-017'); // dynamic-execution: SSTI (render_template_string) — INSEC-004
    expect(ruleIds).toContain('WC-001'); // weak-cryptography: deprecated hash — INSEC-002
    expect(ruleIds).toContain('WC-003'); // weak-cryptography: JWT alg:none — INSEC-002
    expect(ruleIds).toContain('UR-001'); // unsafe-rendering: innerHTML sink — INSEC-003
    expect(ruleIds).toContain('UR-003'); // unsafe-rendering: dangerouslySetInnerHTML — INSEC-003
    expect(ruleIds).toContain('FRAG-001'); // fragile-presentation: invisible-content trap — ADR-110 / CIB-198

    // Rules must be alphabetically sorted for stable registry diffs.
    const sorted = [...ruleIds].sort();
    expect(ruleIds).toEqual(sorted);
  });

  it('hydrates a rule with narrative fields from its family definition', async () => {
    const { registry } = await compilePatterns({
      patternsDir,
      referenceRoot: workspaceRoot,
    });
    if (!registry) throw new Error('registry missing');

    const ap001 = registry.patterns.find((p) => p.id === 'AP-001');
    expect(ap001).toBeDefined();
    expect(ap001!.family).toBe('guardrail-suppression');
    expect(ap001!.family_name).toBe('Guardrail Suppression');
    expect(ap001!.category).toBe('escape-hatch');
    expect(ap001!.explanation.length).toBeGreaterThan(0);
    expect(ap001!.suggestion.length).toBeGreaterThan(0);
    expect(ap001!.nudge.length).toBeGreaterThan(0);
    expect(ap001!.definition_ref).toBe('patterns/guardrail-suppression/definition.anvil');

    // INSEC-001/002: the security families carry the new category through to
    // the compiled pattern so the Rust scanner maps them to a first-class
    // variant instead of the `code-quality` fallback.
    const wc001 = registry.patterns.find((p) => p.id === 'WC-001');
    expect(wc001).toBeDefined();
    expect(wc001!.category).toBe('insecure-construction');
    expect(wc001!.family_name).toBe('Weak Cryptography');
  });

  it('records per-family prefixes and drops the ambiguous legacy AP prefix', async () => {
    const { registry, warnings } = await compilePatterns({
      patternsDir,
      referenceRoot: workspaceRoot,
    });
    if (!registry) throw new Error('registry missing');

    // Single-family prefixes should resolve to their owning family.
    expect(registry.prefixes.GS).toBe('guardrail-suppression');
    expect(registry.prefixes.DD).toBe('deferred-debt');
    expect(registry.prefixes.RL).toBe('responsibility-laundering');
    expect(registry.prefixes.WC).toBe('weak-cryptography'); // INSEC-002
    expect(registry.prefixes.UR).toBe('unsafe-rendering'); // INSEC-003

    // AP is legacy and spans multiple families. Rather than binding it to
    // whichever family happened to be visited first, the compiler drops the
    // ambiguous prefix and surfaces a warning so callers know lookup by `AP`
    // is unsafe.
    expect(registry.prefixes.AP).toBeUndefined();
    expect(warnings.some((w) => /AP/.test(w.detail))).toBe(true);
  });

  it('uses a stable ISO timestamp and relative source_root', async () => {
    const { registry } = await compilePatterns({
      patternsDir,
      referenceRoot: workspaceRoot,
    });
    if (!registry) throw new Error('registry missing');

    expect(registry.schema_version).toBe(1);
    expect(registry.compiled_at).toMatch(/^\d{4}-\d{2}-\d{2}T/);
    expect(registry.source_root).toBe('patterns');
  });
});

describe('compilePatterns — failure modes (synthetic tree)', () => {
  let tmp: string;

  beforeAll(async () => {
    tmp = await mkdtemp(path.join(os.tmpdir(), 'anvil-compile-test-'));
  });

  afterAll(async () => {
    await rm(tmp, { recursive: true, force: true });
  });

  async function writeAnvil(rel: string, content: string): Promise<string> {
    const abs = path.join(tmp, rel);
    await mkdir(path.dirname(abs), { recursive: true });
    await writeFile(abs, content, 'utf8');
    return abs;
  }

  const DEFINITION = [
    '---',
    'id: fam-x',
    'type: definition',
    'name: Family X',
    'category: escape-hatch',
    'targets: [source]',
    '---',
    '',
    '## What It Is',
    'what',
    '',
    "## Why It's Harmful",
    'harm',
    '',
    '## The Spectrum',
    'spectrum',
    '',
    '## The Right Response',
    'fix',
    '',
    '## Detection Signals',
    'signals',
    '',
    '## Example',
    'example',
    '',
  ].join('\n');

  const RULE = [
    '---',
    'id: FX-001',
    'type: rule',
    'family: fam-x',
    'title: t',
    'version: 1',
    'severity: warning',
    'confidence: high',
    'spectrum_position: 2',
    'targets: [source]',
    "detection: { type: regex, pattern: 'x' }",
    '---',
    '',
    'nudge body',
  ].join('\n');

  it('errors when a rule references a family with no definition', async () => {
    const root = path.join(tmp, 'orphan');
    await writeAnvil('orphan/fam-x/FX-001.anvil', RULE);

    const { registry, errors } = await compilePatterns({ patternsDir: root });
    expect(registry).toBeNull();
    expect(errors.some((e) => /no definition\.anvil/.test(e.detail))).toBe(true);
  });

  it('errors on duplicate rule ids across families', async () => {
    const root = path.join(tmp, 'dupe');

    await writeAnvil('dupe/fam-x/definition.anvil', DEFINITION);
    await writeAnvil('dupe/fam-x/FX-001.anvil', RULE);

    // Duplicate FX-001 in a separate family directory, still declaring fam-x.
    await writeAnvil('dupe/fam-y/FX-001.anvil', RULE);
    await writeAnvil(
      'dupe/fam-y/definition.anvil',
      DEFINITION.replace('id: fam-x', 'id: fam-y').replace('name: Family X', 'name: Family Y')
    );

    const { registry, errors } = await compilePatterns({ patternsDir: root });
    expect(registry).toBeNull();
    expect(errors.some((e) => /duplicate rule id/.test(e.detail))).toBe(true);
  });

  it('errors on duplicate family definitions', async () => {
    const root = path.join(tmp, 'dupefam');
    await writeAnvil('dupefam/a/definition.anvil', DEFINITION);
    await writeAnvil('dupefam/b/definition.anvil', DEFINITION);

    const { registry, errors } = await compilePatterns({ patternsDir: root });
    expect(registry).toBeNull();
    expect(errors.some((e) => /duplicate family definition/.test(e.detail))).toBe(true);
  });

  it('errors on prefix collision across non-AP families', async () => {
    const root = path.join(tmp, 'prefixclash');

    await writeAnvil('prefixclash/fam-x/definition.anvil', DEFINITION);
    await writeAnvil('prefixclash/fam-x/FX-001.anvil', RULE);

    const famYDef = DEFINITION.replace('id: fam-x', 'id: fam-y').replace(
      'name: Family X',
      'name: Family Y'
    );
    const famYRule = RULE.replace('id: FX-001', 'id: FX-002').replace(
      'family: fam-x',
      'family: fam-y'
    );
    await writeAnvil('prefixclash/fam-y/definition.anvil', famYDef);
    await writeAnvil('prefixclash/fam-y/FX-002.anvil', famYRule);

    const { registry, errors } = await compilePatterns({ patternsDir: root });
    expect(registry).toBeNull();
    expect(errors.some((e) => /prefix "FX" is already bound/.test(e.detail))).toBe(true);
  });

  it('errors when a required section is missing from a definition', async () => {
    const root = path.join(tmp, 'missing-sections');
    const badDef = [
      '---',
      'id: fam-x',
      'type: definition',
      'name: Family X',
      'category: escape-hatch',
      'targets: [source]',
      '---',
      '',
      '## What It Is',
      'only this',
      '',
    ].join('\n');
    await writeAnvil('missing-sections/fam-x/definition.anvil', badDef);
    await writeAnvil('missing-sections/fam-x/FX-001.anvil', RULE);

    const { registry, errors } = await compilePatterns({ patternsDir: root });
    expect(registry).toBeNull();
    expect(errors.some((e) => /missing required sections/.test(e.detail))).toBe(true);
  });

  it('succeeds on a minimal well-formed tree', async () => {
    const root = path.join(tmp, 'ok');
    await writeAnvil('ok/fam-x/definition.anvil', DEFINITION);
    await writeAnvil('ok/fam-x/FX-001.anvil', RULE);

    const { registry, errors } = await compilePatterns({ patternsDir: root });
    expect(errors).toEqual([]);
    expect(registry).not.toBeNull();
    expect(registry!.patterns).toHaveLength(1);
    expect(registry!.patterns[0]?.id).toBe('FX-001');
    expect(registry!.prefixes.FX).toBe('fam-x');
  });
});
