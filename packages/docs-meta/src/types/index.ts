/**
 * Documentation governance type definitions and schemas
 *
 * Captures the DOCGOV-002 metadata convention defined in
 * `docs/guides/documentation-governance.md`: a five-column metadata table
 * and a two-column Upstream/Downstream relationships table, both declared
 * immediately after the H1 title of a governed document.
 */

import { z } from 'zod';

/**
 * Document type vocabulary.
 * Sourced from `docs/guides/documentation-governance.md` § Document Types.
 */
export const DocTypeSchema = z.enum([
  'APS index',
  'APS module',
  'ADR',
  'Spec',
  'As-built',
  'Runbook',
  'Guide',
  'README',
  'Public docs',
  'Archive',
]);
export type DocType = z.infer<typeof DocTypeSchema>;

/**
 * Authority vocabulary — describes how strongly the document binds readers.
 */
export const DocAuthoritySchema = z.enum(['Authoritative', 'Derived', 'Advisory', 'Historical']);
export type DocAuthority = z.infer<typeof DocAuthoritySchema>;

/**
 * Lifecycle status of the document itself (not of the work it describes).
 */
export const DocStatusSchema = z.enum([
  'Draft',
  'Proposed',
  'Ready',
  'Live',
  'Deprecated',
  'Archived',
]);
export type DocStatus = z.infer<typeof DocStatusSchema>;

/**
 * Five-column metadata table — the first table after the H1.
 *
 * `owner` and `freshness` are deliberately free-text: ownership conventions
 * differ across teams, and freshness anchors vary by document type (tag, SHA,
 * source path, release, dry-run date). The governance guide documents the
 * expected shape but the parser does not enforce it.
 */
export const DocMetadataSchema = z.object({
  type: DocTypeSchema,
  authority: DocAuthoritySchema,
  owner: z.string().min(1),
  status: DocStatusSchema,
  freshness: z.string().min(1),
});
export type DocMetadata = z.infer<typeof DocMetadataSchema>;

/**
 * Upstream/Downstream relationships table — the second table after the H1.
 *
 * Cell contents are comma-separated lists of references (paths, doc names,
 * skill names). Empty cells produce empty arrays so callers can treat the
 * shape uniformly.
 */
export const DocRelationsSchema = z.object({
  upstream: z.array(z.string()),
  downstream: z.array(z.string()),
});
export type DocRelations = z.infer<typeof DocRelationsSchema>;

export const DocFreshnessSchema = z.object({
  reviewedOn: z.string().optional(),
  anchors: z.array(z.string()),
});
export type DocFreshness = z.infer<typeof DocFreshnessSchema>;

export const DocSourceReferenceSchema = z.object({
  path: z.string().min(1),
  line: z.number().optional(),
  context: z.enum(['freshness', 'upstream', 'downstream', 'body']),
});
export type DocSourceReference = z.infer<typeof DocSourceReferenceSchema>;

/**
 * Full parsed governance result for a single document.
 */
export const DocGovernanceSchema = z.object({
  title: z.string().min(1),
  metadata: DocMetadataSchema,
  relations: DocRelationsSchema,
  freshness: DocFreshnessSchema,
  sourceReferences: z.array(DocSourceReferenceSchema),
  sourcePath: z.string().optional(),
  sourceLineNumber: z.number().optional(),
});
export type DocGovernance = z.infer<typeof DocGovernanceSchema>;

/**
 * Parser error with file:line context.
 *
 * Mirrors `ParseError` in `@eddacraft/anvil-aps` so consumers that ingest
 * both libraries see the same shape.
 */
export class ParseError extends Error {
  constructor(
    message: string,
    public readonly sourcePath?: string,
    public readonly lineNumber?: number,
    public readonly context?: string
  ) {
    super(message);
    this.name = 'ParseError';
  }
}
