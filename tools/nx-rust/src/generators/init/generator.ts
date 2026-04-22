import { formatFiles, logger, type Tree } from '@nx/devkit';

export interface InitGeneratorSchema {
  skipFormat?: boolean;
}

const DEFAULT_TOOLCHAIN = `[toolchain]
channel = "stable"
profile = "minimal"
components = ["rustfmt", "clippy"]
`;

const DEFAULT_WORKSPACE_CARGO_TOML = `[workspace]
resolver = "2"
members = []

[workspace.package]
edition = "2021"

[profile.release]
lto = true
codegen-units = 1
`;

/**
 * Write a Cargo workspace root if one doesn't exist yet, plus a
 * rust-toolchain.toml pinning a minimal toolchain. Safe to run multiple
 * times — it only writes missing files.
 */
export default async function initGenerator(
  tree: Tree,
  options: InitGeneratorSchema = {}
): Promise<void> {
  if (!tree.exists('Cargo.toml')) {
    tree.write('Cargo.toml', DEFAULT_WORKSPACE_CARGO_TOML);
    logger.info('Created Cargo.toml workspace root.');
  }

  if (!tree.exists('rust-toolchain.toml')) {
    tree.write('rust-toolchain.toml', DEFAULT_TOOLCHAIN);
    logger.info('Created rust-toolchain.toml.');
  }

  if (!options.skipFormat) {
    await formatFiles(tree);
  }
}
