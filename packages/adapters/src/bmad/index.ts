/**
 * BMAD Adapter Module
 *
 * Export all BMAD adapter functionality.
 */

export { BMADFormatAdapter, createBMADAdapter } from './format-adapter.js';
export type {
  BMADDocument,
  BMADDocumentType,
  BMADRequirement,
  BMADUserStory,
  BMADFrontMatter,
  BMADChangeLogEntry,
  RequirementType,
  DetectionIndicators,
} from './types.js';
export { parseBMAD, parseBMADDocument, bmadToAPS } from './parser.js';
export { serializeToBMAD } from './serializer.js';
export * from './utils.js';
