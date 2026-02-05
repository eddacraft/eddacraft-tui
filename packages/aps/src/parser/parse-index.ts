/**
 * Index file parsing utilities
 * Parses APS index files that organise multiple leaf specs
 */

import { unified } from 'unified';
import remarkParse from 'remark-parse';
import { visit } from 'unist-util-visit';
import type { Root, Heading, List, ListItem, Paragraph, Link } from 'mdast';
import { ParseError, type ModuleMetadata, type Priority } from '../types/index.js';

/**
 * Parsed index file result
 */
export interface ParsedIndex {
  /** Plan title from H1 */
  title: string;

  /** Overview text (optional) */
  overview?: string;

  /** Module definitions */
  modules: ModuleMetadata[];

  /** Open questions (optional) */
  openQuestions?: string[];

  /** Decisions with dates (optional) */
  decisions?: string[];

  /** Source file path */
  sourcePath?: string;
}

/**
 * Parse an APS index file from Markdown content
 *
 * @param content - Markdown content
 * @param sourcePath - Optional source file path for error reporting
 * @returns Parsed index with modules
 */
export async function parseIndex(content: string, sourcePath?: string): Promise<ParsedIndex> {
  const processor = unified().use(remarkParse);
  const ast = processor.parse(content) as Root;

  const result: ParsedIndex = {
    title: '',
    modules: [],
    sourcePath,
  };

  let currentSection: 'root' | 'overview' | 'modules' | 'questions' | 'decisions' = 'root';
  let currentModuleId: string | null = null;
  let currentModuleContent: List[] = [];

  visit(ast, (node) => {
    // H1: Plan title
    if (node.type === 'heading' && (node as Heading).depth === 1) {
      result.title = extractPlainText(node as Heading);
    }

    // H2: Section headers
    if (node.type === 'heading' && (node as Heading).depth === 2) {
      // Save previous module if any
      if (currentModuleId && currentModuleContent.length > 0) {
        const moduleMetadata = parseModuleMetadata(currentModuleId, currentModuleContent);
        result.modules.push(moduleMetadata);
        currentModuleId = null;
        currentModuleContent = [];
      }

      const sectionTitle = extractPlainText(node as Heading).toLowerCase();

      if (sectionTitle === 'overview') {
        currentSection = 'overview';
      } else if (sectionTitle === 'modules') {
        currentSection = 'modules';
      } else if (sectionTitle === 'open questions') {
        currentSection = 'questions';
      } else if (sectionTitle === 'decisions') {
        currentSection = 'decisions';
      } else {
        currentSection = 'root';
      }
    }

    // H3: Module headings (within Modules section)
    if (node.type === 'heading' && (node as Heading).depth === 3 && currentSection === 'modules') {
      // Save previous module if any
      if (currentModuleId && currentModuleContent.length > 0) {
        const moduleMetadata = parseModuleMetadata(currentModuleId, currentModuleContent);
        result.modules.push(moduleMetadata);
      }

      currentModuleId = extractPlainText(node as Heading);
      currentModuleContent = [];
    }

    // Collect lists for current module
    if (node.type === 'list' && currentSection === 'modules' && currentModuleId) {
      currentModuleContent.push(node as List);
    }

    // Collect overview paragraph
    if (node.type === 'paragraph' && currentSection === 'overview') {
      result.overview = extractPlainText(node as Paragraph);
    }

    // Collect open questions from list
    if (node.type === 'list' && currentSection === 'questions') {
      result.openQuestions = extractListItemsAsStrings(node as List);
    }

    // Collect decisions from list
    if (node.type === 'list' && currentSection === 'decisions') {
      result.decisions = extractListItemsAsStrings(node as List);
    }
  });

  // Save last module if any
  if (currentModuleId && currentModuleContent.length > 0) {
    const moduleMetadata = parseModuleMetadata(currentModuleId, currentModuleContent);
    result.modules.push(moduleMetadata);
  }

  // Validate
  if (!result.title) {
    throw new ParseError('Index file must have an H1 title', sourcePath);
  }

  return result;
}

/**
 * Parse module metadata from list items
 * Format: - **Field:** value
 */
function parseModuleMetadata(moduleId: string, lists: List[]): ModuleMetadata {
  const metadata: ModuleMetadata = {
    id: moduleId,
  };

  for (const list of lists) {
    for (const item of list.children) {
      if (item.type !== 'listItem') continue;

      const { key, value } = extractFieldFromListItem(item as ListItem);
      if (!key) continue;

      switch (key) {
        case 'Path':
          metadata.path = value;
          break;
        case 'Scope':
        case 'ID':
          // Support both 'Scope:' and 'ID:' per current APS spec
          metadata.scope = value;
          break;
        case 'Owner':
          metadata.owner = value;
          break;
        case 'Status': {
          // Normalise status values - support both legacy and current spec values
          const normalizedStatus = value.trim();
          if (
            ['Draft', 'Proposed', 'Ready', 'In Progress', 'Complete', 'Done', 'Blocked'].includes(
              normalizedStatus
            )
          ) {
            metadata.status = normalizedStatus as ModuleMetadata['status'];
          }
          break;
        }
        case 'Priority':
          if (value === 'low' || value === 'medium' || value === 'high') {
            metadata.priority = value as Priority;
          }
          break;
        case 'Tags':
          metadata.tags = parseCommaSeparated(value);
          break;
        case 'Dependencies':
          if (value.toLowerCase() === '(none)' || value === '') {
            metadata.dependencies = [];
          } else {
            metadata.dependencies = parseCommaSeparated(value);
          }
          break;
        case 'Packages':
          // Monorepo support: list of affected packages
          if (value.toLowerCase() === '(none)' || value === '') {
            metadata.packages = [];
          } else {
            metadata.packages = parseCommaSeparated(value);
          }
          break;
      }
    }
  }

  return metadata;
}

/**
 * Extract field key and value from a list item
 * Format: **Key:** value or **Key:** [link](url)
 */
function extractFieldFromListItem(item: ListItem): { key: string; value: string } {
  let key = '';
  let value = '';

  for (const child of item.children) {
    if (child.type !== 'paragraph') continue;

    const para = child as Paragraph;
    let foundKey = false;

    for (const node of para.children) {
      if (node.type === 'strong') {
        const strongText = extractPlainTextFromNode(node);
        const match = strongText.match(/^(\w+):$/);
        if (match) {
          key = match[1];
          foundKey = true;
        }
      } else if (foundKey) {
        if (node.type === 'text') {
          value += (node as { value: string }).value;
        } else if (node.type === 'link') {
          // For Path field, extract the URL from the link
          const link = node as Link;
          value += link.url;
        }
      }
    }
  }

  return { key, value: value.trim() };
}

/**
 * Extract plain text from a heading or paragraph
 */
function extractPlainText(node: Heading | Paragraph): string {
  let text = '';
  visit(node, 'text', (textNode) => {
    text += (textNode as { value: string }).value;
  });
  return text;
}

/**
 * Extract plain text from any node
 */
function extractPlainTextFromNode(node: unknown): string {
  let text = '';
  visit(node as Root, 'text', (textNode) => {
    text += (textNode as { value: string }).value;
  });
  return text;
}

/**
 * Extract list items as strings
 */
function extractListItemsAsStrings(list: List): string[] {
  const items: string[] = [];

  for (const item of list.children) {
    if (item.type !== 'listItem') continue;

    let text = '';
    visit(item, 'text', (textNode) => {
      text += (textNode as { value: string }).value;
    });

    if (text.trim()) {
      items.push(text.trim());
    }
  }

  return items;
}

/**
 * Parse comma-separated list
 */
function parseCommaSeparated(value: string): string[] {
  return value
    .split(',')
    .map((item) => item.trim())
    .filter((item) => item.length > 0);
}
