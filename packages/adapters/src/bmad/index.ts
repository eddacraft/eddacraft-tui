/**
 * BMAD Adapter Module
 *
 * Export all BMAD adapter functionality.
 */

export { BMADFormatAdapter, createBMADAdapter } from './format-adapter.js';
export { BMAD_FOLDERS, BMAD_UPSTREAM_VERSION } from './types.js';
export type {
  BMADDocument,
  BMADDocumentType,
  BMADRequirement,
  BMADUserStory,
  BMADFrontMatter,
  BMADChangeLogEntry,
  RequirementType,
  DetectionIndicators,
  BMADAgentYaml,
  BMADAgentMetadata,
  BMADAgentPersona,
  BMADMenuItem,
  BMADAgentPrompt,
  BMADWorkflowYaml,
  BMADTeamBundle,
  BMADTeamYaml,
  BMADModuleYaml,
} from './types.js';
export { parseBMAD, parseBMADDocument, bmadToAPS } from './parser.js';
export { serializeToBMAD } from './serializer.js';
export * from './utils.js';
