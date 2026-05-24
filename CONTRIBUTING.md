# Contributing to eddacraft-tui

> **`eddacraft-tui` is mirrored from the Anvil monorepo.** The canonical source
> for this crate lives at
> [`anvil-001:crates/eddacraft-tui/`](https://github.com/eddacraft/anvil-001/tree/main/crates/eddacraft-tui)
> (private); the public read-only mirror is at
> [`eddacraft/eddacraft-tui`](https://github.com/eddacraft/eddacraft-tui). Two
> contribution paths — pick the section below that matches where you can write
> code.
>
> Governance summary:
> [`docs/policies/eddacraft-tui-mirror.md`](https://github.com/eddacraft/anvil-001/blob/main/docs/policies/eddacraft-tui-mirror.md)
> (CI gate split, backport / conflict policy, topology).

Thanks for your interest in improving the eddacraft Ratatui component library.

## If you are contributing from the public mirror (`eddacraft/eddacraft-tui`)

- **Bugs and feature requests** — please open an issue at
  <https://github.com/eddacraft/eddacraft-tui/issues>. Include a minimal
  reproduction (for bugs) or a clear use case (for features), the crate version,
  `rustc --version`, and any relevant terminal emulator / OS context for
  rendering bugs.
- **Source pull requests** opened against the mirror will be **auto-closed with
  a redirect** by the [`pr-redirect.yml`](.github/workflows/pr-redirect.yml)
  workflow (per D-TUIR-009). The mirror's `main` is force-pushed by automation,
  and any local commits would be overwritten on the next sync — auto-closing
  protects your work from quiet loss.
- **Security issues** — see [`SECURITY.md`](SECURITY.md). The GitHub Security
  Advisory channel and the security email both route to maintainers; do not open
  public issues.
- **Accepted external changes** — if a maintainer accepts a change you proposed
  on a closed PR, they will port it into the canonical Anvil source on your
  behalf. The next mirror sync carries it out; credit goes to the maintainer
  commit with a `Co-Authored-By:` trailer honouring you.

## If you have access to the Anvil monorepo (`eddacraft/anvil-001`)

If you can read or clone `eddacraft/anvil-001`, work happens there. The
eddacraft-tui crate lives at
[`crates/eddacraft-tui/`](https://github.com/eddacraft/anvil-001/tree/main/crates/eddacraft-tui).

### Getting started

```sh
git clone https://github.com/eddacraft/anvil-001.git
cd anvil-001
cargo build -p eddacraft-tui --all-features
```

The full Anvil workspace builds with `cargo build` (no `-p`) — but you do not
need the full workspace to develop the crate.

### Pre-PR local checks

The Anvil-side gate contracts are documented in full at
[`docs/policies/eddacraft-tui-mirror.md`](https://github.com/eddacraft/anvil-001/blob/main/docs/policies/eddacraft-tui-mirror.md).
The minimum local pass before pushing:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p eddacraft-tui --all-features
cargo check -p eddacraft-tui --all-features --release
```

If any command fails, fix the issue before pushing — CI runs the same gates.

### Branching

Anvil-001 uses `main` as the single permanent branch. Feature work branches off
`main` with the standard prefixes (`feat/<topic>`, `fix/<topic>`,
`docs/<topic>`, `chore/<topic>`); PRs target `main`. There is no `dev` branch
(retired by OPMODEL-012 on 2026-05-11).

### Commit messages

Conventional Commits:

```
<type>(<optional scope>): <subject>
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`,
`ci`.

Rules:

- Subject: imperative mood, lowercase, no trailing period, ≤ 50 chars.
- Body (when needed): wrap at 72 chars, explain _why_ not _what_.
- Footer: `Fixes #N`, `Relates to #N`.
- One logical change per commit.

### Pull requests

1. **Keep PRs focused.** One feature or fix per PR. Split mechanical refactors
   from behavioural changes when feasible.
2. **Fill in a short summary** of what changed and why. Include a test plan —
   what you verified locally.
3. **CI must be green.** Authoritative Anvil-side gates are listed in
   [`docs/policies/eddacraft-tui-mirror.md`](https://github.com/eddacraft/anvil-001/blob/main/docs/policies/eddacraft-tui-mirror.md);
   mirror-side gates run on every mirror push after this PR lands.
4. **Resolve every review thread.** Either apply the change or reply with the
   reasoning; do not leave threads unanswered.
5. **No force-pushing after review starts** unless you are rebasing onto `main`
   and you flag it on the PR.

### Snapshot tests

Some widgets use [`insta`](https://insta.rs/) snapshot tests. When a rendered
change is intentional:

```sh
INSTA_UPDATE=always cargo test -p eddacraft-tui --all-features
```

Commit the updated snapshots alongside the source change. If the version string
appears in a rendered snapshot (e.g. shell chrome), bumping the version requires
re-running this command — see [RELEASE.md](RELEASE.md) for the full release
flow.

### Coding standards

- **Follow `rustfmt`.** CI fails on unformatted code.
- **Workspace clippy `-D warnings` is the gate.** The crate carries its own
  `[lints]` block (see `Cargo.toml`); pedantic warnings are treated as
  informational _within the standalone crate's own CI posture_ but become errors
  under Anvil's workspace clippy gate (D-TUIR-019). Resolve them; do not commit
  code that fails the workspace gate.
- **No `unsafe` code.** The crate forbids it (`unsafe_code = "forbid"`).
- **Public API stability.** For user-facing types, prefer `#[non_exhaustive]`
  over adding private fields, and use a `Default` impl plus public field
  mutation as the documented construction pattern.
- **Write tests.** New behaviour needs unit tests; bug fixes need regression
  tests.
- **Comments explain _why_, not _what_.** Well-named code documents itself.

### Releases

Releases are cut by maintainers from `main` via tag pushes
(`eddacraft-tui-vX.Y.Z`); the publish workflow (TUIR-005) fires from canonical
source. The full runbook lives in [RELEASE.md](RELEASE.md). Contributors do not
need to bump versions in feature PRs.

## Security

See [SECURITY.md](SECURITY.md) — the private disclosure process routes through
GitHub Security Advisories on the mirror or the security email. Both reach the
maintainer team.

## Licence

By contributing, you agree that your contributions will be licensed under the
[Apache-2.0 License](LICENSE).
