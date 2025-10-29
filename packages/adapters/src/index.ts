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

// Register adapters
baseRegistry.register(new BMADFormatAdapter());
baseRegistry.register(new SpecKitFormatAdapter());

// Export the registry instance as default export
export { baseRegistry as registry };
