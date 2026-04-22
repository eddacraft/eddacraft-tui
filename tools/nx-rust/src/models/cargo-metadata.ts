/**
 * TypeScript types for `cargo metadata --format-version=1` output.
 *
 * See: https://doc.rust-lang.org/cargo/commands/cargo-metadata.html
 *
 * Only the fields nxrust actually reads are typed precisely — everything else
 * is left loose so forward-compatible cargo versions don't break parsing.
 */

export interface CargoMetadata {
  packages: CargoPackage[];
  workspace_members: string[];
  workspace_default_members?: string[];
  resolve: CargoResolve | null;
  target_directory: string;
  workspace_root: string;
  version: number;
  metadata: unknown;
}

export interface CargoPackage {
  name: string;
  version: string;
  id: string;
  license?: string | null;
  license_file?: string | null;
  description?: string | null;
  source?: string | null;
  dependencies: CargoDependency[];
  targets: CargoTarget[];
  features: Record<string, string[]>;
  manifest_path: string;
  publish?: string[] | null;
  authors?: string[];
  categories?: string[];
  keywords?: string[];
  readme?: string | null;
  repository?: string | null;
  homepage?: string | null;
  documentation?: string | null;
  edition?: string;
  links?: string | null;
  rust_version?: string | null;
  metadata?: unknown;
}

export interface CargoDependency {
  name: string;
  source?: string | null;
  req: string;
  kind?: 'dev' | 'build' | null;
  rename?: string | null;
  optional: boolean;
  uses_default_features: boolean;
  features: string[];
  target?: string | null;
  path?: string | null;
  registry?: string | null;
}

export interface CargoTarget {
  kind: string[];
  crate_types: string[];
  name: string;
  src_path: string;
  edition?: string;
  required_features?: string[];
  doc?: boolean;
  doctest?: boolean;
  test?: boolean;
}

export interface CargoResolve {
  nodes: CargoResolveNode[];
  root: string | null;
}

export interface CargoResolveNode {
  id: string;
  dependencies: string[];
  deps: CargoResolveDep[];
  features: string[];
}

export interface CargoResolveDep {
  name: string;
  pkg: string;
  dep_kinds: CargoResolveDepKind[];
}

export interface CargoResolveDepKind {
  kind: 'dev' | 'build' | null;
  target: string | null;
}
