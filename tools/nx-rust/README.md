# nxrust

**Nx plugin for Rust workspaces.** Wraps `cargo` as Nx executors and generators,
and parses `cargo metadata` into the Nx project graph so `nx affected` works
across your Rust crates.

Spiritual successor to [`@monodon/rust`](https://github.com/Cammisuli/monodon) —
same shape, explicitly Apache-2.0 licensed, targeting Nx 22.

## Install

```sh
pnpm add -D nxrust
# or: npm i -D nxrust  / yarn add -D nxrust
```

Register in `nx.json`:

```json
{
  "plugins": ["nxrust"]
}
```

## Executors

| Executor                 | Wraps           | Cache                             |
| ------------------------ | --------------- | --------------------------------- |
| `nxrust:build`           | `cargo build`   | yes                               |
| `nxrust:check`           | `cargo check`   | yes                               |
| `nxrust:clippy` / `lint` | `cargo clippy`  | yes                               |
| `nxrust:fmt`             | `cargo fmt`     | yes                               |
| `nxrust:run`             | `cargo run`     | no                                |
| `nxrust:test`            | `cargo test`    | yes                               |
| `nxrust:release-publish` | `cargo publish` | no (use via `nx release publish`) |

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
nx g nxrust:crate my-crate

# Binary crate
nx g nxrust:crate my-cli --bin
# or alias:
nx g nxrust:binary my-cli

# Library alias
nx g nxrust:library my-lib
```

Generated crates are added to the root `Cargo.toml` `[workspace.members]`
(comments preserved via `@ltd/j-toml`) and get a minimal `project.json`
pre-wired to the plugin's executors.

## Project graph

The plugin runs `cargo metadata --format-version=1` and emits:

- **Nx project nodes** for every workspace member (keyed by its directory).
- **External nodes** (`cargo:<name>`) for every registry / git dependency.
- **Dependency edges** for every direct dependency resolved via metadata.

This is what makes `nx affected -t test` correct across your Rust crates.

## Requirements

- Node.js ≥ 20
- Nx ≥ 22
- Cargo on `PATH`
- A Cargo workspace at the Nx workspace root (or a single crate at root)

## License

Apache-2.0 © EddaCraft. See [LICENSE](./LICENSE).

This project does not contain any code copied from `@monodon/rust` — it
references its public API shape only. `cargo metadata` is the official Rust
tooling contract.
