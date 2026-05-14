/**
 * Mid-edit Kindling observation builder (MLP-016 / MLP2-030).
 *
 * This file MIRRORS the Rust authoritative implementation at
 * `crates/anvil-intercept/src/kindling_observation.rs`. The Rust
 * side is authoritative; if the two drift, the Rust side wins and
 * this file is updated to match. Wire shape, severity → enforcement
 * mapping, and the empty-diagnostics short-circuit are pinned by
 * the parity tests in `./types.test.ts`.
 *
 * ## Why a TS mirror exists
 *
 * The daemon emits `gate_evaluated` observations from the Rust
 * side, but the editor-side L1 surface (driver-client running in
 * the consumer's editor / language-server) needs to emit when the
 * daemon is unreachable and the embedded fallback path fires.
 * Both paths converge on the same Kindling `SQLite` writer (owned
 * by `packages/kindling-integration`), so the row shape must be
 * byte-compatible across producers.
 *
 * ## Volume-control contract
 *
 * **Pass-no-finding mid-edit calls remain silent.**
 * {@link fromMidEditResponse} returns `null` when the diagnostics
 * array is empty. The caller is free to invoke it on every scan;
 * the helper handles the empty-finding filter so the call site
 * does not have to track that policy independently. This mirrors
 * the Rust `from_midedit_response` returning `Option<...>`.
 *
 * ## Severity → enforcement mapping
 *
 * Kindling's `enforcement` field is a closed three-value enum
 * (`blocking` / `warning` / `informational`). Diagnostics carry
 * the richer `Severity` vocabulary (`info` / `warning` / `error`).
 * The mapping picks the most severe class in the batch:
 *
 * | Highest severity in batch | Kindling `enforcement` |
 * |---------------------------|------------------------|
 * | `error`                   | `blocking`             |
 * | `warning`                 | `warning`              |
 * | `info`                    | `informational`        |
 *
 * Empty diagnostics never produce an observation (volume-control
 * contract), so there is no `informational`-from-no-diagnostics
 * row in the resulting table.
 *
 * DO NOT extend these types with TS-side fields the Rust wire
 * shape does not declare; doing so risks emitting rows the
 * Rust side (or the shared Kindling SQLite consumer) cannot
 * deserialise.
 */

import type { Diagnostic, Severity } from '../diagnostics/index.js';

/**
 * Pinned Kindling observation kind. Schema-matches
 * `GateEvaluatedObservationSchema.kind` in
 * `packages/kindling-integration/src/observation-contract.ts` and
 * the Rust `KIND_GATE_EVALUATED` constant.
 */
export const KIND_GATE_EVALUATED = 'gate_evaluated' as const;

/**
 * Pinned `gate_id` for mid-edit findings. Distinguishes L1
 * mid-edit rows from save-time / pre-commit / pre-push / audit
 * rows that share the `gate_evaluated` kind but live at different
 * layers. Mirrors the Rust `MIDEDIT_GATE_ID` constant.
 */
export const MIDEDIT_GATE_ID = 'midEdit' as const;

/**
 * Kindling `enforcement` field — closed three-value enum, shared
 * with the Rust side. Lowercase / kebab-case to match the Rust
 * `#[serde(rename_all = "kebab-case")]` derivation.
 */
export type Enforcement = 'blocking' | 'warning' | 'informational';

/**
 * Kindling `outcome` field — closed four-value enum. v1 of this
 * helper only emits `fail` because the empty-diagnostics short-
 * circuit prevents the builder from running on a passing
 * evaluation. Mirrors the Rust `Outcome` enum verbatim.
 */
export type Outcome = 'pass' | 'fail' | 'error' | 'skipped';

/**
 * Inputs the caller supplies to scope each observation. Identifies
 * the session, time of emission, evaluation id, and the file
 * being evaluated.
 *
 * Kept as a separate type so the call site can build one once per
 * scan and pass it by reference. `ScanBufferResponse` does not
 * carry these fields because they're per-evaluation context the
 * driver's notification layer owns (`session_id`,
 * traceparent-derived `gate_eval_id`, etc).
 *
 * Mirrors the Rust `ObservationContext` struct field-for-field.
 */
export interface ObservationContext {
  /**
   * Session this observation belongs to. UUID v4 string per the
   * Zod `string().uuid()` contract.
   */
  readonly session_id: string;
  /**
   * ISO 8601 datetime — when the driver observed this scan
   * completing.
   */
  readonly timestamp: string;
  /**
   * Unique evaluation id for joining to traceparent logs.
   */
  readonly gate_eval_id: string;
  /**
   * File path being evaluated. Recorded in
   * `inputs.changed_files` (paths-only; no content per the Zod
   * "no sensitive data" sanitisation requirement).
   */
  readonly file_path: string;
  /**
   * Wall-clock duration the driver spent on the underlying
   * validation call (the `validation.service` boundary per
   * ADR-031). Held as a `number` (JS double) — safe for the
   * `2^53 - 1` ms budget (~285 thousand years).
   */
  readonly duration_ms: number;
}

/**
 * Nested `inputs` object on a `gate_evaluated` observation.
 * Matches `GateEvaluatedObservationSchema.inputs` from the TS
 * contract. `baseline_hash` is reserved for save-time / pre-
 * commit emitters and omitted from mid-edit rows.
 */
export interface ObservationInputs {
  readonly file_count: number;
  readonly changed_files: readonly string[];
  /** Optional. Omitted (key absent on the wire) for mid-edit rows. */
  readonly baseline_hash?: string;
}

/**
 * Kindling `gate_evaluated` observation payload. JSON wire shape
 * matches `GateEvaluatedObservationSchema` from the TS contract:
 * `snake_case` keys, kebab-case enum values, and the same
 * optional / required field policy. Optional fields are omitted
 * from the wire when unset (TS `undefined`), not serialised as
 * `null` — matching the Rust `#[serde(skip_serializing_if =
 * "Option::is_none")]` policy.
 */
export interface GateEvaluatedObservation {
  readonly kind: typeof KIND_GATE_EVALUATED;
  readonly session_id: string;
  readonly timestamp: string;
  readonly gate_eval_id: string;
  readonly gate_id: typeof MIDEDIT_GATE_ID;
  readonly inputs: ObservationInputs;
  readonly outcome: Outcome;
  readonly rules_evaluated: readonly string[];
  /** Optional. Omitted when no rules were violated. */
  readonly rules_violated?: readonly string[];
  readonly enforcement: Enforcement;
  readonly duration_ms: number;
  /** Optional. Always emitted for mid-edit rows post-MLP-016. */
  readonly violation_count?: number;
  /** Optional. Always emitted for mid-edit rows post-MLP-016. */
  readonly warning_count?: number;
}

/**
 * Minimal subset of {@link Diagnostic} this helper actually reads.
 * Lifting the constraint via an interface (rather than reusing
 * `Diagnostic` directly) means the helper can be invoked with
 * either the full driver-client `Diagnostic` shape or any shape
 * carrying just the two fields we touch. Keeps the public surface
 * unsurprising for embedded-fallback callers that produce their
 * own diagnostic-like objects.
 */
export interface DiagnosticLike {
  readonly severity: Severity;
  readonly source: { readonly rule_id: string };
}

/**
 * Minimal shape of the `scan_buffer` response this builder reads.
 * The driver-client's full {@link ScanBufferResponse} is a
 * supertype; this narrows to just the field the helper consumes.
 */
export interface MidEditResponseLike {
  readonly diagnostics: readonly DiagnosticLike[];
}

/**
 * Convert a mid-edit response into a Kindling `gate_evaluated`
 * observation, returning `null` when the response has no
 * diagnostics (volume-control contract).
 *
 * The caller supplies the per-evaluation {@link ObservationContext}
 * (session id, timestamp, etc) so this helper stays a pure
 * converter — testable without a clock or UUID source.
 *
 * Mirrors the Rust `from_midedit_response` returning
 * `Option<GateEvaluatedObservation>` — `null` here corresponds to
 * Rust `None` and `GateEvaluatedObservation` corresponds to
 * Rust `Some(...)`.
 */
export function fromMidEditResponse(
  ctx: ObservationContext,
  response: MidEditResponseLike
): GateEvaluatedObservation | null {
  if (response.diagnostics.length === 0) {
    return null;
  }

  const enforcement = enforcementFor(response.diagnostics);
  const { violationCount, warningCount } = countsFor(response.diagnostics);
  const rulesEvaluated = response.diagnostics.map((d) => d.source.rule_id);
  const rulesViolated = response.diagnostics
    .filter((d) => d.severity === 'error' || d.severity === 'warning')
    .map((d) => d.source.rule_id);

  // Construct fields in the same order the Rust serde derive emits
  // them so `JSON.stringify` produces a row a reader keyed on field
  // order (parser logs, diff tooling) sees identically across
  // languages. Omitted optional fields are dropped via conditional
  // spread so the wire shape matches `#[serde(skip_serializing_if =
  // "Option::is_none")]`.
  const observation: GateEvaluatedObservation = {
    kind: KIND_GATE_EVALUATED,
    session_id: ctx.session_id,
    timestamp: ctx.timestamp,
    gate_eval_id: ctx.gate_eval_id,
    gate_id: MIDEDIT_GATE_ID,
    inputs: {
      file_count: 1,
      changed_files: [ctx.file_path],
    },
    outcome: 'fail',
    rules_evaluated: rulesEvaluated,
    ...(rulesViolated.length > 0 ? { rules_violated: rulesViolated } : {}),
    enforcement,
    duration_ms: ctx.duration_ms,
    violation_count: violationCount,
    warning_count: warningCount,
  };
  return observation;
}

function enforcementFor(diagnostics: readonly DiagnosticLike[]): Enforcement {
  let worst: Enforcement = 'informational';
  for (const diag of diagnostics) {
    const level: Enforcement =
      diag.severity === 'error'
        ? 'blocking'
        : diag.severity === 'warning'
          ? 'warning'
          : 'informational';
    if (levelRank(level) > levelRank(worst)) {
      worst = level;
    }
  }
  return worst;
}

function levelRank(level: Enforcement): 0 | 1 | 2 {
  switch (level) {
    case 'informational':
      return 0;
    case 'warning':
      return 1;
    case 'blocking':
      return 2;
  }
}

function countsFor(diagnostics: readonly DiagnosticLike[]): {
  violationCount: number;
  warningCount: number;
} {
  let violations = 0;
  let warnings = 0;
  for (const diag of diagnostics) {
    if (diag.severity === 'error') {
      violations += 1;
    } else if (diag.severity === 'warning') {
      warnings += 1;
    }
  }
  return { violationCount: violations, warningCount: warnings };
}
