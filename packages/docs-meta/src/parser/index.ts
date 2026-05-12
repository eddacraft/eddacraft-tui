/**
 * Parser module — extract DOCGOV-002 metadata from Markdown documents.
 *
 * @example
 * ```typescript
 * import { parseDocGovernance } from '@eddacraft/anvil-docs-meta/parser';
 *
 * const result = parseDocGovernance(content, 'docs/guides/example.md');
 * result.metadata.type;    // 'Guide'
 * result.relations.upstream; // ['plans/modules/documentation-governance.aps.md', ...]
 * ```
 */

export { parseDocGovernance } from './parse-metadata.js';
export type {
  DocGovernance,
  DocMetadata,
  DocRelations,
  DocType,
  DocAuthority,
  DocStatus,
} from '../types/index.js';
export { ParseError } from '../types/index.js';
