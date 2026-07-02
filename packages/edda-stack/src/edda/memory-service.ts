import type {
  CreateMemoryInput,
  EddaStats,
  IEddaPort,
  ProvenanceResolutionResult,
  RetireMemoryInput,
  UpdateMemoryInput,
} from '../contracts/ports/edda.port.js';
import type {
  CandidateProposal,
  MemoryObject,
  MemoryQuery,
  MemoryQueryResult,
  MemoryStatus,
  MemoryType,
  PromoteProposalInput,
  ProvenanceChain,
} from '../contracts/index.js';
import { MemoryObjectSchema } from '../contracts/index.js';
import type { MemoryId, ProposalId } from '../contracts/identifiers.js';
import type { Timestamp } from '../contracts/temporal.js';
import { EvolutionService } from './evolution-service.js';
import { PromotionService } from './promotion-service.js';
import { ProvenanceService } from './provenance-service.js';
import type { IMemoryStoreOperations, IVersionTracker } from './store-interfaces.js';

export interface MemoryServiceDeps {
  store: IMemoryStoreOperations;
  promotionService: PromotionService;
  provenanceService: ProvenanceService;
  evolutionService: EvolutionService;
  versionTracker?: IVersionTracker;
}

export class MemoryService implements IEddaPort {
  constructor(private readonly deps: MemoryServiceDeps) {}

  async promoteProposal(input: PromoteProposalInput): Promise<MemoryObject> {
    return this.deps.promotionService.promoteProposal(input);
  }

  async createMemory(input: CreateMemoryInput): Promise<MemoryObject> {
    return this.deps.promotionService.createMemory(input);
  }

  /**
   * Internal/test-only escape hatch: bypasses the CAS claim and natural-key
   * idempotency protections of promoteProposal (CIB-118). Must not be wired
   * into a live proposal flow.
   */
  async createMemoryFromProposal(
    input: PromoteProposalInput,
    proposal: CandidateProposal
  ): Promise<MemoryObject> {
    return this.deps.promotionService.createMemoryFromProposal(input, proposal);
  }

  async updateMemory(id: MemoryId, input: UpdateMemoryInput): Promise<MemoryObject | null> {
    const existing = await this.deps.store.getMemory(id);
    if (existing === null) {
      return null;
    }

    const updated = MemoryObjectSchema.parse({
      ...existing,
      statement: input.statement ?? existing.statement,
      context: input.context ? { ...existing.context, ...input.context } : existing.context,
      confidence: input.confidence ?? existing.confidence,
      confidence_rationale: input.confidence_rationale ?? existing.confidence_rationale,
      metadata: input.metadata ?? existing.metadata,
      updated_at: nowTimestamp(),
    });

    await this.deps.store.saveMemory(updated);

    if (this.deps.versionTracker) {
      await this.deps.versionTracker.trackChange(
        [`memories/${updated.type}/${id}.yaml`],
        `Updated memory ${id}`,
        updated.attribution.actor
      );
    }

    return updated;
  }

  async retireMemory(id: MemoryId, input: RetireMemoryInput): Promise<MemoryObject | null> {
    return this.deps.evolutionService.retireMemory(id, input);
  }

  async retireMemoryById(
    id: MemoryId,
    supersededBy: MemoryId | undefined,
    reason: string,
    retiredBy: string
  ): Promise<void> {
    await this.deps.evolutionService.retireMemoryById(id, supersededBy, reason, retiredBy);
  }

  async supersedeMemory(
    oldMemoryId: MemoryId,
    newMemoryInput: CreateMemoryInput
  ): Promise<{ old: MemoryObject; new: MemoryObject }> {
    return this.deps.evolutionService.supersedeMemory(oldMemoryId, newMemoryInput);
  }

  async getMemory(id: MemoryId): Promise<MemoryObject | null> {
    return this.deps.store.getMemory(id);
  }

  async getMemoryByProposalId(proposalId: ProposalId): Promise<MemoryObject | null> {
    return this.deps.store.getMemoryByProposalId(proposalId);
  }

  async queryMemories(query: MemoryQuery): Promise<MemoryQueryResult> {
    return this.deps.store.queryMemories(query);
  }

  async getActiveMemories(): Promise<MemoryObject[]> {
    return this.deps.store.getActiveMemories();
  }

  async getMemoriesByType(type: MemoryType): Promise<MemoryObject[]> {
    return this.deps.store.getMemoriesByType(type);
  }

  async searchMemories(searchText: string): Promise<MemoryObject[]> {
    return this.deps.store.searchMemories(searchText);
  }

  async memoryExists(id: MemoryId): Promise<boolean> {
    return this.deps.store.memoryExists(id);
  }

  async getEvolutionChain(id: MemoryId): Promise<MemoryObject[]> {
    return this.deps.evolutionService.getEvolutionChain(id);
  }

  async getLatestVersion(id: MemoryId): Promise<MemoryObject | null> {
    return this.deps.evolutionService.getLatestVersion(id);
  }

  async resolveProvenance(chain: ProvenanceChain): Promise<ProvenanceResolutionResult> {
    return this.deps.provenanceService.resolveProvenance(chain);
  }

  async isAvailable(): Promise<boolean> {
    return this.deps.store.isAvailable();
  }

  async getStats(): Promise<EddaStats> {
    return this.deps.store.getStats();
  }

  async countMemories(filter?: { status?: MemoryStatus; type?: MemoryType }): Promise<number> {
    return this.deps.store.countMemories(filter);
  }

  async exportMemories(): Promise<MemoryObject[]> {
    return this.deps.store.exportMemories();
  }

  async importMemories(memories: MemoryObject[]): Promise<number> {
    return this.deps.store.importMemories(memories);
  }
}

function nowTimestamp(): Timestamp {
  return new Date().toISOString() as Timestamp;
}
