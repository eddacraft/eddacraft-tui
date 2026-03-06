import { v4 as uuidv4 } from 'uuid';
import type { CandidateProposal, MemoryObject, PromoteProposalInput } from '../contracts/index.js';
import {
  MemoryObjectSchema,
  MEMORY_SCHEMA_VERSION,
  PromoteProposalInputSchema,
  createMemoryId,
  createObservationId,
  createPromotionInput,
  createSessionId,
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

    let proposal: CandidateProposal | null = null;
    if (this.deps.emberPort) {
      proposal = await this.deps.emberPort.getProposal(input.proposal_id);

      if (proposal === null) {
        throw new Error(`Proposal not found: ${input.proposal_id}`);
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

    const resolvedProposal = proposal ?? {
      id: input.proposal_id,
      type: 'pattern',
      status: 'active',
      summary: input.statement ?? input.context.why,
      rationale: input.reason,
      confidence: this.config.min_ember_confidence,
      signals: [],
      provenance: {
        observation_ids: [createObservationId(uuidv4())],
        session_ids: [createSessionId(uuidv4())],
        proposal_id: input.proposal_id,
        earliest_observation: toTimestamp(input.context.when),
        latest_observation: toTimestamp(input.context.when),
      },
      created_at: nowTimestamp(),
      expires_at: nowTimestamp(),
      ttl_days: 30,
    };

    const memory = await this.createMemoryFromProposal(input, resolvedProposal);

    if (this.deps.emberPort) {
      await this.deps.emberPort.markPromoted(input.proposal_id, memory.id, input.promoted_by);
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

  async createMemoryFromProposal(
    input: PromoteProposalInput,
    proposal: CandidateProposal
  ): Promise<MemoryObject> {
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

    await this.deps.store.saveMemory(memory);
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
