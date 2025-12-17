/**
 * Parser module - Markdown parsing for APS documents
 *
 * @example
 * ```typescript
 * import { parseDocument, parseIndex } from '@anvil/aps/parser';
 *
 * // Parse a leaf spec (tasks)
 * const leafContent = await fs.readFile('feature.aps.md', 'utf-8');
 * const doc = await parseDocument(leafContent, 'feature.aps.md');
 * console.log(doc.tasks.length); // 8
 *
 * // Parse an index file (modules)
 * const indexContent = await fs.readFile('plan/APS.md', 'utf-8');
 * const index = await parseIndex(indexContent, 'plan/APS.md');
 * console.log(index.modules.length); // 4
 * ```
 */

export { parseDocument } from './parse-document.js';
export { parseIndex, type ParsedIndex } from './parse-index.js';
export { parseTask, parseTaskHeading, parseTaskFields } from './parse-task.js';
export type {
  Task,
  ParsedDocument,
  ModuleMetadata,
  Confidence,
  TaskStatus,
} from '../types/index.js';
export { ParseError } from '../types/index.js';
