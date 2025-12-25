export {
  parseSuppressions,
  isExpired,
  suppressionMatches,
  findMatchingSuppression,
  ParsedSuppressionSchema,
  type ParsedSuppression,
  type SuppressionScope,
  type SuppressionParseError,
  type ParseResult,
} from './parser.js';

export {
  SuppressionStore,
  SuppressionStoreDataSchema,
  type SuppressionStoreData,
  type SuppressionMatch,
} from './store.js';

export { SuppressionService, type SuppressionStats, type FileSuppressions } from './service.js';
