/**
 * Anvil-Kindling Adapter
 *
 * Maps Anvil's 11 observation kinds (rich, domain-specific schemas)
 * to Kindling core's generic observation model for storage and retrieval.
 *
 * Anvil observation data is serialized to `content` as JSON.
 * The original Anvil kind is preserved in `provenance.anvil_kind`.
 */

import { randomUUID } from 'node:crypto';
import type { KindlingService, Capsule, ID } from '@eddacraft/kindling-core';
import type { Observation as AnvilObservation } from './observation-contract.js';
import { OBSERVATION_CONTRACT_VERSION } from './observation-contract.js';

/** Map Anvil observation kinds to Kindling's generic observation kinds */
const KIND_MAP: Record<AnvilObservation['kind'], string> = {
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

export interface AnvilKindlingAdapterConfig {
  service: KindlingService;
  /** Repo path for scope isolation */
  repoId?: string;
}

/**
 * Bridges Anvil observation emission to Kindling storage.
 *
 * Usage:
 * ```ts
 * const adapter = new AnvilKindlingAdapter({ service });
 * const capsule = adapter.startSession(sessionId, scopeIds);
 * adapter.emit(observation);
 * adapter.endSession(capsule.id);
 * ```
 */
export class AnvilKindlingAdapter {
  private service: KindlingService;
  private repoId: string | undefined;

  constructor(config: AnvilKindlingAdapterConfig) {
    this.service = config.service;
    this.repoId = config.repoId;
  }

  /**
   * Open a Kindling capsule for an Anvil session.
   * Call this when a CLI command starts.
   */
  startSession(sessionId: string, intent: string): Capsule {
    return this.service.openCapsule({
      type: 'session',
      intent,
      scopeIds: {
        sessionId,
        repoId: this.repoId,
      },
    });
  }

  /**
   * Close the capsule for a session.
   * Call this when a CLI command ends.
   */
  endSession(capsuleId: ID, summaryContent?: string): Capsule {
    return this.service.closeCapsule(capsuleId, {
      generateSummary: !!summaryContent,
      summaryContent,
    });
  }

  /**
   * Emit an Anvil observation to Kindling.
   * The rich Anvil schema is serialized to content; the original kind
   * is preserved in provenance for filtering.
   */
  emit(observation: AnvilObservation, capsuleId?: ID): void {
    const kindlingObs = {
      id: randomUUID(),
      kind: KIND_MAP[observation.kind] as 'message' | 'command' | 'error',
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

    this.service.appendObservation(kindlingObs, {
      capsuleId,
      validate: true,
    });
  }
}
