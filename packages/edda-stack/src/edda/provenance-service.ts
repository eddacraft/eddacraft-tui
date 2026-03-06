import type { MemoryObject, ProvenanceChain } from '../contracts/index.js';
import { validateProvenanceIntegrity as validateProvenanceIntegrityUtility } from '../contracts/index.js';
import type { MemoryId } from '../contracts/identifiers.js';
import type { IEmberPort, ProvenanceResolutionResult } from '../contracts/ports/index.js';
import type { IMemoryStoreOperations } from './store-interfaces.js';

export interface ProvenanceServiceDeps {
  store: IMemoryStoreOperations;
  emberPort?: IEmberPort;
}

export class ProvenanceService {
  constructor(private readonly deps: ProvenanceServiceDeps) {}

  async resolveProvenance(chain: ProvenanceChain): Promise<ProvenanceResolutionResult> {
    const missingLinks: string[] = [];
    const warnings: string[] = [];
    const resolvedData: { sessions: string[]; observations: string[]; proposal_id?: string } = {
      sessions: chain.source_sessions as string[],
      observations: chain.kindling_sources.map((source) => source.observation_id as string),
    };

    let resolvedCount = chain.kindling_sources.length + chain.source_sessions.length;
    const totalCount = resolvedCount + (chain.ember_source ? 1 : 0);

    if (chain.ember_source) {
      if (!this.deps.emberPort) {
        warnings.push('Cannot validate Ember proposal reference without emberPort');
        missingLinks.push(`proposal:${chain.ember_source.proposal_id}`);
      } else {
        const proposal = await this.deps.emberPort.getProposal(chain.ember_source.proposal_id);
        if (proposal === null) {
          missingLinks.push(`proposal:${chain.ember_source.proposal_id}`);
        } else {
          resolvedCount += 1;
          resolvedData.proposal_id = proposal.id;
        }
      }
    }

    const integrity = this.validateProvenanceIntegrity(chain);
    if (!integrity.valid) {
      warnings.push(...integrity.issues);
    }

    return {
      complete: missingLinks.length === 0 && warnings.length === 0,
      resolved_count: resolvedCount,
      total_count: totalCount,
      missing_links: missingLinks,
      resolved_data: resolvedData,
      warnings,
    };
  }

  async getMemoryProvenance(
    memoryId: MemoryId
  ): Promise<{ memory: MemoryObject; resolution: ProvenanceResolutionResult } | null> {
    const memory = await this.deps.store.getMemory(memoryId);
    if (memory === null) {
      return null;
    }

    const resolution = await this.resolveProvenance(memory.provenance);
    return { memory, resolution };
  }

  validateProvenanceIntegrity(chain: ProvenanceChain): { valid: boolean; issues: string[] } {
    return validateProvenanceIntegrityUtility(chain);
  }
}
