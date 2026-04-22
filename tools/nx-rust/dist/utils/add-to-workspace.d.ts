import { type Tree } from '@nx/devkit';
/**
 * Add `projectPath` to the root `Cargo.toml`'s `[workspace.members]` array.
 *
 * If the root Cargo.toml has no `[workspace]` section, creates one. If the
 * member is already listed, logs and no-ops. Idempotent.
 */
export declare function addToCargoWorkspace(tree: Tree, projectPath: string): void;
//# sourceMappingURL=add-to-workspace.d.ts.map