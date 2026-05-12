/**
 * Documentation governance metadata parser.
 *
 * Extracts the DOCGOV-002 metadata + relationships tables from Markdown
 * documents. The convention is defined in
 * `docs/guides/documentation-governance.md`: immediately after the H1, a
 * five-column metadata table (Type | Authority | Owner | Status | Freshness)
 * followed by a two-column relationships table (Upstream | Downstream).
 */

import { unified } from 'unified';
import remarkParse from 'remark-parse';
import remarkGfm from 'remark-gfm';
import { visit } from 'unist-util-visit';
import type { Root, Heading, Table, TableRow, TableCell, PhrasingContent } from 'mdast';
import {
  DocGovernanceSchema,
  DocMetadataSchema,
  ParseError,
  type DocGovernance,
  type DocMetadata,
  type DocRelations,
} from '../types/index.js';

const METADATA_HEADERS = ['type', 'authority', 'owner', 'status', 'freshness'] as const;
const RELATIONS_HEADERS = ['upstream', 'downstream'] as const;

/**
 * Parse documentation governance metadata from a Markdown source.
 *
 * @param content - Raw Markdown content of the governed document.
 * @param sourcePath - Optional source path used for error reporting.
 * @returns Validated governance metadata and relationships.
 * @throws {ParseError} When the H1, metadata table, or relationships table
 *   are missing, malformed, or carry an unknown enum value.
 */
export function parseDocGovernance(content: string, sourcePath?: string): DocGovernance {
  const processor = unified().use(remarkParse).use(remarkGfm);
  const ast = processor.parse(content) as Root;

  const h1 = findFirstH1(ast);
  if (!h1) {
    throw new ParseError('Document must have an H1 title', sourcePath);
  }
  const title = extractPlainText(h1).trim();
  if (!title) {
    throw new ParseError(
      'Document H1 title must not be empty',
      sourcePath,
      h1.position?.start.line
    );
  }

  const tablesAfterH1 = collectTablesAfterH1(ast, h1);

  if (tablesAfterH1.length < 1) {
    throw new ParseError(
      'Missing governance metadata table (Type | Authority | Owner | Status | Freshness) after H1',
      sourcePath,
      h1.position?.start.line
    );
  }
  if (tablesAfterH1.length < 2) {
    throw new ParseError(
      'Missing Upstream/Downstream relationships table after the metadata table',
      sourcePath,
      tablesAfterH1[0].position?.start.line
    );
  }

  const metadata = parseMetadataTable(tablesAfterH1[0], sourcePath);
  const relations = parseRelationsTable(tablesAfterH1[1], sourcePath);

  const result: DocGovernance = {
    title,
    metadata,
    relations,
    sourcePath,
    sourceLineNumber: h1.position?.start.line,
  };

  // Final structural validation — defends against future schema changes
  // that this parser has not been updated to honour.
  return DocGovernanceSchema.parse(result);
}

function findFirstH1(ast: Root): Heading | null {
  let found: Heading | null = null;
  visit(ast, 'heading', (node) => {
    if (!found && (node as Heading).depth === 1) {
      found = node as Heading;
      return false;
    }
    return true;
  });
  return found;
}

function collectTablesAfterH1(ast: Root, h1: Heading): Table[] {
  const h1Index = ast.children.indexOf(h1);
  if (h1Index < 0) return [];
  const tables: Table[] = [];
  for (let i = h1Index + 1; i < ast.children.length; i += 1) {
    const node = ast.children[i];
    if (node.type === 'table') {
      tables.push(node as Table);
      if (tables.length === 2) break;
    }
  }
  return tables;
}

function parseMetadataTable(table: Table, sourcePath?: string): DocMetadata {
  const line = table.position?.start.line;
  if (table.children.length < 2) {
    throw new ParseError('Metadata table must have a header row and a data row', sourcePath, line);
  }

  const headerRow = table.children[0] as TableRow;
  const dataRow = table.children[1] as TableRow;

  const headers = headerRow.children.map((cell) =>
    extractCellText(cell as TableCell).toLowerCase()
  );
  if (headers.length !== METADATA_HEADERS.length) {
    throw new ParseError(
      `Metadata table must have ${METADATA_HEADERS.length} columns (${METADATA_HEADERS.join(', ')}); got ${headers.length}`,
      sourcePath,
      line
    );
  }
  for (let i = 0; i < METADATA_HEADERS.length; i += 1) {
    if (headers[i] !== METADATA_HEADERS[i]) {
      throw new ParseError(
        `Metadata table column ${i + 1} must be "${METADATA_HEADERS[i]}"; got "${headers[i]}"`,
        sourcePath,
        line
      );
    }
  }

  if (dataRow.children.length !== METADATA_HEADERS.length) {
    throw new ParseError(
      `Metadata table data row must have ${METADATA_HEADERS.length} cells; got ${dataRow.children.length}`,
      sourcePath,
      dataRow.position?.start.line ?? line
    );
  }

  const cells = dataRow.children.map((cell) => extractCellText(cell as TableCell));
  const candidate = {
    type: cells[0],
    authority: cells[1],
    owner: cells[2],
    status: cells[3],
    freshness: cells[4],
  };

  const result = DocMetadataSchema.safeParse(candidate);
  if (!result.success) {
    const issue = result.error.issues[0];
    const field = issue.path[0]?.toString() ?? 'unknown';
    const offending = (candidate as Record<string, string>)[field] ?? '';
    throw new ParseError(
      formatEnumError(field, offending, issue.message),
      sourcePath,
      dataRow.position?.start.line ?? line
    );
  }

  return result.data;
}

function parseRelationsTable(table: Table, sourcePath?: string): DocRelations {
  const line = table.position?.start.line;
  if (table.children.length < 2) {
    throw new ParseError(
      'Relationships table must have a header row and a data row',
      sourcePath,
      line
    );
  }

  const headerRow = table.children[0] as TableRow;
  const dataRow = table.children[1] as TableRow;

  const headers = headerRow.children.map((cell) =>
    extractCellText(cell as TableCell).toLowerCase()
  );
  if (headers.length !== RELATIONS_HEADERS.length) {
    throw new ParseError(
      `Relationships table must have ${RELATIONS_HEADERS.length} columns (${RELATIONS_HEADERS.join(', ')}); got ${headers.length}`,
      sourcePath,
      line
    );
  }
  for (let i = 0; i < RELATIONS_HEADERS.length; i += 1) {
    if (headers[i] !== RELATIONS_HEADERS[i]) {
      throw new ParseError(
        `Relationships table column ${i + 1} must be "${RELATIONS_HEADERS[i]}"; got "${headers[i]}"`,
        sourcePath,
        line
      );
    }
  }

  if (dataRow.children.length !== RELATIONS_HEADERS.length) {
    throw new ParseError(
      `Relationships table data row must have ${RELATIONS_HEADERS.length} cells; got ${dataRow.children.length}`,
      sourcePath,
      dataRow.position?.start.line ?? line
    );
  }

  return {
    upstream: splitRefs(extractCellText(dataRow.children[0] as TableCell)),
    downstream: splitRefs(extractCellText(dataRow.children[1] as TableCell)),
  };
}

/**
 * Extract the text payload of a table cell, unwrapping backtick-wrapped
 * inline code so `` `path/to/file.md` `` parses as `path/to/file.md`.
 */
function extractCellText(cell: TableCell): string {
  return cell.children.map(phrasingText).join('').trim();
}

function phrasingText(node: PhrasingContent): string {
  switch (node.type) {
    case 'text':
      return node.value;
    case 'inlineCode':
      return node.value;
    case 'emphasis':
    case 'strong':
    case 'delete':
    case 'link':
    case 'linkReference':
      return node.children.map(phrasingText).join('');
    case 'break':
      return ' ';
    default:
      return '';
  }
}

function extractPlainText(node: Heading): string {
  return node.children.map(phrasingText).join('');
}

function splitRefs(value: string): string[] {
  return value
    .split(',')
    .map((part) => part.trim())
    .filter((part) => part.length > 0);
}

function formatEnumError(field: string, offending: string, fallbackMessage: string): string {
  const schemaForField = enumValuesFor(field);
  if (!schemaForField) {
    return `Invalid value for "${field}": ${fallbackMessage}`;
  }
  const valuesList = schemaForField.map((v) => `"${v}"`).join(', ');
  return `Invalid value "${offending}" for "${field}". Valid values: ${valuesList}`;
}

function enumValuesFor(field: string): readonly string[] | null {
  switch (field) {
    case 'type':
      return DocMetadataSchema.shape.type.options;
    case 'authority':
      return DocMetadataSchema.shape.authority.options;
    case 'status':
      return DocMetadataSchema.shape.status.options;
    default:
      return null;
  }
}
