# @eddacraft/nx-rust

**Nx plugin for Rust workspaces.** Wraps `cargo` as Nx executors and generators,
and parses `cargo metadata` into the Nx project graph so `nx affected` works
across your Rust crates.

Spiritual successor to [`@monodon/rust`](https://github.com/Cammisuli/monodon) —
same shape, targeting Nx 22.

> **Vendored package.** This is the in-repo copy of the upstream
> [`eddacraft/nxrust`](https://github.com/EddaCraft/nxrust) plugin. See
> [`SPLIT.md`](./SPLIT.md) for the full set of divergences and the path back to
> a standalone published package. The most important divergence: every executor
> name is `@eddacraft/nx-rust:<name>` here, not `nxrust:<name>` as in upstream
> docs. The tables below use the vendored name.

## Use

The package is wired automatically via the workspace pnpm protocol. `nx.json`
registers the plugin:

```json
{
  "plugins": ["@eddacraft/nx-rust"]
}
```

## Executors

| Executor                             | Wraps                             | Cache                                                                         |
| ------------------------------------ | --------------------------------- | ----------------------------------------------------------------------------- |
| `@eddacraft/nx-rust:build`           | `cargo build`                     | yes                                                                           |
| `@eddacraft/nx-rust:check`           | `cargo check`                     | yes                                                                           |
| `@eddacraft/nx-rust:clippy`/`lint`   | `cargo clippy`                    | yes                                                                           |
| `@eddacraft/nx-rust:fmt`             | `cargo fmt` / `cargo fmt --check` | no by default (`check: true` makes it side-effect-free → yes via `fmt-check`) |
| `@eddacraft/nx-rust:run`             | `cargo run`                       | no                                                                            |
| `@eddacraft/nx-rust:test`            | `cargo test`                      | yes                                                                           |
| `@eddacraft/nx-rust:release-publish` | `cargo publish`                   | no (use via `nx release publish`)                                             |

All executors accept a shared option set:

| Option         | Type                      | Notes                                   |
| -------------- | ------------------------- | --------------------------------------- |
| `toolchain`    | `stable`/`beta`/`nightly` | Translates to `cargo +<toolchain> …`    |
| `target`       | `string`                  | Rust target triple                      |
| `profile`      | `string`                  | `cargo` profile (e.g. `dev`, `release`) |
| `release`      | `boolean`                 | `--release`                             |
| `features`     | `string \| string[]`      | `--features`                            |
| `all-features` | `boolean`                 | `--all-features`                        |
| `target-dir`   | `string`                  | `--target-dir`                          |
| `args`         | `string \| string[]`      | Forwarded after `--`                    |

Individual executors add specialised flags — see each schema.

## Generators

```sh
# Library crate
nx g @eddacraft/nx-rust:crate my-crate

# Binary crate
nx g @eddacraft/nx-rust:crate my-cli --bin
# or alias:
nx g @eddacraft/nx-rust:binary my-cli

# Library alias
nx g @eddacraft/nx-rust:library my-lib
```

Generated crates are added to the root `Cargo.toml` `[workspace.members]`
(comments preserved via `@ltd/j-toml`) and get a minimal `project.json`
pre-wired to the plugin's executors.

## Project graph

The plugin runs `cargo metadata --format-version=1` and emits:

- **Nx project nodes** for every workspace member, located at the crate
  directory. The **Nx project name must match the Cargo package name** for
  workspace dependency resolution to work correctly.
- **External nodes** (`cargo:<name>`) for every registry / git dependency.
- **Dependency edges** for every direct dependency resolved via metadata,
  matching workspace crates by Cargo package name.

This is what makes `nx affected -t test` correct across your Rust crates.

## Requirements

- Node.js ≥ 20
- Nx ≥ 22
- Cargo on `PATH`
- A Cargo workspace at the Nx workspace root (or a single crate at root)

## License

PROPRIETARY in this vendor copy (matches `tools/generators/`). Upstream
[`eddacraft/nxrust`](https://github.com/EddaCraft/nxrust) is Apache-2.0.

This project does not contain any code copied from `@monodon/rust` — it
references its public API shape only. `cargo metadata` is the official Rust
tooling contract.
