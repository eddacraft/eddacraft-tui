import { logger, type Tree } from '@nx/devkit';
import { parseCargoToml, stringifyCargoToml } from '../../utils/toml';

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
  projects: Array<{ name: string; data: { root: string } }> | string[];
  projectGraph?: {
    nodes?: Record<string, { name: string; data?: { root?: string } }>;
  };
  specifier?: string;
  specifierSource?: string;
  currentVersionResolver?: string;
  firstRelease?: boolean;
  preid?: string;
  [key: string]: unknown;
}

export async function releaseVersionGenerator(
  tree: Tree,
  options: ReleaseVersionGeneratorSchema
): Promise<{
  data: Record<string, { currentVersion: string | null; newVersion: string | null }>;
  callback: () => Promise<void>;
}> {
  const data: Record<string, { currentVersion: string | null; newVersion: string | null }> = {};

  const projectList = normaliseProjects(options.projects, options.projectGraph);

  for (const project of projectList) {
    const cargoPath = `${project.root}/Cargo.toml`;
    const raw = tree.read(cargoPath)?.toString();
    if (!raw) {
      logger.warn(`${project.name}: no Cargo.toml at ${cargoPath} — skipping.`);
      data[project.name] = { currentVersion: null, newVersion: null };
      continue;
    }

    const toml = parseCargoToml(raw);
    const rawVersion = toml.package?.version;

    // `version.workspace = true` parses as an object (not a string). Bumping
    // the member's manifest in-place would silently do the wrong thing and
    // make `nx release` report success with zero effect.
    if (rawVersion && typeof rawVersion !== 'string') {
      throw new Error(
        `${project.name}: Cargo.toml inherits its version from [workspace.package] ` +
          '(`version.workspace = true`). Run `nx release version` on the workspace root ' +
          'crate, or drop the inheritance to bump member versions directly.'
      );
    }

    const currentVersion = (rawVersion ?? null) as string | null;
    const newVersion = resolveNewVersion(currentVersion, options.specifier);

    // Only rewrite Cargo.toml when the version actually changes. Otherwise
    // re-serialising the TOML produces a noisy diff with no semantic effect
    // (e.g. when `specifier` is undefined or an unknown bump keyword falls
    // back to `currentVersion`).
    if (newVersion && newVersion !== currentVersion && toml.package) {
      toml.package.version = newVersion;
      tree.write(cargoPath, stringifyCargoToml(toml));
    }

    data[project.name] = { currentVersion, newVersion };
  }

  return {
    data,
    callback: async () => {
      // Side effects like git staging are handled by `nx release` itself.
    },
  };
}

export default releaseVersionGenerator;

function normaliseProjects(
  input: ReleaseVersionGeneratorSchema['projects'],
  projectGraph: ReleaseVersionGeneratorSchema['projectGraph']
): Array<{ name: string; root: string }> {
  if (!Array.isArray(input)) return [];
  return input
    .map((p) => {
      if (typeof p !== 'string') {
        return { name: p.name, root: p.data?.root ?? '' };
      }
      // `nx release version` sometimes passes bare project names. Look the
      // root up from the project graph rather than silently dropping them.
      const node = projectGraph?.nodes?.[p];
      const root = node?.data?.root ?? '';
      if (!root) {
        throw new Error(
          `releaseVersionGenerator: cannot resolve project root for "${p}". ` +
            'Pass `projectGraph` alongside `projects`, or use the array-of-objects form.'
        );
      }
      return { name: p, root };
    })
    .filter((p) => p.root);
}

function resolveNewVersion(current: string | null, specifier: string | undefined): string | null {
  if (!specifier) return current;
  if (/^\d+\.\d+\.\d+(?:[-+].+)?$/.test(specifier)) return specifier;

  if (!current) return null;
  const [baseStr, preRest] = current.split('-', 2);
  const parts = baseStr.split('.').map((n) => parseInt(n, 10));
  const [major = 0, minor = 0, patch = 0] = parts;

  switch (specifier) {
    case 'major':
      return `${major + 1}.0.0`;
    case 'minor':
      return `${major}.${minor + 1}.0`;
    case 'patch':
      return `${major}.${minor}.${patch + 1}`;
    default:
      void preRest;
      return current;
  }
}
