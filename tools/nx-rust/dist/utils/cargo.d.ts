import type { CargoDependency, CargoMetadata, CargoPackage } from '../models/cargo-metadata';
/**
 * Spawn `cargo <args>` with inherited stdio and always-on colour. Returns the
 * success flag. Logs the command in dim text so failures are easy to
 * reproduce.
 *
 * Cargo rejects any flag before `+toolchain`, so if the first arg is a
 * toolchain selector we emit it ahead of `--color always`.
 */
export declare function cargoCommand(...args: string[]): Promise<{
  success: boolean;
}>;
/**
 * Run `cargo metadata --format-version=1` and parse the JSON output. Returns
 * `null` on failure — graph resolution has to be resilient to transient cargo
 * errors (e.g. during `cargo clean`).
 *
 * `cargo metadata` is the supported stable contract for consuming a Cargo
 * workspace; parsing Cargo.toml by hand loses resolved versions, path-dep
 * resolution, and external dependency source info.
 *
 * Uses `execFileSync` (no shell) so cargo arg injection is not possible.
 */
export declare function cargoMetadata(cwd?: string): CargoMetadata | null;
/**
 * True if the package/dep resolves to a registry, git, or out-of-workspace
 * path. Used to decide whether a dep becomes an internal Nx edge or an
 * external `cargo:<name>` node.
 */
export declare function isExternal(
  packageOrDep: CargoPackage | CargoDependency,
  workspaceRoot: string
): boolean;
//# sourceMappingURL=cargo.d.ts.map
