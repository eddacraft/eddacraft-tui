/**
 * Shared option shape for every cargo-wrapping executor.
 *
 * Mirrors the flags every `cargo <subcommand>` accepts so a single
 * `buildCargoArgs` helper can serve every executor. Subcommand-specific
 * options live alongside this interface in each executor's `schema.ts`.
 */

export interface BaseCargoOptions {
  /** Rust toolchain (e.g. stable, beta, nightly) — becomes `cargo +<toolchain>`. */
  toolchain?: string;

  /** Target triple — `--target <triple>`. */
  target?: string;

  /** Named cargo profile — `--profile <name>`. */
  profile?: string;

  /** Shortcut for `--release`. Ignored when `profile` is set. */
  release?: boolean;

  /** `--target-dir <path>`. */
  'target-dir'?: string;

  /**
   * `--features` list. An array is joined with commas into a single flag;
   * a string is passed through verbatim (cargo accepts comma or space
   * separators in one `--features` arg).
   */
  features?: string | string[];

  /** `--all-features`. */
  'all-features'?: boolean;

  /** `--no-default-features`. */
  'no-default-features'?: boolean;

  /** `--locked` — enforce Cargo.lock is up-to-date. */
  locked?: boolean;

  /** `--frozen` — error if Cargo.lock is missing or out-of-date. */
  frozen?: boolean;

  /** `--offline` — run without network access. */
  offline?: boolean;

  /**
   * Override the package name cargo operates on. Defaults to the Nx project
   * name. Use this if the crate's cargo name differs from its Nx project name.
   */
  package?: string;

  /** Raw passthrough after `--`. String is passed as-is; array is spread. */
  args?: string | string[];
}
