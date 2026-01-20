/**
 * Testing Fixtures
 *
 * Factory functions for creating valid test data.
 *
 * @module @eddacraft/anvil-edda-stack/testing/fixtures
 */

// Proposal Fixtures
export {
  // Generic factory
  createProposalFixture,
  // Type-specific factories
  createValidDecisionProposal,
  createValidPatternProposal,
  createValidWarningProposal,
  createValidLessonProposal,
  createValidAnomalyProposal,
  createValidConstraintProposal,
  // Status variant factories
  createActiveProposal,
  createExpiredProposal,
  createPromotedProposal,
  createDismissedProposal,
  // Batch factories
  createProposalsOfAllTypes,
  createProposalsOfAllStatuses,
  // Types
  type ProposalFixtureOverrides,
} from './proposals.js';

// Memory Fixtures
export {
  // Generic factory
  createMemoryFixture,
  // Type-specific factories
  createValidDecisionMemory,
  createValidPatternMemory,
  createValidConstraintMemory,
  createValidWarningMemory,
  createValidDoctrineMemory,
  createValidLessonMemory,
  // Status variant factories
  createActiveMemory,
  createSupersededMemory,
  createRetiredMemory,
  // Evolution chain factories
  createSupersedesMemory,
  createEvolutionChain,
  createMultiLevelEvolutionChain,
  // Batch factories
  createMemoriesOfAllTypes,
  createMemoriesOfAllStatuses,
  // Types
  type MemoryFixtureOverrides,
} from './memories.js';
