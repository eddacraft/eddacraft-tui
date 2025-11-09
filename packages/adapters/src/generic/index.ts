/**
 * Generic Markdown Adapter
 *
 * Exports for generic markdown format adapter.
 */

export { GenericMarkdownAdapter, createGenericMarkdownAdapter } from './format-adapter.js';
export type { GenericDocument, GenericIndicators } from './types.js';
export { parseGeneric } from './parser.js';
export { serializeToGeneric } from './serializer.js';
// Note: Utils are not exported to avoid conflicts with BMAD utils
