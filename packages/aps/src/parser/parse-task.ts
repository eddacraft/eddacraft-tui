/**
 * Task parsing utilities
 * Extracts task information from Markdown AST nodes
 */

import type { Heading, Paragraph, Strong, List, PhrasingContent } from 'mdast';
import { ParseError, type Task, type Confidence, type TaskStatus } from '../types/index.js';

/**
 * Extract task ID and title from H3 heading text
 * Format: "SCOPE-NUMBER: Task title"
 * - Scope: 1-10 uppercase alphanumeric characters
 * - Number: 3-digit zero-padded (001-999)
 */
export function parseTaskHeading(heading: Heading): { id: string; title: string } {
  if (heading.depth !== 3) {
    throw new ParseError(
      'Task headings must be H3 (###)',
      undefined,
      undefined,
      'parseTaskHeading'
    );
  }

  const text = extractPlainText(heading);
  // Extract ID and title - ID must match TASK_ID_REGEX format
  const match = text.match(/^([A-Z0-9]{1,10}-\d{3}):\s*(.+)/);

  if (!match) {
    throw new ParseError(
      `Invalid task heading format. Expected "SCOPE-NNN: Title" (e.g., AUTH-001), got: "${text}"`,
      undefined,
      undefined,
      'parseTaskHeading'
    );
  }

  return {
    id: match[1],
    title: match[2].trim(),
  };
}

/**
 * Extract plain text from AST node (handles inline formatting)
 */
function extractPlainText(node: Heading | Paragraph | PhrasingContent): string {
  if ('value' in node && typeof node.value === 'string') {
    return node.value;
  }

  if ('children' in node && Array.isArray(node.children)) {
    return node.children.map((child) => extractPlainText(child as PhrasingContent)).join('');
  }

  return '';
}

/**
 * Parse task fields from paragraph and list nodes
 * Fields are in format: **FieldName:** value
 */
export function parseTaskFields(paragraphs: Paragraph[], lists: List[]): Partial<Task> {
  const fields: Partial<Task> = {};
  let lastField: string | null = null;
  let inlineInputs: string | null = null;

  // First pass: extract all field key-value pairs from paragraphs
  for (const para of paragraphs) {
    const fieldMatches = extractFieldsFromParagraph(para);

    for (const [key, value] of Object.entries(fieldMatches)) {
      // Handle Inputs specially - may be inline text or followed by a list
      if (key === 'Inputs') {
        if (value.trim() === '') {
          // Empty value - expect a list to follow
          lastField = key;
        } else {
          // Inline text value - store for later (list takes precedence if present)
          inlineInputs = value.trim();
          lastField = key;
        }
        continue;
      }

      assignField(fields, key, value);
      lastField = key;
    }
  }

  // Second pass: handle Inputs field
  // Lists take precedence over inline text
  if (lastField === 'Inputs' && lists.length > 0) {
    fields.inputs = extractListItems(lists[0]);
  } else if (inlineInputs !== null) {
    // Use inline text as a single-item array
    fields.inputs = [inlineInputs];
  }

  return fields;
}

/**
 * Extract field key-value pairs from a paragraph containing bold markers
 */
function extractFieldsFromParagraph(para: Paragraph): Record<string, string> {
  const fields: Record<string, string> = {};
  let currentKey = '';
  let currentValue = '';
  let inField = false;

  for (const child of para.children) {
    if (child.type === 'strong') {
      // Check if this strong node contains a field name
      const strongText = extractPlainText(child as Strong);
      const fieldMatch = strongText.match(/^([\w-]+(?:\s+[\w-]+)*):$/);

      if (fieldMatch) {
        // Save previous field if exists (even if value is empty)
        if (currentKey) {
          fields[currentKey] = currentValue.trim().replace(/\s+/g, ' ');
        }

        currentKey = fieldMatch[1];
        currentValue = '';
        inField = true;
      }
    } else if (inField) {
      // Extract text from any phrasing content node (text, inlineCode, etc.)
      // This handles validation commands in backticks and other inline formatting
      if (child.type === 'break') {
        // Convert breaks to spaces to handle multi-line values
        currentValue += ' ';
      } else {
        currentValue += extractPlainText(child as PhrasingContent);
      }
    }
  }

  // Save last field (even if value is empty)
  if (currentKey) {
    fields[currentKey] = currentValue.trim().replace(/\s+/g, ' ');
  }

  return fields;
}

/**
 * Assign a parsed field value to the appropriate Task property
 */
function assignField(task: Partial<Task>, key: string, value: string): void {
  const normalizedKey = key.replace(/\s+/g, '');

  switch (normalizedKey) {
    case 'Intent':
      task.intent = value;
      break;

    case 'ExpectedOutcome':
    case 'Outcome':
      task.expectedOutcome = value;
      break;

    case 'Validation':
    case 'Test':
      // Support both "Validation:" and "Test:" field names per APS spec
      task.validation = value;
      break;

    case 'Confidence':
      task.confidence = parseConfidence(value);
      break;

    case 'Scopes':
      task.scopes = parseCommaSeparated(value);
      break;

    case 'NonScope':
    case 'Non-scope':
      task.nonScope = parseCommaSeparated(value);
      break;

    case 'Files':
      task.files = parseCommaSeparated(value);
      break;

    case 'Tags':
      task.tags = parseCommaSeparated(value);
      break;

    case 'Dependencies':
      task.dependencies = parseCommaSeparated(value);
      break;

    case 'Risks':
      task.risks = parseCommaSeparated(value);
      break;

    case 'Packages': {
      // Monorepo support: list of affected packages
      const trimmed = value.trim();
      if (!trimmed || trimmed.toLowerCase() === '(none)') {
        task.packages = [];
      } else {
        task.packages = parseCommaSeparated(value);
      }
      break;
    }

    case 'Link':
      task.link = value;
      break;

    case 'Status':
      task.status = parseStatus(value);
      break;

    // 'Inputs' is handled separately as a list
  }
}

/**
 * Parse confidence value from string
 */
function parseConfidence(value: string): Confidence {
  const normalized = value.toLowerCase().trim();
  if (normalized === 'low' || normalized === 'medium' || normalized === 'high') {
    return normalized;
  }
  return 'medium'; // default
}

/**
 * Parse task status from string.
 * Accepts canonical tokens (open, locked, completed, cancelled) and
 * common prose aliases used in human-authored plan files.
 */
function parseStatus(value: string): TaskStatus {
  // Strip parenthetical suffixes like "(2026-02-15)" before matching
  const normalized = value
    .replace(/\s*\(.*\)\s*$/, '')
    .toLowerCase()
    .trim();

  const aliases: Record<string, TaskStatus> = {
    open: 'open',
    locked: 'locked',
    completed: 'completed',
    cancelled: 'cancelled',
    complete: 'completed',
    done: 'completed',
    'in progress': 'locked',
    'in-progress': 'locked',
    draft: 'open',
    ready: 'open',
    blocked: 'locked',
    canceled: 'cancelled',
    // Narrative lifecycle labels documented in plans/aps-rules.md
    // ("Lifecycle Narrative Labels"). Authors land these on task `Status:`
    // lines (e.g. MLP2-068 advancing In Progress → Merged); without the
    // alias `@eddacraft/anvil-aps` consumers parsing task fields see
    // `open` for every shipped task, which diverges from the schema
    // intent. (scripts/aps/drift-check.mjs has its own pattern table —
    // `DONE_PATTERNS` — so the two surfaces share one lifecycle view.)
    merged: 'completed',
    released: 'completed',
    shipped: 'completed',
    'released/shipped': 'completed',
    archived: 'completed',
    'complete/archived': 'completed',
  };

  if (normalized in aliases) {
    return aliases[normalized];
  }

  // Match as a leading prefix with a word boundary so trailing prose like
  // `Merged 2026-05-17 — all four sub-tasks landed` or
  // `Complete — merged 2026-04-29 via PR #1186` still resolves. This
  // mirrors `DONE_PATTERNS` in scripts/aps/drift-check.mjs so both
  // surfaces share the same view of the narrative lifecycle.
  //
  // Longest keys are tried first so `released/shipped` wins over the
  // shorter `released` prefix when the compound form is present.
  const keys = Object.keys(aliases).sort((a, b) => b.length - a.length);
  for (const key of keys) {
    if (normalized.startsWith(key)) {
      const next = normalized.charAt(key.length);
      if (next === '' || !/\w/.test(next)) {
        return aliases[key];
      }
    }
  }

  return 'open';
}

/**
 * Parse comma-separated list into array
 */
function parseCommaSeparated(value: string): string[] {
  return value
    .split(',')
    .map((item) => item.trim())
    .filter((item) => item.length > 0);
}

/**
 * Extract list items as strings
 */
function extractListItems(list: List): string[] {
  const items: string[] = [];

  for (const item of list.children) {
    if (item.type === 'listItem' && item.children.length > 0) {
      const firstChild = item.children[0];
      if (firstChild && firstChild.type === 'paragraph') {
        items.push(extractPlainText(firstChild));
      }
    }
  }

  return items;
}

/**
 * Parse a complete task from AST nodes
 * @param heading - H3 heading node with task ID and title
 * @param content - Array of paragraph and list nodes containing task fields
 * @param sourcePath - Optional source file path
 * @param lineNumber - Optional line number where task starts
 */
export function parseTask(
  heading: Heading,
  content: Array<Paragraph | List>,
  sourcePath?: string,
  lineNumber?: number
): Task {
  const { id, title } = parseTaskHeading(heading);

  const paragraphs = content.filter((node) => node.type === 'paragraph') as Paragraph[];
  const lists = content.filter((node) => node.type === 'list') as List[];

  const fields = parseTaskFields(paragraphs, lists);

  if (!fields.intent) {
    throw new ParseError(
      `Task ${id} is missing required field: Intent`,
      sourcePath,
      lineNumber,
      'parseTask'
    );
  }

  return {
    id,
    title,
    intent: fields.intent,
    expectedOutcome: fields.expectedOutcome,
    validation: fields.validation,
    confidence: fields.confidence ?? 'medium',
    scopes: fields.scopes,
    nonScope: fields.nonScope,
    files: fields.files,
    tags: fields.tags,
    dependencies: fields.dependencies,
    inputs: fields.inputs,
    risks: fields.risks,
    packages: fields.packages,
    link: fields.link,
    status: fields.status,
    sourcePath,
    sourceLineNumber: lineNumber,
  };
}
