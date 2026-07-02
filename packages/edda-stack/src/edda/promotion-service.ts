import { v4 as uuidv4 } from 'uuid';
import type {
  CandidateProposal,
  MemoryId,
  MemoryObject,
  PromoteProposalInput,
  ProposalId,
} from '../contracts/index.js';
import {
  MemoryObjectSchema,
  MEMORY_SCHEMA_VERSION,
  PromoteProposalInputSchema,
  ProposalAlreadyResolvedError,
  createMemoryId,
  createPromotionInput,
  expandProvenanceSummary,
} from '../contracts/index.js';
import type { CreateMemoryInput, IEmberPort } from '../contracts/ports/index.js';
import type { Timestamp } from '../contracts/temporal.js';
import type { EddaPromotionConfig } from './config.js';
import { EddaPromotionConfigSchema } from './config.js';
import type { IMemoryStoreOperations, IVersionTracker } from './store-interfaces.js';

export interface PromotionServiceDeps {
  store: IMemoryStoreOperations;
  emberPort?: IEmberPort;
  versionTracker?: IVersionTracker;
  config?: EddaPromotionConfig;
}

const DEFAULT_PROMOTION_CONFIG: EddaPromotionConfig = EddaPromotionConfigSchema.parse({});

export class PromotionService {
  private readonly config: EddaPromotionConfig;

  constructor(private readonly deps: PromotionServiceDeps) {
    this.config = deps.config ?? DEFAULT_PROMOTION_CONFIG;
  }

  async promoteProposal(input: PromoteProposalInput): Promise<MemoryObject> {
    const validation = this.validatePromotionInput(input);
    if (!validation.valid) {
      throw new Error(`Invalid promotion input: ${validation.errors.join('; ')}`);
    }

    // Idempotency by natural key (CIB-118): if this proposal already produced
    // a memory, a re-fired promotion returns that memory instead of creating
    // a duplicate. The first promotion wins; later inputs are ignored.
    const existingMemory = await this.deps.store.getMemoryByProposalId(input.proposal_id);
    if (existingMemory) {
      if (this.deps.emberPort) {
        try {
          // Repair path: ensure the proposal is recorded as promoted. This is
          // a no-op when the same promotion was already recorded.
          await this.deps.emberPort.markPromoted(
            input.proposal_id,
            existingMemory.id,
            input.promoted_by
          );
        } catch (error) {
          if (!(error instanceof ProposalAlreadyResolvedError)) {
            throw error;
          }
          // Already terminal — the memory exists, so the promotion effect is
          // already in place.
        }
      }
      return existingMemory;
    }

    let proposal: CandidateProposal | null = null;
    if (this.deps.emberPort) {
      proposal = await this.deps.emberPort.getProposal(input.proposal_id);

      if (proposal === null) {
        throw new Error(`Proposal not found: ${input.proposal_id}`);
      }

      if (proposal.status === 'promoted') {
        throw new Error(
          `Proposal ${input.proposal_id} is already promoted but no memory exists for it (inconsistent state)`
        );
      }

      if (proposal.status !== 'active') {
        throw new Error(`Proposal is not active: ${input.proposal_id}`);
      }

      if (proposal.confidence < this.config.min_ember_confidence) {
        throw new Error(
          `Proposal confidence ${proposal.confidence} is below minimum ${this.config.min_ember_confidence}`
        );
      }
    }

    // When no emberPort is available, construct a synthetic proposal using
    // the proposal ID as a deterministic observation/session ID. This avoids
    // generating random UUIDs that corrupt the provenance chain — the IDs
    // are traceable back to the promotion request rather than being noise.
    const resolvedProposal = proposal ?? {
      id: input.proposal_id,
      type: 'pattern',
      status: 'active',
      summary: input.statement ?? input.context.why,
      rationale: input.reason,
      confidence: this.config.min_ember_confidence,
      signals: [],
      provenance: {
        observation_ids: [input.proposal_id],
        session_ids: [input.proposal_id],
        proposal_id: input.proposal_id,
        earliest_observation: toTimestamp(input.context.when),
        latest_observation: toTimestamp(input.context.when),
      },
      created_at: nowTimestamp(),
      expires_at: nowTimestamp(),
      ttl_days: 30,
    };

    const memory = this.buildMemoryFromProposal(input, resolvedProposal);

    // Save-then-claim (CIB-118). The memory row is written speculatively
    // FIRST — a fresh-UUID row is harmless if the claim below fails or the
    // process dies in between (the proposal stays active and the next attempt
    // repairs the mark through the upfront getMemoryByProposalId branch).
    // The reverse ordering would brick the proposal on a transient save
    // failure: a promoted proposal whose memory_id does not exist, with no
    // recovery path past the CAS.
    await this.deps.store.saveMemory(memory);

    // Claim the proposal via the store's compare-and-set on 'active' —
    // exactly one concurrent promotion can win the claim.
    if (this.deps.emberPort) {
      try {
        await this.deps.emberPort.markPromoted(input.proposal_id, memory.id, input.promoted_by);
      } catch (error) {
        if (error instanceof ProposalAlreadyResolvedError) {
          // Lost the claim to a concurrent promotion. Our just-saved row is
          // an orphaned-but-harmless speculative write; resolve the winner
          // through the proposal's recorded resolution rather than the
          // natural-key lookup, which could now find our own row.
          const winner = await this.getRecordedWinnerMemory(input.proposal_id, memory.id);
          if (winner) {
            return winner;
          }
        }
        throw error;
      }
    }

    if (this.deps.versionTracker) {
      await this.deps.versionTracker.trackChange(
        [`memories/${memory.type}/${memory.id}.yaml`],
        `Promoted Ember proposal ${input.proposal_id} to Edda memory ${memory.id}`,
        input.promoted_by
      );
    }

    return memory;
  }

  /**
   * Look up the memory recorded on the proposal's resolution after a lost
   * promotion claim. Returns null when the resolution is missing or points at
   * our own speculative row (which would mean the claim error was spurious).
   */
  private async getRecordedWinnerMemory(
    proposalId: ProposalId,
    ownMemoryId: MemoryId
  ): Promise<MemoryObject | null> {
    if (!this.deps.emberPort) {
      return null;
    }
    const current = await this.deps.emberPort.getProposal(proposalId);
    const winnerId = current?.resolution?.memory_id;
    if (!winnerId || winnerId === ownMemoryId) {
      return null;
    }
    return this.deps.store.getMemory(createMemoryId(winnerId));
  }

  /**
   * Internal/test-only escape hatch: builds and saves a memory WITHOUT the
   * CAS claim and natural-key idempotency protections that promoteProposal
   * provides (CIB-118). Must not be wired into a live proposal flow — use
   * promoteProposal for anything that touches a real proposal lifecycle.
   */
  async createMemoryFromProposal(
    input: PromoteProposalInput,
    proposal: CandidateProposal
  ): Promise<MemoryObject> {
    const memory = this.buildMemoryFromProposal(input, proposal);
    await this.deps.store.saveMemory(memory);
    return memory;
  }

  private buildMemoryFromProposal(
    input: PromoteProposalInput,
    proposal: CandidateProposal
  ): MemoryObject {
    const promotionInput = createPromotionInput(proposal, input.promoted_by, input.reason, {
      memoryType: input.type,
      confidence: input.confidence,
      statement: input.statement,
      conditions: input.context.conditions,
      scope: input.context.scope,
      tags: input.context.tags,
    });

    const createdAt = nowTimestamp();
    const provenance = expandProvenanceSummary(proposal.provenance);
    provenance.ember_source = {
      proposal_id: proposal.id,
      proposal_type: proposal.type,
      confidence: proposal.confidence,
      created_at: proposal.created_at,
    };

    const memory = MemoryObjectSchema.parse({
      id: createMemoryId(uuidv4()),
      type: promotionInput.type,
      status: 'active',
      schema_version: MEMORY_SCHEMA_VERSION,
      statement: promotionInput.statement ?? proposal.summary,
      context: input.context,
      metadata: input.metadata ?? proposal.metadata,
      confidence: promotionInput.confidence,
      confidence_rationale: input.confidence_rationale ?? promotionInput.confidence_rationale,
      provenance,
      attribution: {
        actor: input.promoted_by,
        timestamp: createdAt,
        method: 'cli_command',
        reason: input.reason,
      },
      evolution: {
        supersedes: [],
      },
      created_at: createdAt,
    });

    return memory;
  }

  async createMemory(input: CreateMemoryInput): Promise<MemoryObject> {
    const createdAt = nowTimestamp();

    const memory = MemoryObjectSchema.parse({
      id: createMemoryId(uuidv4()),
      type: input.type,
      status: 'active',
      schema_version: MEMORY_SCHEMA_VERSION,
      statement: input.statement,
      context: input.context,
      metadata: input.metadata,
      confidence: input.confidence,
      confidence_rationale: input.confidence_rationale,
      provenance: input.provenance,
      attribution: {
        actor: input.created_by,
        timestamp: createdAt,
        method: 'cli_command',
        reason: input.reason,
      },
      evolution: {
        supersedes: [],
      },
      created_at: createdAt,
    });

    await this.deps.store.saveMemory(memory);

    if (this.deps.versionTracker) {
      await this.deps.versionTracker.trackChange(
        [`memories/${memory.type}/${memory.id}.yaml`],
        `Created Edda memory ${memory.id}`,
        input.created_by
      );
    }

    return memory;
  }

  validatePromotionInput(input: PromoteProposalInput): { valid: boolean; errors: string[] } {
    const errors: string[] = [];

    const parsed = PromoteProposalInputSchema.safeParse(input);
    if (!parsed.success) {
      errors.push(...parsed.error.issues.map((issue) => issue.message));
    }

    if (this.config.require_reason && input.reason.trim().length === 0) {
      errors.push('Promotion reason is required');
    }

    if (this.config.require_attribution && input.promoted_by.trim().length === 0) {
      errors.push('Promotion attribution is required');
    }

    return {
      valid: errors.length === 0,
      errors,
    };
  }
}

function nowTimestamp(): Timestamp {
  return new Date().toISOString() as Timestamp;
}

function toTimestamp(value: string): Timestamp {
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return nowTimestamp();
  }
  return parsed.toISOString() as Timestamp;
}
