/**
 * Reliability-budget quarantine ledger.
 *
 * The DRVR-007 spec contract (see
 * `plans/specs/anvil-driver-framework/editor-and-mcp-driver-design.md`
 * §2.3a + §2.6) requires that the daemon's reliability budget be
 * keyed on something stronger than `driverName`. Wave 1 documented
 * that contract; Wave 2 (this DRVR-001 PR) lands the in-process
 * client-side ledger that survives reconnect.
 *
 * Identity rule (TS-side, v1):
 *   - Quarantine is keyed on the **daemon-minted credential**,
 *     specifically `correlation.originating_driver_id` carried in
 *     INTD-015's outbound envelope. The driver client picks it up
 *     during the handshake and threads it through every reliability
 *     event.
 *   - **NEVER** keyed on `driverName` from the local manifest.
 *     `driverName` is self-declared and trivial to spoof from a
 *     hostile same-UID peer (cf. DRVR-007 spec §2.3b).
 *   - If no `originating_driver_id` is yet known (e.g. failures
 *     before the handshake completes), reliability accounting is
 *     paused. Pre-handshake failures are operational facts the
 *     daemon already sees via `SO_PEERCRED`; double-budgeting them
 *     here would create a self-DoS where a flapping daemon causes
 *     the client to permanently quarantine itself before any rule
 *     ever fired.
 *
 * Persistence:
 *   - In-process only in v1. Cross-process drivers (e.g. an editor
 *     that spawns a fresh process per workspace) will need an
 *     on-disk ledger; the schema for that store is documented in
 *     {@link QUARANTINE_PERSISTENCE_NOTE} below but not implemented
 *     here. Implementing on-disk persistence in this PR would touch
 *     filesystem layout decisions outside DRVR-001's scope; the
 *     orchestrator can pick it up in a follow-up.
 *
 * Decision policy:
 *   - Sliding-window failure counter; quarantine fires when the
 *     count crosses the configured threshold. The window is wall-
 *     clock based, not sample-count based, so a low-traffic driver
 *     does not get a permanent strike from a single bad day three
 *     weeks ago.
 *   - Quarantine survives reconnect (it lives in this in-memory
 *     ledger, which the {@link DriverClient} retains across
 *     transport drops).
 *   - Cooldown: once quarantined, the driver's identity stays in
 *     quarantine for `cooldownMs` (default 5 minutes) before it can
 *     attempt to recover. The consumer reads {@link
 *     ReliabilityBudget.isQuarantined} on each request and surfaces
 *     `anvil-driver-quarantined` if so.
 */

export interface ReliabilityBudgetOptions {
  /** Number of failures within `windowMs` that triggers quarantine.
   *  Default: 5. Lower values increase false-positive risk; higher
   *  values delay shedding load from a genuinely flapping driver. */
  failureThreshold?: number;
  /** Sliding-window length in milliseconds. Default: 60_000 (1 min). */
  windowMs?: number;
  /** Cooldown in milliseconds before a quarantined driver can
   *  re-attempt. Default: 300_000 (5 min). */
  cooldownMs?: number;
  /** Clock function — injectable for tests. Default: `Date.now`. */
  now?: () => number;
}

export interface ReliabilityRecord {
  /** Stable identity (daemon-minted `originating_driver_id`). */
  identity: string;
  /** Wall-clock timestamps (ms since epoch) of failures inside the
   *  window. Trimmed lazily on each record / query. */
  failures: number[];
  /** When `quarantinedUntil > now`, the identity is in quarantine.
   *  Set to 0 when not quarantined. */
  quarantinedUntil: number;
}

export const QUARANTINE_PERSISTENCE_NOTE = `
DRVR-001 v1 ships an in-process ReliabilityBudget. A future on-disk
ledger would carry this shape per identity:

  {
    "identity": "<originating_driver_id>",
    "failures": [<unix_ms>, ...],   // trimmed to windowMs
    "quarantinedUntil": <unix_ms>   // 0 when not quarantined
  }

Storage location candidate: $XDG_DATA_HOME/anvil/driver-quarantine.json
(Linux), %APPDATA%\\anvil\\driver-quarantine.json (Windows). Locking,
atomic writes, and corruption-tolerance are out of scope for the
in-process v1.
`.trim();

export class ReliabilityBudget {
  private readonly failureThreshold: number;
  private readonly windowMs: number;
  private readonly cooldownMs: number;
  private readonly now: () => number;
  private readonly records = new Map<string, ReliabilityRecord>();

  public constructor(options: ReliabilityBudgetOptions = {}) {
    this.failureThreshold = options.failureThreshold ?? 5;
    this.windowMs = options.windowMs ?? 60_000;
    this.cooldownMs = options.cooldownMs ?? 300_000;
    this.now = options.now ?? Date.now;
    if (this.failureThreshold <= 0) {
      throw new RangeError('failureThreshold must be positive');
    }
    if (this.windowMs <= 0) {
      throw new RangeError('windowMs must be positive');
    }
    if (this.cooldownMs < 0) {
      throw new RangeError('cooldownMs must be non-negative');
    }
  }

  /**
   * Record a successful transaction. Resets the failure window for
   * `identity` because a successful round-trip is the strongest
   * signal that the driver is no longer flapping.
   */
  public recordSuccess(identity: string | undefined): void {
    if (identity === undefined || identity.length === 0) {
      return;
    }
    const record = this.records.get(identity);
    if (record === undefined) {
      return;
    }
    record.failures = [];
    // Successful round-trip ends quarantine early. The cooldown is
    // there to bridge the recovery period; once the driver works
    // again, holding it in quarantine punishes the consumer.
    record.quarantinedUntil = 0;
  }

  /**
   * Record a failure. Returns `true` if the failure pushed the
   * driver across the threshold (the consumer should surface a
   * structured `anvil-driver-quarantined` error on the NEXT request,
   * not the current one — the failed call already carries its own
   * structured error).
   */
  public recordFailure(identity: string | undefined): boolean {
    if (identity === undefined || identity.length === 0) {
      // Pre-handshake failures are not counted (see module header).
      return false;
    }
    const now = this.now();
    let record = this.records.get(identity);
    if (record === undefined) {
      record = { identity, failures: [], quarantinedUntil: 0 };
      this.records.set(identity, record);
    }

    // Trim the sliding window before counting.
    const cutoff = now - this.windowMs;
    record.failures = record.failures.filter((ts) => ts > cutoff);
    record.failures.push(now);

    if (record.failures.length >= this.failureThreshold && record.quarantinedUntil <= now) {
      record.quarantinedUntil = now + this.cooldownMs;
      return true;
    }
    return false;
  }

  /**
   * Check whether `identity` is currently in quarantine. Lazily
   * clears the flag when `cooldownMs` expires.
   */
  public isQuarantined(identity: string | undefined): boolean {
    if (identity === undefined || identity.length === 0) {
      return false;
    }
    const record = this.records.get(identity);
    if (record === undefined) {
      return false;
    }
    const now = this.now();
    if (record.quarantinedUntil <= now) {
      // Cooldown elapsed — reset state so the next failure starts a
      // fresh window.
      if (record.quarantinedUntil > 0) {
        record.quarantinedUntil = 0;
        record.failures = [];
      }
      return false;
    }
    return true;
  }

  /**
   * Snapshot of the current state for telemetry / tests. The
   * returned objects are clones — mutating them does not affect the
   * ledger.
   */
  public snapshot(): ReliabilityRecord[] {
    return Array.from(this.records.values()).map((rec) => ({
      identity: rec.identity,
      failures: [...rec.failures],
      quarantinedUntil: rec.quarantinedUntil,
    }));
  }

  /** Forget all state. Used by tests; not exposed on the public
   *  surface of the package. */
  public clearForTests(): void {
    this.records.clear();
  }
}
