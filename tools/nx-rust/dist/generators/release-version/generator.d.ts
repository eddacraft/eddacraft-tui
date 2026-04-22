import { type Tree } from '@nx/devkit';
/**
 * Minimal `nx release version` implementation for Rust crates. Reads the
 * current version from each project's Cargo.toml and applies the requested
 * specifier (a full version string or a semver bump keyword) to the file.
 *
 * Semver-bump resolution is intentionally simple — `nx release` already does
 * the spec parsing upstream and passes us a resolved version in most flows.
 * We only fall back to bump keywords for local-only releases.
 */
export interface ReleaseVersionGeneratorSchema {
  projects:
    | Array<{
        name: string;
        data: {
          root: string;
        };
      }>
    | string[];
  projectGraph?: {
    nodes?: Record<
      string,
      {
        name: string;
        data?: {
          root?: string;
        };
      }
    >;
  };
  specifier?: string;
  specifierSource?: string;
  currentVersionResolver?: string;
  firstRelease?: boolean;
  preid?: string;
  [key: string]: unknown;
}
export declare function releaseVersionGenerator(
  tree: Tree,
  options: ReleaseVersionGeneratorSchema
): Promise<{
  data: Record<
    string,
    {
      currentVersion: string | null;
      newVersion: string | null;
    }
  >;
  callback: () => Promise<void>;
}>;
export default releaseVersionGenerator;
//# sourceMappingURL=generator.d.ts.map
