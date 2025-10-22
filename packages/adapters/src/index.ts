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
// TODO: SpecKit adapters need to be migrated to FormatAdapter interface
// import { SpecKitImportAdapter, SpecKitExportAdapter } from './speckit/index.js';

// Register SpecKit adapters
// TODO: Implement FormatAdapter interface in SpecKit adapters before auto-registration
// Currently SpecKit adapters use BaseAdapter interface. Need to:
// 1. Add metadata property with FormatMetadata type
// 2. Implement detect() method for format auto-detection
// 3. Implement parse() and serialize() methods for content handling
// 4. Implement validate() method that returns FormatValidationResult
// See: packages/adapters/src/base/types.ts for FormatAdapter interface
// baseRegistry.register(new SpecKitImportAdapter());
// baseRegistry.register(new SpecKitExportAdapter());

// Export the registry instance as default export
export { baseRegistry as registry };
