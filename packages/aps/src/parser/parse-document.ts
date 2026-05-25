/**
 * Document parsing utilities
 * Converts APS Markdown documents to structured data
 */

import { unified } from 'unified';
import remarkParse from 'remark-parse';
import { visit } from 'unist-util-visit';
import type { Root, Heading, Paragraph, List, Strong } from 'mdast';
import { parseTask } from './parse-task.js';
import { ParseError, type Task, type ParsedDocument, type ModuleMetadata } from '../types/index.js';

/**
 * Parse an APS leaf spec document from Markdown content
 *
 * This function parses leaf specs (documents with tasks).
 * For index files (documents with modules), use `parseIndex` instead.
 *
 * @param content - Markdown content of a leaf spec
 * @param sourcePath - Optional source file path for error reporting
 * @returns Parsed document with tasks
 */
export async function parseDocument(content: string, sourcePath?: string): Promise<ParsedDocument> {
  // Parse Markdown to AST
  const processor = unified().use(remarkParse);
  const ast = processor.parse(content) as Root;

  // Extract document structure
  const structure = extractStructure(ast, sourcePath);

  // Validate structure
  if (!structure.title) {
    throw new ParseError('Document must have an H1 title', sourcePath);
  }

  return {
    title: structure.title,
    metadata: structure.metadata,
    tasks: structure.tasks,
    sourcePath,
  };
}

/**
 * Extract document structure from AST
 */
interface DocumentStructure {
  title: string | null;
  metadata?: ModuleMetadata;
  tasks: Task[];
}

type TaskHeading = { heading: Heading; lineNumber: number };

function extractStructure(ast: Root, sourcePath?: string): DocumentStructure {
  const structure: DocumentStructure = {
    title: null,
    tasks: [],
  };

  let currentSection: 'root' | 'tasks' = 'root';
  let currentTaskHeading: TaskHeading | null = null;
  let currentTaskContent: Array<Paragraph | List> = [];

  visit(ast, (node, index, parent) => {
    if (node.type === 'heading') {
      const heading = node as Heading;

      // H1: Document title
      if (heading.depth === 1) {
        structure.title = extractPlainText(heading);

        // Check next sibling for metadata line
        if (parent && index !== null && index !== undefined) {
          const nextSibling = (parent as Root).children[index + 1];
          if (nextSibling && nextSibling.type === 'paragraph') {
            structure.metadata = parseMetadataLine(nextSibling as Paragraph);
          }
        }
      }

      // H2: Section headers
      if (heading.depth === 2) {
        // Save previous task if any
        if (currentTaskHeading) {
          saveTask(currentTaskHeading, currentTaskContent, sourcePath, structure);
          currentTaskHeading = null;
          currentTaskContent = [];
        }

        const sectionTitle = extractPlainText(heading).toLowerCase();
        if (sectionTitle === 'tasks' || sectionTitle === 'work items') {
          currentSection = 'tasks';
        } else {
          currentSection = 'root';
        }
      }

      // H3: Task headings
      if (heading.depth === 3 && currentSection === 'tasks') {
        // Save previous task if any
        if (currentTaskHeading) {
          saveTask(currentTaskHeading, currentTaskContent, sourcePath, structure);
          currentTaskHeading = null;
          currentTaskContent = [];
        }

        // Start new task
        currentTaskHeading = {
          heading,
          lineNumber: node.position?.start.line ?? 0,
        };
        currentTaskContent = [];
      }
    }

    // Collect content for current task
    if (currentTaskHeading && (node.type === 'paragraph' || node.type === 'list')) {
      currentTaskContent.push(node as Paragraph | List);
    }
  });

  // Save last task if any
  if (currentTaskHeading) {
    saveTask(currentTaskHeading, currentTaskContent, sourcePath, structure);
  }

  return structure;
}

/**
 * Helper to save a task from heading and content
 */
function saveTask(
  taskHeading: TaskHeading,
  taskContent: Array<Paragraph | List>,
  sourcePath: string | undefined,
  structure: DocumentStructure
): void {
  try {
    const task = parseTask(taskHeading.heading, taskContent, sourcePath, taskHeading.lineNumber);
    structure.tasks.push(task);
  } catch (error) {
    if (error instanceof ParseError) {
      throw error;
    }
    throw new ParseError(
      `Failed to parse task: ${error instanceof Error ? error.message : String(error)}`,
      sourcePath
    );
  }
}

/**
 * Extract plain text from heading or paragraph node
 */
function extractPlainText(node: Heading | Paragraph): string {
  let text = '';

  visit(node, 'text', (textNode) => {
    text += (textNode as { value: string }).value;
  });

  return text;
}

/**
 * Parse module metadata line (immediately after H1)
 * Format: **Scope:** AUTH **Owner:** @alice **Priority:** high
 */
function parseMetadataLine(para: Paragraph): ModuleMetadata {
  const metadata: ModuleMetadata = {};
  let currentKey = '';
  let currentValue = '';

  for (const child of para.children) {
    if (child.type === 'strong') {
      // Save previous field (even if value is empty, so handlers like Packages can default)
      if (currentKey) {
        assignMetadataField(metadata, currentKey, currentValue.trim());
      }

      // Extract new field key
      let strongText = '';
      visit(child as Strong, 'text', (textNode) => {
        strongText += (textNode as { value: string }).value;
      });
      const match = strongText.match(/^(\w+):$/);
      if (match) {
        currentKey = match[1];
        currentValue = '';
      }
    } else if (child.type === 'text' && currentKey) {
      currentValue += (child as { value: string }).value;
    }
  }

  // Save last field (even if value is empty)
  if (currentKey) {
    assignMetadataField(metadata, currentKey, currentValue.trim());
  }

  return metadata;
}

/**
 * Assign metadata field value
 */
function assignMetadataField(metadata: ModuleMetadata, key: string, value: string): void {
  switch (key) {
    case 'Scope':
    case 'ID':
      // Support both 'Scope:' and 'ID:' per current APS spec
      metadata.scope = value;
      break;
    case 'Owner':
      metadata.owner = value;
      break;
    case 'Status': {
      // Normalise status values to match ModuleStatusSchema
      // Legacy values are mapped to canonical equivalents: Draft→Proposed, Complete→Done
      const statusMap: Record<string, ModuleMetadata['status']> = {
        Draft: 'Proposed',
        Proposed: 'Proposed',
        Ready: 'Ready',
        'In Progress': 'In Progress',
        Complete: 'Done',
        Done: 'Done',
        Blocked: 'Blocked',
      };
      const mapped = statusMap[value.trim()];
      if (mapped) {
        metadata.status = mapped;
      }
      break;
    }
    case 'Priority':
      if (value === 'low' || value === 'medium' || value === 'high') {
        metadata.priority = value;
      }
      break;
    case 'Tags':
      metadata.tags = value.split(',').map((t) => t.trim());
      break;
    case 'Dependencies':
      metadata.dependencies = value.split(',').map((d) => d.trim());
      break;
    case 'Packages': {
      // Monorepo support: list of affected packages
      const trimmed = value.trim();
      if (!trimmed || trimmed.toLowerCase() === '(none)') {
        metadata.packages = [];
      } else {
        metadata.packages = trimmed
          .split(',')
          .map((p) => p.trim())
          .filter((p) => p.length > 0);
      }
      break;
    }
  }
}
