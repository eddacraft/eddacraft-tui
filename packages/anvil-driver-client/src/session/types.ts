/**
 * Session-key wire shape (MLP-014 / MLP2-023 / MLP2-029).
 *
 * This file MIRRORS the Rust authoritative type at
 * `crates/anvil-intercept-proto/src/session.rs`. The Rust side is
 * authoritative; if the two drift, the Rust side wins and this file
 * is updated to match. Field names, JSON serialisation, and env-var
 * constants are pinned by:
 *
 * - `plans/modules/multilayer-protection.aps.md` (MLP-014 footnote 1)
 * - `plans/modules/multilayer-protection-v2.aps.md` (MLP2-023 +
 *   MLP2-029)
 * - `plans/modules/intercept-launcher.aps.md` (INTL-003 / INTL-004)
 *
 * The driver-client mirrors these types inline (rather than
 * depending on a generated TS-from-Rust contracts package) per the
 * pattern set by `diagnostics/types.ts`. When a generator lands,
 * this module switches to importing from it without changing the
 * public surface — consumers import from
 * `@eddacraft/anvil-driver-client` either way.
 *
 * DO NOT extend these types with TS-side fields the Rust wire shape
 * does not declare; doing so risks emitting envelopes the daemon
 * (or other Rust consumers) cannot deserialise.
 */

/**
 * Environment variable carrying the daemon-minted `AgentTag` from a
 * launcher to its child process. Advisory only — the daemon MUST
 * cross-check the env-supplied tag against the `AgentTag` it issued
 * for this PID lineage at INTL-003 before honouring it. See
 * ADR-037 §D-2 for the witness-chain authentication backstop.
 *
 * Same constant value as `ANVIL_AGENT_TAG_ENV` in
 * `crates/anvil-intercept-proto/src/session.rs`.
 */
export const ANVIL_AGENT_TAG_ENV = 'ANVIL_AGENT_TAG' as const;

/**
 * Environment variable carrying the per-task identifier that scopes
 * fence isolation in multi-agent worktrees (MLP-014). Same trust
 * caveat as `ANVIL_AGENT_TAG_ENV`: env is forgeable by any same-UID
 * peer, so absence triggers a process-tree walk fallback rather than
 * being treated as authoritative.
 *
 * Same constant value as `ANVIL_TASK_ID_ENV` in
 * `crates/anvil-intercept-proto/src/session.rs`.
 */
export const ANVIL_TASK_ID_ENV = 'ANVIL_TASK_ID' as const;

/**
 * Composite identity for a session within a worktree. Minted by the
 * daemon at INTL-003 registration time from the launcher-supplied
 * `(driver_id, claimed_agent_id)` plus the kernel-reported
 * `pid_starttime`. Combined with `WorktreeKey` in MLP-014 / MLP2-023
 * to form the per-task fence scope.
 *
 * **Trust model.** `AgentTag` is not authenticated identity. Any
 * same-UID process can claim any `driver_id` / `claimed_agent_id`
 * pair; `pid_starttime` makes after-the-fact PID reuse detectable
 * but does not prove the process started where the launcher said it
 * did. The daemon honours a tag only when it matches a registration
 * it issued in this session; the witness chain (ADR-037 §D-2) and
 * `validate_at_l4` (ADR-037 §D-5) are the authentication backstops.
 *
 * Wire serialisation: lower-snake-case JSON object with three
 * fields. Pinned by the Rust `#[derive(Serialize, Deserialize)]` on
 * the source struct — see the parity test in `./types.test.ts`.
 */
export interface AgentTag {
  /**
   * Identifies the driver framework that launched the agent
   * (`anvil-run`, `claude-code-pretool`, `direct-mcp`, …). Drawn
   * from the surface driver registry; never user-supplied.
   */
  readonly driver_id: string;

  /**
   * Free-form identifier the driver claims for this agent instance.
   * Opaque to the proto layer; the daemon may apply per-driver
   * well-formedness rules before honouring.
   */
  readonly claimed_agent_id: string;

  /**
   * Process start time as Unix seconds since epoch, captured at
   * spawn. Defends against PID reuse — a recycled PID with a
   * different `pid_starttime` is treated as a different session.
   *
   * Held as `number` (JS double) on the TS side; safe for the
   * `2^53 - 1` second budget (~285 million years from epoch),
   * matching the Rust `u64` on the wire for any realistic value.
   */
  readonly pid_starttime: number;
}

/**
 * Build an {@link AgentTag} from raw parts. No validation: the
 * daemon's session registry is the single authority on which tags
 * are honoured. This mirrors the Rust `AgentTag::new(...)`
 * constructor.
 */
export function makeAgentTag(
  driverId: string,
  claimedAgentId: string,
  pidStarttime: number
): AgentTag {
  return {
    driver_id: driverId,
    claimed_agent_id: claimedAgentId,
    pid_starttime: pidStarttime,
  };
}

/**
 * Parse a JSON-shaped value into an {@link AgentTag}, validating
 * the three required keys + their primitive types. Returns the tag
 * on success or throws `TypeError` describing the first malformed
 * field — callers that prefer a `Result`-style API wrap this in a
 * `try`.
 *
 * Forward-compat: extra keys on the wire are silently dropped, per
 * the Rust struct's lack of `#[serde(deny_unknown_fields)]`. This
 * lets a future field land in Rust without breaking TS clients
 * built against the pre-extension wire shape.
 */
export function parseAgentTag(value: unknown): AgentTag {
  if (value === null || typeof value !== 'object') {
    throw new TypeError(
      `AgentTag must be a JSON object, got ${value === null ? 'null' : typeof value}`
    );
  }
  const obj = value as Record<string, unknown>;
  const driverId = obj.driver_id;
  if (typeof driverId !== 'string') {
    throw new TypeError(`AgentTag.driver_id must be a string, got ${typeof driverId}`);
  }
  const claimedAgentId = obj.claimed_agent_id;
  if (typeof claimedAgentId !== 'string') {
    throw new TypeError(`AgentTag.claimed_agent_id must be a string, got ${typeof claimedAgentId}`);
  }
  const pidStarttime = obj.pid_starttime;
  if (
    typeof pidStarttime !== 'number' ||
    !Number.isFinite(pidStarttime) ||
    !Number.isInteger(pidStarttime) ||
    pidStarttime < 0
  ) {
    throw new TypeError(
      `AgentTag.pid_starttime must be a non-negative integer, got ${String(pidStarttime)}`
    );
  }
  return {
    driver_id: driverId,
    claimed_agent_id: claimedAgentId,
    pid_starttime: pidStarttime,
  };
}
