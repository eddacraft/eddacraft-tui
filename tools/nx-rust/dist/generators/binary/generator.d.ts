import type { Tree } from '@nx/devkit';
import type { CrateGeneratorSchema } from '../crate/schema';
export type BinaryGeneratorSchema = Omit<CrateGeneratorSchema, 'bin'>;
/**
 * Alias for `nxrust:crate --bin`. Kept as a distinct generator so it shows up
 * in `nx list` with its own description and `x-type: application` metadata.
 */
export default function binaryGenerator(tree: Tree, options: BinaryGeneratorSchema): Promise<void>;
//# sourceMappingURL=generator.d.ts.map
