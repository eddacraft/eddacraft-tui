export {
  WARNING_ID_PATTERN,
  ParsedWarningIdSchema,
  type ParsedWarningId,
  generateWarningId,
  createWarningId,
  parseWarningId,
  isValidWarningId,
  findWarningById,
  findWarningsByRule,
  findWarningsByFile,
  indexWarningsById,
  getWarningIds,
  generateShortId,
  resolveShortId,
} from './warning-id.js';

export type { Location, Warning, WarningSeverity, WarningCategory, Confidence } from './types.js';
