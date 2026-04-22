import type { Tree } from '@nx/devkit';
import type { CrateGeneratorSchema } from '../crate/schema';
export type LibraryGeneratorSchema = Omit<CrateGeneratorSchema, 'bin'>;
/** Alias for `nxrust:crate` (library is the default). */
export default function libraryGenerator(
  tree: Tree,
  options: LibraryGeneratorSchema
): Promise<void>;
//# sourceMappingURL=generator.d.ts.map
