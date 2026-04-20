import { describe, expect, it } from 'vitest';

import {
  extractSections,
  getExplanation,
  getSuggestion,
  REQUIRED_DEFINITION_SECTIONS,
  validateDefinitionSections,
} from './sections.js';

describe('extractSections', () => {
  it('splits a body into H2 sections in document order', () => {
    const body = [
      'preamble line',
      '',
      '## First',
      'first body',
      '',
      '## Second',
      'second body line 1',
      'second body line 2',
    ].join('\n');

    const { preamble, sections, byHeading } = extractSections(body);

    expect(preamble).toBe('preamble line');
    expect(sections).toEqual([
      { heading: 'First', body: 'first body' },
      { heading: 'Second', body: 'second body line 1\nsecond body line 2' },
    ]);
    expect(byHeading.get('first')).toBe('first body');
    expect(byHeading.get('second')).toBe('second body line 1\nsecond body line 2');
  });

  it('ignores H2-looking lines inside fenced code blocks', () => {
    const body = [
      '## Real',
      'before',
      '```md',
      '## Not a heading',
      'still code',
      '```',
      'after',
    ].join('\n');

    const { sections } = extractSections(body);

    expect(sections).toHaveLength(1);
    expect(sections[0]?.heading).toBe('Real');
    expect(sections[0]?.body).toContain('## Not a heading');
    expect(sections[0]?.body).toContain('after');
  });

  it('returns an empty section body when a heading has no following content', () => {
    const body = ['## Empty', '', '## Next', 'content'].join('\n');
    const { byHeading } = extractSections(body);
    expect(byHeading.get('empty')).toBe('');
    expect(byHeading.get('next')).toBe('content');
  });
});

describe('validateDefinitionSections', () => {
  it('reports all required headings as missing when body is empty', () => {
    const report = validateDefinitionSections(extractSections(''));
    expect(report.ok).toBe(false);
    expect(report.missing).toEqual([...REQUIRED_DEFINITION_SECTIONS]);
    expect(report.empty).toEqual([]);
  });

  it('separates missing from empty sections', () => {
    const body = [
      '## What It Is',
      'present',
      "## Why It's Harmful",
      '',
      '## The Right Response',
      'ok',
    ].join('\n');
    const report = validateDefinitionSections(extractSections(body));

    expect(report.missing).toContain('The Spectrum');
    expect(report.missing).toContain('Detection Signals');
    expect(report.missing).toContain('Example');
    expect(report.empty).toEqual(["Why It's Harmful"]);
  });

  it('passes when every required section is present and non-empty', () => {
    const body = REQUIRED_DEFINITION_SECTIONS.map((h) => `## ${h}\ncontent for ${h}`).join('\n\n');
    const report = validateDefinitionSections(extractSections(body));
    expect(report.ok).toBe(true);
    expect(report.missing).toEqual([]);
    expect(report.empty).toEqual([]);
  });
});

describe('getExplanation / getSuggestion', () => {
  it('return the bodies of the harmful / right-response sections', () => {
    const body = ["## Why It's Harmful", 'harm text', '', '## The Right Response', 'fix text'].join(
      '\n'
    );
    const sections = extractSections(body);
    expect(getExplanation(sections)).toBe('harm text');
    expect(getSuggestion(sections)).toBe('fix text');
  });

  it('return empty strings when sections are absent (compiler enforces presence upstream)', () => {
    const sections = extractSections('## Something Else\nbody');
    expect(getExplanation(sections)).toBe('');
    expect(getSuggestion(sections)).toBe('');
  });
});
