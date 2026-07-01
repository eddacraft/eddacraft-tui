/**
 * Zod schemas for `.anvil` file frontmatter.
 *
 * Two file types share the format, discriminated by the `type` field:
 *   - `definition` — family definition. Body is rich narrative markdown.
 *   - `rule`       — detection rule. Body is the nudge text.
 */

import { z } from 'zod';

// =============================================================================
// Shared vocabulary
// =============================================================================

export const ArtifactTypeSchema = z.enum([
  'source',
  'pr-description',
  'commit-message',
  'agent-output',
]);
export type ArtifactType = z.infer<typeof ArtifactTypeSchema>;

export const AnvilSeveritySchema = z.enum(['error', 'warning', 'info']);
export type AnvilSeverity = z.infer<typeof AnvilSeveritySchema>;

export const AnvilConfidenceSchema = z.enum(['high', 'medium', 'low']);
export type AnvilConfidence = z.infer<typeof AnvilConfidenceSchema>;

// Known family categories. New families may introduce new categories; the
// schema accepts any non-empty string so the compiler does not block on
// category extension. The enum of well-known values is exported for callers
// that want to branch on known categories.
export type KnownCategory =
  | 'escape-hatch'
  | 'type-evasion'
  | 'error-handling'
  | 'accountability'
  | 'deferred-debt'
  | 'insecure-construction';

export const KNOWN_CATEGORIES: readonly KnownCategory[] = [
  'escape-hatch',
  'type-evasion',
  'error-handling',
  'accountability',
  'deferred-debt',
  // ADR-087 / INSEC-001: security-class families (weak-cryptography,
  // unsafe-rendering). Kept as a *known* category so the Rust scanner maps
  // it to a first-class variant instead of the `code-quality` fallback.
  'insecure-construction',
];

const CategorySchema = z
  .string()
  .min(1)
  .regex(/^[a-z][a-z0-9-]*$/, 'category must be kebab-case');

const FamilyIdSchema = z
  .string()
  .min(1)
  .regex(/^[a-z][a-z0-9-]*$/, 'family id must be kebab-case');

const RuleIdSchema = z
  .string()
  .min(1)
  .regex(/^[A-Z]{2,5}-\d{3}$/, 'rule id must match <PREFIX>-<NNN> (e.g. AP-001)');

// =============================================================================
// Detection configuration
// =============================================================================

// Only JS-legal regex flag characters — rejects typos like 'x' or 'gi ms'
// at build time rather than at scanner startup.
const REGEX_FLAGS = /^[gimsuyvd]*$/;

const RegexDetectionSchema = z
  .object({
    type: z.literal('regex'),
    pattern: z.string().min(1),
    flags: z
      .string()
      .regex(REGEX_FLAGS, 'flags must contain only JS regex flag characters (gimsuyvd)')
      .optional(),
  })
  .superRefine((val, ctx) => {
    try {
      RegExp(val.pattern, val.flags ?? '');
    } catch (err) {
      ctx.addIssue({
        code: 'custom',
        path: ['pattern'],
        message: `invalid regex: ${err instanceof Error ? err.message : String(err)}`,
      });
    }
  });

const AstDetectionSchema = z.object({
  type: z.literal('ast'),
  ast_query: z.string().min(1),
});

export const DetectionSchema = z.discriminatedUnion('type', [
  RegexDetectionSchema,
  AstDetectionSchema,
]);
export type Detection = z.infer<typeof DetectionSchema>;

// =============================================================================
// Definition frontmatter
// =============================================================================

export const DefinitionFrontmatterSchema = z.object({
  id: FamilyIdSchema,
  type: z.literal('definition'),
  name: z.string().min(1),
  category: CategorySchema,
  targets: z.array(ArtifactTypeSchema).nonempty(),
  related: z.array(FamilyIdSchema).default([]),
  tensions: z.array(FamilyIdSchema).default([]),
  // If omitted, the compiler derives it from sibling rule files.
  rules: z.array(RuleIdSchema).optional(),
});
export type DefinitionFrontmatter = z.infer<typeof DefinitionFrontmatterSchema>;

// =============================================================================
// Rule frontmatter
// =============================================================================

export const RuleFrontmatterSchema = z.object({
  id: RuleIdSchema,
  type: z.literal('rule'),
  family: FamilyIdSchema,
  title: z.string().min(1),
  version: z.number().int().positive(),

  severity: AnvilSeveritySchema,
  confidence: AnvilConfidenceSchema,
  spectrum_position: z.number().int().positive(),

  targets: z.array(ArtifactTypeSchema).nonempty(),

  detection: DetectionSchema,

  file_extensions: z
    .array(z.string().regex(/^\.[a-z0-9]+$/, 'file extensions must be lowercase'))
    .optional(),
  allowlist: z.array(z.string()).default([]),

  related: z.array(RuleIdSchema).default([]),

  enabled: z.boolean().default(true),
  opt_in: z.boolean().default(false),
});
export type RuleFrontmatter = z.infer<typeof RuleFrontmatterSchema>;

// =============================================================================
// Discriminated file-level schema
// =============================================================================

export const AnvilFrontmatterSchema = z.discriminatedUnion('type', [
  DefinitionFrontmatterSchema,
  RuleFrontmatterSchema,
]);
export type AnvilFrontmatter = z.infer<typeof AnvilFrontmatterSchema>;

// =============================================================================
// Compiled pattern registry
// =============================================================================

/**
 * The hydrated shape that the scanner consumes. Combines rule detection
 * mechanics with narrative context pulled from the family definition, so a
 * warning emitted from a rule can be rendered without needing to resolve the
 * definition file at runtime.
 */
export const CompiledPatternSchema = z.object({
  // From rule frontmatter
  id: RuleIdSchema,
  family: FamilyIdSchema,
  title: z.string().min(1),
  version: z.number().int().positive(),
  severity: AnvilSeveritySchema,
  confidence: AnvilConfidenceSchema,
  spectrum_position: z.number().int().positive(),
  targets: z.array(ArtifactTypeSchema).nonempty(),
  detection: DetectionSchema,
  file_extensions: z.array(z.string()).optional(),
  allowlist: z.array(z.string()),
  nudge: z.string().min(1),
  related: z.array(RuleIdSchema),
  enabled: z.boolean(),
  opt_in: z.boolean(),

  // From family definition
  family_name: z.string().min(1),
  category: CategorySchema,
  explanation: z.string().min(1),
  suggestion: z.string().min(1),
  definition_ref: z.string().min(1),
  tensions: z.array(FamilyIdSchema),
  related_families: z.array(FamilyIdSchema),
});
export type CompiledPattern = z.infer<typeof CompiledPatternSchema>;

export const PrefixRegistrySchema = z.record(z.string().regex(/^[A-Z]{2,5}$/), FamilyIdSchema);
export type PrefixRegistry = z.infer<typeof PrefixRegistrySchema>;

export const CompiledRegistrySchema = z.object({
  /** Bumped when the compiler output shape changes. */
  schema_version: z.literal(1),
  /** ISO-8601 timestamp at compile time (UTC). */
  compiled_at: z
    .string()
    .regex(
      /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z$/,
      'compiled_at must be an ISO-8601 UTC timestamp'
    ),
  /** Source directory the compiler walked, relative to invocation cwd. */
  source_root: z.string(),
  /** Alphabetically sorted by rule id. */
  patterns: z.array(CompiledPatternSchema),
  /** Map of rule prefix (e.g. "GS") → family id. */
  prefixes: PrefixRegistrySchema,
  /**
   * Families declared by the set of compiled definitions — useful for tools
   * that want to enumerate or navigate family definitions.
   */
  families: z.array(
    z.object({
      id: FamilyIdSchema,
      name: z.string().min(1),
      category: CategorySchema,
      definition_ref: z.string().min(1),
      rules: z.array(RuleIdSchema),
      related: z.array(FamilyIdSchema),
      tensions: z.array(FamilyIdSchema),
    })
  ),
});
export type CompiledRegistry = z.infer<typeof CompiledRegistrySchema>;
