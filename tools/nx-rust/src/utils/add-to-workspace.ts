import TOML from '@ltd/j-toml';
import { logger, type Tree } from '@nx/devkit';
import { parseCargoToml, stringifyCargoToml } from './toml';

/**
 * Add `projectPath` to the root `Cargo.toml`'s `[workspace.members]` array.
 *
 * If the root Cargo.toml has no `[workspace]` section, creates one. If the
 * member is already listed, logs and no-ops. Idempotent.
 */
export function addToCargoWorkspace(tree: Tree, projectPath: string): void {
  const rootPath = 'Cargo.toml';
  const existing = tree.read(rootPath)?.toString();

  const cleanPath = projectPath.replace(/^\.\//, '');

  if (!existing) {
    // Bootstrap a minimal workspace root if the consumer didn't run `init` yet.
    // `TOML.Section(...)` marks the object so j-toml emits `[workspace]`
    // rather than the dotted `workspace.resolver = ...` form.
    tree.write(
      rootPath,
      stringifyCargoToml({
        workspace: TOML.Section({ resolver: '2', members: [cleanPath] }),
      } as never)
    );
    return;
  }

  const toml = parseCargoToml(existing);
  toml.workspace ??= { members: [] };

  const members = (toml.workspace.members ??= []) as string[];
  if (isAlreadyMember(members, cleanPath)) {
    logger.info(`${cleanPath} is already a workspace member`);
    return;
  }

  toml.workspace.members = [...members, cleanPath];
  tree.write(rootPath, stringifyCargoToml(toml));
}

/**
 * True if `cleanPath` is covered by any entry in `members`, respecting
 * simple `*` globs (e.g. `crates/*` matches `crates/foo`). We only need
 * single-segment matching — `**` in workspace members is unusual enough to
 * fall back to the literal add path.
 */
function isAlreadyMember(members: string[], cleanPath: string): boolean {
  for (const entry of members) {
    if (entry === cleanPath) return true;
    if (!entry.includes('*')) continue;
    // Translate a cargo-style glob to a regex: escape regex metacharacters,
    // then turn `*` into `[^/]+`.
    const pattern = '^' + entry.replace(/[.+^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '[^/]+') + '$';
    if (new RegExp(pattern).test(cleanPath)) return true;
  }
  return false;
}
