/**
 * Edda — Canonical Memory System
 *
 * Git-backed, versioned, human-curated memory storage for Anvil.
 * Edda stores institutional knowledge: decisions, patterns, constraints,
 * warnings, doctrines, and lessons — with full provenance tracking.
 *
 * @module @eddacraft/anvil-edda-stack/edda
 */

// Configuration
export * from './config.js';

// Store interfaces (for dependency injection)
export type {
  IMemoryStoreOperations,
  IVersionTracker,
  VersionEntry as StoreVersionEntry,
} from './store-interfaces.js';

// Serialisation
export {
  serialiseMemory,
  deserialiseMemory,
  serialiseIndex,
  deserialiseIndex,
  MemoryIndexEntrySchema,
  MemoryIndexSchema,
} from './serialisation.js';
export type { MemoryIndex, MemoryIndexEntry } from './serialisation.js';

// Storage
export { MemoryStore } from './memory-store.js';

// Version tracking
export { VersionTracker } from './version-tracker.js';
export type { VersionEntry } from './version-tracker.js';

// Services
export { PromotionService } from './promotion-service.js';
export type { PromotionServiceDeps } from './promotion-service.js';

export { ProvenanceService } from './provenance-service.js';
export type { ProvenanceServiceDeps } from './provenance-service.js';

export { EvolutionService } from './evolution-service.js';
export type { EvolutionServiceDeps } from './evolution-service.js';

export { MemoryService } from './memory-service.js';
export type { MemoryServiceDeps } from './memory-service.js';
export * from './migration/index.js';
