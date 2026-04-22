import { type Tree } from '@nx/devkit';
export interface InitGeneratorSchema {
    skipFormat?: boolean;
}
/**
 * Write a Cargo workspace root if one doesn't exist yet, plus a
 * rust-toolchain.toml pinning a minimal toolchain. Safe to run multiple
 * times — it only writes missing files.
 */
export default function initGenerator(tree: Tree, options?: InitGeneratorSchema): Promise<void>;
//# sourceMappingURL=generator.d.ts.map