import { v4 as uuidv4 } from 'uuid';
import type { CreateMemoryInput, RetireMemoryInput } from '../contracts/ports/index.js';
import { MemoryObjectSchema, MEMORY_SCHEMA_VERSION, createMemoryId } from '../contracts/index.js';
import type { MemoryObject } from '../contracts/edda-memory.js';
import type { MemoryId } from '../contracts/identifiers.js';
import type { Timestamp } from '../contracts/temporal.js';
import type { IMemoryStoreOperations, IVersionTracker } from './store-interfaces.js';

export interface EvolutionServiceDeps {
  store: IMemoryStoreOperations;
  versionTracker?: IVersionTracker;
  /** Default attribution method. Defaults to 'cli_command'. */
  defaultMethod?: string;
}

export class EvolutionService {
  /**
   * Per-memory in-process locks serialising state transitions (CIB-118).
   * Without this, two interleaved supersedes of the same memory can both
   * observe status 'active' and both create a replacement.
   */
  private readonly transitionLocks = new Map<MemoryId, Promise<void>>();

  constructor(private readonly deps: EvolutionServiceDeps) {}

  async supersedeMemory(
    oldMemoryId: MemoryId,
    newMemoryInput: CreateMemoryInput
  ): Promise<{ old: MemoryObject; new: MemoryObject }> {
    return this.withTransitionLock(oldMemoryId, () =>
      this.supersedeMemoryLocked(oldMemoryId, newMemoryInput)
    );
  }

  private async supersedeMemoryLocked(
    oldMemoryId: MemoryId,
    newMemoryInput: CreateMemoryInput
  ): Promise<{ old: MemoryObject; new: MemoryObject }> {
    const oldMemory = await this.deps.store.getMemory(oldMemoryId);
    if (oldMemory === null) {
      throw new Error(`Memory not found: ${oldMemoryId}`);
    }

    if (oldMemory.status !== 'active') {
      throw new Error(
        `Cannot supersede memory ${oldMemoryId}: current status is '${oldMemory.status}' (must be 'active')`
      );
    }

    const newMemory = MemoryObjectSchema.parse({
      id: createMemoryId(uuidv4()),
      type: newMemoryInput.type,
      status: 'active',
      schema_version: MEMORY_SCHEMA_VERSION,
      statement: newMemoryInput.statement,
      context: newMemoryInput.context,
      metadata: newMemoryInput.metadata,
      confidence: newMemoryInput.confidence,
      confidence_rationale: newMemoryInput.confidence_rationale,
      provenance: newMemoryInput.provenance,
      attribution: {
        actor: newMemoryInput.created_by,
        timestamp: nowTimestamp(),
        method: this.deps.defaultMethod ?? 'cli_command',
        reason: newMemoryInput.reason,
      },
      evolution: {
        supersedes: [oldMemoryId],
      },
      created_at: nowTimestamp(),
    });

    const retiredOldMemory = createRetiredMemory(oldMemory, {
      status: 'superseded',
      reason: `Superseded by memory ${newMemory.id}`,
      retiredBy: newMemoryInput.created_by,
      supersededBy: newMemory.id,
    });
    const fallbackRetiredMemory = createRetiredMemory(oldMemory, {
      status: 'retired',
      reason: `Supersede by ${newMemory.id} failed; reverted to retired`,
      retiredBy: newMemoryInput.created_by,
    });

    await this.deps.store.saveMemory(retiredOldMemory);

    try {
      await this.deps.store.saveMemory(newMemory);
    } catch (error) {
      try {
        await this.deps.store.saveMemory(fallbackRetiredMemory);
      } catch (rollbackError) {
        const saveErr = error instanceof Error ? error.message : String(error);
        const rbErr =
          rollbackError instanceof Error ? rollbackError.message : String(rollbackError);
        throw new Error(
          `Failed to save replacement memory (${saveErr}) and rollback also failed (${rbErr}) — memory ${oldMemoryId} may be stuck in superseded state`,
          { cause: rollbackError }
        );
      }
      throw error;
    }

    if (this.deps.versionTracker) {
      await this.deps.versionTracker.trackChange(
        [
          `memories/${oldMemory.type}/${oldMemoryId}.yaml`,
          `memories/${newMemory.type}/${newMemory.id}.yaml`,
        ],
        `Superseded memory ${oldMemoryId} with ${newMemory.id}`,
        newMemoryInput.created_by
      );
    }

    return {
      old: retiredOldMemory,
      new: newMemory,
    };
  }

  async retireMemory(id: MemoryId, input: RetireMemoryInput): Promise<MemoryObject | null> {
    return this.withTransitionLock(id, async () => {
      const memory = await this.deps.store.getMemory(id);
      if (memory === null) {
        return null;
      }

      if (memory.status !== 'active') {
        // Terminal states are immutable (CIB-118): retiring an already
        // retired or superseded memory is a no-op so supersession links and
        // the original retirement record are never overwritten.
        return memory;
      }

      const retired = createRetiredMemory(memory, {
        status: 'retired',
        reason: input.reason,
        retiredBy: input.retired_by,
        supersededBy: input.superseded_by,
      });

      await this.deps.store.saveMemory(retired);

      if (this.deps.versionTracker) {
        await this.deps.versionTracker.trackChange(
          [`memories/${memory.type}/${id}.yaml`],
          `Retired memory ${id}`,
          input.retired_by
        );
      }

      return retired;
    });
  }

  async retireMemoryById(
    id: MemoryId,
    supersededBy: MemoryId | undefined,
    reason: string,
    retiredBy: string
  ): Promise<void> {
    await this.withTransitionLock(id, async () => {
      const memory = await this.deps.store.getMemory(id);
      if (memory === null) {
        return;
      }

      if (memory.status !== 'active') {
        // Terminal states are immutable (CIB-118) — see retireMemory.
        return;
      }

      const retired = createRetiredMemory(memory, {
        status: supersededBy ? 'superseded' : 'retired',
        reason,
        retiredBy,
        supersededBy,
      });

      await this.deps.store.saveMemory(retired);

      if (this.deps.versionTracker) {
        await this.deps.versionTracker.trackChange(
          [`memories/${memory.type}/${id}.yaml`],
          `Retired memory ${id}`,
          retiredBy
        );
      }
    });
  }

  /**
   * Serialise state transitions per memory id. Queued callers run in FIFO
   * order; a failed transition does not block the next caller.
   *
   * NOT reentrant: calling withTransitionLock for the same id from inside a
   * locked section deadlocks (the inner call waits on the outer gate).
   */
  private async withTransitionLock<T>(id: MemoryId, fn: () => Promise<T>): Promise<T> {
    const previous = this.transitionLocks.get(id) ?? Promise.resolve();

    let release!: () => void;
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    this.transitionLocks.set(id, gate);

    await previous;
    try {
      return await fn();
    } finally {
      release();
      if (this.transitionLocks.get(id) === gate) {
        this.transitionLocks.delete(id);
      }
    }
  }

  async getEvolutionChain(id: MemoryId): Promise<MemoryObject[]> {
    const start = await this.deps.store.getMemory(id);
    if (start === null) {
      return [];
    }

    const visited = new Set<MemoryId>();
    const chain: MemoryObject[] = [];
    let root = start;

    while (root.evolution.supersedes.length > 0) {
      visited.add(root.id);
      const previousId = root.evolution.supersedes[0];
      if (visited.has(previousId)) {
        break;
      }

      const previous = await this.deps.store.getMemory(previousId);
      if (previous === null) {
        break;
      }

      root = previous;
    }

    visited.clear();

    let current: MemoryObject | null = root;
    while (current !== null && !visited.has(current.id)) {
      chain.push(current);
      visited.add(current.id);

      const nextId = current.evolution.superseded_by;
      if (!nextId) {
        break;
      }

      current = await this.deps.store.getMemory(nextId);
    }

    return chain;
  }

  async getLatestVersion(id: MemoryId): Promise<MemoryObject | null> {
    let current = await this.deps.store.getMemory(id);
    if (current === null) {
      return null;
    }

    const visited = new Set<MemoryId>();
    while (current.evolution.superseded_by) {
      const nextId = current.evolution.superseded_by;
      if (visited.has(nextId)) {
        break;
      }

      const next = await this.deps.store.getMemory(nextId);
      if (next === null) {
        break;
      }

      visited.add(current.id);
      current = next;
    }

    return current;
  }
}

function nowTimestamp(): Timestamp {
  return new Date().toISOString() as Timestamp;
}

interface RetiredMemoryOptions {
  status: 'retired' | 'superseded';
  reason: string;
  retiredBy: string;
  supersededBy?: MemoryId;
}

function createRetiredMemory(memory: MemoryObject, options: RetiredMemoryOptions): MemoryObject {
  const timestamp = nowTimestamp();

  return MemoryObjectSchema.parse({
    ...memory,
    status: options.status,
    evolution: {
      ...memory.evolution,
      retired_at: timestamp,
      retired_reason: options.reason,
      retired_by: options.retiredBy,
      superseded_by: options.supersededBy,
    },
    updated_at: timestamp,
  });
}
