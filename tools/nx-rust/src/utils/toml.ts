import TOML from '@ltd/j-toml';
import type { Tree } from '@nx/devkit';
import type { CargoToml } from '../models/cargo-toml';

/**
 * Parse a Cargo.toml string, preserving comments so generators don't mangle
 * hand-authored manifests on round-trip.
 */
export function parseCargoToml(source: string): CargoToml {
  return TOML.parse(source, { x: { comment: true } }) as unknown as CargoToml;
}

/**
 * Read a project's Cargo.toml from the Nx Tree. Throws if the file is missing
 * — generators that call this always need it.
 */
export function readCargoTomlFromTree(tree: Tree, relativePath: string): CargoToml {
  const raw = tree.read(relativePath)?.toString();
  if (!raw) {
    throw new Error(`Cannot find Cargo.toml at ${relativePath}`);
  }
  return parseCargoToml(raw);
}

/**
 * Serialise a parsed Cargo.toml back to a string. `newlineAround: 'section'`
 * preserves the blank-line convention used by `cargo new`.
 */
export function stringifyCargoToml(toml: CargoToml): string {
  // @ltd/j-toml.stringify accepts any object shape at runtime; the exported
  // .Table type is a symbol-branded marker we don't need here.
  const result = TOML.stringify(toml as unknown as Parameters<typeof TOML.stringify>[0], {
    newlineAround: 'section',
  });
  return Array.isArray(result) ? result.join('\n') : result;
}
