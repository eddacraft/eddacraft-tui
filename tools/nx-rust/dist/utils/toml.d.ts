import type { Tree } from '@nx/devkit';
import type { CargoToml } from '../models/cargo-toml';
/**
 * Parse a Cargo.toml string, preserving comments so generators don't mangle
 * hand-authored manifests on round-trip.
 */
export declare function parseCargoToml(source: string): CargoToml;
/**
 * Read a project's Cargo.toml from the Nx Tree. Throws if the file is missing
 * — generators that call this always need it.
 */
export declare function readCargoTomlFromTree(tree: Tree, relativePath: string): CargoToml;
/**
 * Serialise a parsed Cargo.toml back to a string. `newlineAround: 'section'`
 * preserves the blank-line convention used by `cargo new`.
 */
export declare function stringifyCargoToml(toml: CargoToml): string;
//# sourceMappingURL=toml.d.ts.map