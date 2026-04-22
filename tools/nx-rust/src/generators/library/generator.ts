import type { Tree } from '@nx/devkit';
import crateGenerator from '../crate/generator';
import type { CrateGeneratorSchema } from '../crate/schema';

export type LibraryGeneratorSchema = Omit<CrateGeneratorSchema, 'bin'>;

/** Alias for `@eddacraft/nx-rust:crate` (library is the default). */
export default async function libraryGenerator(
  tree: Tree,
  options: LibraryGeneratorSchema
): Promise<void> {
  return crateGenerator(tree, { ...options, bin: false });
}
