import { describe, expect, it } from 'vitest';
import { parseDocGovernance } from './parse-metadata.js';
import { ParseError } from '../types/index.js';

const HAPPY_PATH = `# Documentation Governance

| Type  | Authority     | Owner  | Status | Freshness                                                                        |
| ----- | ------------- | ------ | ------ | -------------------------------------------------------------------------------- |
| Guide | Authoritative | DOCGOV | Live   | Last reviewed 2026-05-11 against \`plans/modules/documentation-governance.aps.md\` |

| Upstream                                                                           | Downstream                                                                           |
| ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| \`plans/modules/documentation-governance.aps.md\`, \`AGENTS.md\`, \`plans/aps-rules.md\` | \`docs/README.md\`, \`docs/guides/README.md\`, \`AGENTS.md\`, future \`docs-workflow\` skill |

Body content follows here.
`;

describe('parseDocGovernance', () => {
  it('parses the canonical happy-path metadata and relationships', () => {
    const result = parseDocGovernance(HAPPY_PATH, 'docs/guides/documentation-governance.md');

    expect(result.title).toBe('Documentation Governance');
    expect(result.metadata).toEqual({
      type: 'Guide',
      authority: 'Authoritative',
      owner: 'DOCGOV',
      status: 'Live',
      freshness: 'Last reviewed 2026-05-11 against plans/modules/documentation-governance.aps.md',
    });
    expect(result.relations.upstream).toEqual([
      'plans/modules/documentation-governance.aps.md',
      'AGENTS.md',
      'plans/aps-rules.md',
    ]);
    expect(result.relations.downstream).toEqual([
      'docs/README.md',
      'docs/guides/README.md',
      'AGENTS.md',
      'future docs-workflow skill',
    ]);
    expect(result.sourcePath).toBe('docs/guides/documentation-governance.md');
    expect(result.sourceLineNumber).toBe(1);
  });

  it('throws ParseError when the H1 is missing', () => {
    const content = `## Subheading only

| Type  | Authority     | Owner  | Status | Freshness |
| ----- | ------------- | ------ | ------ | --------- |
| Guide | Authoritative | DOCGOV | Live   | x         |

| Upstream | Downstream |
| -------- | ---------- |
| a        | b          |
`;
    expect(() => parseDocGovernance(content)).toThrow(ParseError);
    expect(() => parseDocGovernance(content)).toThrow(/H1 title/);
  });

  it('throws ParseError when the metadata table is missing', () => {
    const content = `# Title

Just some prose, no tables here.
`;
    expect(() => parseDocGovernance(content)).toThrow(/metadata table/);
  });

  it('throws ParseError when the Upstream/Downstream table is missing', () => {
    const content = `# Title

| Type  | Authority     | Owner  | Status | Freshness |
| ----- | ------------- | ------ | ------ | --------- |
| Guide | Authoritative | DOCGOV | Live   | x         |

Some prose but no second table.
`;
    expect(() => parseDocGovernance(content)).toThrow(/Upstream\/Downstream/);
  });

  it('throws ParseError when the metadata table has the wrong column count', () => {
    const content = `# Title

| Type  | Authority     | Owner  | Status |
| ----- | ------------- | ------ | ------ |
| Guide | Authoritative | DOCGOV | Live   |

| Upstream | Downstream |
| -------- | ---------- |
| a        | b          |
`;
    expect(() => parseDocGovernance(content)).toThrow(/columns/);
  });

  it('throws ParseError naming the offending field on an unknown enum value', () => {
    const content = `# Title

| Type         | Authority     | Owner  | Status | Freshness |
| ------------ | ------------- | ------ | ------ | --------- |
| NotARealType | Authoritative | DOCGOV | Live   | 2026-01-01 |

| Upstream | Downstream |
| -------- | ---------- |
| a        | b          |
`;
    let captured: ParseError | undefined;
    try {
      parseDocGovernance(content, 'fixture.md');
    } catch (err) {
      captured = err as ParseError;
    }
    expect(captured).toBeInstanceOf(ParseError);
    expect(captured?.message).toMatch(/type/);
    expect(captured?.message).toMatch(/NotARealType/);
    expect(captured?.message).toMatch(/Guide/);
    expect(captured?.sourcePath).toBe('fixture.md');
  });

  it('throws ParseError when the Status enum value is unknown', () => {
    const content = `# Title

| Type  | Authority     | Owner  | Status   | Freshness |
| ----- | ------------- | ------ | -------- | --------- |
| Guide | Authoritative | DOCGOV | NotReal  | 2026-01-01 |

| Upstream | Downstream |
| -------- | ---------- |
| a        | b          |
`;
    expect(() => parseDocGovernance(content)).toThrow(/status/);
  });

  it('unwraps backtick-wrapped cells in both tables', () => {
    const content = `# Title

| Type  | Authority     | Owner    | Status | Freshness |
| ----- | ------------- | -------- | ------ | --------- |
| Guide | Authoritative | \`DOCGOV\` | Live   | \`2026-01-01\` |

| Upstream     | Downstream |
| ------------ | ---------- |
| \`a/b/c.md\` | \`d/e.md\` |
`;
    const result = parseDocGovernance(content);
    expect(result.metadata.owner).toBe('DOCGOV');
    expect(result.metadata.freshness).toBe('2026-01-01');
    expect(result.relations.upstream).toEqual(['a/b/c.md']);
    expect(result.relations.downstream).toEqual(['d/e.md']);
  });

  it('splits comma-separated Upstream and Downstream cells into arrays', () => {
    const content = `# Title

| Type  | Authority     | Owner | Status | Freshness  |
| ----- | ------------- | ----- | ------ | ---------- |
| Guide | Authoritative | x     | Live   | 2026-01-01 |

| Upstream         | Downstream         |
| ---------------- | ------------------ |
| one, two, three  | alpha, beta        |
`;
    const result = parseDocGovernance(content);
    expect(result.relations.upstream).toEqual(['one', 'two', 'three']);
    expect(result.relations.downstream).toEqual(['alpha', 'beta']);
  });

  it('accepts every enumerated Type value', () => {
    const types = [
      'APS index',
      'APS module',
      'ADR',
      'Spec',
      'As-built',
      'Runbook',
      'Guide',
      'README',
      'Public docs',
      'Archive',
    ] as const;

    for (const type of types) {
      const content = `# Title

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| ${type} | Derived   | x     | Live   | 2026-01-01 |

| Upstream | Downstream |
| -------- | ---------- |
| a        | b          |
`;
      const result = parseDocGovernance(content);
      expect(result.metadata.type).toBe(type);
    }
  });

  it('produces empty arrays when Upstream/Downstream cells are blank', () => {
    const content = `# Title

| Type    | Authority  | Owner | Status   | Freshness  |
| ------- | ---------- | ----- | -------- | ---------- |
| Archive | Historical | x     | Archived | 2026-01-01 |

| Upstream | Downstream |
| -------- | ---------- |
|          |            |
`;
    const result = parseDocGovernance(content);
    expect(result.relations.upstream).toEqual([]);
    expect(result.relations.downstream).toEqual([]);
  });
});
