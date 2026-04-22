/**
 * Loose shape for a parsed `Cargo.toml`. `@ltd/j-toml` returns plain objects
 * with TOML.Section / TOML.inline markers, which we preserve by round-tripping
 * through `stringifyCargoToml`.
 */
export interface CargoToml {
  package?: {
    name?: string;
    version?: string;
    edition?: string;
    description?: string;
    license?: string;
    [key: string]: unknown;
  };
  workspace?: {
    members?: string[];
    resolver?: string;
    package?: Record<string, unknown>;
    dependencies?: Record<string, unknown>;
    [key: string]: unknown;
  };
  dependencies?: Record<string, unknown>;
  'dev-dependencies'?: Record<string, unknown>;
  'build-dependencies'?: Record<string, unknown>;
  lib?: Record<string, unknown>;
  bin?: Array<Record<string, unknown>>;
  [key: string]: unknown;
}
//# sourceMappingURL=cargo-toml.d.ts.map
