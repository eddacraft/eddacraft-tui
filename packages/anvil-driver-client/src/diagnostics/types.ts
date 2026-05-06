/**
 * Canonical inner `Diagnostic` shape — `anvil.diagnostic.v1`.
 *
 * This file MIRRORS the Rust authoritative type at
 * `crates/anvil-kernel-types/src/diagnostics.rs`. The Rust side is
 * authoritative; if the two drift, the Rust side wins and this file is
 * updated to match. Field names and JSON serialisation are pinned by the
 * coordination spec at `plans/specs/2026-04-26-diagnostic-envelope-coordination.md`.
 *
 * The driver-client mirrors these types inline rather than depending on
 * a generated TS-from-Rust contracts package because no such bridge
 * exists in this repo today. When one lands, this module switches to
 * importing from it without changing the public surface — consumers
 * import from `@eddacraft/anvil-driver-client` either way.
 *
 * DO NOT extend these types with Anvil-specific fields the wire spec
 * does not declare; doing so risks producing diagnostics the daemon
 * (or other consumers) cannot deserialise.
 */

/**
 * Schema-version constant for the inner diagnostic shape. Distinct
 * from any outer envelope `schema` field. Bumps to
 * `anvil.diagnostic.v2` only on breaking changes; additive evolution
 * stays on `v1` per the spec's versioning rules.
 */
export const DIAGNOSTIC_SCHEMA_VERSION = 'anvil.diagnostic.v1' as const;

/** Rule severity — distinct from the control decision. */
export type Severity = 'info' | 'warning' | 'error';

/**
 * Coarse routing/filtering grouping. Closed list per the spec — new
 * values require a spec amendment before producers emit them.
 *
 * Unknown values arriving on the wire MUST be surfaced (treated as
 * `other`) rather than dropped, per the spec's forward-compat rule.
 */
export type Category =
  | 'secret'
  | 'antipattern'
  | 'boundary'
  | 'policy'
  | 'reasoning'
  | 'command-safety'
  | 'architecture'
  | 'other';

/**
 * Mode discriminator. Identifies which path produced the diagnostic
 * and the consumer expectation.
 *
 * `Mode` is `string` rather than a closed union so the client can
 * forward unknown modes a future producer introduces (e.g.
 * `remote-edit`) without dropping the diagnostic. Branch on
 * `KNOWN_MODES.includes(mode)` for known-mode handling.
 */
export type KnownMode = 'save-time' | 'mid-edit' | 'gate' | 'watch';
export type Mode = KnownMode | (string & {});

export const KNOWN_MODES: readonly KnownMode[] = [
  'save-time',
  'mid-edit',
  'gate',
  'watch',
] as const;

/**
 * File anchor for a diagnostic. `line`/`column` are 1-based when
 * present; path-only rules and deleted-file diagnostics omit them per
 * the envelope spec ("`line` may be `null`"). `end_line` /
 * `end_column` are optional and span the end of the flagged region
 * when present.
 */
export interface DiagnosticLocation {
  /** Workspace-relative path emitted by the daemon (post-redaction
   *  for MCP responses; absolute paths are never crossed by the daemon
   *  through the redaction contract). */
  file: string;
  line?: number;
  column?: number;
  end_line?: number;
  end_column?: number;
}

/** Provenance for a diagnostic. */
export interface DiagnosticSource {
  /** Stable rule id across Anvil. */
  rule_id: string;
  /** Producing crate or sub-module (e.g. `anvil-checks::secrets`). */
  source_module: string;
}

/**
 * Canonical inner diagnostic shape — `anvil.diagnostic.v1`.
 *
 * Fields and ordering mirror
 * `crates/anvil-kernel-types/src/diagnostics.rs::Diagnostic`. JSON
 * field names use snake_case to match the Rust serde defaults — do
 * not switch to camelCase even though the surrounding TS code uses it.
 */
export interface Diagnostic {
  /** Always `"anvil.diagnostic.v1"` for v1 producers. */
  schema_version: typeof DIAGNOSTIC_SCHEMA_VERSION | string;
  /** ULID minted by the producing rule run. */
  id: string;
  severity: Severity;
  /** ≤ 200 chars per the spec. */
  summary: string;
  location: DiagnosticLocation;
  category: Category | string;
  source: DiagnosticSource;
  remediation_hint?: string;
  mode: Mode;
}
