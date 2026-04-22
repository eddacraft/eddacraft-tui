import { type Tree } from '@nx/devkit';
export interface BaseGeneratorInput {
  name: string;
  directory?: string;
  edition?: '2015' | '2018' | '2021' | '2024';
  tags?: string;
}
export type NormalizedGeneratorOptions<T extends BaseGeneratorInput> = Omit<T, 'edition'> & {
  projectName: string;
  projectRoot: string;
  projectDirectory: string;
  cargoName: string;
  libName: string;
  edition: '2015' | '2018' | '2021' | '2024';
  parsedTags: string[];
};
export declare function normalizeOptions<T extends BaseGeneratorInput>(
  _tree: Tree,
  options: T
): NormalizedGeneratorOptions<T>;
//# sourceMappingURL=normalize-options.d.ts.map
