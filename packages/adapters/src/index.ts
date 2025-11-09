/**
 * Anvil Adapters Package
 *
 * Provides format adapters for converting between external planning formats
 * (SpecKit, BMAD, etc.) and the Anvil Plan Spec (APS).
 */

// Export base framework (types, registry, utils)
export * from './base/index.js';

// Export SpecKit adapter
export * from './speckit/index.js';

// Export BMAD adapter
export * from './bmad/index.js';

// Export Generic adapter
export * from './generic/index.js';

// Export common types for backward compatibility
export type {
  SpecContext,
  ExternalSpec,
  ConversionResult,
  ConversionError,
  ConversionWarning,
} from './common/types.js';

// Auto-register adapters when module is imported
import { registry as baseRegistry } from './base/index.js';
import { BMADFormatAdapter } from './bmad/index.js';
import { SpecKitFormatAdapter } from './speckit/index.js';
import { GenericMarkdownAdapter } from './generic/index.js';

// Register adapters in priority order
// Specific adapters (BMAD, SpecKit) should be registered first
// Generic adapter is registered last as fallback
baseRegistry.register(new BMADFormatAdapter());
baseRegistry.register(new SpecKitFormatAdapter());
baseRegistry.register(new GenericMarkdownAdapter());

// Export the registry instance as default export
export { baseRegistry as registry };
