/**
 * Kindling Bootstrap
 *
 * Initializes the complete Kindling stack for CLI commands.
 * Returns a KindlingContext that provides:
 *   - service: Anvil's KindlingService (for emitters)
 *   - adapter: AnvilKindlingAdapter (for capsule lifecycle)
 *   - close(): cleanup function
 *
 * Returns null when Kindling is disabled in config (privacy-first default).
 */

import { resolve, dirname } from 'node:path';
import { mkdirSync } from 'node:fs';
import { randomUUID } from 'node:crypto';
import { createDebugger } from '@eddacraft/anvil-core';
import {
  KindlingService as CoreKindlingService,
  type KindlingStore as CoreKindlingStore,
  type Observation as CoreObservation,
} from '@eddacraft/kindling-core';
import { openDatabase, closeDatabase, SqliteKindlingStore } from '@eddacraft/kindling-store-sqlite';
import { LocalFtsProvider } from '@eddacraft/kindling-provider-local';
import {
  loadKindlingConfig,
  createKindlingService,
  AnvilKindlingAdapter,
  type KindlingConfig,
  type Observation as AnvilObservation,
  type IKindlingStore,
  type QueryRequest,
  type QueryResponse,
  OBSERVATION_CONTRACT_VERSION,
} from '@eddacraft/anvil-kindling-integration';

// Map Anvil observation kinds to Kindling core's generic observation kinds
const KIND_MAP: Record<AnvilObservation['kind'], CoreObservation['kind']> = {
  session_start: 'message',
  session_end: 'message',
  plan_created: 'message',
  plan_edited: 'message',
  plan_approved: 'message',
  plan_rejected: 'message',
  action_executed: 'command',
  gate_evaluated: 'command',
  constraint_applied: 'message',
  human_input: 'message',
  error: 'error',
};

/**
 * IKindlingStore implementation that bridges Anvil observations to
 * the kindling-core KindlingService via the adapter's mapping logic.
 *
 * Observations emitted through Anvil's KindlingService flow through
 * this bridge to kindling-core's appendObservation(), with optional
 * capsule linking when a session is active.
 */
class KindlingCoreBridge implements IKindlingStore {
  private capsuleId: string | undefined;

  constructor(
    private coreService: CoreKindlingService,
    private repoId?: string
  ) {}

  setCapsuleId(id: string | undefined): void {
    this.capsuleId = id;
  }

  async emit(observation: AnvilObservation): Promise<void> {
    const kindlingObs: CoreObservation = {
      id: randomUUID(),
      kind: KIND_MAP[observation.kind],
      content: JSON.stringify(observation),
      provenance: {
        anvil_kind: observation.kind,
        anvil_contract_version: OBSERVATION_CONTRACT_VERSION,
      },
      ts: Date.now(),
      scopeIds: {
        sessionId: observation.session_id,
        repoId: this.repoId,
      },
      redacted: false,
    };

    this.coreService.appendObservation(kindlingObs, {
      capsuleId: this.capsuleId,
      validate: true,
    });
  }

  async query(_request: QueryRequest): Promise<QueryResponse> {
    // Query support is handled via Kindling CLI / retrieve API directly.
    // This no-op keeps the interface satisfied for Anvil's service layer.
    return {
      metadata: {
        query_id: randomUUID(),
        executed_at: new Date().toISOString(),
        contract_version: '1.0.0',
        result_count: 0,
        truncated: false,
        truncation_reason: 'none',
      },
      observations: [],
    };
  }

  async close(): Promise<void> {
    // DB lifecycle managed by KindlingContext.close()
  }
}

/**
 * The complete Kindling context for a CLI session.
 */
export interface KindlingContext {
  /** Anvil's KindlingService — pass to emitters */
  service: ReturnType<typeof createKindlingService>;
  /** Adapter for capsule lifecycle (startSession / endSession) */
  adapter: AnvilKindlingAdapter;
  /** Bridge store — call setCapsuleId() when a session capsule is opened */
  bridge: KindlingCoreBridge;
  /** Active Kindling configuration */
  config: KindlingConfig;
  /** Release all resources (close database) */
  close: () => void;
}

const log = createDebugger('service');

/**
 * Initialize the Kindling stack for a CLI command.
 *
 * @param workspaceRoot - Absolute path to the project root
 * @returns KindlingContext, or null if Kindling is disabled
 */
export function initKindling(workspaceRoot: string): KindlingContext | null {
  log(`initKindling: root=${workspaceRoot}`);
  const config = loadKindlingConfig(workspaceRoot);

  if (!config.enabled) {
    log('initKindling: disabled by config');
    return null;
  }

  try {
    // Resolve database path (relative to workspace root)
    const dbPath = resolve(workspaceRoot, config.database_path);

    // Ensure parent directory exists
    mkdirSync(dirname(dbPath), { recursive: true });

    // Open SQLite database with migrations
    const db = openDatabase({ path: dbPath });

    // Create kindling-core store + provider + service
    const coreStore = new SqliteKindlingStore(db) as unknown as CoreKindlingStore;
    const provider = new LocalFtsProvider(db);
    const coreService = new CoreKindlingService({ store: coreStore, provider });

    // Create bridge: IKindlingStore → kindling-core
    const bridge = new KindlingCoreBridge(coreService, workspaceRoot);

    // Create Anvil's KindlingService wrapping the bridge
    const service = createKindlingService(config, bridge);

    // Create adapter for capsule lifecycle
    const adapter = new AnvilKindlingAdapter({ service: coreService, repoId: workspaceRoot });

    log(`initKindling: stack initialized dbPath=${dbPath}`);
    return {
      service,
      adapter,
      bridge,
      config,
      close: () => {
        log('initKindling: closing database');
        closeDatabase(db);
      },
    };
  } catch {
    // Bootstrap failure must never break CLI commands — degrade gracefully
    log('initKindling: bootstrap failed, degrading gracefully');
    return null;
  }
}
