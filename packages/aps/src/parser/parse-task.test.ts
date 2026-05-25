/**
 * Tests for parse-task module
 */

import { describe, it, expect } from 'vitest';
import { parseTaskHeading, parseTaskFields, parseTask } from './parse-task.js';
import { ParseError } from '../types/index.js';
import type { Heading, Paragraph, Strong, Text, List, ListItem, InlineCode, Break } from 'mdast';

// --- Helpers to build mdast nodes ---

function heading(depth: 1 | 2 | 3, text: string): Heading {
  return { type: 'heading', depth, children: [{ type: 'text', value: text }] };
}

function paragraph(children: Paragraph['children']): Paragraph {
  return { type: 'paragraph', children };
}

function strong(text: string): Strong {
  return { type: 'strong', children: [{ type: 'text', value: text }] };
}

function text(value: string): Text {
  return { type: 'text', value };
}

function inlineCode(value: string): InlineCode {
  return { type: 'inlineCode', value };
}

function br(): Break {
  return { type: 'break' };
}

function list(items: string[]): List {
  return {
    type: 'list',
    ordered: false,
    children: items.map(
      (item): ListItem => ({
        type: 'listItem',
        children: [paragraph([text(item)])],
      })
    ),
  };
}

/** Build a paragraph with bold field key-value pairs: **Key:** value */
function fieldParagraph(
  fields: Record<string, string | { type: 'inlineCode'; value: string }>
): Paragraph {
  const children: Paragraph['children'] = [];
  const entries = Object.entries(fields);

  for (let i = 0; i < entries.length; i++) {
    const [key, value] = entries[i];
    children.push(strong(`${key}:`));
    if (typeof value === 'string') {
      children.push(text(` ${value}`));
    } else {
      children.push(text(' '));
      children.push(inlineCode(value.value));
    }
    if (i < entries.length - 1) {
      children.push(text('\n'));
    }
  }

  return paragraph(children);
}

// --- Tests ---

describe('parseTaskHeading', () => {
  it('should parse a valid H3 heading with task ID and title', () => {
    const h = heading(3, 'AUTH-001: Create user database model');
    const result = parseTaskHeading(h);
    expect(result).toEqual({ id: 'AUTH-001', title: 'Create user database model' });
  });

  it('should handle numeric scope prefixes', () => {
    const h = heading(3, 'LLM2-007: Update language model');
    const result = parseTaskHeading(h);
    expect(result).toEqual({ id: 'LLM2-007', title: 'Update language model' });
  });

  it('should handle single-character scope', () => {
    const h = heading(3, 'A-001: Short scope');
    const result = parseTaskHeading(h);
    expect(result).toEqual({ id: 'A-001', title: 'Short scope' });
  });

  it('should handle max-length scope (10 chars)', () => {
    const h = heading(3, 'ABCDEFGHIJ-999: Max length scope');
    const result = parseTaskHeading(h);
    expect(result).toEqual({ id: 'ABCDEFGHIJ-999', title: 'Max length scope' });
  });

  it('should trim whitespace from title', () => {
    const h = heading(3, 'TEST-001:   Whitespace title  ');
    const result = parseTaskHeading(h);
    expect(result).toEqual({ id: 'TEST-001', title: 'Whitespace title' });
  });

  it('should throw ParseError for non-H3 headings', () => {
    expect(() => parseTaskHeading(heading(1, 'TEST-001: Title'))).toThrow(ParseError);
    expect(() => parseTaskHeading(heading(2, 'TEST-001: Title'))).toThrow(ParseError);
    expect(() => parseTaskHeading(heading(1, 'TEST-001: Title'))).toThrow(
      /Task headings must be H3/
    );
  });

  it('should throw ParseError for heading without colon separator', () => {
    const h = heading(3, 'TEST-001 No colon');
    expect(() => parseTaskHeading(h)).toThrow(ParseError);
    expect(() => parseTaskHeading(h)).toThrow(/Invalid task heading format/);
  });

  it('should throw ParseError for heading without task ID', () => {
    const h = heading(3, 'Just a title');
    expect(() => parseTaskHeading(h)).toThrow(ParseError);
  });

  it('should throw ParseError for lowercase scope', () => {
    const h = heading(3, 'auth-001: Lowercase scope');
    expect(() => parseTaskHeading(h)).toThrow(ParseError);
  });

  it('should throw ParseError for non-zero-padded numbers', () => {
    const h = heading(3, 'TEST-1: Not padded');
    expect(() => parseTaskHeading(h)).toThrow(ParseError);
  });

  it('should throw ParseError for scope exceeding 10 characters', () => {
    const h = heading(3, 'ABCDEFGHIJK-001: Too long scope');
    expect(() => parseTaskHeading(h)).toThrow(ParseError);
  });

  it('should throw ParseError for empty title after colon', () => {
    const h = heading(3, 'TEST-001:');
    expect(() => parseTaskHeading(h)).toThrow(ParseError);
  });

  it('should extract text from heading with inline formatting', () => {
    // Heading with strong child wrapping part of the text
    const h: Heading = {
      type: 'heading',
      depth: 3,
      children: [
        { type: 'text', value: 'TEST-001: ' },
        { type: 'strong', children: [{ type: 'text', value: 'Bold title' }] },
      ],
    };
    const result = parseTaskHeading(h);
    expect(result).toEqual({ id: 'TEST-001', title: 'Bold title' });
  });
});

describe('parseTaskFields', () => {
  it('should parse Intent field', () => {
    const para = fieldParagraph({ Intent: 'Create the user model' });
    const result = parseTaskFields([para], []);
    expect(result.intent).toBe('Create the user model');
  });

  it('should parse Expected Outcome field (with space)', () => {
    const para = paragraph([strong('Expected Outcome:'), text(' User model is created')]);
    const result = parseTaskFields([para], []);
    expect(result.expectedOutcome).toBe('User model is created');
  });

  it('should parse Outcome field as temporary alias for Expected Outcome', () => {
    const para = paragraph([strong('Outcome:'), text(' User model is created')]);
    const result = parseTaskFields([para], []);
    expect(result.expectedOutcome).toBe('User model is created');
  });

  it('should parse Validation field', () => {
    const para = fieldParagraph({ Validation: 'pnpm test' });
    const result = parseTaskFields([para], []);
    expect(result.validation).toBe('pnpm test');
  });

  it('should parse Test field as alias for Validation', () => {
    const para = fieldParagraph({ Test: 'npm run test' });
    const result = parseTaskFields([para], []);
    expect(result.validation).toBe('npm run test');
  });

  it('should parse Confidence field values', () => {
    for (const level of ['low', 'medium', 'high'] as const) {
      const para = fieldParagraph({ Confidence: level });
      const result = parseTaskFields([para], []);
      expect(result.confidence).toBe(level);
    }
  });

  it('should default invalid confidence to medium', () => {
    const para = fieldParagraph({ Confidence: 'extreme' });
    const result = parseTaskFields([para], []);
    expect(result.confidence).toBe('medium');
  });

  it('should parse Scopes as comma-separated list', () => {
    const para = fieldParagraph({ Scopes: 'AUTH, DB, API' });
    const result = parseTaskFields([para], []);
    expect(result.scopes).toEqual(['AUTH', 'DB', 'API']);
  });

  it('should parse NonScope field', () => {
    const para = fieldParagraph({ NonScope: 'database, external APIs' });
    const result = parseTaskFields([para], []);
    expect(result.nonScope).toEqual(['database', 'external APIs']);
  });

  it('should parse Non-scope field (hyphenated)', () => {
    const para = paragraph([strong('Non-scope:'), text(' database, external APIs')]);
    const result = parseTaskFields([para], []);
    expect(result.nonScope).toEqual(['database', 'external APIs']);
  });

  it('should parse Files field', () => {
    const para = fieldParagraph({ Files: 'src/model.ts, src/service.ts' });
    const result = parseTaskFields([para], []);
    expect(result.files).toEqual(['src/model.ts', 'src/service.ts']);
  });

  it('should parse Tags field', () => {
    const para = fieldParagraph({ Tags: 'testing, security, storage' });
    const result = parseTaskFields([para], []);
    expect(result.tags).toEqual(['testing', 'security', 'storage']);
  });

  it('should parse Dependencies field', () => {
    const para = fieldParagraph({ Dependencies: 'AUTH-001, AUTH-002' });
    const result = parseTaskFields([para], []);
    expect(result.dependencies).toEqual(['AUTH-001', 'AUTH-002']);
  });

  it('should parse Risks field', () => {
    const para = fieldParagraph({ Risks: 'data loss, downtime' });
    const result = parseTaskFields([para], []);
    expect(result.risks).toEqual(['data loss', 'downtime']);
  });

  it('should parse Packages field', () => {
    const para = fieldParagraph({ Packages: '@app/core, @app/utils' });
    const result = parseTaskFields([para], []);
    expect(result.packages).toEqual(['@app/core', '@app/utils']);
  });

  it('should handle Packages: (none) as empty array', () => {
    const para = fieldParagraph({ Packages: '(none)' });
    const result = parseTaskFields([para], []);
    expect(result.packages).toEqual([]);
  });

  it('should handle empty Packages value as empty array', () => {
    const para = paragraph([strong('Packages:'), text('')]);
    const result = parseTaskFields([para], []);
    expect(result.packages).toEqual([]);
  });

  it('should parse Link field', () => {
    const para = fieldParagraph({ Link: 'https://jira.example.com/PROJ-123' });
    const result = parseTaskFields([para], []);
    expect(result.link).toBe('https://jira.example.com/PROJ-123');
  });

  it('should parse Status field', () => {
    for (const status of ['open', 'locked', 'completed', 'cancelled'] as const) {
      const para = fieldParagraph({ Status: status });
      const result = parseTaskFields([para], []);
      expect(result.status).toBe(status);
    }
  });

  it('should default invalid status to open', () => {
    const para = fieldParagraph({ Status: 'unknown' });
    const result = parseTaskFields([para], []);
    expect(result.status).toBe('open');
  });

  it('should accept prose status aliases', () => {
    const cases: Array<[string, string]> = [
      ['Complete', 'completed'],
      ['complete', 'completed'],
      ['Done', 'completed'],
      ['In Progress', 'locked'],
      ['in-progress', 'locked'],
      ['Draft', 'open'],
      ['Ready', 'open'],
      ['Blocked', 'locked'],
      ['canceled', 'cancelled'],
    ];
    for (const [input, expected] of cases) {
      const para = fieldParagraph({ Status: input });
      const result = parseTaskFields([para], []);
      expect(result.status, `"${input}" should map to "${expected}"`).toBe(expected);
    }
  });

  it('should accept narrative status values with trailing prose', () => {
    // Real module text appends free-form prose after the status token
    // (PR refs, dates, em-dashes). Without prefix-boundary matching the
    // alias lookup falls through to `open` for these forms.
    const cases: Array<[string, string]> = [
      ['Merged 2026-05-17 — all four umbrella-required sub-tasks landed', 'completed'],
      ['Complete — merged 2026-04-29 via PR #1186', 'completed'],
      ['Done — landed via PR #100', 'completed'],
      ['In Progress — investigating', 'locked'],
      ['Blocked on upstream review', 'locked'],
    ];
    for (const [input, expected] of cases) {
      const para = fieldParagraph({ Status: input });
      const result = parseTaskFields([para], []);
      expect(result.status, `"${input}" should map to "${expected}"`).toBe(expected);
    }
  });

  it('should not match alias keys that are word-prefixes of other words', () => {
    // `releasednonsense` must not match the `released` alias — the
    // boundary check rejects it when the next char is alphanumeric.
    for (const input of ['releasednonsense', 'shipped123', 'mergedoverflow']) {
      const para = fieldParagraph({ Status: input });
      const result = parseTaskFields([para], []);
      expect(result.status, `"${input}" must NOT match a lifecycle alias`).toBe('open');
    }
  });

  it('should accept narrative lifecycle aliases as completed', () => {
    // plans/aps-rules.md documents these labels as the post-In Progress
    // lifecycle. Without aliases, anything past `Merged` defaulted to
    // `open` and the drift-check progress count missed every shipped
    // task in modules using the convention.
    const cases: Array<[string, string]> = [
      ['Merged', 'completed'],
      ['merged', 'completed'],
      ['Released', 'completed'],
      ['Shipped', 'completed'],
      ['Released/Shipped', 'completed'],
      ['released/shipped', 'completed'],
      ['Archived', 'completed'],
      ['Complete/Archived', 'completed'],
    ];
    for (const [input, expected] of cases) {
      const para = fieldParagraph({ Status: input });
      const result = parseTaskFields([para], []);
      expect(result.status, `"${input}" should map to "${expected}"`).toBe(expected);
    }
  });

  it('should strip parenthetical suffixes from status values', () => {
    const para = fieldParagraph({ Status: 'Complete (2026-02-15)' });
    const result = parseTaskFields([para], []);
    expect(result.status).toBe('completed');
  });

  it('should parse inline Inputs as single-item array', () => {
    const para = fieldParagraph({ Inputs: 'Database credentials' });
    const result = parseTaskFields([para], []);
    expect(result.inputs).toEqual(['Database credentials']);
  });

  it('should parse Inputs with following list', () => {
    const para = paragraph([strong('Inputs:'), text('')]);
    const inputList = list(['Input one', 'Input two']);
    const result = parseTaskFields([para], [inputList]);
    expect(result.inputs).toEqual(['Input one', 'Input two']);
  });

  it('should prefer list over inline text for Inputs', () => {
    const para = paragraph([strong('Inputs:'), text(' Inline text')]);
    const inputList = list(['List item one', 'List item two']);
    const result = parseTaskFields([para], [inputList]);
    expect(result.inputs).toEqual(['List item one', 'List item two']);
  });

  it('should handle empty comma-separated values', () => {
    const para = fieldParagraph({ Tags: 'a,, b' });
    const result = parseTaskFields([para], []);
    expect(result.tags).toEqual(['a', 'b']);
  });

  it('should parse multiple fields from single paragraph', () => {
    const para = paragraph([
      strong('Intent:'),
      text(' Do something'),
      strong('Confidence:'),
      text(' high'),
    ]);
    const result = parseTaskFields([para], []);
    expect(result.intent).toBe('Do something');
    expect(result.confidence).toBe('high');
  });

  it('should parse fields across multiple paragraphs', () => {
    const p1 = fieldParagraph({ Intent: 'Do something' });
    const p2 = fieldParagraph({ Confidence: 'high' });
    const p3 = fieldParagraph({ Tags: 'a, b' });
    const result = parseTaskFields([p1, p2, p3], []);
    expect(result.intent).toBe('Do something');
    expect(result.confidence).toBe('high');
    expect(result.tags).toEqual(['a', 'b']);
  });

  it('should handle inline code in field values', () => {
    const para = paragraph([
      strong('Validation:'),
      text(' '),
      inlineCode('pnpm -F anvil-core test'),
    ]);
    const result = parseTaskFields([para], []);
    expect(result.validation).toBe('pnpm -F anvil-core test');
  });

  it('should handle break nodes in field values', () => {
    const para = paragraph([strong('Intent:'), text(' Line one'), br(), text('Line two')]);
    const result = parseTaskFields([para], []);
    expect(result.intent).toBe('Line one Line two');
  });

  it('should collapse whitespace in field values', () => {
    const para = paragraph([strong('Intent:'), text('  Multiple   spaces   here  ')]);
    const result = parseTaskFields([para], []);
    expect(result.intent).toBe('Multiple spaces here');
  });
});

describe('parseTask', () => {
  it('should parse a complete task with all fields', () => {
    const h = heading(3, 'AUTH-001: Create user database model');
    const content = [
      fieldParagraph({ Intent: 'Create the user database model for authentication' }),
      paragraph([strong('Expected Outcome:'), text(' User model with all required fields')]),
      paragraph([strong('Validation:'), text(' '), inlineCode('pnpm test')]),
      fieldParagraph({ Confidence: 'high' }),
      fieldParagraph({ Scopes: 'AUTH, DB' }),
      fieldParagraph({ Tags: 'auth, database' }),
      fieldParagraph({ Dependencies: 'INFRA-001' }),
      fieldParagraph({ Files: 'src/model.ts, src/types.ts' }),
    ];

    const task = parseTask(h, content, 'plan.aps.md', 10);

    expect(task.id).toBe('AUTH-001');
    expect(task.title).toBe('Create user database model');
    expect(task.intent).toBe('Create the user database model for authentication');
    expect(task.expectedOutcome).toBe('User model with all required fields');
    expect(task.validation).toBe('pnpm test');
    expect(task.confidence).toBe('high');
    expect(task.scopes).toEqual(['AUTH', 'DB']);
    expect(task.tags).toEqual(['auth', 'database']);
    expect(task.dependencies).toEqual(['INFRA-001']);
    expect(task.files).toEqual(['src/model.ts', 'src/types.ts']);
    expect(task.sourcePath).toBe('plan.aps.md');
    expect(task.sourceLineNumber).toBe(10);
  });

  it('should parse a minimal task with only Intent', () => {
    const h = heading(3, 'MIN-001: Minimal task');
    const content = [fieldParagraph({ Intent: 'Just an intent' })];
    const task = parseTask(h, content);

    expect(task.id).toBe('MIN-001');
    expect(task.title).toBe('Minimal task');
    expect(task.intent).toBe('Just an intent');
    expect(task.confidence).toBe('medium'); // default
    expect(task.expectedOutcome).toBeUndefined();
    expect(task.validation).toBeUndefined();
    expect(task.scopes).toBeUndefined();
    expect(task.tags).toBeUndefined();
    expect(task.sourcePath).toBeUndefined();
    expect(task.sourceLineNumber).toBeUndefined();
  });

  it('should throw ParseError when Intent is missing', () => {
    const h = heading(3, 'TEST-001: No intent task');
    const content = [fieldParagraph({ Confidence: 'high' })];

    expect(() => parseTask(h, content)).toThrow(ParseError);
    expect(() => parseTask(h, content)).toThrow(/missing required field: Intent/);
  });

  it('should throw ParseError when heading is invalid', () => {
    const h = heading(3, 'bad heading');
    const content = [fieldParagraph({ Intent: 'Intent' })];

    expect(() => parseTask(h, content)).toThrow(ParseError);
  });

  it('should handle mixed paragraphs and lists', () => {
    const h = heading(3, 'TEST-001: Task with inputs');
    const content: Array<Paragraph | List> = [
      fieldParagraph({ Intent: 'Task with list inputs' }),
      paragraph([strong('Inputs:'), text('')]),
      list(['Input one', 'Input two', 'Input three']),
    ];

    const task = parseTask(h, content);
    expect(task.inputs).toEqual(['Input one', 'Input two', 'Input three']);
  });

  it('should set sourcePath and sourceLineNumber when provided', () => {
    const h = heading(3, 'TEST-001: Tracked task');
    const content = [fieldParagraph({ Intent: 'Tracked' })];
    const task = parseTask(h, content, '/path/to/plan.md', 42);

    expect(task.sourcePath).toBe('/path/to/plan.md');
    expect(task.sourceLineNumber).toBe(42);
  });

  it('should handle task with Packages field', () => {
    const h = heading(3, 'TEST-001: Multi-package task');
    const content = [
      fieldParagraph({ Intent: 'Affects multiple packages' }),
      fieldParagraph({ Packages: '@app/core, @app/utils' }),
    ];

    const task = parseTask(h, content);
    expect(task.packages).toEqual(['@app/core', '@app/utils']);
  });

  it('should handle task with Link field', () => {
    const h = heading(3, 'TEST-001: Linked task');
    const content = [
      fieldParagraph({ Intent: 'Has external link' }),
      fieldParagraph({ Link: 'https://jira.example.com/PROJ-123' }),
    ];

    const task = parseTask(h, content);
    expect(task.link).toBe('https://jira.example.com/PROJ-123');
  });

  it('should handle task with Status field', () => {
    const h = heading(3, 'TEST-001: Status task');
    const content = [
      fieldParagraph({ Intent: 'Has status' }),
      fieldParagraph({ Status: 'completed' }),
    ];

    const task = parseTask(h, content);
    expect(task.status).toBe('completed');
  });

  it('should handle task with Risks field', () => {
    const h = heading(3, 'TEST-001: Risky task');
    const content = [
      fieldParagraph({ Intent: 'Has risks' }),
      fieldParagraph({ Risks: 'data loss, performance degradation' }),
    ];

    const task = parseTask(h, content);
    expect(task.risks).toEqual(['data loss', 'performance degradation']);
  });
});
