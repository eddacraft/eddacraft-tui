/**
 * `.anvil` file format — schemas, parser, compiler.
 *
 * Phase 1 of ANVFMT: turns the family/rule `.anvil` source tree into a
 * compiled pattern registry the scanner can load at runtime without needing
 * YAML or markdown support in hot paths. Phase 2 wires the scanner to the
 * compiled registry.
 */

export {
  ArtifactTypeSchema,
  type ArtifactType,
  AnvilSeveritySchema,
  type AnvilSeverity,
  AnvilConfidenceSchema,
  type AnvilConfidence,
  KNOWN_CATEGORIES,
  type KnownCategory,
  DetectionSchema,
  type Detection,
  DefinitionFrontmatterSchema,
  type DefinitionFrontmatter,
  RuleFrontmatterSchema,
  type RuleFrontmatter,
  AnvilFrontmatterSchema,
  type AnvilFrontmatter,
  CompiledPatternSchema,
  type CompiledPattern,
  PrefixRegistrySchema,
  type PrefixRegistry,
  CompiledRegistrySchema,
  type CompiledRegistry,
} from './schemas.js';

export {
  extractSections,
  validateDefinitionSections,
  getExplanation,
  getSuggestion,
  REQUIRED_DEFINITION_SECTIONS,
  type MarkdownSection,
  type MarkdownSections,
  type DefinitionSectionValidation,
  type RequiredDefinitionSection,
} from './sections.js';

export {
  AnvilParseError,
  parseAnvilSource,
  type ParsedAnvilFile,
  type ParsedDefinitionFile,
  type ParsedRuleFile,
} from './parse.js';

export {
  compilePatterns,
  compileAndWrite,
  discoverAnvilFiles,
  type AnvilCompileOptions,
  type AnvilCompileIssue,
  type AnvilCompileResult,
} from './compile.js';
