/**
 * Warning types — minimal active surface
 *
 * Extracted from the archived `core/antipattern/types.ts` (under
 * ADR-033, 2026-04-29) so active consumers (`warnings/warning-id`,
 * `explain/explain-service`, etc.) keep a typed handle on warnings
 * produced by the Rust scanner.
 *
 * The full zod schemas, fingerprint utilities, and producer types
 * (AntiPattern, ScanResult) live in the archive and are not
 * reproduced here. This file only carries the consumer-side
 * shape.
 */

export interface Location {
  file: string;
  line: number;
  column?: number;
  endLine?: number;
  endColumn?: number;
}

export type WarningSeverity = 'error' | 'warning' | 'info';
export type WarningCategory = 'anti-pattern' | 'boundary' | 'architecture';
export type Confidence = 'high' | 'medium' | 'low';

/**
 * A warning emitted by the scanner.
 *
 * Field coverage matches what active TS consumers read; the Rust
 * scanner emits a superset of these fields. Optional fields are
 * marked optional regardless of whether the producer always sets
 * them — consumers must tolerate missing values.
 */
export interface Warning {
  id: string;
  fingerprint?: string;
  category: WarningCategory;
  severity: WarningSeverity;
  confidence: Confidence;
  title: string;
  message: string;
  pattern?: string;
  explanation?: string;
  location: Location;
  source?: string;
  suggestion?: string;
  links?: string[];
  [key: string]: unknown;
}

/**
 * Per-severity counters that accompany a `WarningResult`.
 */
export interface WarningSummary {
  total: number;
  errors: number;
  warnings: number;
  info: number;
  suppressed: number;
}

/**
 * Collection of warnings from a check run — embeds in `GateResult.details`
 * via the `ports` `CheckDetails` shape.
 *
 * Matches the archived zod-derived `WarningResultSchema` from
 * `anvil-archive/anvil-ts-scanner/core-antipattern/types.ts`. Carried here as a
 * plain interface so active consumers (`@eddacraft/anvil-ports`,
 * runtime gate adapters) can refer to the shape without pulling zod.
 */
export interface WarningResult {
  warnings: Warning[];
  summary: WarningSummary;
  patterns_checked: string[];
}
