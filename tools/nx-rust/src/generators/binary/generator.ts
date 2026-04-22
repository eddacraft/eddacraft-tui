import type { Tree } from '@nx/devkit';
import crateGenerator from '../crate/generator';
import type { CrateGeneratorSchema } from '../crate/schema';

export type BinaryGeneratorSchema = Omit<CrateGeneratorSchema, 'bin'>;

/**
 * Alias for `@eddacraft/nx-rust:crate --bin`. Kept as a distinct generator so it shows up
 * in `nx list` with its own description and `x-type: application` metadata.
 */
export default async function binaryGenerator(
  tree: Tree,
  options: BinaryGeneratorSchema
): Promise<void> {
  return crateGenerator(tree, { ...options, bin: true });
}
