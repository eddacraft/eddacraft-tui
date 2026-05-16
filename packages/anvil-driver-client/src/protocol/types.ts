/**
 * DRVR-002 / DRVR-008: Editor-driver protocol method names, capability
 * vocabulary, and JSON-RPC parameter shapes.
 *
 * This module MIRRORS the Rust authoritative type at
 * `crates/anvil-intercept-proto/src/protocol.rs`. The Rust side is
 * authoritative; if the two drift, the Rust side wins and this file is
 * updated to match. Constants and serialised forms are pinned by the
 * protocol design spec at
 * `plans/specs/2026-05-06-editor-driver-protocol.md`.
 *
 * Keep this file aligned with Rust:
 *
 * - JSON wire-strings (the `ANVIL_*` constants) are the same characters
 *   on both sides. Tests at the bottom pin the values; CI fails if either
 *   side drifts.
 * - The `Capability` union mirrors the kebab-cased Rust enum
 *   (`attached` | `participating`) — do not add a `read-only` alias even
 *   though the spec uses that prose label; serialisation must match
 *   Rust exactly.
 * - `Diagnostic` payloads carried inside method `params` reuse the
 *   canonical inner shape from `../diagnostics/types.ts`. DO NOT
 *   redefine it here; the protocol layer imports it.
 *
 * Why this lives in `@eddacraft/anvil-driver-client`: the same package
 * already mirrors `Diagnostic`. Keeping the protocol vocabulary alongside
 * lets a TS driver pick up `import { ANVIL_ENFORCEMENT_ACK, Capability }
 * from '@eddacraft/anvil-driver-client'` without juggling another
 * dependency. Per DRVR-002 the package extends rather than spawning a
 * sibling contracts package.
 */

import type { Diagnostic } from '../diagnostics/types.js';

// ---------------------------------------------------------------------
// Method-name constants
// ---------------------------------------------------------------------
//
// Each constant is `as const` so the type system carries the literal
// through. Drivers branching on `method === ANVIL_ENFORCEMENT_ACK` get
// type-narrowing for free, and a typo is a compile error rather than a
// runtime mystery.

/** Server → client notification carrying `Diagnostic` payloads. */
export const ANVIL_PUBLISH_DIAGNOSTICS = 'anvil/publishDiagnostics' as const;

/** Client → server request: scan a mid-edit buffer for diagnostics. */
export const ANVIL_SCAN_BUFFER = 'anvil/scan_buffer' as const;

/**
 * Client → server: confirms an enforcement decision was carried out.
 *
 * **DRVR-008's central method.** A driver that does not advertise this
 * method in its manifest's `supported_anvil_methods` is capped at
 * `Capability::Attached` regardless of `.anvil.yaml` requesting
 * participation.
 */
export const ANVIL_ENFORCEMENT_ACK = 'anvil/enforcement/ack' as const;

/** Client → server: request a gate-result stream / one-shot snapshot. */
export const ANVIL_GATE_REQUEST = 'anvil/gate/request' as const;

/** Client → server: validate and normalise a `@anvil-ignore` comment. */
export const ANVIL_SUPPRESSION_APPLY = 'anvil/suppression/apply' as const;

/** Client → server: current session / fence / driver state for a worktree. */
export const ANVIL_STATUS_QUERY = 'anvil/status/query' as const;

/**
 * Every `anvil/` method the v1 protocol declares. Useful for tests and
 * for driver-side advertisement helpers.
 */
export const ALL_ANVIL_METHODS = [
  ANVIL_PUBLISH_DIAGNOSTICS,
  ANVIL_SCAN_BUFFER,
  ANVIL_ENFORCEMENT_ACK,
  ANVIL_GATE_REQUEST,
  ANVIL_SUPPRESSION_APPLY,
  ANVIL_STATUS_QUERY,
] as const;

/** Type-level union of all known method names. */
export type AnvilMethodName = (typeof ALL_ANVIL_METHODS)[number];

// ---------------------------------------------------------------------
// Capability vocabulary
// ---------------------------------------------------------------------

/**
 * Capability lattice for the §3.3 state machine.
 *
 * - `attached` — read-only diagnostic mode. Default after handshake.
 *   Subscribes to telemetry, renders diagnostics, applies suppressions.
 * - `participating` — enforcement-candidate mode. Receives enforcement
 *   decisions; ack-or-refuse contract per §2.5. Reaching this state
 *   requires both the DRVR-007 allowlist check AND the DRVR-008 method
 *   advertisement.
 *
 * Mirrors the kebab-cased Rust enum
 * `anvil_intercept_proto::protocol::Capability`.
 */
export type Capability = 'attached' | 'participating';

/** All capability values; useful for branching exhaustively. */
export const ALL_CAPABILITIES: readonly Capability[] = ['attached', 'participating'] as const;

/**
 * Reasons a `participating` request can be downgraded. Mirrors
 * `CapabilityDowngradeReason` on the Rust side.
 *
 * - `not-enforcement-candidate` — the driver advertised it does not
 *   want enforcement; daemon honours that.
 * - `missing-enforcement-ack-method` — DRVR-008 central case: driver
 *   asked for `participating` but did not advertise
 *   `anvil/enforcement/ack` in its manifest.
 */
export type CapabilityDowngradeReason =
  | 'not-enforcement-candidate'
  | 'missing-enforcement-ack-method';

/**
 * Structured capability-downgrade event. The daemon emits this to the
 * driver alongside the accepted capability so the operator-facing
 * status surface (status bar / MCP error metadata) can name the
 * specific reason rather than reporting a silent demotion.
 */
export interface CapabilityDowngrade {
  requested: Capability;
  negotiated: Capability;
  reason: CapabilityDowngradeReason;
  /** Methods the driver advertised; captured at downgrade time. */
  advertised_methods: string[];
}

// ---------------------------------------------------------------------
// Driver manifest (DRVR-008 slice)
// ---------------------------------------------------------------------

/**
 * v1 slice of the §2.2 driver manifest as it appears on the wire.
 *
 * The full manifest lives in the editor-driver protocol design spec
 * (§2.2). DRVR-008 adds `supported_anvil_methods`; the rest of the
 * fields will land with DRVR-001 / DRVR-003 as those consumers wire up.
 *
 * Field naming follows snake_case to match the Rust serde-default
 * convention used elsewhere in the wire protocol (the manifest crosses
 * the same JSON-RPC transport that `Diagnostic` rides on).
 */
export interface DriverManifestSlice {
  /** Absolute paths the driver claims it operates on. */
  workspace_roots: string[];
  /**
   * `anvil/` JSON-RPC method names this driver advertises support for.
   * An empty list models a stock LSP client that does not speak the
   * `anvil/` namespace at all.
   */
  supported_anvil_methods: string[];
}

// ---------------------------------------------------------------------
// Method parameter / result shapes
// ---------------------------------------------------------------------
//
// Each method's params / result is documented inline rather than buried
// in the spec. Driver implementers branching on a method name get the
// shape from this file's types.

/** `anvil/publishDiagnostics` — server → client notification params. */
export interface AnvilPublishDiagnosticsParams {
  /** Document URI in `file://` form. */
  uri: string;
  /** Document version the diagnostics apply to (per LSP convention). */
  version?: number;
  /** Canonical inner shape from `../diagnostics/types.ts`. */
  diagnostics: Diagnostic[];
}

/** `anvil/scan_buffer` — client → server request params. */
export interface AnvilScanBufferParams {
  /** Workspace-relative path of the buffer being edited. */
  path: string;
  /** In-flight buffer text. */
  text: string;
  /** Document version (per LSP convention). */
  version: number;
  /**
   * `mid-edit` (or `midEdit` alias) for the typical didChange path.
   * The daemon's `ScanBufferMode::parse` accepts both forms; the
   * driver-client and Rust launcher emit `midEdit` today.
   */
  mode: 'mid-edit' | 'midEdit';
  /**
   * MLP2-025c: raw `ANVIL_AGENT_TAG` env value the writer process
   * inherited from its launcher (anvil-run sets it via
   * `set_attribution_env`). The daemon decodes via
   * `anvil_attribution::env::agent_tag_from_env_value` at the
   * boundary; malformed values fold to `Cross::Spoofed`.
   *
   * Omit when `ANVIL_AGENT_TAG` is not present in the writer's
   * env — the daemon treats absence as `Cross::Untagged` (the
   * pre-MLP2-025 enforcement path).
   */
  env_agent_tag?: string;
}

/** `anvil/scan_buffer` — server → client result. */
export interface AnvilScanBufferResult {
  version: number;
  diagnostics: Diagnostic[];
  /** True when the daemon capped the diagnostic set for one scan. */
  truncated: boolean;
  /**
   * MLP2-025c: populated when the daemon's spoof cross-check
   * refused the write because the supplied `env_agent_tag` did
   * not match any daemon-issued tag on the writer's PID lineage.
   * Mutually exclusive with diagnostics (the rule engine never
   * ran for this request).
   */
  spoof_block?: AnvilScanBufferSpoofBlock;
}

/** MLP2-025c: shape of the daemon's spoof-block side effect. */
export interface AnvilScanBufferSpoofBlock {
  /** Always `degraded:spoofed-attribution` for v1. */
  reason: string;
  /** Canonicalised worktree the daemon fenced. */
  fenced_worktree: string;
}

/** `anvil/enforcement/ack` — client → server request params. */
export interface AnvilEnforcementAckParams {
  /** Daemon-minted decision id from the inbound `enforcement.decision` event. */
  decision_id: string;
  /** Daemon-minted correlation id for log lookup. */
  correlation_id: string;
}

/** `anvil/gate/request` — client → server request params. */
export interface AnvilGateRequestParams {
  /** Workspace root the driver is operating on. */
  workspace_root: string;
  /** Optional gate profile selector — same vocabulary as `anvil gate --profile`. */
  profile?: string;
}

/** `anvil/suppression/apply` — client → server request params. */
export interface AnvilSuppressionApplyParams {
  /** File path the suppression is being applied to. */
  file: string;
  /** Rule id being suppressed. */
  rule_id: string;
  /** Free-text reason supplied by the user. */
  reason: string;
}

/** `anvil/suppression/apply` — server → client result. */
export interface AnvilSuppressionApplyResult {
  /** Normalised `@anvil-ignore` comment to insert via `workspace/applyEdit`. */
  comment: string;
}

/** `anvil/status/query` — client → server request params. */
export interface AnvilStatusQueryParams {
  /** Workspace root to query. */
  workspace_root: string;
}
