import { describe, expect, it } from 'vitest';

import { AnvilParseError, parseAnvilSource } from './parse.js';

function makeRule(overrides: Record<string, string> = {}): string {
  const defaults: Record<string, string> = {
    id: 'GS-001',
    type: 'rule',
    family: 'guardrail-suppression',
    title: 'example rule',
    version: '1',
    severity: 'warning',
    confidence: 'high',
    spectrum_position: '3',
    targets: '[source]',
    detection: "{ type: regex, pattern: 'foo' }",
    ...overrides,
  };

  const yamlLines = Object.entries(defaults).map(([k, v]) => `${k}: ${v}`);
  return ['---', ...yamlLines, '---', '', 'body text'].join('\n');
}

function makeDefinition(overrides: Record<string, string> = {}): string {
  const defaults: Record<string, string> = {
    id: 'guardrail-suppression',
    type: 'definition',
    name: 'Guardrail Suppression',
    category: 'escape-hatch',
    targets: '[source]',
    ...overrides,
  };

  const yamlLines = Object.entries(defaults).map(([k, v]) => `${k}: ${v}`);
  return ['---', ...yamlLines, '---', '', '## What It Is', 'body'].join('\n');
}

describe('parseAnvilSource — happy path', () => {
  it('parses a valid rule file', () => {
    const parsed = parseAnvilSource('/tmp/GS-001.anvil', makeRule());
    expect(parsed.kind).toBe('rule');
    if (parsed.kind !== 'rule') throw new Error('unreachable');
    expect(parsed.frontmatter.id).toBe('GS-001');
    expect(parsed.frontmatter.family).toBe('guardrail-suppression');
    expect(parsed.frontmatter.severity).toBe('warning');
    expect(parsed.frontmatter.detection).toEqual({ type: 'regex', pattern: 'foo' });
    expect(parsed.body).toBe('body text');
    expect(parsed.frontmatter.allowlist).toEqual([]);
    expect(parsed.frontmatter.enabled).toBe(true);
    expect(parsed.frontmatter.opt_in).toBe(false);
  });

  it('parses a valid definition file', () => {
    const parsed = parseAnvilSource('/tmp/def.anvil', makeDefinition());
    expect(parsed.kind).toBe('definition');
    if (parsed.kind !== 'definition') throw new Error('unreachable');
    expect(parsed.frontmatter.id).toBe('guardrail-suppression');
    expect(parsed.frontmatter.category).toBe('escape-hatch');
    expect(parsed.frontmatter.related).toEqual([]);
    expect(parsed.frontmatter.tensions).toEqual([]);
    expect(parsed.body).toContain('## What It Is');
  });

  it('strips a leading BOM', () => {
    const parsed = parseAnvilSource('/tmp/GS-001.anvil', '\uFEFF' + makeRule());
    expect(parsed.kind).toBe('rule');
  });
});

describe('parseAnvilSource — error cases', () => {
  it('throws when opening delimiter is missing', () => {
    expect(() => parseAnvilSource('/tmp/x.anvil', 'no frontmatter here')).toThrow(AnvilParseError);
  });

  it('throws when closing delimiter is missing', () => {
    const raw = ['---', 'id: GS-001', 'type: rule', 'body without close'].join('\n');
    expect(() => parseAnvilSource('/tmp/x.anvil', raw)).toThrow(/closing `---`/);
  });

  it('throws on invalid YAML', () => {
    const raw = ['---', 'id: [unterminated', '---', '', 'body'].join('\n');
    expect(() => parseAnvilSource('/tmp/x.anvil', raw)).toThrow(/invalid YAML/);
  });

  it('throws when frontmatter is a scalar (not a mapping)', () => {
    const raw = ['---', "'just a string'", '---', '', 'body'].join('\n');
    expect(() => parseAnvilSource('/tmp/x.anvil', raw)).toThrow(/must be a YAML mapping/);
  });

  it('throws when frontmatter is an array rather than an object', () => {
    // Arrays survive the typeof-object guard; zod rejects them with a clear message.
    const raw = ['---', '- just', '- a', '- list', '---', '', 'body'].join('\n');
    expect(() => parseAnvilSource('/tmp/x.anvil', raw)).toThrow(/frontmatter validation failed/);
  });

  it('throws when body is empty', () => {
    const raw = [
      '---',
      'id: GS-001',
      'type: rule',
      'family: guardrail-suppression',
      'title: t',
      'version: 1',
      'severity: warning',
      'confidence: high',
      'spectrum_position: 1',
      'targets: [source]',
      "detection: { type: regex, pattern: 'x' }",
      '---',
      '',
    ].join('\n');
    expect(() => parseAnvilSource('/tmp/x.anvil', raw)).toThrow(/body is empty/);
  });

  it('throws when rule id does not match <PREFIX>-NNN', () => {
    expect(() => parseAnvilSource('/tmp/x.anvil', makeRule({ id: 'not-a-rule' }))).toThrow(
      /frontmatter validation failed/
    );
  });

  it('throws when severity is unknown', () => {
    expect(() => parseAnvilSource('/tmp/x.anvil', makeRule({ severity: 'critical' }))).toThrow(
      /frontmatter validation failed/
    );
  });

  it('throws when targets is empty', () => {
    expect(() => parseAnvilSource('/tmp/x.anvil', makeRule({ targets: '[]' }))).toThrow(
      /frontmatter validation failed/
    );
  });

  it('throws when detection discriminator is missing', () => {
    expect(() =>
      parseAnvilSource('/tmp/x.anvil', makeRule({ detection: "{ pattern: 'foo' }" }))
    ).toThrow(/frontmatter validation failed/);
  });

  it('throws when type is not rule or definition', () => {
    expect(() => parseAnvilSource('/tmp/x.anvil', makeRule({ type: 'unknown' }))).toThrow(
      /frontmatter validation failed/
    );
  });
});
