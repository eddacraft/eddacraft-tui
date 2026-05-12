/**
 * @eddacraft/anvil-docs-meta — Documentation governance metadata library
 *
 * Parses the DOCGOV-002 metadata convention (five-column metadata table plus
 * Upstream/Downstream relationships table) declared immediately after the H1
 * of governed Markdown documents. Consumed by the DOCGOV-005 `docs:check`
 * orchestrator and reused by DOCGOV-006 (as-built freshness) and DOCGOV-007
 * (generated indexes).
 *
 * The canonical convention lives in `docs/guides/documentation-governance.md`.
 *
 * @module @eddacraft/anvil-docs-meta
 */

export * from './parser/index.js';
export * from './types/index.js';
