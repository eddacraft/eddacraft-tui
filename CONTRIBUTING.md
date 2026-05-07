# Contributing to eddacraft-tui

Thanks for your interest in improving the eddacraft Ratatui component
library. This guide covers the development workflow, coding standards,
and what to expect from the review process.

## Getting Started

### Prerequisites

- Rust (stable toolchain; the project builds on the latest stable)
- `rustfmt` and `clippy` components: `rustup component add rustfmt clippy`

### Clone and build

```sh
git clone https://github.com/eddacraft/eddacraft-tui.git
cd eddacraft-tui
cargo build --all-features
```

### Run the full local check

Before opening a PR, run the same checks CI runs:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features
cargo test --all-features
cargo publish --dry-run --all-features
```

(CI matches the `--all-features` `publish --dry-run` row, since the
crate ships the `image` and `big-text` features and only the
all-features tarball exercises every gate.)

If any command fails, fix the issue before pushing.

## Branching Model

- **`dev`** — default working branch. All feature branches and fixes
  target `dev`.
- **`main`** — published branch. Only receives merged PRs from `dev`
  via release PRs. Do not push feature work here.
- **Feature branches** — short-lived, branched from `dev`. Naming:
  `feat/<topic>`, `fix/<topic>`, `docs/<topic>`, `refactor/<topic>`,
  `chore/<topic>`.

## Commit Messages

This project uses [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<optional scope>): <subject>
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`,
`chore`, `ci`.

Rules:

- Subject: imperative mood, lowercase, no trailing period, ≤ 50 chars.
- Body (when needed): wrap at 72 chars, explain _why_ not _what_.
- Footer: `Fixes #N`, `Relates to #N`.
- One logical change per commit — prefer small, atomic commits.

Examples:

```
feat(spinner): add anvil preset
fix(progress_bar): clamp display_fraction to [0, 1]
docs: clarify shell branding options in README
```

## Pull Requests

1. **Branch from `dev`** and target `dev` when opening the PR.
2. **Keep PRs focused.** One feature or fix per PR. Split mechanical
   refactors from behavioural changes when feasible.
3. **Fill in a short summary** of what changed and why. Include a
   test plan — what you verified locally.
4. **CI must be green.** `main` requires `Check (default)`,
   `Check (all-features)`, `Check (no-default-features)`,
   `MSRV (1.88.0)`, `Supply chain (audit + deny)`, and `CodeQL`.
   `dev` runs the same matrix without enforcing required contexts.
5. **Resolve every review thread.** Either apply the change or reply
   with the reasoning; do not leave threads unanswered.
6. **No force-pushing after review starts** unless you are rebasing
   on `dev` and you tell the reviewer.

### Snapshot tests

Some widgets use [`insta`](https://insta.rs/) snapshot tests. When a
rendered change is intentional:

```sh
INSTA_UPDATE=always cargo test
```

Commit the updated snapshots alongside the source change. If the
version string appears in a rendered snapshot (e.g. shell chrome),
bumping the version requires re-running this command — see
[RELEASE.md](RELEASE.md) for the full release flow.

## Coding Standards

- **Follow `rustfmt`.** CI fails on unformatted code.
- **Zero clippy warnings at deny-level.** Pedantic warnings are
  treated as informational per `Cargo.toml`; resolve them when
  practical but they do not block CI.
- **No `unsafe` code.** The crate forbids it (`unsafe_code = "forbid"`).
- **Public API stability.** For user-facing types, prefer
  `#[non_exhaustive]` over adding private fields, and use a `Default`
  impl plus public field mutation as the documented construction
  pattern (see `CheckProgress`, `ParallelProgressState`).
- **Write tests.** New behaviour needs unit tests; bug fixes need
  regression tests.
- **Comments explain _why_, not _what_.** Well-named code documents
  itself; reserve comments for non-obvious constraints.

## Reporting Bugs and Requesting Features

Open an issue at
<https://github.com/eddacraft/eddacraft-tui/issues> with:

- A minimal reproduction (for bugs) or a clear use case (for features)
- The crate version and `rustc --version`
- Any relevant terminal emulator / OS context for rendering bugs

## Security

If you discover a security vulnerability, **do not open a public
issue**. See [SECURITY.md](SECURITY.md) for the private disclosure
process.

## Releases

Releases are cut by maintainers from `main`. The full runbook lives
in [RELEASE.md](RELEASE.md). Contributors do not need to bump
versions in feature PRs.

## Licence

By contributing, you agree that your contributions will be licensed
under the [Apache-2.0 License](LICENSE).
