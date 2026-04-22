import type { ExecutorContext } from '@nx/devkit';
import type { BaseCargoOptions } from '../models/base-options';
/**
 * Turn a cargo subcommand + a normalised option bag into the argv cargo wants.
 *
 * Shape:
 *   [+toolchain?] <subcommand> [--key value | --flag]* -p <package> [-- <args>]
 *
 * Handles kebab-case option keys, scalar flags (`--release`), string values
 * (`--target x86_64-...`), array values — `--features` and `--bin` are joined
 * (features comma-separated, bins repeated as one string) and everything else
 * repeats — plus passthrough `args` split between `cargo <sub>` and the binary
 * under `--`.
 *
 * Kept as a pure function so it's unit-testable without touching cargo.
 */
export declare function buildCargoArgs<T extends BaseCargoOptions>(subcommand: string, options: T, context: Pick<ExecutorContext, 'projectName'>): string[];
//# sourceMappingURL=build-command.d.ts.map